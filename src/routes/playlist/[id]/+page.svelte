<script lang="ts">
  import { goto } from '$app/navigation';
  import { fetchSubsonicPlaylistDetail, type SubsonicSong } from '$lib/api';
  import { Play, Shuffle, Sparkles } from '@lucide/svelte';
  import { focusTrack, playQueue, playingFrom, smartShuffleMode, shuffleEnabled } from '$lib/stores/player';
  import { Button } from '$lib/components/ui';
  import SongContextMenu from '$lib/components/SongContextMenu.svelte';
  import {
    DropdownMenu,
    DropdownMenuTrigger,
    DropdownMenuContent,
    DropdownMenuItem,
    DropdownMenuSeparator,
  } from '$lib/components/ui/dropdown-menu';

  let { data } = $props<{ data: { id: string } }>();

  let loading = $state(false);
  let error = $state('');
  let playlistName = $state('');
  let coverArtUrl = $state('');
  let songCount = $state(0);
  let songs = $state<SubsonicSong[]>([]);

  $effect(() => {
    const id = data.id;
    loading = true;
    error = '';
    fetchSubsonicPlaylistDetail(id)
      .then((detail) => {
        playlistName = detail.playlist.name;
        coverArtUrl = detail.playlist.coverArtUrl;
        songCount = detail.playlist.songCount;
        songs = detail.songs;
      })
      .catch((err) => {
        error = err instanceof Error ? err.message : 'Failed to load playlist.';
      })
      .finally(() => {
        loading = false;
      });
  });

  function formatDuration(seconds: number): string {
    const h = Math.floor(seconds / 3600);
    const m = Math.floor((seconds % 3600) / 60);
    const s = seconds % 60;
    if (h > 0) return `${h}:${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}`;
    return `${m}:${String(s).padStart(2, '0')}`;
  }

  function totalDuration(): number {
    return songs.reduce((acc, s) => acc + (s.duration ?? 0), 0);
  }

  function initials(name: string): string {
    return name
      .split(' ')
      .filter(Boolean)
      .slice(0, 2)
      .map((part) => part[0]?.toUpperCase() ?? '')
      .join('');
  }

  function playSong(index: number) {
    const song = songs[index];
    if (!song) return;
    focusTrack.set({
      title: song.title,
      artist: song.artist,
      imageUrl: song.coverArtUrl,
      source: 'subsonic',
      album: song.album
    });
    playingFrom.set({ type: 'playlist', name: playlistName, href: `/playlist/${encodeURIComponent(data.id)}` });
    playQueue(songs, index);
  }

  function playAll() {
    if (!songs.length) return;
    const list = ($shuffleEnabled || $smartShuffleMode) ? [...songs].sort(() => Math.random() - 0.5) : songs;
    focusTrack.set({ title: list[0].title, artist: list[0].artist, imageUrl: list[0].coverArtUrl, source: 'subsonic', album: list[0].album });
    playingFrom.set({ type: 'playlist', name: playlistName, href: `/playlist/${encodeURIComponent(data.id)}` });
    playQueue(list, 0);
  }
</script>

<div class="mb-6 flex gap-4">
  {#if coverArtUrl}
    <img class="h-36 w-36 shrink-0 rounded-lg object-cover shadow-lg" src={coverArtUrl} alt={playlistName} />
  {:else}
    <div class="flex h-36 w-36 shrink-0 items-center justify-center rounded-lg bg-gradient-to-br from-slate-500 to-slate-700 text-2xl font-black shadow-lg">
      {initials(playlistName || '?')}
    </div>
  {/if}
  <div class="flex flex-col justify-end gap-2">
    <p class="text-xs font-semibold uppercase tracking-widest text-muted-foreground">Playlist</p>
    <h2 class="text-3xl font-bold tracking-tight">{playlistName || '…'}</h2>
    {#if songs.length}
      <p class="text-sm text-muted-foreground">
        {songCount} songs · {formatDuration(totalDuration())}
      </p>
    {/if}
    <div class="flex items-center gap-3 mt-1">
      <!-- Big play button -->
      <button
        onclick={playAll}
        disabled={!songs.length}
        class="flex size-14 shrink-0 items-center justify-center rounded-full bg-primary text-primary-foreground shadow-lg transition-all duration-150 hover:scale-105 hover:brightness-110 active:scale-95 disabled:opacity-40 disabled:cursor-not-allowed"
        aria-label="Play all"
      >
        <Play class="size-6 translate-x-0.5" fill="currentColor" />
      </button>

      <!-- Shuffle mode selector -->
      <DropdownMenu>
        <DropdownMenuTrigger>
          {#snippet child({ props })}
            <button
              {...props}
              disabled={!songs.length}
              class="grid size-10 shrink-0 place-items-center rounded-md transition disabled:opacity-40 disabled:cursor-not-allowed {$smartShuffleMode || $shuffleEnabled ? 'text-primary' : 'text-muted-foreground hover:text-foreground'}"
              aria-label="Shuffle options"
              title={$smartShuffleMode ? 'Smart Shuffle on' : $shuffleEnabled ? 'Shuffle on' : 'Shuffle off'}
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
          <DropdownMenuItem onclick={() => { shuffleEnabled.set(true); smartShuffleMode.set(false); }} class="gap-3 {$shuffleEnabled && !$smartShuffleMode ? 'text-primary' : ''}">
            <Shuffle class="size-4 shrink-0" />
            Shuffle
            {#if $shuffleEnabled && !$smartShuffleMode}<span class="ml-auto size-1.5 rounded-full bg-primary"></span>{/if}
          </DropdownMenuItem>
          <DropdownMenuItem onclick={() => { shuffleEnabled.set(true); smartShuffleMode.set(true); }} class="gap-3 {$smartShuffleMode ? 'text-primary' : ''}">
            <Sparkles class="size-4 shrink-0" />
            Smart Shuffle
            {#if $smartShuffleMode}<span class="ml-auto size-1.5 rounded-full bg-primary"></span>{/if}
          </DropdownMenuItem>
          <DropdownMenuSeparator />
          <DropdownMenuItem onclick={() => { shuffleEnabled.set(false); smartShuffleMode.set(false); }} class="gap-3 {!$shuffleEnabled && !$smartShuffleMode ? 'text-primary' : 'text-muted-foreground'}">
            <span class="size-4 shrink-0 flex items-center justify-center text-xs font-bold">—</span>
            Off
            {#if !$shuffleEnabled && !$smartShuffleMode}<span class="ml-auto size-1.5 rounded-full bg-primary"></span>{/if}
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
    </div>
  </div>
</div>

{#if error}
  <p class="mb-3 text-sm text-destructive">{error}</p>
{/if}
{#if loading}
  <p class="mb-3 text-sm text-muted-foreground">Loading playlist…</p>
{/if}

{#if songs.length}
  <div class="mt-2">
    <!-- Column headers -->
    <div class="grid items-center gap-4 border-b border-border/40 px-4 pb-2 text-xs font-medium uppercase tracking-wider text-muted-foreground/50"
         style="grid-template-columns: 2.5rem 1fr 1fr 4rem">
      <span class="text-center">#</span>
      <span>Title</span>
      <span class="hidden md:block">Album</span>
      <span class="text-right">Duration</span>
    </div>

    <!-- Rows -->
    <div class="mt-1 space-y-0.5">
      {#each songs as song, index (song.id + '-' + index)}
        <SongContextMenu {song} onplay={() => playSong(index)}>
          <button
            class="group grid w-full items-center gap-4 rounded-md px-4 py-2.5 text-left transition-colors duration-150 hover:bg-white/5"
            style="grid-template-columns: 2.5rem 1fr 1fr 4rem"
            onclick={() => playSong(index)}
          >
            <!-- Track # / Play icon crossfade -->
            <span class="relative flex h-7 w-7 shrink-0 items-center justify-center mx-auto">
              <span class="absolute inset-0 flex items-center justify-center text-sm tabular-nums text-muted-foreground transition-all duration-150 group-hover:scale-50 group-hover:opacity-0">{index + 1}</span>
              <span class="absolute inset-0 flex items-center justify-center scale-50 opacity-0 transition-all duration-150 group-hover:scale-100 group-hover:opacity-100">
                <Play class="size-4" fill="currentColor" />
              </span>
            </span>

            <!-- Title + cover art -->
            <div class="flex min-w-0 items-center gap-3">
              {#if song.coverArtUrl}
                <img class="size-10 shrink-0 rounded-md object-cover shadow-md" src={song.coverArtUrl} alt={song.title} loading="lazy" />
              {:else}
                <div class="flex size-10 shrink-0 items-center justify-center rounded-md bg-gradient-to-br from-slate-500 to-slate-700 text-xs font-bold shadow-md">
                  {initials(song.title)}
                </div>
              {/if}
              <div class="min-w-0">
                <p class="truncate text-sm font-medium transition-colors duration-150 group-hover:text-foreground">{song.title}</p>
                <span
                  role="link"
                  tabindex="0"
                  class="truncate text-xs text-muted-foreground hover:underline hover:text-foreground transition-colors duration-150 cursor-pointer"
                  onclick={(e) => { e.stopPropagation(); goto(`/artist/${encodeURIComponent(song.artist)}`); }}
                  onkeydown={(e) => { if (e.key === 'Enter') { e.stopPropagation(); goto(`/artist/${encodeURIComponent(song.artist)}`); } }}
                >{song.artist}</span>
              </div>
            </div>

            <!-- Album -->
            <span class="hidden truncate text-sm text-muted-foreground md:block">{song.album}</span>

            <!-- Duration -->
            <span class="text-right text-xs tabular-nums text-muted-foreground">{formatDuration(song.duration ?? 0)}</span>
          </button>
        </SongContextMenu>
      {/each}
    </div>
  </div>
{:else if !loading && !error}
  <p class="text-sm text-muted-foreground">This playlist has no tracks.</p>
{/if}
