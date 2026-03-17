<script lang="ts">
  import { onMount } from 'svelte';
  import { Play, Pause, Shuffle, ChevronLeft, ChevronRight, Sparkles, Mic2 } from '@lucide/svelte';

  import {
    DESKTOP_PLAYBACK_CACHE_UPDATED_EVENT,
    desktopPlaybackCachedIds,
    fetchArtistAlbums,
    fetchAlbumSongs,
    searchSongs,
    type Album,
    type Song
  } from '$lib/servers';
  import {
    getArtistInfo,
    getArtistTopTracks,
    type DiscoveryArtistInfo as ArtistInfo,
    type DiscoverySong as LastFmSong
  } from '$lib/discovery';
  import { focusTrack, playQueue, playingFrom, shuffleEnabled, addRecentlyPlayed, smartShuffleMode, enableShuffle, enableSmartShuffle, disableShuffle, isPlaying, togglePlayRequest } from '$lib/stores/player';
  import { backendSettings } from '$lib/stores/backend-settings';
  import { toast } from 'svelte-sonner';
  import SongContextMenu from '$lib/components/SongContextMenu.svelte';
  import AlbumContextMenu from '$lib/components/AlbumContextMenu.svelte';
  import ExternalSourceBadge from '$lib/components/ExternalSourceBadge.svelte';
  import SongTechBadge from '$lib/components/SongTechBadge.svelte';
  import { mergeAlbums, mergeAlbumSongs, type MergedAlbum } from '$lib/media-merge';
  import { formatClockDuration } from '$lib/utils';
  import { libraryRefresh } from '$lib/stores/ui-state';
  import { isTauri } from '$lib/tauri';
  import {
    DropdownMenu,
    DropdownMenuTrigger,
    DropdownMenuContent,
    DropdownMenuItem,
    DropdownMenuSeparator,
  } from '$lib/components/ui/dropdown-menu';

  let { data } = $props<{ data: { name: string } }>();

  let loading = $state(false);
  let error = $state('');

  let artistInfo = $state<ArtistInfo | null>(null);
  let topTracks = $state<LastFmSong[]>([]);
  let subsonicSongs = $state<Song[]>([]);
  let albums = $state<Album[]>([])
  let mergedAlbums = $state<MergedAlbum[]>([]);
  let showAllTracks = $state(false);
  let cachedSongIds = $state<Set<string>>(new Set());

  let albumSongs = $state<Record<string, Song[]>>({});
  let albumLoading = $state<Record<string, boolean>>({});
  let artistLoadVersion = 0;
  const ARTIST_LOAD_TIMEOUT_MS = 12000;

  let carouselEl = $state<HTMLDivElement | null>(null);
  const artistHref = $derived(`/artist/${encodeURIComponent(data.name)}`);
  const artistIsActive = $derived($playingFrom.href === artistHref);
  const desktopPlayback = isTauri();

  function withTimeout<T>(promise: Promise<T>, ms: number, label: string): Promise<T> {
    return new Promise<T>((resolve, reject) => {
      const timeout = window.setTimeout(() => {
        reject(new Error(`${label} timed out.`));
      }, ms);

      promise.then(
        (value) => {
          window.clearTimeout(timeout);
          resolve(value);
        },
        (reason) => {
          window.clearTimeout(timeout);
          reject(reason);
        }
      );
    });
  }

  async function loadArtist(name: string) {
    const loadVersion = ++artistLoadVersion;
    loading = true;
    error = '';
    artistInfo = null;
    topTracks = [];
    subsonicSongs = [];
    albums = [];
    mergedAlbums = [];
    albumSongs = {};
    albumLoading = {};
    showAllTracks = false;
    try {
      const [infoResult, topResult, subResult, albumResult] = await Promise.allSettled([
        withTimeout(getArtistInfo(name), ARTIST_LOAD_TIMEOUT_MS, 'Artist metadata load'),
        withTimeout(getArtistTopTracks(name, 10), ARTIST_LOAD_TIMEOUT_MS, 'Artist top tracks load'),
        withTimeout(searchSongs(name, 30), ARTIST_LOAD_TIMEOUT_MS, 'Artist songs load'),
        withTimeout(fetchArtistAlbums(name, 24), ARTIST_LOAD_TIMEOUT_MS, 'Artist albums load')
      ]);
      if (loadVersion !== artistLoadVersion) return;
      artistInfo = infoResult.status === 'fulfilled' ? infoResult.value : null;
      topTracks = topResult.status === 'fulfilled' ? topResult.value : [];
      subsonicSongs = subResult.status === 'fulfilled' ? subResult.value : [];
      albums = albumResult.status === 'fulfilled' ? albumResult.value : [];
      mergedAlbums = mergeAlbums(albums);
      if (!desktopPlayback) {
        cachedSongIds = new Set();
      } else {
        desktopPlaybackCachedIds(subsonicSongs)
          .then((ids) => {
            if (loadVersion !== artistLoadVersion) return;
            cachedSongIds = new Set(ids);
          })
          .catch(() => {
            if (loadVersion !== artistLoadVersion) return;
            cachedSongIds = new Set();
          });
      }

      if (!artistInfo && topTracks.length === 0 && subsonicSongs.length === 0 && albums.length === 0) {
        throw new Error('Failed to load artist.');
      }
    } catch (err) {
      if (loadVersion !== artistLoadVersion) return;
      error = err instanceof Error ? err.message : 'Failed to load artist.';
    } finally {
      if (loadVersion !== artistLoadVersion) return;
      loading = false;
    }
  }

  $effect(() => {
    const refresh = $libraryRefresh;
    const metadataProvider = $backendSettings.metadataProvider;
    const recommendationProvider = $backendSettings.recommendationProvider;
    void refresh;
    void metadataProvider;
    void recommendationProvider;
    loadArtist(data.name);
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

  function findSubsonicMatch(title: string): Song | null {
    const needle = title.toLowerCase();
    return subsonicSongs.find((s) => s.title.toLowerCase() === needle) ?? null;
  }

  const playableTopTracks = $derived(
    topTracks
      .map((t) => ({ lfm: t, sub: findSubsonicMatch(t.title) }))
      .filter((r) => r.sub !== null) as Array<{ lfm: LastFmSong; sub: Song }>
  );

  const visibleTopTracks = $derived(
    showAllTracks ? playableTopTracks : playableTopTracks.slice(0, 5)
  );

  function playTopTrack(index: number) {
    const list = playableTopTracks.map((r) => r.sub!);
    const song = list[index];
    focusTrack.set({ title: song.title, artist: song.artist, imageUrl: song.coverArtUrl, source: 'library', album: song.album });
    playQueue(list, index);
    playingFrom.set({ type: 'artist', name: data.name, href: `/artist/${encodeURIComponent(data.name)}` });
  }

  function playAllTopTracks() {
    const list = playableTopTracks.map((r) => r.sub!);
    if (!list.length) return;
    const ordered = ($shuffleEnabled || $smartShuffleMode) ? [...list].sort(() => Math.random() - 0.5) : list;
    focusTrack.set({ title: ordered[0].title, artist: ordered[0].artist, imageUrl: ordered[0].coverArtUrl, source: 'library', album: ordered[0].album });
    playQueue(ordered, 0);
    playingFrom.set({ type: 'artist', name: data.name, href: `/artist/${encodeURIComponent(data.name)}` });
  }

  async function ensureMergedAlbumSongs(album: MergedAlbum): Promise<Song[]> {
    if (!albumSongs[album.id]) {
      albumLoading = { ...albumLoading, [album.id]: true };
      try {
        const fetchedResults = await Promise.allSettled(
          album.sourceIds.map((sourceId) =>
            withTimeout(fetchAlbumSongs(sourceId), ARTIST_LOAD_TIMEOUT_MS, 'Album tracks load').catch(() => [])
          )
        );
        const fetched = fetchedResults.flatMap((result) =>
          result.status === 'fulfilled' ? [result.value] : []
        );
        albumSongs = { ...albumSongs, [album.id]: mergeAlbumSongs(fetched.flat()) };
      } catch {
        albumSongs = { ...albumSongs, [album.id]: [] };
      } finally {
        albumLoading = { ...albumLoading, [album.id]: false };
      }
    }

    return albumSongs[album.id] ?? [];
  }

  async function loadAndPlayAlbum(album: MergedAlbum, startIndex = 0) {
    const songs = await ensureMergedAlbumSongs(album);
    if (!songs?.length) return;
    const song = songs[startIndex];
    focusTrack.set({ title: song.title, artist: song.artist, imageUrl: song.coverArtUrl, source: 'library', album: song.album });
    playQueue(songs, startIndex);
    addRecentlyPlayed({ id: album.id, name: album.name, coverArtUrl: album.coverArtUrl, href: `/album/${encodeURIComponent(album.id)}`, type: 'album' });
    playingFrom.set({ type: 'album', name: album.name, href: `/album/${encodeURIComponent(album.id)}` });
  }

  // Shuffle dropdown
  let smartShuffleFetching = $state(false);
  let shuffleAllArtist = $state(false);
  const lastFmApiKey = $derived($backendSettings.lastFmApiKey);

  function activateShuffle() {
    smartShuffleMode.set(false);
    enableShuffle();
    shuffleAllArtist = false;
  }

  function activateSmartShuffle() {
    enableSmartShuffle();
    shuffleAllArtist = false;
  }

  function activateShuffleAll() {
    smartShuffleMode.set(false);
    enableShuffle();
    shuffleAllArtist = true;
  }

  function deactivateShuffle() {
    disableShuffle();
    shuffleAllArtist = false;
  }

  async function playOrShuffleAll() {
    if (shuffleAllArtist) {
        const toastId = toast.loading(`Loading all ${data.name} songs…`);
      try {
        const allSongResults = await Promise.allSettled(
          mergedAlbums.flatMap((album) =>
            album.sourceIds.map((sourceId) =>
              withTimeout(fetchAlbumSongs(sourceId), ARTIST_LOAD_TIMEOUT_MS, 'Album tracks load').catch(() => [])
            )
          )
        );
        const allSongs = mergeAlbumSongs(
          allSongResults.flatMap((result) => (result.status === 'fulfilled' ? result.value : []))
        );
        if (!allSongs.length) throw new Error('No songs found');
        for (let i = allSongs.length - 1; i > 0; i--) {
          const j = Math.floor(Math.random() * (i + 1));
          [allSongs[i], allSongs[j]] = [allSongs[j], allSongs[i]];
        }
        playQueue(allSongs, 0);
        playingFrom.set({ type: 'artist', name: data.name, href: `/artist/${encodeURIComponent(data.name)}` });
        toast.success(`Shuffling all ${data.name} songs`, { id: toastId });
      } catch {
        toast.error('Failed to load artist songs', { id: toastId });
      }
    } else {
      playAllTopTracks();
    }
  }

  function scrollCarousel(dir: -1 | 1) {
    if (!carouselEl) return;
    carouselEl.scrollBy({ left: dir * 220, behavior: 'smooth' });
  }

  function fmtListeners(n: number): string {
    if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M listeners`;
    if (n >= 1_000) return `${(n / 1_000).toFixed(0)}K listeners`;
    return `${n} listeners`;
  }

  function fmt(seconds: number): string {
    return formatClockDuration(seconds);
  }

  function initials(name: string): string {
    return name.split(' ').filter(Boolean).slice(0, 2).map((p) => p[0]?.toUpperCase() ?? '').join('');
  }
</script>

<!-- Hero -->
<div class="page-hero app-glass relative -mx-4 -mt-4 mb-8 overflow-hidden rounded-[2rem]">
  {#if artistInfo?.imageUrl}
    <div class="absolute inset-0">
      <img class="h-full w-full object-cover object-top opacity-30" src={artistInfo.imageUrl} alt="" aria-hidden="true" />
      <div class="absolute inset-0 bg-gradient-to-b from-black/30 via-background/35 to-background"></div>
    </div>
  {:else}
    <div class="absolute inset-0 bg-gradient-to-br from-primary/18 via-background/20 to-background"></div>
  {/if}

  <div class="relative grid gap-8 px-6 pb-7 pt-8 sm:px-8 lg:grid-cols-[minmax(0,1fr)_220px] lg:items-end lg:gap-10 lg:pb-9 lg:pt-10">
    <div>
      {#if loading}
        <div class="h-16 w-56 animate-pulse rounded bg-white/10"></div>
      {:else}
        <p class="mb-2 text-[11px] font-semibold uppercase tracking-[0.28em] text-muted-foreground">Artist</p>
        <h1 class="app-section-title max-w-3xl text-5xl font-black tracking-tight sm:text-6xl lg:text-7xl">{data.name}</h1>
        <div class="mt-4 flex flex-wrap gap-2 text-xs font-medium">
          {#if artistInfo?.listeners}
            <span class="app-card-soft rounded-full px-3 py-1.5">{fmtListeners(artistInfo.listeners)}</span>
          {/if}
          {#if mergedAlbums.length > 0}
            <span class="app-card-soft rounded-full px-3 py-1.5">{mergedAlbums.length} releases</span>
          {/if}
          {#if playableTopTracks.length > 0}
            <span class="app-card-soft rounded-full px-3 py-1.5">{playableTopTracks.length} playable tracks</span>
          {/if}
        </div>
        {#if artistInfo?.tags?.length}
          <div class="mt-4 flex flex-wrap gap-2">
            {#each artistInfo.tags as tag (tag)}
              <span class="app-card-soft rounded-full px-3 py-1 text-xs font-medium">{tag}</span>
            {/each}
          </div>
        {/if}
      {/if}
      <div class="mt-6 flex items-center gap-3">
        <button
          class="grid size-14 shrink-0 place-items-center rounded-full bg-white text-black shadow-[0_16px_40px_rgba(0,0,0,0.35)] transition duration-200 hover:scale-[1.04] disabled:opacity-40"
          onclick={() => artistIsActive ? togglePlayRequest.update((n) => n + 1) : playOrShuffleAll()}
          disabled={playableTopTracks.length === 0 && !shuffleAllArtist}
          aria-label={artistIsActive && $isPlaying ? 'Pause' : 'Play'}
        >
          {#if artistIsActive && $isPlaying}
            <Pause class="size-6" fill="currentColor" />
          {:else}
            <Play class="size-6 translate-x-0.5" fill="currentColor" />
          {/if}
        </button>
        <!-- Shuffle dropdown -->
        <DropdownMenu>
          <DropdownMenuTrigger>
            {#snippet child({ props })}
              <button
                {...props}
                class="app-round-button flex h-11 items-center gap-2 rounded-full px-4 text-sm font-medium transition-colors {$smartShuffleMode || $shuffleEnabled ? 'text-primary' : 'text-muted-foreground hover:text-foreground'}"
                aria-label="Shuffle options"
                title={$smartShuffleMode ? 'Smart Shuffle on' : $shuffleEnabled ? 'Shuffle on' : 'Shuffle off'}
                disabled={playableTopTracks.length === 0 && mergedAlbums.length === 0}
              >
                {#if $smartShuffleMode}
                  <Sparkles class="size-4 {smartShuffleFetching ? 'animate-pulse' : ''}" />
                  Smart Shuffle
                {:else if $shuffleEnabled}
                  <Shuffle class="size-4" />
                  Shuffle On
                {:else}
                  <Shuffle class="size-4" />
                  Shuffle
                {/if}
              </button>
            {/snippet}
          </DropdownMenuTrigger>
          <DropdownMenuContent align="start" class="min-w-56">
            <DropdownMenuItem onclick={activateShuffle} class="gap-3 {$shuffleEnabled && !$smartShuffleMode ? 'text-primary' : ''}">
              <Shuffle class="size-4 shrink-0" />
              <div>
                <p class="font-medium">Shuffle</p>
                <p class="text-xs text-muted-foreground">Play popular tracks in random order</p>
              </div>
              {#if $shuffleEnabled && !$smartShuffleMode}
                <span class="ml-auto size-1.5 rounded-full bg-primary"></span>
              {/if}
            </DropdownMenuItem>
            <DropdownMenuItem onclick={activateSmartShuffle} class="gap-3 {$smartShuffleMode ? 'text-primary' : ''}">
              <Sparkles class="size-4 shrink-0" />
              <div>
                <p class="font-medium">Smart Shuffle</p>
                <p class="text-xs text-muted-foreground">Weaves in Last.fm recommendations</p>
              </div>
              {#if $smartShuffleMode}
                <span class="ml-auto size-1.5 rounded-full bg-primary"></span>
              {/if}
            </DropdownMenuItem>
            <DropdownMenuSeparator />
            <DropdownMenuItem onclick={activateShuffleAll} disabled={mergedAlbums.length === 0} class="gap-3 {shuffleAllArtist ? 'text-primary' : ''}">
              <Mic2 class="size-4 shrink-0" />
              <div>
                <p class="font-medium">Shuffle All Songs</p>
                <p class="truncate max-w-40 text-xs text-muted-foreground">{data.name} · entire discography</p>
              </div>
              {#if shuffleAllArtist}<span class="ml-auto size-1.5 rounded-full bg-primary"></span>{/if}
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
    </div>

    <div class="app-card hidden h-[250px] overflow-hidden rounded-[1.75rem] lg:flex">
      {#if artistInfo?.imageUrl}
        <img class="h-full w-full object-cover" src={artistInfo.imageUrl} alt={data.name} loading="lazy" />
      {:else}
        <div class="grid h-full w-full place-items-center bg-gradient-to-br from-secondary to-accent text-5xl font-black">
          {initials(data.name)}
        </div>
      {/if}
    </div>
  </div>
</div>

{#if error}
  <p class="mb-4 text-sm text-destructive">{error}</p>
{/if}

<!-- Popular -->
{#if loading}
  <div class="mb-6 space-y-2">
    {#each Array(5) as _, i (i)}
      <div class="flex h-12 items-center gap-3 rounded-md px-3">
        <div class="h-4 w-4 animate-pulse rounded bg-muted"></div>
        <div class="size-10 animate-pulse rounded bg-muted"></div>
        <div class="h-4 w-48 animate-pulse rounded bg-muted"></div>
      </div>
    {/each}
  </div>
{:else if playableTopTracks.length > 0}
  <section class="page-section mb-8">
    <div class="mb-4 flex items-end justify-between gap-4">
      <div>
        <p class="text-[11px] font-semibold uppercase tracking-[0.28em] text-muted-foreground/60">Highlights</p>
        <h2 class="app-section-title text-xl font-bold">Popular</h2>
      </div>
    </div>
    <div class="space-y-1">
      {#each visibleTopTracks as { lfm, sub }, i (lfm.id)}
        <SongContextMenu song={sub} onplay={() => playTopTrack(i)}>
          <button
            class="stagger-row group flex w-full items-center gap-3 rounded-md px-3 py-2.5 text-left transition-colors duration-150 hover:bg-white/5"
            style:--stagger-index={i}
            onclick={() => playTopTrack(i)}
          >
            <span class="w-5 shrink-0 text-center text-sm tabular-nums text-muted-foreground group-hover:hidden">{i + 1}</span>
            <span class="hidden w-5 shrink-0 place-items-center group-hover:grid">
              <Play class="size-3.5" fill="currentColor" />
            </span>
            {#if sub.coverArtUrl}
              <img class="size-10 shrink-0 rounded object-cover" src={sub.coverArtUrl} alt={sub.title} loading="lazy" />
            {:else}
              <div class="grid size-10 shrink-0 place-items-center rounded bg-secondary text-xs font-bold">{initials(sub.title)}</div>
            {/if}
            <div class="min-w-0 flex-1">
              <div class="flex items-center gap-2">
                <p class="whitespace-normal break-words text-sm font-medium leading-tight">{sub.title}</p>
                <SongTechBadge
                  id={sub.id}
                  cached={desktopPlayback ? cachedSongIds.has(sub.id) : null}
                  audioFormat={sub.audioFormat}
                  bitrateKbps={sub.bitrateKbps}
                  compact
                />
              </div>
              {#if lfm.listeners}
                <p class="text-xs text-muted-foreground">{lfm.listeners.toLocaleString()} plays</p>
              {/if}
            </div>
            <span class="text-xs tabular-nums text-muted-foreground">{fmt(sub.duration)}</span>
          </button>
        </SongContextMenu>
      {/each}
    </div>
    {#if playableTopTracks.length > 5}
      <button
        class="mt-2 px-3 text-sm font-semibold text-muted-foreground hover:text-foreground"
        onclick={() => (showAllTracks = !showAllTracks)}
      >{showAllTracks ? 'Show less' : `See ${playableTopTracks.length - 5} more`}</button>
    {/if}
  </section>
{/if}

<!-- Albums carousel -->
{#if mergedAlbums.length > 0}
  <section class="page-section mb-8">
    <div class="mb-3 flex items-center justify-between">
      <div>
        <p class="text-[11px] font-semibold uppercase tracking-[0.28em] text-muted-foreground/60">Collection</p>
        <h2 class="app-section-title text-xl font-bold">Discography</h2>
      </div>
      <div class="flex gap-1">
        <button class="app-round-button grid size-8 place-items-center rounded-full" onclick={() => scrollCarousel(-1)} aria-label="Scroll left">
          <ChevronLeft class="size-4" />
        </button>
        <button class="app-round-button grid size-8 place-items-center rounded-full" onclick={() => scrollCarousel(1)} aria-label="Scroll right">
          <ChevronRight class="size-4" />
        </button>
      </div>
    </div>
    <div bind:this={carouselEl} class="flex gap-4 overflow-x-auto pb-3" style="scrollbar-width:none;-ms-overflow-style:none">
      {#each mergedAlbums as album, index (album.id)}
        <AlbumContextMenu album={album} onplay={() => loadAndPlayAlbum(album)}>
        <div class="app-card app-hover stagger-item group relative flex w-44 shrink-0 flex-col gap-2 rounded-2xl p-3 text-left" style={`--stagger-index:${index};`}>
          <a
            href="/album/{encodeURIComponent(album.id)}"
            class="absolute inset-0 z-10 rounded-2xl"
            aria-label="Open {album.name}"
          ></a>
          <div class="relative w-full">
            {#if album.coverArtUrl}
              <img class="aspect-square w-full rounded-md object-cover shadow-md" src={album.coverArtUrl} alt={album.name} loading="lazy" />
            {:else}
              <div class="grid aspect-square w-full place-items-center rounded-md bg-gradient-to-br from-slate-600 to-slate-800 text-xl font-bold">{initials(album.name)}</div>
            {/if}
            <div class="absolute bottom-2 right-2 z-20 grid size-10 translate-y-1 place-items-center rounded-full bg-primary text-primary-foreground opacity-0 shadow-lg transition group-hover:translate-y-0 group-hover:opacity-100">
              <button
                onclick={(e) => { e.preventDefault(); loadAndPlayAlbum(album); }}
                aria-label="Play {album.name}"
                class="grid size-full place-items-center rounded-full"
              >
                {#if albumLoading[album.id]}
                  <span class="block size-4 animate-spin rounded-full border-2 border-primary-foreground border-t-transparent"></span>
                {:else}
                  <Play class="size-4 translate-x-px" fill="currentColor" />
                {/if}
              </button>
            </div>
          </div>
          <div class="min-w-0">
            <div class="flex items-center gap-2">
              <p class="truncate text-sm font-semibold">{album.name}</p>
              <ExternalSourceBadge id={album.id} class="shrink-0" />
            </div>
            <p class="text-xs text-muted-foreground">{album.year ? `${album.year} · ` : ''}{album.songCount} songs</p>
          </div>
        </div>
        </AlbumContextMenu>
      {/each}
    </div>
  </section>
{/if}

<!-- Fans also like -->
{#if artistInfo?.similarArtists?.length}
  <section class="page-section mb-8">
    <div class="mb-4">
      <p class="text-[11px] font-semibold uppercase tracking-[0.28em] text-muted-foreground/60">Related</p>
      <h2 class="app-section-title text-xl font-bold">Fans also like</h2>
    </div>
    <div class="flex flex-wrap gap-3">
      {#each artistInfo.similarArtists as artist, index (artist.name)}
        <a
          href="/artist/{encodeURIComponent(artist.name)}"
          class="app-card app-hover stagger-item flex flex-col items-center gap-2 rounded-2xl p-3 text-center"
          style={`--stagger-index:${index};`}
        >
          {#if artist.imageUrl}
            <img class="size-16 rounded-full object-cover" src={artist.imageUrl} alt={artist.name} loading="lazy" />
          {:else}
            <div class="grid size-16 place-items-center rounded-full bg-gradient-to-br from-slate-500 to-slate-700 text-sm font-bold">{initials(artist.name)}</div>
          {/if}
          <p class="max-w-[80px] truncate text-xs font-medium">{artist.name}</p>
        </a>
      {/each}
    </div>
  </section>
{/if}
