<script lang="ts">
  import { onMount } from 'svelte';
  import { Play, Shuffle, Clock3, ArrowLeft } from '@lucide/svelte';
  import { page } from '$app/state';
  import { goto } from '$app/navigation';

  function goBack() {
    if (window.history.length > 1) {
      window.history.back();
    } else {
      goto('/');
    }
  }

  import {
    fetchAlbumDetail,
    type Album,
    type Song
  } from '$lib/api';
  import { focusTrack, playQueue, playingFrom, toggleShuffle, shuffleEnabled, smartShuffleMode } from '$lib/stores/player';
  import SongContextMenu from '$lib/components/SongContextMenu.svelte';
  import {
    DropdownMenu,
    DropdownMenuTrigger,
    DropdownMenuContent,
    DropdownMenuItem,
    DropdownMenuSeparator,
  } from '$lib/components/ui/dropdown-menu';
  import { Sparkles } from '@lucide/svelte';

  let { data } = $props<{ data: { id: string } }>();

  let loading = $state(true);
  let error = $state('');
  let album = $state<(Album & { genre?: string }) | null>(null);
  let songs = $state<Song[]>([])

  onMount(async () => {
    loading = true;
    error = '';
    try {
      const detail = await fetchAlbumDetail(data.id);
      album = detail.album;
      songs = detail.songs;
    } catch (err) {
      error = err instanceof Error ? err.message : 'Failed to load album.';
    } finally {
      loading = false;
    }
  });

  function playFrom(index: number) {
    if (!songs.length) return;
    const song = songs[index];
    focusTrack.set({
      title: song.title,
      artist: song.artist,
      imageUrl: song.coverArtUrl,
      source: 'library',
      album: song.album
    });
    playQueue(songs, index);
    if (album) playingFrom.set({ type: 'album', name: album.name, href: `/album/${encodeURIComponent(data.id)}` });
  }

  function playAll() {
    playFrom(0);
  }

  function activateShuffle() { smartShuffleMode.set(false); shuffleEnabled.set(true); }
  function activateSmartShuffle() { shuffleEnabled.set(true); smartShuffleMode.set(true); }
  function deactivateShuffle() { shuffleEnabled.set(false); smartShuffleMode.set(false); }

  function fmt(seconds: number): string {
    if (!isFinite(seconds) || !seconds) return '';
    const m = Math.floor(seconds / 60);
    const s = seconds % 60;
    return `${m}:${String(s).padStart(2, '0')}`;
  }

  function fmtDuration(totalSeconds: number): string {
    if (!isFinite(totalSeconds) || !totalSeconds) return '';
    const h = Math.floor(totalSeconds / 3600);
    const m = Math.floor((totalSeconds % 3600) / 60);
    if (h > 0) return `${h} hr ${m} min`;
    return `${m} min`;
  }

  function initials(name: string): string {
    return name.split(' ').filter(Boolean).slice(0, 2).map((p) => p[0]?.toUpperCase() ?? '').join('');
  }
</script>

<div class="w-full">
  <!-- Back button -->
  <button
    class="mb-4 flex items-center gap-1.5 text-sm text-muted-foreground transition hover:text-foreground"
    onclick={goBack}
    aria-label="Go back"
  >
    <ArrowLeft class="size-4" />
    Back
  </button>

  <!-- Hero -->
  <div class="mb-8 flex flex-col gap-6 sm:flex-row sm:items-end">
    {#if loading}
      <div class="aspect-square w-48 shrink-0 animate-pulse rounded-lg bg-muted shadow-2xl"></div>
    {:else if album?.coverArtUrl}
      <img
        class="aspect-square w-48 shrink-0 rounded-lg object-cover shadow-2xl"
        src={album.coverArtUrl}
        alt={album?.name ?? ''}
      />
    {:else}
      <div class="grid aspect-square w-48 shrink-0 place-items-center rounded-lg bg-gradient-to-br from-slate-600 to-slate-800 text-4xl font-black shadow-2xl">
        {album ? initials(album.name) : ''}
      </div>
    {/if}

    <div class="min-w-0 flex-1">
      {#if loading}
        <div class="mb-2 h-4 w-16 animate-pulse rounded bg-muted"></div>
        <div class="mb-3 h-10 w-64 animate-pulse rounded bg-muted"></div>
        <div class="h-4 w-48 animate-pulse rounded bg-muted"></div>
      {:else if album}
        <p class="mb-1 text-xs font-semibold uppercase tracking-widest text-muted-foreground">Album</p>
        <h1 class="mb-1 text-4xl font-black tracking-tight sm:text-5xl">{album.name}</h1>
        <div class="flex flex-wrap items-center gap-1.5 text-sm text-muted-foreground">
          <a
            href="/artist/{encodeURIComponent(album.artist)}"
            class="font-semibold text-foreground hover:underline"
          >{album.artist}</a>
          {#if album.year}
            <span>·</span>
            <span>{album.year}</span>
          {/if}
          {#if album.genre}
            <span>·</span>
            <span>{album.genre}</span>
          {/if}
          <span>·</span>
          <span>{album.songCount} songs</span>
          {#if album.duration}
            <span>·</span>
            <span>{fmtDuration(album.duration)}</span>
          {/if}
        </div>
      {/if}

      {#if !loading}
        <div class="mt-5 flex items-center gap-3">
          <button
            class="grid size-14 shrink-0 place-items-center rounded-full bg-primary text-background shadow-lg transition hover:scale-105 disabled:opacity-40"
            onclick={playAll}
            disabled={songs.length === 0}
            aria-label="Play album"
          >
            <Play class="size-6 translate-x-0.5" fill="currentColor" />
          </button>
          <DropdownMenu>
            <DropdownMenuTrigger>
              {#snippet child({ props })}
                <button
                  {...props}
                  class="grid size-10 shrink-0 place-items-center rounded-md transition {$smartShuffleMode || $shuffleEnabled ? 'text-primary' : 'text-muted-foreground hover:text-foreground'}"
                  aria-label="Shuffle options"
                  title={$smartShuffleMode ? 'Smart Shuffle on' : $shuffleEnabled ? 'Shuffle on' : 'Shuffle off'}
                  disabled={songs.length === 0}
                >
                  {#if $smartShuffleMode}
                    <Sparkles class="size-4" />
                  {:else}
                    <Shuffle class="size-4" />
                  {/if}
                </button>
              {/snippet}
            </DropdownMenuTrigger>
            <DropdownMenuContent align="start" class="min-w-44">
              <DropdownMenuItem onclick={activateShuffle} class="gap-3 {$shuffleEnabled && !$smartShuffleMode ? 'text-primary' : ''}">
                <Shuffle class="size-4 shrink-0" />
                Shuffle
                {#if $shuffleEnabled && !$smartShuffleMode}<span class="ml-auto size-1.5 rounded-full bg-primary"></span>{/if}
              </DropdownMenuItem>
              <DropdownMenuItem onclick={activateSmartShuffle} class="gap-3 {$smartShuffleMode ? 'text-primary' : ''}">
                <Sparkles class="size-4 shrink-0" />
                Smart Shuffle
                {#if $smartShuffleMode}<span class="ml-auto size-1.5 rounded-full bg-primary"></span>{/if}
              </DropdownMenuItem>
              <DropdownMenuSeparator />
              <DropdownMenuItem onclick={deactivateShuffle} class="gap-3 {!$shuffleEnabled && !$smartShuffleMode ? 'text-primary' : 'text-muted-foreground'}">
                <span class="size-4 shrink-0 flex items-center justify-center text-xs font-bold">—</span>
                Off
                {#if !$shuffleEnabled && !$smartShuffleMode}<span class="ml-auto size-1.5 rounded-full bg-primary"></span>{/if}
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        </div>
      {/if}
    </div>
  </div>

  {#if error}
    <p class="mb-4 text-sm text-destructive">{error}</p>
  {/if}

  <!-- Track list -->
  <div class="rounded-lg">
    <!-- Header row -->
    <div class="mb-1 grid items-center gap-3 border-b px-3 pb-2 text-xs font-semibold uppercase tracking-wider text-muted-foreground"
      style="grid-template-columns: 2rem 1fr 6rem"
    >
      <span class="text-center">#</span>
      <span>Title</span>
      <span class="flex items-center justify-end gap-1"><Clock3 class="size-3.5" /></span>
    </div>

    {#if loading}
      {#each Array(8) as _, i (i)}
        <div class="flex h-14 items-center gap-3 px-3">
          <div class="w-8 shrink-0"></div>
          <div class="size-10 animate-pulse rounded bg-muted"></div>
          <div class="h-4 flex-1 animate-pulse rounded bg-muted"></div>
          <div class="h-4 w-12 animate-pulse rounded bg-muted"></div>
        </div>
      {/each}
    {:else}
      {#each songs as song, i (song.id)}
        <SongContextMenu {song} onplay={() => playFrom(i)}>
          <button
            class="group grid w-full items-center gap-3 rounded-md px-3 py-2 text-left transition hover:bg-accent"
            style="grid-template-columns: 2rem 1fr 6rem"
            onclick={() => playFrom(i)}
          >
            <span class="text-center text-sm tabular-nums text-muted-foreground group-hover:hidden">{i + 1}</span>
            <span class="hidden place-items-center group-hover:grid">
              <Play class="size-3.5" fill="currentColor" />
            </span>

            <div class="flex min-w-0 items-center gap-3">
              {#if song.coverArtUrl}
                <img
                  class="size-10 shrink-0 rounded object-cover"
                  src={song.coverArtUrl}
                  alt={song.title}
                  loading="lazy"
                />
              {:else}
                <div class="grid size-10 shrink-0 place-items-center rounded bg-secondary text-xs font-bold">
                  {initials(song.title)}
                </div>
              {/if}
              <div class="min-w-0">
                <p class="truncate text-sm font-medium">{song.title}</p>
                <p class="truncate text-xs text-muted-foreground">{song.artist}</p>
              </div>
            </div>

            <span class="text-right text-xs tabular-nums text-muted-foreground">{fmt(song.duration)}</span>
          </button>
        </SongContextMenu>
      {/each}
    {/if}
  </div>
</div>
