<script lang="ts">
  import { onMount } from 'svelte';
  import { Play, Music2, Search, X } from '@lucide/svelte';

  import { DESKTOP_PLAYBACK_CACHE_UPDATED_EVENT, desktopPlaybackCachedIds, searchSongs, type Song } from '$lib/servers';
  import { focusTrack, playQueue, playingFrom } from '$lib/stores/player';
  import { SongRow } from '$lib/components/media';
  import { libraryRefresh } from '$lib/stores/ui-state';
  import { isTauri } from '$lib/tauri';

  // All songs loaded from the library on mount.
  let allSongs = $state<Song[]>([]);
  let query = $state('');
  let loading = $state(false);
  let error = $state('');
  let songsLoadVersion = 0;
  let cachedSongIds = $state<Set<string>>(new Set());
  const desktopPlayback = isTauri();

  // Client-side filter when a query is present.
  const songs = $derived(
    query.trim()
      ? allSongs.filter((s) => {
          const q = query.toLowerCase();
          return (
            s.title.toLowerCase().includes(q) ||
            s.artist.toLowerCase().includes(q) ||
            s.album.toLowerCase().includes(q)
          );
        })
      : allSongs
  );

  function clearSearch() {
    query = '';
  }

  function playSong(index: number) {
    const song = songs[index];
    if (!song) return;
    focusTrack.set({ title: song.title, artist: song.artist, imageUrl: song.coverArtUrl, source: 'library', album: song.album });
    playQueue(songs, index);
    playingFrom.set({ type: 'search', name: 'All Songs', href: '/songs' });
  }

  function playAll() {
    if (!songs.length) return;
    playSong(0);
  }

  function loadSongs() {
    const loadVersion = ++songsLoadVersion;
    loading = true;
    error = '';
    // Fetch all library songs — empty query returns the full catalogue on most servers.
    searchSongs('', 500)
      .then((s) => {
        if (loadVersion !== songsLoadVersion) return;
        allSongs = s;
        if (!desktopPlayback) {
          cachedSongIds = new Set();
          return;
        }
        desktopPlaybackCachedIds(s)
          .then((ids) => {
            if (loadVersion !== songsLoadVersion) return;
            cachedSongIds = new Set(ids);
          })
          .catch(() => {
            if (loadVersion !== songsLoadVersion) return;
            cachedSongIds = new Set();
          });
      })
      .catch((err) => {
        if (loadVersion !== songsLoadVersion) return;
        error = err instanceof Error ? err.message : 'Failed to load songs.';
      })
      .finally(() => {
        if (loadVersion !== songsLoadVersion) return;
        loading = false;
      });
  }

  $effect(() => {
    const refresh = $libraryRefresh;
    void refresh;
    loadSongs();
  });

  onMount(() => {
    if (!desktopPlayback) return;

    function handleDesktopCacheUpdated(event: Event) {
      const songId = (event as CustomEvent<{ songId?: string }>).detail?.songId;
      if (!songId) return;
      cachedSongIds = new Set([...cachedSongIds, songId]);
    }

    window.addEventListener(DESKTOP_PLAYBACK_CACHE_UPDATED_EVENT, handleDesktopCacheUpdated);
    return () => {
      window.removeEventListener(DESKTOP_PLAYBACK_CACHE_UPDATED_EVENT, handleDesktopCacheUpdated);
    };
  });

</script>

<div class="space-y-6">
  <!-- Header -->
  <div class="flex items-end justify-between gap-4">
    <div>
      <h1 class="text-3xl font-bold">Songs</h1>
      <p class="mt-1 text-sm text-muted-foreground">
        {#if loading}
          Loading…
        {:else if query.trim()}
          {songs.length} of {allSongs.length} song{allSongs.length !== 1 ? 's' : ''}
        {:else}
          {allSongs.length} song{allSongs.length !== 1 ? 's' : ''}
        {/if}
      </p>
    </div>
    {#if songs.length > 0}
      <button
        class="flex size-12 shrink-0 items-center justify-center rounded-full bg-primary text-primary-foreground shadow-lg transition hover:scale-105 hover:brightness-110 active:scale-95"
        onclick={playAll}
        aria-label="Play all"
      >
        <Play class="size-5 translate-x-0.5 text-muted-foreground" fill="currentColor" />
      </button>
    {/if}
  </div>

  <!-- Search / filter bar -->
  <div class="relative">
    <Search class="absolute left-3 top-1/2 size-4 -translate-y-1/2 text-muted-foreground pointer-events-none" />
    <input
      type="search"
      placeholder="Filter songs…"
      bind:value={query}
      class="w-full rounded-lg border border-border/50 bg-secondary/60 py-2.5 pl-9 pr-10 text-sm placeholder:text-muted-foreground focus:border-border focus:outline-none focus:ring-1 focus:ring-ring"
    />
    {#if query}
      <button class="absolute right-3 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground transition-colors" onclick={clearSearch}>
        <X class="size-4" />
      </button>
    {/if}
  </div>

  <!-- Error -->
  {#if error}
    <div class="rounded-lg border border-rose-500/30 bg-rose-500/10 px-4 py-3 text-sm text-rose-300">{error}</div>
  {/if}

  <!-- Song list -->
  {#if songs.length > 0}
    <div>
      <!-- Column headers -->
      <div
        class="grid items-center gap-3 border-b border-border/40 px-3 pb-2 text-xs font-medium uppercase tracking-wider text-muted-foreground/50"
        style="grid-template-columns: 2rem 2.5rem 1fr 1fr 4rem"
      >
        <span class="text-center">#</span>
        <span></span>
        <span>Title</span>
        <span class="hidden md:block">Album</span>
        <span class="text-right">Duration</span>
      </div>

      <div class="mt-1 space-y-0.5">
        {#each songs as song, index (song.id + '-' + index)}
          <SongRow
            {song}
            {index}
            showAlbum
            onplay={() => playSong(index)}
            cached={desktopPlayback ? cachedSongIds.has(song.id) : null}
            staggerIndex={index}
          />
        {/each}
      </div>
    </div>
  {:else if !loading && !error}
    <div class="flex flex-col items-center justify-center gap-3 py-20 text-center">
      <Music2 class="size-12 text-muted-foreground/30" />
      <p class="text-sm text-muted-foreground">
        {query.trim() ? 'No songs matched your filter.' : 'No songs found in your library.'}
      </p>
    </div>
  {/if}
</div>

<style>
  .song-row {
    animation: song-row-in 280ms cubic-bezier(0.2, 0.9, 0.25, 1) both;
    animation-delay: min(calc(var(--row-index) * 11ms), 260ms);
    transition:
      transform 150ms ease,
      background-color 150ms ease,
      box-shadow 150ms ease;
  }

  .song-row:hover {
    transform: translateX(2px);
    box-shadow: inset 0 1px 0 rgb(255 255 255 / 0.025);
  }

  @keyframes song-row-in {
    from {
      opacity: 0;
      transform: translateY(8px);
    }

    to {
      opacity: 1;
      transform: translateY(0);
    }
  }
</style>
