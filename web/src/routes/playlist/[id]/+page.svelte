<script lang="ts">
  import { fetchPlaylistDetail, type Song } from '$lib/servers';
  import { Play, Pause, Shuffle, Sparkles } from '@lucide/svelte';
  import { startQueue, playingFrom, activateShuffle, smartShuffleMode, shuffleEnabled, enableSmartShuffle, disableShuffle, isPlaying, togglePlayRequest } from '$lib/stores/player';
  import { SongRow } from '$lib/components/media';
  import { formatClockDuration, initials, shuffleArray, sumDuration } from '$lib/utils';
  import { libraryRefresh } from '$lib/stores/ui-state';
  import { DesktopCache } from '$lib/hooks/use-desktop-cache.svelte';
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
  let songs = $state<Song[]>([]);
  const cache = new DesktopCache();
  let playlistLoadVersion = 0;
  const playlistHref = $derived(`/playlist/${encodeURIComponent(data.id)}`);
  const playlistIsActive = $derived($playingFrom.href === playlistHref);

  $effect(() => {
    const id = data.id;
    const refresh = $libraryRefresh;
    const loadVersion = ++playlistLoadVersion;
    void refresh;

    // Reset immediately so the old playlist doesn't flash while loading
    loading = true;
    error = '';
    playlistName = '';
    coverArtUrl = '';
    songCount = 0;
    songs = [];
    cache.reset();

    fetchPlaylistDetail(id)
      .then((detail) => {
        if (loadVersion !== playlistLoadVersion) return;
        playlistName = detail.playlist.name;
        coverArtUrl = detail.playlist.coverArtUrl;
        songCount = detail.playlist.songCount;
        songs = detail.songs;
        void cache.load(detail.songs, loadVersion, () => playlistLoadVersion);
      })
      .catch((err) => {
        if (loadVersion !== playlistLoadVersion) return;
        error = err instanceof Error ? err.message : 'Failed to load playlist.';
      })
      .finally(() => {
        if (loadVersion !== playlistLoadVersion) return;
        loading = false;
      });
  });




  function playSong(index: number) {
    if (!songs[index]) return;
    startQueue(songs, index, { type: 'playlist', name: playlistName, href: playlistHref });
  }

  function playAll() {
    if (!songs.length) return;
    const list = ($shuffleEnabled || $smartShuffleMode) ? shuffleArray(songs) : songs;
    startQueue(list, 0, { type: 'playlist', name: playlistName, href: playlistHref });
  }
</script>

<!-- Hero -->
<div class="page-hero app-glass mb-6 flex gap-4 rounded-[2rem] p-5">
  {#if loading}
    <div class="h-36 w-36 shrink-0 animate-pulse rounded-2xl bg-muted"></div>
    <div class="flex flex-col justify-end gap-3 flex-1">
      <div class="h-3 w-16 animate-pulse rounded-full bg-muted"></div>
      <div class="h-8 w-48 animate-pulse rounded-lg bg-muted"></div>
      <div class="h-3 w-32 animate-pulse rounded-full bg-muted"></div>
      <div class="mt-1 h-14 w-14 animate-pulse rounded-full bg-muted"></div>
    </div>
  {:else}
    {#if coverArtUrl}
      <img class="h-36 w-36 shrink-0 rounded-2xl object-cover shadow-lg" src={coverArtUrl} alt={playlistName} />
    {:else}
      <div class="app-card flex h-36 w-36 shrink-0 items-center justify-center rounded-2xl bg-gradient-to-br from-secondary to-accent text-2xl font-black shadow-lg">
        {initials(playlistName || '?')}
      </div>
    {/if}
    <div class="flex flex-col justify-end gap-2">
      <p class="text-xs font-semibold uppercase tracking-widest text-muted-foreground">Playlist</p>
      <h2 class="app-section-title text-3xl font-bold tracking-tight">{playlistName || '…'}</h2>
      {#if songs.length}
        <p class="text-sm text-muted-foreground">
          {songCount} songs · {formatClockDuration(sumDuration(songs))}
        </p>
      {/if}
      <div class="flex items-center gap-3 mt-1">
        <button
          onclick={() => playlistIsActive ? togglePlayRequest.update((n) => n + 1) : playAll()}
          disabled={!songs.length}
          class="flex size-14 shrink-0 items-center justify-center rounded-full bg-primary text-primary-foreground shadow-lg transition-all duration-150 hover:scale-105 hover:brightness-110 active:scale-95 disabled:opacity-40 disabled:cursor-not-allowed"
          aria-label={playlistIsActive && $isPlaying ? 'Pause playlist' : 'Play playlist'}
        >
          {#if playlistIsActive && $isPlaying}
            <Pause class="size-6" fill="currentColor" />
          {:else}
            <Play class="size-6 translate-x-0.5" fill="currentColor" />
          {/if}
        </button>

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
  {/if}
</div>

{#if error}
  <p class="mb-3 text-sm text-destructive">{error}</p>
{/if}

<!-- Track list -->
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

  {#if loading}
    <div class="mt-1 space-y-0.5">
      {#each Array(8) as _, i (i)}
        <div class="grid items-center gap-3 rounded-lg px-3 py-2.5" style="grid-template-columns: 2rem 2.5rem 1fr 1fr 4rem">
          <div class="h-3 w-3 mx-auto animate-pulse rounded-full bg-muted"></div>
          <div class="h-10 w-10 animate-pulse rounded-md bg-muted"></div>
          <div class="space-y-2">
            <div class="h-3 animate-pulse rounded-full bg-muted" style="width:{50 + (i * 17) % 35}%"></div>
            <div class="h-2.5 animate-pulse rounded-full bg-muted" style="width:{25 + (i * 11) % 25}%"></div>
          </div>
          <div class="hidden h-2.5 animate-pulse rounded-full bg-muted md:block" style="width:{40 + (i * 13) % 30}%"></div>
          <div class="ml-auto h-2.5 w-8 animate-pulse rounded-full bg-muted"></div>
        </div>
      {/each}
    </div>
  {:else if songs.length}
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
  {:else if !error}
    <p class="py-8 text-center text-sm text-muted-foreground">This playlist has no tracks.</p>
  {/if}
</div>
