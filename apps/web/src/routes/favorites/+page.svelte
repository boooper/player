<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { Play, Pause, Shuffle, Heart, Sparkles } from '@lucide/svelte';

  import { DESKTOP_PLAYBACK_CACHE_UPDATED_EVENT, desktopPlaybackCachedIds, fetchStarredSongs, type Song } from '$lib/servers';
  import { focusTrack, playQueue, playingFrom, starredSongIds, smartShuffleMode, shuffleEnabled, queue, currentIndex, isPlaying, togglePlayRequest, enableShuffle, enableSmartShuffle, disableShuffle } from '$lib/stores/player';
  import SongContextMenu from '$lib/components/SongContextMenu.svelte';
  import SongArtistLinks from '$lib/components/SongArtistLinks.svelte';
  import SongTechBadge from '$lib/components/SongTechBadge.svelte';
  import { formatClockDuration } from '$lib/utils';
  import {
    DropdownMenu,
    DropdownMenuTrigger,
    DropdownMenuContent,
    DropdownMenuItem,
    DropdownMenuSeparator,
  } from '$lib/components/ui/dropdown-menu';
  import { libraryRefresh } from '$lib/stores/ui-state';
  import { isTauri } from '$lib/tauri';

  let loading = $state(false);
  let error = $state('');
  // allStarredSongs holds the server-fetched list; songs derives from it
  // by filtering against the live starredSongIds store. This avoids the
  // race condition where a server re-fetch fires before the star/unstar
  // API call completes and returns stale data.
  let allStarredSongs = $state<Song[]>([])
  let songs = $derived(allStarredSongs.filter(s => $starredSongIds.has(s.id)));
  const favoritesHref = '/favorites';
  const favoritesIsActive = $derived($playingFrom.href === favoritesHref);
  const favoriteSongCount = $derived(songs.length);
  let cachedSongIds = $state<Set<string>>(new Set());
  const desktopPlayback = isTauri();

  let favoritesLoadVersion = 0;

  function loadFavorites() {
    const loadVersion = ++favoritesLoadVersion;
    loading = true;
    error = '';
    fetchStarredSongs()
      .then((s) => {
        if (loadVersion !== favoritesLoadVersion) return;
        allStarredSongs = s;
        if (!desktopPlayback) {
          cachedSongIds = new Set();
        } else {
          desktopPlaybackCachedIds(s)
            .then((ids) => {
              if (loadVersion !== favoritesLoadVersion) return;
              cachedSongIds = new Set(ids);
            })
            .catch(() => {
              if (loadVersion !== favoritesLoadVersion) return;
              cachedSongIds = new Set();
            });
        }
        // If the layout hasn't populated starredSongIds yet (initial direct load),
        // seed it from our result so songs is derived correctly right away.
        if ($starredSongIds.size === 0 && s.length > 0) {
          starredSongIds.set(new Set(s.map(song => song.id)));
        }
      })
      .catch((err) => {
        if (loadVersion !== favoritesLoadVersion) return;
        error = err instanceof Error ? err.message : 'Failed to load favorites.';
      })
      .finally(() => {
        if (loadVersion !== favoritesLoadVersion) return;
        loading = false;
      });
  }

  $effect(() => {
    const refresh = $libraryRefresh;
    void refresh;
    loadFavorites();
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

  function formatDuration(seconds: number): string {
    return formatClockDuration(seconds);
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
      source: 'library',
      album: song.album
    });
    playQueue(songs, index);
    playingFrom.set({ type: 'favorites', name: 'Liked Songs', href: favoritesHref });
  }

  function playAll() {
    if (!songs.length) return;
    const list = ($shuffleEnabled || $smartShuffleMode) ? [...songs].sort(() => Math.random() - 0.5) : songs;
    focusTrack.set({ title: list[0].title, artist: list[0].artist, imageUrl: list[0].coverArtUrl, source: 'library', album: list[0].album });
    playQueue(list, 0);
    playingFrom.set({ type: 'favorites', name: 'Liked Songs', href: favoritesHref });
  }

  const currentTrackId = $derived($queue[$currentIndex]?.id ?? '');
</script>

<div class="page-hero app-glass mb-6 flex gap-4 rounded-[2rem] p-5">
  <div class="relative h-36 w-36 shrink-0 overflow-hidden rounded-2xl shadow-lg">
    {#if songs.length >= 4}
      <div class="grid h-full w-full grid-cols-2 grid-rows-2">
        {#each songs.slice(0, 4) as song (song.id)}
          <img class="h-full w-full object-cover" src={song.coverArtUrl} alt={song.title} loading="lazy" />
        {/each}
      </div>
      <div class="absolute inset-0 bg-gradient-to-br from-black/10 via-transparent to-black/45"></div>
    {:else}
      <div class="app-card flex h-full w-full items-center justify-center bg-gradient-to-br from-secondary to-accent">
        <Heart class="size-14 fill-white text-white" />
      </div>
    {/if}
  </div>
  <div class="flex flex-col justify-end gap-2">
    <p class="text-xs font-semibold uppercase tracking-widest text-muted-foreground">Playlist</p>
    <h2 class="app-section-title text-3xl font-bold tracking-tight">Liked Songs</h2>
    {#if favoriteSongCount}
      <p class="text-sm text-muted-foreground">
        {favoriteSongCount} songs · {formatDuration(totalDuration())}
      </p>
    {/if}
    <div class="flex items-center gap-3 mt-1">
      <!-- Big play button -->
      <button
        onclick={() => favoritesIsActive ? togglePlayRequest.update((n) => n + 1) : playAll()}
        disabled={!songs.length}
        class="flex size-14 shrink-0 items-center justify-center rounded-full bg-primary text-primary-foreground shadow-lg transition-all duration-150 hover:scale-105 hover:brightness-110 active:scale-95 disabled:opacity-40 disabled:cursor-not-allowed"
        aria-label={favoritesIsActive && $isPlaying ? 'Pause liked songs' : 'Play liked songs'}
      >
        {#if favoritesIsActive && $isPlaying}
          <Pause class="size-6 text-muted-foreground" fill="currentColor" />
        {:else}
          <Play class="size-6 translate-x-0.5 text-muted-foreground" fill="currentColor" />
        {/if}
      </button>

      <!-- Shuffle mode selector -->
      <DropdownMenu>
        <DropdownMenuTrigger>
          {#snippet child({ props })}
            <button
              {...props}
              disabled={!songs.length}
              class="app-round-button grid size-10 shrink-0 place-items-center rounded-full transition disabled:opacity-40 disabled:cursor-not-allowed {$smartShuffleMode || $shuffleEnabled ? 'text-primary' : 'text-muted-foreground hover:text-foreground'}"
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
          <DropdownMenuItem onclick={() => { smartShuffleMode.set(false); enableShuffle(); }} class="gap-3 {$shuffleEnabled && !$smartShuffleMode ? 'text-primary' : ''}">
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
  </div>
</div>

{#if error}
  <p class="mb-3 text-sm text-destructive">{error}</p>
{/if}
{#if loading}
  <p class="mb-3 text-sm text-muted-foreground">Loading liked songs…</p>
{/if}

{#if songs.length}
  <div class="page-section mt-2">
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
        {@const isCurrentTrack = song.id === currentTrackId}
        <SongContextMenu {song} onplay={() => playSong(index)}>
          <button
            class="stagger-row group grid w-full items-center gap-4 rounded-md px-4 py-2.5 text-left transition-colors duration-150 hover:bg-white/5 {isCurrentTrack ? 'bg-primary/5' : ''}"
            style="grid-template-columns: 2.5rem 1fr 1fr 4rem"
            style:--stagger-index={index}
            onclick={() => isCurrentTrack ? togglePlayRequest.update(n => n + 1) : playSong(index)}
          >
            <!-- Track # / Play icon crossfade -->
            <span class="relative flex h-7 w-7 shrink-0 items-center justify-center mx-auto">
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
                <span class="absolute inset-0 flex items-center justify-center text-sm tabular-nums text-muted-foreground transition-all duration-150 group-hover:scale-50 group-hover:opacity-0">{index + 1}</span>
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
                  {initials(song.title)}
                </div>
              {/if}
              <div class="min-w-0">
                <div class="flex min-w-0 items-center gap-2">
                  <p class="whitespace-normal break-words text-sm font-medium leading-tight transition-colors duration-150 group-hover:text-foreground {isCurrentTrack ? 'text-primary' : ''}">{song.title}</p>
                  <SongTechBadge
                    id={song.id}
                    cached={desktopPlayback ? cachedSongIds.has(song.id) : null}
                    audioFormat={song.audioFormat}
                    bitrateKbps={song.bitrateKbps}
                    compact
                  />
                </div>
                <SongArtistLinks
                  artist={song.artist}
                  class="truncate text-xs text-muted-foreground"
                  linkClass="hover:underline hover:text-foreground transition-colors duration-150"
                />
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
  <p class="text-sm text-muted-foreground">No starred songs found on your Subsonic server.</p>
{/if}
