<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { Play, Pause, Music2, Search, X, Clock } from '@lucide/svelte';

  import { searchSongs, type Song } from '$lib/api';
  import { focusTrack, playQueue, playingFrom, recentlyPlayedSongs, queue, currentIndex, isPlaying, togglePlayRequest } from '$lib/stores/player';
  import SongContextMenu from '$lib/components/SongContextMenu.svelte';

  // All songs loaded from the library on mount.
  let allSongs = $state<Song[]>([]);
  let query = $state('');
  let loading = $state(false);
  let error = $state('');

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

  function formatDuration(seconds: number): string {
    const h = Math.floor(seconds / 3600);
    const m = Math.floor((seconds % 3600) / 60);
    const s = seconds % 60;
    if (h > 0) return `${h}:${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}`;
    return `${m}:${String(s).padStart(2, '0')}`;
  }

  function initials(name: string): string {
    return name.split(' ').filter(Boolean).slice(0, 2).map((p) => p[0]?.toUpperCase() ?? '').join('');
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

  onMount(() => {
    loading = true;
    // Fetch all library songs — empty query returns the full catalogue on most servers.
    searchSongs('', 500)
      .then((s) => { allSongs = s; })
      .catch((err) => { error = err instanceof Error ? err.message : 'Failed to load songs.'; })
      .finally(() => { loading = false; });
  });

  const currentTrackId = $derived($queue[$currentIndex]?.id ?? '');
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

  <!-- Recently Played -->
  {#if $recentlyPlayedSongs.length > 0 && !query.trim()}
    <div class="space-y-2">
      <div class="flex items-center gap-2 text-xs font-medium uppercase tracking-wider text-muted-foreground/60">
        <Clock class="size-3.5" />
        <span>Recently Played</span>
      </div>
      <div class="space-y-0.5">
        {#each $recentlyPlayedSongs.slice(0, 5) as song (song.id)}
          <SongContextMenu {song} onplay={() => { focusTrack.set({ title: song.title, artist: song.artist, imageUrl: song.coverArtUrl, source: 'library', album: song.album }); playQueue([song], 0); playingFrom.set({ type: 'search', name: 'All Songs', href: '/songs' }); }}>
            <button
              class="group grid w-full items-center gap-4 rounded-md px-4 py-2.5 text-left transition-colors duration-150 hover:bg-white/5"
              style="grid-template-columns: 2.5rem 1fr 1fr 4rem"
              onclick={() => { focusTrack.set({ title: song.title, artist: song.artist, imageUrl: song.coverArtUrl, source: 'library', album: song.album }); playQueue([song], 0); playingFrom.set({ type: 'search', name: 'All Songs', href: '/songs' }); }}
            >
              <span class="relative flex size-7 shrink-0 items-center justify-center mx-auto">
                <Clock class="absolute inset-0 m-auto size-3.5 text-muted-foreground/50 transition-all duration-150 group-hover:scale-50 group-hover:opacity-0" />
                <span class="absolute inset-0 flex items-center justify-center scale-50 opacity-0 transition-all duration-150 group-hover:scale-100 group-hover:opacity-100">
                  <Play class="size-4" fill="currentColor" />
                </span>
              </span>
              <div class="flex min-w-0 items-center gap-3">
                {#if song.coverArtUrl}
                  <img class="size-10 shrink-0 rounded-md object-cover shadow-md" src={song.coverArtUrl} alt={song.title} loading="lazy" />
                {:else}
                  <div class="flex size-10 shrink-0 items-center justify-center rounded-md bg-gradient-to-br from-slate-500 to-slate-700 text-xs font-bold shadow-md">
                    <Music2 class="size-4 text-white/50" />
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
              <span class="hidden truncate text-sm text-muted-foreground md:block">{song.album}</span>
              <span class="text-right text-xs tabular-nums text-muted-foreground">{formatDuration(song.duration ?? 0)}</span>
            </button>
          </SongContextMenu>
        {/each}
      </div>
    </div>
  {/if}

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
        class="grid items-center gap-4 border-b border-border/40 px-4 pb-2 text-xs font-medium uppercase tracking-wider text-muted-foreground/50"
        style="grid-template-columns: 2.5rem 1fr 1fr 4rem"
      >
        <span class="text-center">#</span>
        <span>Title</span>
        <span class="hidden md:block">Album</span>
        <span class="text-right">Duration</span>
      </div>

      <div class="mt-1 space-y-0.5">
        {#each songs as song, index (song.id + '-' + index)}
          {@const isCurrentTrack = song.id === currentTrackId}
          <SongContextMenu {song} onplay={() => playSong(index)}>
            <button
              class="group grid w-full items-center gap-4 rounded-md px-4 py-2.5 text-left transition-colors duration-150 hover:bg-white/5 {isCurrentTrack ? 'bg-primary/5' : ''}"
              style="grid-template-columns: 2.5rem 1fr 1fr 4rem"
              onclick={() => isCurrentTrack ? togglePlayRequest.update(n => n + 1) : playSong(index)}
            >
              <!-- Track # / Play icon crossfade -->
              <span class="relative flex size-7 shrink-0 items-center justify-center mx-auto">
                {#if isCurrentTrack}
                  <span class="flex items-end gap-[2px] transition-all duration-150 group-hover:opacity-0 group-hover:scale-50">
                    <span class="w-[3px] rounded-[1px] bg-primary origin-bottom" style="height: 12px; animation: equalizer 0.8s ease-in-out infinite 0s; animation-play-state: {$isPlaying ? 'running' : 'paused'};"></span>
                    <span class="w-[3px] rounded-[1px] bg-primary origin-bottom" style="height: 8px; animation: equalizer 0.8s ease-in-out infinite 0.25s; animation-play-state: {$isPlaying ? 'running' : 'paused'};"></span>
                    <span class="w-[3px] rounded-[1px] bg-primary origin-bottom" style="height: 12px; animation: equalizer 0.8s ease-in-out infinite 0.5s; animation-play-state: {$isPlaying ? 'running' : 'paused'};"></span>
                  </span>
                  <span class="absolute inset-0 flex items-center justify-center scale-50 opacity-0 transition-all duration-150 group-hover:scale-100 group-hover:opacity-100 text-primary">
                    {#if $isPlaying}
                      <Pause class="size-4" fill="currentColor" />
                    {:else}
                      <Play class="size-4" fill="currentColor" />
                    {/if}
                  </span>
                {:else}
                  <span class="absolute inset-0 flex items-center justify-center text-sm tabular-nums text-muted-foreground transition-all duration-150 group-hover:scale-50 group-hover:opacity-0">
                    {index + 1}
                  </span>
                  <span class="absolute inset-0 flex items-center justify-center scale-50 opacity-0 transition-all duration-150 group-hover:scale-100 group-hover:opacity-100">
                    <Play class="size-4" fill="currentColor" />
                  </span>
                {/if}
              </span>

              <!-- Title + cover art -->
              <div class="flex min-w-0 items-center gap-3">
                {#if song.coverArtUrl}
                  <img class="size-10 shrink-0 rounded-md object-cover shadow-md" src={song.coverArtUrl} alt={song.title} loading="lazy" />
                {:else}
                  <div class="flex size-10 shrink-0 items-center justify-center rounded-md bg-gradient-to-br from-slate-500 to-slate-700 text-xs font-bold shadow-md">
                    <Music2 class="size-4 text-white/50" />
                  </div>
                {/if}
                <div class="min-w-0">
                  <p class="truncate text-sm font-medium transition-colors duration-150 group-hover:text-foreground {isCurrentTrack ? 'text-primary' : ''}">{song.title}</p>
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
              <span
                role="link"
                tabindex="0"
                class="hidden truncate text-sm text-muted-foreground hover:underline hover:text-foreground transition-colors duration-150 cursor-pointer md:block"
                onclick={(e) => { e.stopPropagation(); goto(`/album/${encodeURIComponent(song.albumId)}`); }}
                onkeydown={(e) => { if (e.key === 'Enter') { e.stopPropagation(); goto(`/album/${encodeURIComponent(song.albumId)}`); } }}
              >{song.album}</span>

              <!-- Duration -->
              <span class="text-right text-xs tabular-nums text-muted-foreground">{formatDuration(song.duration ?? 0)}</span>
            </button>
          </SongContextMenu>
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
