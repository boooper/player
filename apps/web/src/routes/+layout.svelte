<script lang="ts">
  import '../app.css';
  import { onMount } from 'svelte';
  import { fly, fade } from 'svelte/transition';
  import { cubicOut, cubicIn } from 'svelte/easing';
  import { afterNavigate, goto } from '$app/navigation';

  import { Home, Settings, ChevronLeft, ChevronRight, Library } from '@lucide/svelte';
  import PlayerBar from '$lib/components/PlayerBar.svelte';
  import SearchBar from '$lib/components/SearchBar.svelte';
  import NowPlayingPanel from '$lib/components/NowPlayingPanel.svelte';
  import LibraryList from '$lib/components/LibraryList.svelte';
  import { fetchAudioDbArtistPhoto } from '$lib/audiodb';
  import { openUrl } from '$lib/tauri';
  import { initDrpc } from '$lib/drpc';
  import {
    fetchAppSettings,
    savePlaybackPrefs,
    fetchLikedArtists,
    fetchServiceHealth,
    fetchPlaylistSongs,
    fetchPlaylists,
    fetchStarredSongs,
    fetchLyrics,
    type LyricsResult,
    type Song,
  } from '$lib/api';
  import {
    currentIndex,
    focusTrack,
    isPlaying,
    playQueue,
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
  import { appSettings, libraryRefresh } from '$lib/stores/settings';
  import { Button, ScrollArea, Toaster, SidebarProvider, Sidebar, SidebarContent, SidebarHeader, SidebarTrigger, SidebarRail, SidebarInset } from '$lib/components/ui';

  let { children } = $props();

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
    return lrc
      .split('\n')
      .map((line) => {
        const m = line.match(/^\[(\d{2}):(\d{2})[.:]?(\d{2,3})?\](.*)/);
        if (!m) return null;
        const time = parseInt(m[1]) * 60 + parseInt(m[2]) + (m[3] ? parseInt(m[3].padEnd(3, '0')) / 1000 : 0);
        return { time, text: m[4].trim() };
      })
      .filter((l): l is LrcLine => l !== null && l.text.length > 0);
  }

  const parsedLyrics = $derived(lyricsData?.syncedLyrics ? parseLrc(lyricsData.syncedLyrics) : []);

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

  async function bootstrapAppSettings() {
    try {
      const settings = await fetchAppSettings();
      appSettings.update((current) => ({
        ...current,
        lastFmApiKey: settings.lastFmApiKey,
        recommendationProvider: settings.recommendationProvider,
        metadataProvider: settings.metadataProvider,
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
    const v = $libraryRefresh;
    if (v > 0) reloadLibraryData();
  });

  async function reloadLibraryData() {
    loading = true;
    error = '';
    try {
      const stored = await fetchLikedArtists();
      likedArtists = stored.map((item) => item.name);
      fetchArtistPhotos(likedArtists);
    } catch (err) {
      error = err instanceof Error ? err.message : 'Failed to load library.';
    } finally {
      loading = false;
    }

    try {
      subsonicPlaylists.set(await fetchPlaylists());
    } catch {}

    try {
      const starred = await fetchStarredSongs();
      starredSongs = starred;
      starredSongIds.set(new Set(starred.map((s) => s.id)));
    } catch {}

    try {
      const payload = await fetchServiceHealth();
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
      playQueue(songs, 0);
      focusTrack.set({ title: songs[0].title, artist: songs[0].artist, imageUrl: songs[0].coverArtUrl, source: 'library', album: songs[0].album });
      const pl = $subsonicPlaylists.find((p) => p.id === playlistId);
      if (pl) {
        addRecentlyPlayed({ id: pl.id, name: pl.name, coverArtUrl: pl.coverArtUrl, href: `/playlist/${encodeURIComponent(pl.id)}`, type: 'playlist' });
        playingFrom.set({ type: 'playlist', name: pl.name, href: `/playlist/${encodeURIComponent(pl.id)}` });
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
    await Promise.all(
      missing.map(async (name) => {
        const url = await fetchAudioDbArtistPhoto(name);
        artistPhotos = { ...artistPhotos, [name]: url };
      })
    );
  }

  function playSongFromDropdown(song: Song) {
    playQueue([song], 0);
    focusTrack.set({ title: song.title, artist: song.artist, imageUrl: song.coverArtUrl, source: 'library', album: song.album });
    playingFrom.set({ type: 'artist', name: song.artist, href: `/artist/${encodeURIComponent(song.artist)}` });
    shouldAutoplay.set(true);
  }

  function gotoArtistFromDropdown(name: string) {
    goto(`/artist/${encodeURIComponent(name)}`);
  }

  let searchBar: { reset: () => void; setQuery: (q: string) => void } | undefined;

  onMount(() => {
    document.documentElement.classList.add('dark');
    const cleanupDrpc = initDrpc();

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

    Promise.all([bootstrapAppSettings(), reloadLibraryData()]);

    return () => {
      document.removeEventListener('click', handleExternalLink);
      document.removeEventListener('keydown', handleKeydown);
      cleanupDrpc();
    };
  });

  afterNavigate(() => {
    searchBar?.reset();
  });
</script>

<!-- Full-width top bar -->
<header class="fixed left-0 right-0 top-0 z-20 flex h-14 shrink-0 items-center border-b border-border/30 bg-background/95 px-4 backdrop-blur-md">
  <div class="flex shrink-0 items-center gap-1 pr-3">
    <Button variant="ghost" size="icon" class="size-8 rounded-full text-muted-foreground hover:text-foreground" onclick={() => window.history.back()}><ChevronLeft class="size-4" /></Button>
    <Button variant="ghost" size="icon" class="size-8 rounded-full text-muted-foreground hover:text-foreground" onclick={() => window.history.forward()}><ChevronRight class="size-4" /></Button>
    <div class="h-4 w-px bg-border/60 mx-1"></div>
    <a href="/"><Button variant="ghost" size="icon" class="size-8 rounded-full text-muted-foreground hover:text-foreground"><Home class="size-4" /></Button></a>
    <a href="/settings"><Button variant="ghost" size="icon" class="size-8 rounded-full text-muted-foreground hover:text-foreground" title="Settings"><Settings class="size-4" /></Button></a>
  </div>

  <div class="flex flex-1 justify-center">
    <SearchBar
      bind:this={searchBar}
      onPlaySong={playSongFromDropdown}
      onGotoArtist={gotoArtistFromDropdown}
    />
  </div>

  <div class="hidden shrink-0 items-center gap-4 pl-3 md:flex">
    {#each [
      { label: 'Library', status: subsonicStatus },
      { label: 'Last.fm', status: lastfmStatus },
    ] as svc (svc.label)}
      <span class="flex items-center gap-1.5" title="{svc.label}: {svc.status}">
        <span class="size-1.5 rounded-full transition-colors {svc.status === 'online' ? 'bg-emerald-400' : svc.status === 'offline' ? 'bg-rose-400 animate-pulse' : svc.status === 'missing' ? 'bg-amber-400' : 'bg-muted-foreground animate-pulse'}"></span>
        <span class="text-xs text-muted-foreground">{svc.label}</span>
      </span>
    {/each}
  </div>
</header>

<SidebarProvider style="margin-top: 3.5rem; height: calc(100svh - 3.5rem); min-height: calc(100svh - 3.5rem); overflow: hidden;">
  <Sidebar collapsible="icon" class="border-r border-border/20 bg-background" style="top: 3.5rem; height: calc(100svh - 3.5rem);">
    <SidebarHeader class="px-2 pt-4 pb-2">
      <div class="flex items-center justify-between px-2 group-data-[collapsible=icon]:justify-center">
        <div class="flex items-center gap-2 group-data-[collapsible=icon]:hidden">
          <Library class="size-5 text-foreground/80" />
          <span class="text-sm font-bold">Your Library</span>
        </div>
        <SidebarTrigger class="size-8 rounded-full text-muted-foreground hover:bg-white/10 hover:text-foreground transition-colors" />
      </div>
    </SidebarHeader>

    <SidebarContent class="px-0">
      <ScrollArea class="h-full">
        <LibraryList
          {likedArtists}
          {artistPhotos}
          {starredSongs}
          {selectedPlaylistId}
          onPlayPlaylist={playPlaylist}
        />
      </ScrollArea>
    </SidebarContent>

    <SidebarRail />
  </Sidebar>

  <SidebarInset class="flex h-full flex-col overflow-hidden">
    <div class="flex min-h-0 flex-1 overflow-hidden">
      <ScrollArea class="h-full flex-1 min-w-0" bind:viewportRef={lyricsScrollRef}>
        {#if $showLyrics}
          {@const track = $queue[$currentIndex] ?? null}
          <div
            class="relative min-h-full"
            in:fly={{ y: 20, duration: 380, easing: cubicOut }}
            out:fly={{ y: 20, duration: 250, easing: cubicIn }}
          >
            {#if track?.coverArtUrl}
              <img
                src={track.coverArtUrl}
                class="pointer-events-none absolute inset-0 h-full w-full object-cover opacity-[0.06] blur-3xl scale-110"
                aria-hidden="true"
                alt=""
              />
            {/if}
            <div class="relative px-8 pt-10 pb-8">
              <div class="mx-auto max-w-2xl">
                {#if track}
                  <div class="mb-10 flex items-center gap-5" style="animation: lyric-header-in 0.4s ease both">
                    {#if track.coverArtUrl}
                      <img src={track.coverArtUrl} class="size-16 rounded-xl object-cover shadow-lg ring-1 ring-white/10" alt={track.title} />
                    {/if}
                    <div>
                      <p class="text-[10px] font-semibold uppercase tracking-widest text-muted-foreground mb-1">Lyrics</p>
                      <p class="text-2xl font-bold leading-tight">{track.title}</p>
                      <p class="text-sm text-muted-foreground mt-0.5">{track.artist}</p>
                    </div>
                  </div>
                {/if}
                {#if lyricsLoading}
                  <div class="flex h-48 items-center justify-center">
                    <div class="space-y-3 w-full max-w-sm">
                      {#each [1,0.7,0.5,0.8,0.6] as w}
                        <div class="h-5 rounded-full bg-foreground/10 animate-pulse" style="width:{w*100}%"></div>
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
                        class="group relative mb-2 block w-full cursor-pointer select-none rounded-lg px-4 py-1.5 text-left transition-[font-size,color,opacity,background-color] duration-300 ease-out {i === currentLyricIdx ? 'text-[28px] font-extrabold text-foreground bg-white/[0.03]' : Math.abs(i - currentLyricIdx) === 1 ? 'text-xl font-semibold text-foreground/35' : Math.abs(i - currentLyricIdx) === 2 ? 'text-lg font-medium text-foreground/20' : 'text-base font-medium text-foreground/10 hover:text-foreground/30'}"
                        onclick={() => seekRequest.set(line.time)}
                      >
                        {#if i === currentLyricIdx}
                          <span class="absolute left-0 top-1/2 -translate-y-1/2 w-[3px] h-[60%] rounded-full bg-primary transition-all duration-300"></span>
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
          <div
            class="px-8 pt-8 pb-8"
            in:fade={{ duration: 220, easing: cubicOut }}
            out:fade={{ duration: 150 }}
          >
            {@render children()}
          </div>
        {/if}
      </ScrollArea>

      <NowPlayingPanel open={rightOpen} />
    </div>

    <PlayerBar />
    <Toaster richColors />
  </SidebarInset>
</SidebarProvider>
