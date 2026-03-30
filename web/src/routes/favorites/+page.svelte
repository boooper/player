<script lang="ts">
  import { Play, Pause, Shuffle, Heart, Sparkles } from '@lucide/svelte';

  import { fetchStarredSongs, type Song } from '$lib/servers';
  import { startQueue, playingFrom, starredSongIds, activateShuffle, smartShuffleMode, shuffleEnabled, isPlaying, togglePlayRequest, enableSmartShuffle, disableShuffle } from '$lib/stores/player';
  import { SongRow } from '$lib/components/media';
  import { formatClockDuration, shuffleArray, sumDuration } from '$lib/utils';
  import {
    DropdownMenu,
    DropdownMenuTrigger,
    DropdownMenuContent,
    DropdownMenuItem,
    DropdownMenuSeparator,
  } from '$lib/components/ui/dropdown-menu';
  import { libraryRefresh } from '$lib/stores/ui-state';
  import { DesktopCache } from '$lib/hooks/use-desktop-cache.svelte';

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
  const cache = new DesktopCache();

  let favoritesLoadVersion = 0;

  function loadFavorites() {
    const loadVersion = ++favoritesLoadVersion;
    loading = true;
    error = '';
    fetchStarredSongs()
      .then((s) => {
        if (loadVersion !== favoritesLoadVersion) return;
        allStarredSongs = s;
        void cache.load(s, loadVersion, () => favoritesLoadVersion);
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



  function playSong(index: number) {
    if (!songs[index]) return;
    startQueue(songs, index, { type: 'favorites', name: 'Liked Songs', href: favoritesHref });
  }

  function playAll() {
    if (!songs.length) return;
    const list = ($shuffleEnabled || $smartShuffleMode) ? shuffleArray(songs) : songs;
    startQueue(list, 0, { type: 'favorites', name: 'Liked Songs', href: favoritesHref });
  }

</script>

<div class="page-hero app-glass mb-6 flex gap-4 rounded-[2rem] p-5">
  <div class="relative h-36 w-36 shrink-0 overflow-hidden rounded-2xl shadow-lg">
    {#if songs.length >= 4}
      <div class="grid h-full w-full grid-cols-2 grid-rows-2">
        {#each songs.slice(0, 4) as song (song.id)}
          <img class="h-full w-full object-cover" src={song.coverArtUrl} alt={song.title} />
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
    {#if songs.length}
      <p class="text-sm text-muted-foreground">
        {favoriteSongCount} songs · {formatClockDuration(sumDuration(songs))}
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
        <DropdownMenuTrigger tooltip={$smartShuffleMode ? 'Smart Shuffle on' : $shuffleEnabled ? 'Shuffle on' : 'Shuffle'}>
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
    <div class="grid items-center gap-3 border-b border-border/40 px-3 pb-2 text-xs font-medium uppercase tracking-wider text-muted-foreground/50"
         style="grid-template-columns: 2rem 2.5rem 1fr 1fr 4rem">
      <span class="text-center">#</span>
      <span></span>
      <span>Title</span>
      <span class="hidden md:block">Album</span>
      <span class="text-right">Duration</span>
    </div>

    <!-- Rows -->
    <div class="mt-1 space-y-0.5">
      {#each songs as song, index (song.id + '-' + index)}
        <SongRow
          {song}
          {index}
          showAlbum
          onplay={() => playSong(index)}
          cached={cache.enabled ? cache.ids.has(song.id) : null}
          staggerIndex={index}
        />
      {/each}
    </div>
  </div>
{:else if !loading && !error}
  <p class="text-sm text-muted-foreground">No starred songs found on your Subsonic server.</p>
{/if}

