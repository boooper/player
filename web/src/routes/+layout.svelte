<script lang="ts">
  import '../app.css';
  import { onMount } from 'svelte';
  import { afterNavigate, goto } from '$app/navigation';

  import { Library, Plus, Cog, ChevronLeft, ChevronRight, Minus, Square, Minimize2, X, Home, Search as SearchIcon } from '@lucide/svelte';
  import { IsMobile } from '$lib/hooks/is-mobile.svelte';
  import { page } from '$app/stores';
  import PlayerBar from '$lib/components/PlayerBar.svelte';
  import NowPlayingPanel from '$lib/components/NowPlayingPanel.svelte';
  import SearchBar from '$lib/components/SearchBar.svelte';
  import { getArtistArtworkMap } from '$lib/discovery';
  import { withTimeout } from '$lib/utils';
  import { openUrl, isTauri, initTauriWindow, startDragging, minimizeWindow, toggleMaximizeWindow, closeWindow, getOsPlatform, watchMaximized } from '$lib/tauri';
  import { initDrpc, stopDrpc } from '$lib/drpc';
  import {
    fetchAppSettings,
    getActiveServerType,
    fetchListenBrainzToken,
    savePlaybackPrefs,
    fetchLikedArtists,
    fetchServiceHealth,
    fetchPlaylistSongs,
    fetchPlaylists,
    fetchStarredSongs,
    fetchLyrics,
    type LyricsResult,
    type Song,
  } from '$lib/servers';
  import {
    currentIndex,
    startQueue,
    hydratePlayerUiState,
    isPlaying,
    queue,
    shouldAutoplay,
    subsonicPlaylists,
    starredSongIds,
    addRecentlyPlayed,
    showQueue,
    playingFrom,
    showLyrics,
    currentTime,
    seekRequest,
    volume,
    shuffleEnabled,
    smartShuffleMode,
    repeatMode,
  } from '$lib/stores/player';
  import { backendSettings } from '$lib/stores/backend-settings';
  import { libraryRefresh, activeServerType } from '$lib/stores/ui-state';
  import LibraryList from '$lib/components/LibraryList.svelte';
  import {
    ScrollArea,
    Toaster,
    SidebarProvider,
    Sidebar,
    SidebarContent,
    SidebarHeader,
    ResizableHandle,
    ResizablePanel,
    ResizablePanelGroup,
  } from '$lib/components/ui';
  import { TooltipProvider, Tooltip, TooltipTrigger, TooltipContent } from '$lib/components/ui/tooltip';

  let { children } = $props();

  const LIBRARY_COMPACT_THRESHOLD = 11;
  let libraryPaneSize = $state(18);

  // Header / window state
  let searchBar = $state<{ focus: () => void } | null>(null);
  let runningInTauri = $state(false);
  let isMac = $state(false);
  let windowMaximized = $state(false);

  let libraryList = $state<{ openCreatePlaylist: () => void } | null>(null);
  const libraryCompact = $derived(libraryPaneSize <= LIBRARY_COMPACT_THRESHOLD);

  let likedArtists = $state<string[]>([]);
  let artistPhotos = $state<Record<string, string>>({});
  let starredSongs = $state<Song[]>([])
  let selectedPlaylistId = $state('');

  let loading = $state(false);
  let error = $state('');
  let lastfmStatus = $state<'checking' | 'online' | 'offline' | 'missing'>('checking');
  let subsonicStatus = $state<'checking' | 'online' | 'offline' | 'missing'>('checking');

  // Lyrics
  let lyricsData = $state<LyricsResult | null>(null);
  let lyricsLoading = $state(false);
  let lyricsTrackId = $state('');
  let lyricsScrollRef = $state<HTMLElement | null>(null);

  type LrcLine = { time: number; text: string };
  function parseLrc(lrc: string): LrcLine[] {
    const result: LrcLine[] = [];
    for (const line of lrc.split('\n')) {
      const m = line.match(/^\[(\d{2}):(\d{2})[.:]?(\d{2,3})?\](.*)/);
      if (!m) continue;
      const text = m[4].trim();
      if (!text.length) continue;
      const time = parseInt(m[1]) * 60 + parseInt(m[2]) + (m[3] ? parseInt(m[3].padEnd(3, '0')) / 1000 : 0);
      result.push({ time, text });
    }
    return result;
  }

  const parsedLyrics = $derived(lyricsData?.syncedLyrics ? parseLrc(lyricsData.syncedLyrics) : []);

  function lyricLineClass(i: number): string {
    const d = Math.abs(i - currentLyricIdx);
    if (i === currentLyricIdx) return 'bg-primary/10 text-[28px] font-extrabold text-foreground';
    if (d === 1) return 'text-xl font-semibold text-foreground/35';
    if (d === 2) return 'text-lg font-medium text-foreground/20';
    return 'text-base font-medium text-foreground/10 hover:text-foreground/30';
  }

  const currentLyricIdx = $derived.by(() => {
    if (!parsedLyrics.length) return -1;
    const t = $currentTime;
    let idx = 0;
    for (let i = 0; i < parsedLyrics.length; i++) {
      if (parsedLyrics[i].time <= t) idx = i;
      else break;
    }
    return idx;
  });

  $effect(() => {
    const track = $queue[$currentIndex];
    if (!$showLyrics || !track) return;
    if (track.id === lyricsTrackId) return;
    lyricsTrackId = track.id;
    lyricsLoading = true;
    lyricsData = null;
    fetchLyrics(track.artist, track.title, track.album, track.duration)
      .then((r) => { lyricsData = r; })
      .catch(() => { lyricsData = null; })
      .finally(() => { lyricsLoading = false; });
  });

  $effect(() => {
    const idx = currentLyricIdx;
    if (idx < 0 || !lyricsScrollRef) return;
    const el = lyricsScrollRef.querySelector<HTMLElement>(`[data-lyric-idx="${idx}"]`);
    if (!el) return;
    const target = el.offsetTop - lyricsScrollRef.clientHeight / 2 + el.offsetHeight / 2;
    lyricsScrollRef.scrollTo({ top: target, behavior: 'smooth' });
  });

  const rightOpen = $derived($showQueue);

  let playbackPrefsReady = $state(false);
  let cleanupDrpc: () => void = () => {};
  let drpcReady = $state(false);
  const discordRpcEnabled = $derived($backendSettings.discordRpcEnabled);

  // Mobile responsive
  const isMobile = new IsMobile();
  const mobilePageTitle = $derived.by(() => {
    const path = $page.url.pathname;
    if (path === '/') return 'Madrify';
    if (path.startsWith('/search')) return 'Search';
    if (path.startsWith('/songs')) return 'Songs';
    if (path.startsWith('/artist/')) return 'Artist';
    if (path === '/artist') return 'Artists';
    if (path.startsWith('/album/')) return 'Album';
    if (path === '/album') return 'Albums';
    if (path.startsWith('/favorites')) return 'Favorites';
    if (path.startsWith('/for-you')) return 'For You';
    if (path.startsWith('/playlist')) return 'Playlist';
    if (path.startsWith('/settings')) return 'Settings';
    if (path.startsWith('/library')) return 'Library';
    return 'Madrify';
  });
  const isMobileLibraryRoute = $derived(
    ['/songs', '/album', '/artist', '/favorites', '/playlist', '/for-you', '/library'].some(p => $page.url.pathname.startsWith(p))
  );

  async function bootstrapAppSettings() {
    try {
      const [settings, lbzToken] = await Promise.all([
        fetchAppSettings(),
        fetchListenBrainzToken().catch(() => ''),
        getActiveServerType().then((t) => activeServerType.set(t)).catch(() => {}),
      ]);
      backendSettings.update((current) => ({
        ...current,
        lastFmApiKey: settings.lastFmApiKey,
        recommendationProvider: settings.recommendationProvider,
        metadataProvider: settings.metadataProvider,
        listenBrainzUsername: settings.listenBrainzUsername,
        listenBrainzToken: lbzToken,
        crossfadeSeconds: settings.crossfadeSeconds,
        gaplessEnabled: settings.gaplessEnabled,
        normalizationEnabled: settings.normalizationEnabled,
        normalizationMode: settings.normalizationMode,
        eqEnabled: settings.eqEnabled,
        eqPreset: settings.eqPreset,
        eqBands: settings.eqBands,
        discordRpcEnabled: settings.discordRpcEnabled,
      }));
      volume.set(settings.volume);
      shuffleEnabled.set(settings.shuffleEnabled);
      smartShuffleMode.set(settings.smartShuffleMode);
      repeatMode.set(settings.repeatMode);
    } catch {
      // Keep store defaults when settings API is unavailable.
    } finally {
      playbackPrefsReady = true;
    }
  }

  let _prefsSaveTimer = 0;
  $effect(() => {
    if (!playbackPrefsReady) return;
    const s = $shuffleEnabled;
    const sm = $smartShuffleMode;
    const rm = $repeatMode;
    clearTimeout(_prefsSaveTimer);
    _prefsSaveTimer = window.setTimeout(() => savePlaybackPrefs(s, sm, rm), 300);
  });

  $effect(() => {
    if (!drpcReady) return;
    cleanupDrpc();
    cleanupDrpc = discordRpcEnabled ? initDrpc() : () => {};
  });

  $effect(() => {
    const v = $libraryRefresh;
    if (v > 0) {
      reloadLibraryData();
      getActiveServerType().then((t) => activeServerType.set(t)).catch(() => {});
    }
  });

  async function reloadLibraryData() {
    loading = true;
    error = '';
    const [likedResult, playlistsResult, starredResult, healthResult] = await Promise.allSettled([
      withTimeout(fetchLikedArtists(), 15_000),
      withTimeout(fetchPlaylists(), 15_000),
      withTimeout(fetchStarredSongs(), 15_000),
      withTimeout(fetchServiceHealth(), 10_000),
    ]);

    try {
      if (likedResult.status !== 'fulfilled') {
        throw likedResult.reason;
      }
      const stored = likedResult.value;
      likedArtists = stored.map((item) => item.name);
      fetchArtistPhotos(likedArtists);
    } catch (err) {
      error = err instanceof Error ? err.message : (typeof err === 'string' ? err : 'Failed to load library.');
    } finally {
      loading = false;
    }

    if (playlistsResult.status === 'fulfilled') {
      subsonicPlaylists.set(playlistsResult.value);
    }

    if (starredResult.status === 'fulfilled') {
      const starred = starredResult.value;
      starredSongs = starred;
      starredSongIds.set(new Set(starred.map((s) => s.id)));
    }

    try {
      if (healthResult.status !== 'fulfilled') {
        throw healthResult.reason;
      }
      const payload = healthResult.value;
      lastfmStatus = payload?.lastfm ?? 'offline';
      subsonicStatus = payload?.subsonic ?? 'offline';
    } catch {
      lastfmStatus = 'offline';
      subsonicStatus = 'offline';
    }
  }

  async function playPlaylist(playlistId: string) {
    if (!playlistId) return;
    selectedPlaylistId = playlistId;
    try {
      const songs = await fetchPlaylistSongs(playlistId);
      if (!songs.length) return;
      const pl = $subsonicPlaylists.find((p) => p.id === playlistId);
      if (pl) {
        const href = `/playlist/${encodeURIComponent(pl.id)}`;
        startQueue(songs, 0, { type: 'playlist', name: pl.name, href });
        addRecentlyPlayed({ id: pl.id, name: pl.name, coverArtUrl: pl.coverArtUrl, href, type: 'playlist' });
      }
      shouldAutoplay.set(true);
    } catch {}
  }

  async function fetchArtistPhotos(names: string[]) {
    const missing = names.filter((n) => artistPhotos[n] === undefined);
    if (!missing.length) return;
    const patch: Record<string, string> = {};
    for (const n of missing) patch[n] = '';
    artistPhotos = { ...artistPhotos, ...patch };
    const next = await getArtistArtworkMap(missing);
    artistPhotos = { ...artistPhotos, ...next };
  }

  function playSongFromDropdown(song: Song) {
    startQueue([song], 0, { type: 'artist', name: song.artist, href: `/artist/${encodeURIComponent(song.artist)}` });
    shouldAutoplay.set(true);
  }

  function gotoArtistFromDropdown(name: string) {
    goto(`/artist/${encodeURIComponent(name)}`);
  }

  onMount(() => {
    document.documentElement.classList.add('dark');
    drpcReady = true;

    runningInTauri = isTauri();
    if (runningInTauri) {
      isMac = getOsPlatform() === 'macos';
      void initTauriWindow();
      void watchMaximized((m) => { windowMaximized = m; });
    }

    function handleExternalLink(e: MouseEvent) {
      const target = (e.target as HTMLElement).closest('a');
      if (!target || !target.href) return;
      try {
        const url = new URL(target.href);
        if (url.origin !== window.location.origin) {
          e.preventDefault();
          openUrl(target.href);
        }
      } catch {}
    }

    function handleKeydown(e: KeyboardEvent) {
      if (e.code !== 'Space') return;
      const tag = (e.target as HTMLElement).tagName;
      if (tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT' || (e.target as HTMLElement).isContentEditable) return;
      e.preventDefault();
      window.dispatchEvent(new CustomEvent('player:toggle-play'));
    }

    document.addEventListener('click', handleExternalLink);
    document.addEventListener('keydown', handleKeydown);

    void hydratePlayerUiState();
    Promise.all([bootstrapAppSettings(), reloadLibraryData()]);

    return () => {
      document.removeEventListener('click', handleExternalLink);
      document.removeEventListener('keydown', handleKeydown);
      cleanupDrpc();
      void stopDrpc();
      drpcReady = false;
    };
  });

  afterNavigate(() => {
    showLyrics.set(false);
  });
</script>

{#if isMobile.current}
<!-- ─── MOBILE LAYOUT ─────────────────────────────────────────────── -->
<TooltipProvider>
<div class="fixed inset-0 flex flex-col overflow-hidden bg-background">

  <!-- Mobile header: sits below OS status bar -->
  <header
    class="flex shrink-0 items-center gap-2 border-b border-white/[0.06] bg-background/95 px-3 backdrop-blur-sm"
    style="-webkit-backdrop-filter:blur(20px); padding-top: calc(var(--sat) + 0.5rem); padding-bottom: 0.5rem; min-height: calc(var(--sat) + 3.25rem)"
  >
    <button
      onclick={() => window.history.back()}
      class="flex size-9 items-center justify-center rounded-full text-muted-foreground transition-colors active:bg-white/10"
      aria-label="Back"
    ><ChevronLeft class="size-5" /></button>
    <span class="flex-1 truncate text-[17px] font-semibold">{mobilePageTitle}</span>
    <a href="/settings" class="flex size-9 items-center justify-center rounded-full text-muted-foreground transition-colors active:bg-white/10" aria-label="Settings">
      <Cog class="size-[18px]" />
    </a>
  </header>

  <!-- Scrollable content -->
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div
    bind:this={lyricsScrollRef}
    class="flex-1 overflow-y-auto overscroll-contain"
    style="padding-bottom: calc(var(--sab) + 10rem)"
  >
    {#if $showLyrics}
      {@const track = $queue[$currentIndex] ?? null}
      <div class="relative min-h-full px-5 pb-6 pt-6">
        {#if track?.coverArtUrl}
          <img src={track.coverArtUrl} class="pointer-events-none absolute inset-0 h-full w-full scale-110 object-cover opacity-[0.06] blur-3xl" aria-hidden="true" alt="" />
        {/if}
        <div class="relative">
          {#if track}
            <div class="mb-8 flex items-center gap-4" style="animation: lyric-header-in 0.4s ease both">
              {#if track.coverArtUrl}
                <img src={track.coverArtUrl} class="size-14 rounded-xl object-cover shadow-lg ring-1 ring-border/40" alt={track.title} />
              {/if}
              <div>
                <p class="mb-0.5 text-[10px] font-semibold uppercase tracking-widest text-muted-foreground">Lyrics</p>
                <p class="text-xl font-bold leading-tight">{track.title}</p>
                <p class="text-sm text-muted-foreground">{track.artist}</p>
                {#if lyricsData?.provider}
                  <span class="mt-1.5 inline-block rounded-full border border-border/70 bg-background/40 px-2 py-0.5 text-[10px] font-semibold uppercase tracking-wide text-muted-foreground">{lyricsData.provider}</span>
                {/if}
              </div>
            </div>
          {/if}
          {#if lyricsLoading}
            <div class="flex h-48 items-center justify-center">
              <div class="w-full max-w-sm space-y-3">
                {#each [1, 0.7, 0.5, 0.8, 0.6] as w, i (i)}
                  <div class="h-5 animate-pulse rounded-full bg-foreground/10" style="width:{w * 100}%"></div>
                {/each}
              </div>
            </div>
          {:else if track && lyricsData?.instrumental}
            <div class="flex h-48 items-center justify-center">
              <p class="text-sm text-muted-foreground">This track is instrumental.</p>
            </div>
          {:else if parsedLyrics.length > 0}
            <div class="py-4">
              {#each parsedLyrics as line, i (line.time)}
                <button type="button" data-lyric-idx={i}
                  style="animation: lyric-in 0.35s ease both; animation-delay: {Math.min(i * 18, 400)}ms"
                  class="group relative mb-2 block w-full cursor-pointer select-none rounded-lg px-3 py-1.5 text-left transition-[font-size,color,opacity,background-color] duration-300 ease-out {lyricLineClass(i)}"
                  onclick={() => seekRequest.set(line.time)}
                >
                  {#if i === currentLyricIdx}
                    <span class="absolute left-0 top-1/2 h-[60%] w-[3px] -translate-y-1/2 rounded-full bg-primary transition-all duration-300"></span>
                  {/if}
                  {line.text}
                </button>
              {/each}
            </div>
          {:else if lyricsData?.plainLyrics}
            <div class="py-4">
              {#each lyricsData.plainLyrics.split('\n') as line, i (i)}
                <p class="mb-2 text-lg leading-relaxed {line.trim() ? 'text-foreground/70' : 'mb-5'}">{line || '\u00a0'}</p>
              {/each}
            </div>
          {:else if !lyricsLoading && track}
            <div class="flex h-48 items-center justify-center">
              <p class="text-sm text-muted-foreground">No lyrics found for this track.</p>
            </div>
          {/if}
        </div>
      </div>
    {:else}
      <div class="px-4 pb-4 pt-3">
        {@render children()}
      </div>
    {/if}
  </div>

  <!-- PlayerBar renders mini player + full player as fixed overlays -->
  <PlayerBar />

  <!-- Bottom navigation: sits above OS nav bar -->
  <nav
    class="fixed left-0 right-0 z-40 flex items-stretch border-t border-white/[0.06] bg-background/95 backdrop-blur-sm"
    style="bottom: 0; padding-bottom: var(--sab); -webkit-backdrop-filter:blur(20px)"
  >
    <a href="/"
      class="flex h-14 flex-1 flex-col items-center justify-center gap-0.5 transition-colors {$page.url.pathname === '/' ? 'text-primary' : 'text-muted-foreground'}"
    >
      <Home class="size-[22px]" />
      <span class="text-[10px] font-medium">Home</span>
    </a>
    <a href="/search"
      class="flex h-14 flex-1 flex-col items-center justify-center gap-0.5 transition-colors {$page.url.pathname.startsWith('/search') ? 'text-primary' : 'text-muted-foreground'}"
    >
      <SearchIcon class="size-[22px]" />
      <span class="text-[10px] font-medium">Search</span>
    </a>
    <a href="/library"
      class="flex h-14 flex-1 flex-col items-center justify-center gap-0.5 transition-colors {isMobileLibraryRoute ? 'text-primary' : 'text-muted-foreground'}"
    >
      <Library class="size-[22px]" />
      <span class="text-[10px] font-medium">Library</span>
    </a>
    <a href="/settings"
      class="flex h-14 flex-1 flex-col items-center justify-center gap-0.5 transition-colors {$page.url.pathname.startsWith('/settings') ? 'text-primary' : 'text-muted-foreground'}"
    >
      <Cog class="size-[22px]" />
      <span class="text-[10px] font-medium">Settings</span>
    </a>
  </nav>
</div>
<Toaster richColors />
</TooltipProvider>

{:else}
<!-- ─── DESKTOP LAYOUT ─────────────────────────────────────────────── -->
<TooltipProvider>
<!-- SVG filter defs for liquid-glass refraction -->
<svg width="0" height="0" aria-hidden="true" style="position:absolute;pointer-events:none;overflow:hidden">
  <defs>
    <filter id="lg-distort" x="-4%" y="-4%" width="108%" height="108%" color-interpolation-filters="sRGB">
      <feTurbulence type="fractalNoise" baseFrequency="0.012 0.016" numOctaves="2" seed="3" stitchTiles="stitch" result="noise"/>
      <feDisplacementMap in="SourceGraphic" in2="noise" scale="6" xChannelSelector="R" yChannelSelector="G"/>
    </filter>
  </defs>
</svg>

<div class="app-bg" aria-hidden="true"></div>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<header
  class="fixed left-0 right-0 top-0 z-20 flex h-12 shrink-0 items-center"
  onmousedown={(e) => {
    if (!runningInTauri || e.button !== 0 || e.detail >= 2) return;
    if ((e.target as HTMLElement).closest('button, a, input, textarea, [role="button"], [role="tab"]')) return;
    startDragging();
  }}
  ondblclick={(e) => {
    if (!runningInTauri) return;
    if ((e.target as HTMLElement).closest('button, a, input, textarea, [role="button"], [role="tab"]')) return;
    toggleMaximizeWindow();
  }}
>
  {#if runningInTauri && isMac}
    <div class="w-[72px] shrink-0"></div>
  {/if}

  <!-- Back / Forward -->
  <div class="flex shrink-0 items-center gap-0.5 px-2">
    <Tooltip>
      <TooltipTrigger>{#snippet child({ props })}<button {...props} class="flex size-7 items-center justify-center rounded-full text-muted-foreground transition-colors hover:bg-white/[0.06] hover:text-foreground" onclick={() => window.history.back()} aria-label="Back"><ChevronLeft class="size-3.5" /></button>{/snippet}</TooltipTrigger>
      <TooltipContent>Back</TooltipContent>
    </Tooltip>
    <Tooltip>
      <TooltipTrigger>{#snippet child({ props })}<button {...props} class="flex size-7 items-center justify-center rounded-full text-muted-foreground transition-colors hover:bg-white/[0.06] hover:text-foreground" onclick={() => window.history.forward()} aria-label="Forward"><ChevronRight class="size-3.5" /></button>{/snippet}</TooltipTrigger>
      <TooltipContent>Forward</TooltipContent>
    </Tooltip>
  </div>

  <!-- Center: always-visible search -->
  <div class="flex flex-1 justify-center px-4">
    <SearchBar
      bind:this={searchBar}
      onPlaySong={playSongFromDropdown}
      onGotoArtist={gotoArtistFromDropdown}
    />
  </div>

  <!-- Settings -->
  <div class="shrink-0 px-1">
    <Tooltip>
      <TooltipTrigger>{#snippet child({ props })}<a href="/settings" {...props}><button class="flex size-7 items-center justify-center rounded-full text-muted-foreground transition-colors hover:bg-white/[0.06] hover:text-foreground" aria-label="Settings"><Cog class="size-3.5" /></button></a>{/snippet}</TooltipTrigger>
      <TooltipContent>Settings</TooltipContent>
    </Tooltip>
  </div>

  <!-- Windows: custom window controls -->
  {#if runningInTauri && !isMac}
    <div class="flex shrink-0 items-stretch self-stretch">
      <Tooltip>
        <TooltipTrigger>{#snippet child({ props })}<button {...props} class="flex w-11 items-center justify-center text-muted-foreground/60 transition-colors hover:bg-white/[0.06] hover:text-foreground" onclick={minimizeWindow} aria-label="Minimize"><Minus class="size-3.5" /></button>{/snippet}</TooltipTrigger>
        <TooltipContent>Minimize</TooltipContent>
      </Tooltip>
      <Tooltip>
        <TooltipTrigger>{#snippet child({ props })}<button {...props} class="flex w-11 items-center justify-center text-muted-foreground/60 transition-colors hover:bg-white/[0.06] hover:text-foreground" onclick={toggleMaximizeWindow} aria-label={windowMaximized ? 'Restore' : 'Maximize'}>{#if windowMaximized}<Minimize2 class="size-3" />{:else}<Square class="size-3" />{/if}</button>{/snippet}</TooltipTrigger>
        <TooltipContent>{windowMaximized ? 'Restore' : 'Maximize'}</TooltipContent>
      </Tooltip>
      <Tooltip>
        <TooltipTrigger>{#snippet child({ props })}<button {...props} class="flex w-11 items-center justify-center text-muted-foreground/60 transition-colors hover:bg-rose-500 hover:text-white" onclick={closeWindow} aria-label="Close"><X class="size-3.5" /></button>{/snippet}</TooltipTrigger>
        <TooltipContent>Close</TooltipContent>
      </Tooltip>
    </div>
  {/if}
</header>

<SidebarProvider style="margin-top: 3rem; height: calc(100svh - 3rem); min-height: calc(100svh - 3rem); overflow: hidden;">
  <div class="app-shell-main flex h-full min-w-0 flex-1 flex-col overflow-hidden">
    {#snippet mainPanel()}
      <ScrollArea class="h-full w-full min-w-0" bind:viewportRef={lyricsScrollRef}>
          {#if $showLyrics}
            {@const track = $queue[$currentIndex] ?? null}
            <div class="relative min-h-full">
              {#if track?.coverArtUrl}
                <img src={track.coverArtUrl} class="pointer-events-none absolute inset-0 h-full w-full scale-110 object-cover opacity-[0.06] blur-3xl" aria-hidden="true" alt="" />
              {/if}
              <div class="relative px-8 pb-8 pt-10">
                <div class="mx-auto max-w-2xl">
                  {#if track}
                    <div class="mb-10 flex items-center gap-5" style="animation: lyric-header-in 0.4s ease both">
                      {#if track.coverArtUrl}
                        <img src={track.coverArtUrl} class="size-16 rounded-xl object-cover shadow-lg ring-1 ring-border/40" alt={track.title} />
                      {/if}
                      <div>
                        <p class="mb-1 text-[10px] font-semibold uppercase tracking-widest text-muted-foreground">Lyrics</p>
                        <p class="text-2xl font-bold leading-tight">{track.title}</p>
                        <p class="mt-0.5 text-sm text-muted-foreground">{track.artist}</p>
                        {#if lyricsData?.provider}
                          <div class="mt-2 flex items-center gap-2">
                            <span class="rounded-full border border-border/70 bg-background/40 px-2 py-0.5 text-[10px] font-semibold uppercase tracking-wide text-muted-foreground">{lyricsData.provider}</span>
                            {#if lyricsData.cached}
                              <span class="rounded-full border border-emerald-500/30 bg-emerald-500/10 px-2 py-0.5 text-[10px] font-semibold uppercase tracking-wide text-emerald-300">Cached</span>
                            {/if}
                          </div>
                        {/if}
                      </div>
                    </div>
                  {/if}
                  {#if lyricsLoading}
                    <div class="flex h-48 items-center justify-center">
                      <div class="w-full max-w-sm space-y-3">
                        {#each [1, 0.7, 0.5, 0.8, 0.6] as w, i (i)}
                          <div class="h-5 animate-pulse rounded-full bg-foreground/10" style="width:{w * 100}%"></div>
                        {/each}
                      </div>
                    </div>
                  {:else if track && lyricsData?.instrumental}
                    <div class="flex h-48 items-center justify-center">
                      <p class="text-sm text-muted-foreground">This track is instrumental.</p>
                    </div>
                  {:else if parsedLyrics.length > 0}
                    <div class="py-4">
                      {#each parsedLyrics as line, i (line.time)}
                        <button
                          type="button"
                          data-lyric-idx={i}
                          style="animation: lyric-in 0.35s ease both; animation-delay: {Math.min(i * 18, 400)}ms"
                          class="group relative mb-2 block w-full cursor-pointer select-none rounded-lg px-4 py-1.5 text-left transition-[font-size,color,opacity,background-color] duration-300 ease-out {lyricLineClass(i)}"
                          onclick={() => seekRequest.set(line.time)}
                        >
                          {#if i === currentLyricIdx}
                            <span class="absolute left-0 top-1/2 h-[60%] w-[3px] -translate-y-1/2 rounded-full bg-primary transition-all duration-300"></span>
                          {/if}
                          {line.text}
                        </button>
                      {/each}
                    </div>
                  {:else if lyricsData?.plainLyrics}
                    <div class="py-4" style="animation: lyric-in 0.4s ease both">
                      {#each lyricsData.plainLyrics.split('\n') as line, i (i)}
                        <p class="mb-2 text-lg leading-relaxed {line.trim() ? 'text-foreground/70' : 'mb-5'}">{line || '\u00a0'}</p>
                      {/each}
                    </div>
                  {:else if !lyricsLoading && track}
                    <div class="flex h-48 items-center justify-center">
                      <p class="text-sm text-muted-foreground">No lyrics found for this track.</p>
                    </div>
                  {:else if !track}
                    <div class="flex h-48 items-center justify-center">
                      <p class="text-sm text-muted-foreground">Play a track to see lyrics.</p>
                    </div>
                  {/if}
                </div>
              </div>
            </div>
          {:else}
            <div class="px-8 pb-8 pt-8">
              {@render children()}
            </div>
          {/if}
        </ScrollArea>
    {/snippet}

    <ResizablePanelGroup direction="horizontal" autoSaveId="madrify-shell-layout" class="min-h-0 flex-1">
      <ResizablePanel
        defaultSize={18}
        minSize={6}
        maxSize={28}
        class="min-h-0"
        onResize={(size) => { libraryPaneSize = size; }}
      >
        <Sidebar
          collapsible="none"
          class="h-full border-r border-white/[0.06]"
          style="--sidebar-width: 100%; background: transparent;"
        >
          <SidebarHeader class={libraryCompact ? 'px-1 py-2' : 'px-2 pt-4 pb-2'}>
            <div class="flex items-center px-2 {libraryCompact ? 'justify-center' : 'gap-2'}">
              <Library class="size-5 text-foreground/80" />
              {#if !libraryCompact}
                <span class="flex-1 text-sm font-bold">Your Library</span>
                <Tooltip>
                  <TooltipTrigger>{#snippet child({ props })}<button {...props} type="button" onclick={() => libraryList?.openCreatePlaylist()} class="flex size-6 items-center justify-center rounded-full text-muted-foreground transition-colors hover:bg-white/[0.08] hover:text-foreground" aria-label="Create playlist"><Plus class="size-3.5" /></button>{/snippet}</TooltipTrigger>
                  <TooltipContent>Create playlist</TooltipContent>
                </Tooltip>
              {/if}
            </div>
          </SidebarHeader>
          <SidebarContent class="px-0">
            <ScrollArea class="h-full">
              <LibraryList
                bind:this={libraryList}
                {likedArtists}
                {artistPhotos}
                {starredSongs}
                {selectedPlaylistId}
                onPlayPlaylist={playPlaylist}
                compact={libraryCompact}
              />
            </ScrollArea>
          </SidebarContent>
        </Sidebar>
      </ResizablePanel>

      <ResizableHandle />

      <ResizablePanel defaultSize={82} minSize={38} class="min-h-0">
        {#if rightOpen}
          <ResizablePanelGroup direction="horizontal" autoSaveId="madrify-shell-right-panel" class="min-h-0 h-full">
            <ResizablePanel defaultSize={76} minSize={50} class="min-h-0">
              {@render mainPanel()}
            </ResizablePanel>
            <ResizableHandle />
            <ResizablePanel defaultSize={24} minSize={18} maxSize={36} class="min-h-0">
              <NowPlayingPanel open={rightOpen} />
            </ResizablePanel>
          </ResizablePanelGroup>
        {:else}
          {@render mainPanel()}
        {/if}
      </ResizablePanel>
    </ResizablePanelGroup>

    <PlayerBar />
    <Toaster richColors />
  </div>
</SidebarProvider>
</TooltipProvider>
{/if}
