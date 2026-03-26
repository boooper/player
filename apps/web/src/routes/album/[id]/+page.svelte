<script lang="ts">
  import { Play, Pause, Shuffle, Clock3, ArrowLeft, Sparkles } from '@lucide/svelte';
  import { goto } from '$app/navigation';
  import {
    fetchAlbumDetail,
    fetchAlbumSongs,
    fetchArtistAlbums,
    type Album,
    type Song
  } from '$lib/servers';
  import { findAlbumGroupIds, mergeAlbumSongs } from '$lib/media-merge';
  import { startQueue, playingFrom, activateShuffle, enableSmartShuffle, disableShuffle, shuffleEnabled, smartShuffleMode, isPlaying, togglePlayRequest } from '$lib/stores/player';
  import ExternalSourceBadge from '$lib/components/ExternalSourceBadge.svelte';
  import { SongRow } from '$lib/components/media';
  import {
    DropdownMenu,
    DropdownMenuTrigger,
    DropdownMenuContent,
    DropdownMenuItem,
    DropdownMenuSeparator,
  } from '$lib/components/ui/dropdown-menu';
  import { libraryRefresh } from '$lib/stores/ui-state';
  import { initials, withTimeout, formatDurationHuman, sumDuration } from '$lib/utils';
  import { DesktopCache } from '$lib/hooks/use-desktop-cache.svelte';

  function goBack() {
    if (window.history.length > 1) {
      window.history.back();
    } else {
      goto('/');
    }
  }

  function buildAlbum(base: Album, trackList: Song[]): Album & { genre?: string } {
    return { ...base, songCount: trackList.length, duration: sumDuration(trackList) };
  }

  let { data } = $props<{ data: { id: string } }>();

  const albumHref = $derived(`/album/${encodeURIComponent(data.id)}`);
  const albumIsActive = $derived($playingFrom.href === albumHref);

  let loading = $state(true);
  let merging = $state(false);
  let error = $state('');
  let album = $state<(Album & { genre?: string }) | null>(null);
  let songs = $state<Song[]>([])
  const cache = new DesktopCache();
  let albumLoadVersion = 0;
  const ALBUM_LOAD_TIMEOUT_MS = 12000;

  async function loadAlbum() {
    const loadVersion = ++albumLoadVersion;
    loading = true;
    error = '';
    try {
      const detail = await fetchAlbumDetail(data.id);
      if (loadVersion !== albumLoadVersion) return;

      // Show initial content immediately — don't wait for related album merging.
      album = buildAlbum(detail.album, detail.songs);
      songs = detail.songs;
      loading = false;
      merging = true;

      // Background: find related album editions across providers and merge.
      const artistAlbums = await withTimeout(
        fetchArtistAlbums(detail.album.artist, 50),
        ALBUM_LOAD_TIMEOUT_MS,
        'Related albums load'
      ).catch(() => [] as Album[]);
      const relatedIds = findAlbumGroupIds(artistAlbums, detail.album)
        .filter((id) => id !== detail.album.id);

      if (relatedIds.length > 0) {
        const songResults = await Promise.allSettled(
          relatedIds.map((albumId) =>
            withTimeout(fetchAlbumSongs(albumId), ALBUM_LOAD_TIMEOUT_MS, 'Album tracks load').catch(() => [] as Song[])
          )
        );
        const extra = songResults.flatMap((r) => (r.status === 'fulfilled' ? r.value : []));
        const merged = mergeAlbumSongs([...detail.songs, ...extra]);
        if (loadVersion !== albumLoadVersion) return;
        const resolved = merged.length ? merged : detail.songs;
        album = buildAlbum(detail.album, resolved);
        songs = resolved;
      }

      if (loadVersion !== albumLoadVersion) return;
      merging = false;
      void cache.load(songs, loadVersion, () => albumLoadVersion);
    } catch (err) {
      if (loadVersion !== albumLoadVersion) return;
      error = err instanceof Error ? err.message : 'Failed to load album.';
      loading = false;
      merging = false;
    }
  }

  $effect(() => {
    const refresh = $libraryRefresh;
    void refresh;
    loadAlbum();
  });


  function playFrom(index: number) {
    if (!songs.length) return;
    if (album) startQueue(songs, index, { type: 'album', name: album.name, href: albumHref });
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
  <div class="page-hero app-glass mb-8 flex flex-col gap-6 rounded-[2rem] p-5 sm:flex-row sm:items-end">
    {#if loading}
      <div class="aspect-square w-48 shrink-0 animate-pulse rounded-2xl bg-muted shadow-2xl"></div>
    {:else if album?.coverArtUrl}
      <img
        class="aspect-square w-48 shrink-0 rounded-2xl object-cover shadow-2xl"
        src={album.coverArtUrl}
        alt={album?.name ?? ''}
      />
    {:else}
      <div class="app-card grid aspect-square w-48 shrink-0 place-items-center rounded-2xl bg-gradient-to-br from-secondary to-accent text-4xl font-black shadow-2xl">
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
        <div class="mb-1 flex flex-wrap items-center gap-2">
          <h1 class="app-section-title text-4xl font-black tracking-tight sm:text-5xl">{album.name}</h1>
          <ExternalSourceBadge id={album.id} />
        </div>
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
            <span>{formatDurationHuman(album.duration)}</span>
          {/if}
        </div>
      {/if}

      {#if !loading}
        <div class="mt-5 flex items-center gap-3">
          <button
            class="relative grid size-14 shrink-0 place-items-center rounded-full bg-primary text-primary-foreground shadow-lg transition hover:scale-105 disabled:opacity-40"
            onclick={() => albumIsActive ? togglePlayRequest.update((n) => n + 1) : playFrom(0)}
            disabled={songs.length === 0}
            aria-label={albumIsActive && $isPlaying ? 'Pause album' : 'Play album'}
          >
            {#if albumIsActive && $isPlaying}
              <Pause class="size-6" fill="currentColor" />
            {:else}
              <Play class="size-6 translate-x-0.5" fill="currentColor" />
            {/if}
            {#if merging}
              <span class="absolute inset-0 rounded-full border-2 border-primary-foreground/30 border-t-primary-foreground animate-spin pointer-events-none"></span>
            {/if}
          </button>
          <DropdownMenu>
            <DropdownMenuTrigger>
              {#snippet child({ props })}
                <button
                  {...props}
                  class="app-round-button grid size-10 shrink-0 place-items-center rounded-full transition {$smartShuffleMode || $shuffleEnabled ? 'text-primary' : 'text-muted-foreground hover:text-foreground'}"
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
              <DropdownMenuItem onclick={enableSmartShuffle} class="gap-3 {$smartShuffleMode ? 'text-primary' : ''}">
                <Sparkles class="size-4 shrink-0" />
                Smart Shuffle
                {#if $smartShuffleMode}<span class="ml-auto size-1.5 rounded-full bg-primary"></span>{/if}
              </DropdownMenuItem>
              <DropdownMenuSeparator />
              <DropdownMenuItem onclick={disableShuffle} class="gap-3 {!$shuffleEnabled && !$smartShuffleMode ? 'text-primary' : 'text-muted-foreground'}">
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
  <div class="page-section rounded-lg">
    <!-- Header row -->
    <div class="mb-1 grid items-center gap-3 border-b px-3 pb-2 text-xs font-semibold uppercase tracking-wider text-muted-foreground"
      style="grid-template-columns: 2rem 2.5rem 1fr 4rem"
    >
      <span class="text-center">#</span>
      <span></span>
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
        <SongRow
          {song}
          index={i}
          onplay={() => playFrom(i)}
          cached={cache.enabled ? cache.ids.has(song.id) : null}
          staggerIndex={i}
        />
      {/each}
    {/if}
  </div>
</div>
