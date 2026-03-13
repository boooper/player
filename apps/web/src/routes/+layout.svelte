<script lang="ts">
  import '../app.css';
  import { onMount } from 'svelte';
  import { fly, fade } from 'svelte/transition';
  import { cubicOut, cubicIn } from 'svelte/easing';
  import { afterNavigate, goto } from '$app/navigation';

  import { Home, Heart, Music, User, Search, PanelRight, Settings, Play, Pause, ChevronDown, ChevronUp, ChevronLeft, ChevronRight, Clock, X, Library, ListMusic } from '@lucide/svelte';
  import PlayerBar from '$lib/components/PlayerBar.svelte';
  import { fetchAudioDbArtistPhoto } from '$lib/audiodb';
  import { openUrl } from '$lib/tauri';
  import { initDrpc } from '$lib/drpc';
  import {
    fetchAppSettings,
    fetchLikedArtists,
    fetchServiceHealth,
    fetchSubsonicPlaylistSongs,
    fetchSubsonicPlaylists,
    fetchSubsonicStarredSongs,
    fetchSubsonicSimilar,
    fetchSubsonicAlbumDetail,
    removeLikedArtist,
    saveLikedArtist,
    searchSubsonicSongs,
    fetchLyrics,
    type LyricsResult,
    type SubsonicSong
  } from '$lib/api';
  import { getArtistInfo, type ArtistInfo } from '$lib/metadata';
  import { currentIndex, focusTrack, isPlaying, playQueue, queue, shouldAutoplay, subsonicPlaylists, starredSongIds, addRecentlyPlayed, showQueue, playingFrom, showLyrics, currentTime, seekRequest } from '$lib/stores/player';
  import { appSettings, libraryRefresh } from '$lib/stores/settings';
  import * as Dialog from '$lib/components/ui/dialog';
  import { Badge, Button, Input, ScrollArea, Toaster, SidebarProvider, Sidebar, SidebarContent, SidebarHeader, SidebarFooter, SidebarGroup, SidebarGroupLabel, SidebarGroupContent, SidebarMenu, SidebarMenuItem, SidebarMenuButton, SidebarRail, SidebarInset, SidebarSeparator, SidebarTrigger } from '$lib/components/ui';

  let { children } = $props();

  const lastFmApiKey = $derived($appSettings.lastFmApiKey);
  const hasLastFmKey = $derived(Boolean(lastFmApiKey));

  let likedArtists = $state<string[]>([]);
  let artistPhotos = $state<Record<string, string>>({});
  let starredSongs = $state<SubsonicSong[]>([]);
  let starredSongCount = $derived($starredSongIds.size);
  let playlistError = $state('');
  let selectedPlaylistId = $state('');
  let topSearch = $state('');
  let dropdownSongs = $state<SubsonicSong[]>([]);
  let dropdownArtists = $state<string[]>([]);
  let dropdownOpen = $state(false);
  let searchFocused = $state(false);
  let recentSearches = $state<string[]>([]);

  const RECENT_KEY = 'naviarr_recent_searches';

  function loadRecentSearches() {
    try {
      recentSearches = JSON.parse(localStorage.getItem(RECENT_KEY) ?? '[]');
    } catch {
      recentSearches = [];
    }
  }

  function saveRecentSearch(q: string) {
    const trimmed = q.trim();
    if (!trimmed) return;
    const updated = [trimmed, ...recentSearches.filter((r) => r.toLowerCase() !== trimmed.toLowerCase())].slice(0, 8);
    recentSearches = updated;
    localStorage.setItem(RECENT_KEY, JSON.stringify(updated));
  }

  function removeRecentSearch(q: string) {
    const updated = recentSearches.filter((r) => r !== q);
    recentSearches = updated;
    localStorage.setItem(RECENT_KEY, JSON.stringify(updated));
  }
  let dropdownLoading = $state(false);
  let dropdownTimer = 0;
  let loading = $state(false);
  let error = $state('');
  let lastfmStatus = $state<'checking' | 'online' | 'offline' | 'missing'>('checking');
  // ─── Lyrics ────────────────────────────────────────────────────────────────
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

  // ─── End Lyrics ─────────────────────────────────────────────────────────────

  let rightOpen = $state(true);

  // Sync right panel with showQueue store so PlayerBar cover art can toggle it
  $effect(() => {
    rightOpen = $showQueue;
  });
  let subsonicStatus = $state<'checking' | 'online' | 'offline' | 'missing'>('checking');
  const upNext = $derived.by(() => {
    const items = $queue;
    if (!items.length || $currentIndex >= items.length - 1) return [];
    return items
      .slice($currentIndex + 1)
      .map((song, i) => ({ index: $currentIndex + 1 + i, song }));
  });

  // ── Now Playing panel extra sections ─────────────────────────────────────
  const currentSong = $derived($queue[$currentIndex] ?? null);

  let relatedSongs = $state<SubsonicSong[]>([]);
  let relatedLoading = $state(false);

  let panelArtistInfo = $state<ArtistInfo | null>(null);
  let panelArtistLoading = $state(false);
  let aboutDialogOpen = $state(false);
  let upNextExpanded = $state(false);
  let relatedExpanded = $state(false);
  let creditsExpanded = $state(false);

  let creditsAlbum = $state<{ year?: number; genre?: string } | null>(null);

  let _panelSongId = '';
  let _panelArtist = '';

  $effect(() => {
    const song = currentSong;
    if (!song) return;

    if (song.id !== _panelSongId) {
      _panelSongId = song.id;
      relatedSongs = [];
      relatedExpanded = false;
      creditsExpanded = false;
      relatedLoading = true;
      fetchSubsonicSimilar(song.id, 6)
        .then((s) => { relatedSongs = s; })
        .catch(() => {})
        .finally(() => { relatedLoading = false; });

      creditsAlbum = null;
      if (song.albumId) {
        fetchSubsonicAlbumDetail(song.albumId)
          .then((d) => { creditsAlbum = { year: d.album?.year, genre: d.album?.genre }; })
          .catch(() => {});
      }
    }

    if ($focusTrack?.artist && $focusTrack.artist !== _panelArtist) {
      _panelArtist = $focusTrack.artist;
      panelArtistInfo = null;
      panelArtistLoading = true;
      aboutDialogOpen = false;
      getArtistInfo($focusTrack.artist)
        .then((info) => { panelArtistInfo = info; })
        .catch(() => {})
        .finally(() => { panelArtistLoading = false; });
    }
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
      subsonicPlaylists.set(await fetchSubsonicPlaylists());
      playlistError = '';
    } catch (err) {
      playlistError = err instanceof Error ? err.message : 'Failed to load Subsonic playlists.';
    }

    try {
      const starred = await fetchSubsonicStarredSongs();
      starredSongs = starred;
      starredSongIds.set(new Set(starred.map((s) => s.id)));
    } catch {
      // ignore
    }

    try {
      const payload = await fetchServiceHealth();
      lastfmStatus = payload?.lastfm ?? 'offline';
      subsonicStatus = payload?.subsonic ?? 'offline';
    } catch {
      lastfmStatus = 'offline';
      subsonicStatus = 'offline';
    }
  }

  async function bootstrapAppSettings() {
    try {
      const settings = await fetchAppSettings();
      appSettings.update((current) => ({
        ...current,
        lastFmApiKey: settings.lastFmApiKey,
        recommendationProvider: settings.recommendationProvider,
        metadataProvider: settings.metadataProvider
      }));
    } catch {
      // Keep store defaults when settings API is unavailable.
    }
  }

  // Re-load library data whenever settings are saved (Subsonic profile activated,
  // Last.fm key saved, etc.). The store starts at 0 so the initial run is a no-op;
  // only increments from the settings page trigger a reload.
  $effect(() => {
    const v = $libraryRefresh;
    if (v > 0) reloadLibraryData();
  });

  onMount(async () => {
    document.documentElement.classList.add('dark');
    topSearch = new URL(window.location.href).searchParams.get('q') ?? '';
    loadRecentSearches();

    const cleanupDrpc = initDrpc();

    // In Tauri, intercept external links so they open in the system browser
    // instead of navigating the app's webview.
    function handleExternalLink(e: MouseEvent) {
      const target = (e.target as HTMLElement).closest('a');
      if (!target || !target.href) return;
      try {
        const url = new URL(target.href);
        if (url.origin !== window.location.origin) {
          e.preventDefault();
          openUrl(target.href);
        }
      } catch {
        // ignore malformed hrefs
      }
    }
    document.addEventListener('click', handleExternalLink);

    await Promise.all([bootstrapAppSettings(), reloadLibraryData()]);

    return () => {
      document.removeEventListener('click', handleExternalLink);
      cleanupDrpc();
    };
  });

  afterNavigate(() => {
    topSearch = new URL(window.location.href).searchParams.get('q') ?? '';
    dropdownOpen = false;
  });

  async function toggleLikedArtist(name: string) {
    if (likedArtists.includes(name)) {
      likedArtists = likedArtists.filter((artist) => artist !== name);
      try {
        await removeLikedArtist(name);
      } catch {
        likedArtists = [...likedArtists, name];
      }
      return;
    }

    likedArtists = [...likedArtists, name];
    try {
      await saveLikedArtist(name);
    } catch {
      likedArtists = likedArtists.filter((artist) => artist !== name);
    }
  }

  async function playPlaylist(playlistId: string) {
    if (!playlistId) return;
    selectedPlaylistId = playlistId;
    playlistError = '';

    try {
      const songs = await fetchSubsonicPlaylistSongs(playlistId);
      if (!songs.length) {
        playlistError = 'This playlist has no tracks.';
        return;
      }

      playQueue(songs, 0);
      focusTrack.set({
        title: songs[0].title,
        artist: songs[0].artist,
        imageUrl: songs[0].coverArtUrl,
        source: 'subsonic',
        album: songs[0].album
      });
      const pl = $subsonicPlaylists.find((p) => p.id === playlistId);
      if (pl) {
        addRecentlyPlayed({ id: pl.id, name: pl.name, coverArtUrl: pl.coverArtUrl, href: `/playlist/${encodeURIComponent(pl.id)}`, type: 'playlist' });
        playingFrom.set({ type: 'playlist', name: pl.name, href: `/playlist/${encodeURIComponent(pl.id)}` });
      }
    } catch (err) {
      playlistError = err instanceof Error ? err.message : 'Failed to load playlist tracks.';
    }
  }

  async function fetchArtistPhotos(names: string[]) {
    const missing = names.filter((n) => artistPhotos[n] === undefined);
    if (!missing.length) return;
    // Optimistically mark as empty to avoid duplicate fetches
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

  function initials(name: string): string {
    return name
      .split(' ')
      .filter(Boolean)
      .slice(0, 2)
      .map((part) => part[0]?.toUpperCase() ?? '')
      .join('');
  }

  function submitTopSearch(event: SubmitEvent) {
    event.preventDefault();
    dropdownOpen = false;
    const query = topSearch.trim();
    if (!query) return;
    saveRecentSearch(query);
    goto(`/search?q=${encodeURIComponent(query)}`);
  }

  function onSearchInput() {
    const q = topSearch.trim();
    clearTimeout(dropdownTimer);
    if (q.length < 2) {
      dropdownSongs = [];
      dropdownArtists = [];
      dropdownOpen = true;
      return;
    }
    dropdownLoading = true;
    dropdownOpen = true;
    dropdownTimer = window.setTimeout(async () => {
      try {
        const songs = await searchSubsonicSongs(q, 8);
        dropdownSongs = songs.slice(0, 5);
        const seen = new Set<string>();
        dropdownArtists = songs
          .map((s) => s.artist)
          .filter((a) => { if (seen.has(a)) return false; seen.add(a); return true; })
          .slice(0, 4);
        fetchArtistPhotos(dropdownArtists);
      } catch {
        dropdownSongs = [];
        dropdownArtists = [];
      } finally {
        dropdownLoading = false;
      }
    }, 280);
  }

  function playSongFromDropdown(song: SubsonicSong) {
    dropdownOpen = false;
    playQueue([song], 0);
    focusTrack.set({ title: song.title, artist: song.artist, imageUrl: song.coverArtUrl, source: 'subsonic', album: song.album });
    playingFrom.set({ type: 'artist', name: song.artist, href: `/artist/${encodeURIComponent(song.artist)}` });
    shouldAutoplay.set(true);
  }

  function gotoArtistFromDropdown(name: string) {
    dropdownOpen = false;
    goto(`/artist/${encodeURIComponent(name)}`);
  }

  function statusClass(status: 'checking' | 'online' | 'offline' | 'missing'): string {
    if (status === 'online') return 'border-emerald-500/40 bg-emerald-500/15 text-emerald-200';
    if (status === 'missing') return 'border-amber-500/40 bg-amber-500/15 text-amber-200';
    if (status === 'offline') return 'border-rose-500/40 bg-rose-500/15 text-rose-200';
    return 'border-border bg-secondary text-muted-foreground';
  }

  function playFromUpNext(index: number) {
    const song = $queue[index];
    if (!song) return;
    currentIndex.set(index);
    shouldAutoplay.set(true);
    focusTrack.set({
      title: song.title,
      artist: song.artist,
      imageUrl: song.coverArtUrl,
      source: 'subsonic',
      album: song.album
    });
  }
</script>

<!-- Full-width top bar — fixed above everything including the sidebar -->
<header class="fixed left-0 right-0 top-0 z-20 flex h-14 shrink-0 items-center border-b border-border/30 bg-background/95 px-4 backdrop-blur-md">
  <!-- Left: nav buttons -->
  <div class="flex shrink-0 items-center gap-1 pr-3">
    <Button variant="ghost" size="icon" class="size-8 rounded-full text-muted-foreground hover:text-foreground" onclick={() => window.history.back()}><ChevronLeft class="size-4" /></Button>
    <Button variant="ghost" size="icon" class="size-8 rounded-full text-muted-foreground hover:text-foreground" onclick={() => window.history.forward()}><ChevronRight class="size-4" /></Button>
    <div class="h-4 w-px bg-border/60 mx-1"></div>
    <a href="/"><Button variant="ghost" size="icon" class="size-8 rounded-full text-muted-foreground hover:text-foreground"><Home class="size-4" /></Button></a>
    <a href="/settings"><Button variant="ghost" size="icon" class="size-8 rounded-full text-muted-foreground hover:text-foreground" title="Settings"><Settings class="size-4" /></Button></a>
  </div>
  <!-- Center: Search -->
  <div class="flex flex-1 justify-center">
    <div class="relative transition-all duration-300 ease-in-out {searchFocused ? 'w-full max-w-2xl' : 'w-full max-w-lg'}">
      <form onsubmit={submitTopSearch}>
        <Search class="pointer-events-none absolute left-4 top-1/2 size-4 -translate-y-1/2 transition-colors duration-200 {searchFocused ? 'text-primary' : 'text-muted-foreground'}" />
        <Input
          bind:value={topSearch}
          class="h-10 rounded-full border-transparent bg-white/[0.06] pl-11 pr-4 text-sm placeholder:text-muted-foreground/60 transition-all duration-300 focus-visible:border-primary/40 focus-visible:bg-white/[0.09] focus-visible:ring-0 focus-visible:shadow-[0_0_0_3px_oklch(0.645_0.246_16.439_/_0.15)]"
          placeholder="What do you want to play?"
          oninput={onSearchInput}
          onfocus={() => {
            searchFocused = true;
            dropdownOpen = true;
            if (topSearch.trim().length >= 2) onSearchInput();
          }}
          onblur={() => { searchFocused = false; dropdownOpen = false; }}
        />
      </form>
      {#if dropdownOpen && topSearch.trim().length < 2 && recentSearches.length > 0}
        <div
          role="listbox"
          aria-label="Recent searches"
          tabindex="-1"
          class="absolute left-0 top-full z-50 mt-1 w-full max-w-lg overflow-hidden rounded-xl border border-border bg-popover shadow-2xl"
          onmousedown={(e) => e.preventDefault()}
        >
          <div class="px-3 pb-2 pt-2">
            <p class="mb-1 text-[11px] font-semibold uppercase tracking-wide text-muted-foreground">Recent searches</p>
            <div class="space-y-0.5">
              {#each recentSearches as term (term)}
                <div class="flex items-center gap-1">
                  <button
                    class="flex flex-1 items-center gap-3 rounded-md px-2 py-1.5 text-left hover:bg-accent"
                    onclick={() => { topSearch = term; dropdownOpen = false; saveRecentSearch(term); goto(`/search?q=${encodeURIComponent(term)}`); }}
                  >
                    <Clock class="size-3.5 shrink-0 text-muted-foreground" />
                    <span class="truncate text-sm">{term}</span>
                  </button>
                  <button
                    class="rounded p-1 text-muted-foreground hover:text-foreground"
                    aria-label="Remove"
                    onclick={() => removeRecentSearch(term)}
                  >
                    <X class="size-3.5" />
                  </button>
                </div>
              {/each}
            </div>
          </div>
        </div>
      {/if}
      {#if dropdownOpen && topSearch.trim().length >= 2}
        <div
          role="listbox"
          aria-label="Search results"
          tabindex="-1"
          class="absolute left-0 top-full z-50 mt-1 w-full max-w-lg overflow-hidden rounded-xl border border-border bg-popover shadow-2xl"
          onmousedown={(e) => e.preventDefault()}
        >
          {#if dropdownLoading}
            <div class="px-4 py-3 text-sm text-muted-foreground">Searching…</div>
          {:else}
            {#if dropdownSongs.length > 0}
              <div class="px-3 pb-1 pt-2">
                <p class="mb-1 text-[11px] font-semibold uppercase tracking-wide text-muted-foreground">Songs</p>
                <div class="space-y-0.5">
                  {#each dropdownSongs as song (song.id)}
                    <button
                      class="flex w-full items-center gap-3 rounded-md px-2 py-1.5 text-left hover:bg-accent"
                      onclick={() => playSongFromDropdown(song)}
                    >
                      {#if song.coverArtUrl}
                        <img class="size-8 rounded object-cover" src={song.coverArtUrl} alt={song.title} />
                      {:else}
                        <span class="grid size-8 place-items-center rounded bg-secondary text-[10px] font-bold">{initials(song.title)}</span>
                      {/if}
                      <span class="min-w-0 flex-1">
                        <span class="block truncate text-sm font-medium">{song.title}</span>
                        <span class="block truncate text-xs text-muted-foreground">{song.artist}</span>
                      </span>
                      <Music class="ml-auto size-3.5 shrink-0 text-muted-foreground" />
                    </button>
                  {/each}
                </div>
              </div>
            {/if}
            {#if dropdownArtists.length > 0}
              {#if dropdownSongs.length > 0}
                <div class="mx-3 my-1 border-t border-border/60"></div>
              {/if}
              <div class="px-3 pb-2 pt-1">
                <p class="mb-1 text-[11px] font-semibold uppercase tracking-wide text-muted-foreground">Artists</p>
                <div class="space-y-0.5">
                  {#each dropdownArtists as artist (artist)}
                    <button
                      class="flex w-full items-center gap-3 rounded-md px-2 py-1.5 text-left hover:bg-accent"
                      onclick={() => gotoArtistFromDropdown(artist)}
                    >
                      {#if artistPhotos[artist]}
                        <img src={artistPhotos[artist]} alt={artist} class="size-8 shrink-0 rounded-full object-cover" />
                      {:else}
                        <span class="grid size-8 shrink-0 place-items-center rounded-full bg-gradient-to-br from-slate-500 to-slate-700 text-[10px] font-bold">{initials(artist)}</span>
                      {/if}
                      <span class="min-w-0 flex-1 truncate text-sm font-medium">{artist}</span>
                      <User class="ml-auto size-3.5 shrink-0 text-muted-foreground" />
                    </button>
                  {/each}
                </div>
              </div>
            {/if}
            {#if dropdownSongs.length === 0 && dropdownArtists.length === 0}
              <div class="px-4 py-3 text-sm text-muted-foreground">No results found.</div>
            {/if}
          {/if}
          <div class="border-t border-border/60 px-3 py-1.5">
            <button
              class="text-xs text-muted-foreground hover:text-foreground"
              onclick={() => { dropdownOpen = false; goto(`/search?q=${encodeURIComponent(topSearch.trim())}`); }}
            >See all results for "{topSearch}"</button>
          </div>
        </div>
      {/if}
    </div>
  </div>
  <!-- Right: status badges -->
  <div class="hidden shrink-0 items-center gap-2 pl-3 md:flex">
    <Badge class={statusClass(subsonicStatus)}>Subsonic: {subsonicStatus}</Badge>
    <Badge class={statusClass(lastfmStatus)}>Last.fm: {lastfmStatus}</Badge>
  </div>
</header>

<SidebarProvider style="margin-top: 3.5rem; height: calc(100svh - 3.5rem); min-height: calc(100svh - 3.5rem); overflow: hidden;">
  <!-- Left sidebar -->
  <Sidebar collapsible="icon" class="border-r border-border/20 bg-background" style="top: 3.5rem; height: calc(100svh - 3.5rem);">

    <!-- Header -->
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
        <ul class="m-0 list-none px-2 pb-4 pt-1 space-y-0.5">

          <!-- Liked Songs -->
          <li>
            <a
              href="/favorites"
              class="flex items-center gap-3 rounded-md px-2 py-2 transition-colors hover:bg-white/10"
            >
              <!-- Thumbnail -->
              <div class="relative size-10 shrink-0 flex-none overflow-hidden rounded-md">
                {#if starredSongs.length >= 4}
                  <div class="grid h-full w-full grid-cols-2 grid-rows-2">
                    {#each Array(4) as _, i}
                      <img src={starredSongs[i].coverArtUrl} alt="" class="h-full w-full object-cover" />
                    {/each}
                  </div>
                {:else}
                  <div class="flex h-full w-full items-center justify-center bg-gradient-to-br from-indigo-500 to-indigo-800">
                    <Heart class="size-5 text-white" fill="white" />
                  </div>
                {/if}
              </div>
              <!-- Text -->
              <div class="min-w-0 flex-1 group-data-[collapsible=icon]:hidden">
                <p class="truncate text-sm font-medium leading-tight text-foreground">Liked Songs</p>
                <p class="mt-0.5 truncate text-xs text-muted-foreground">Playlist &bull; {starredSongCount} songs</p>
              </div>
            </a>
          </li>

          <!-- Playlists -->
          {#each $subsonicPlaylists as playlist (playlist.id)}
            {@const isActive = selectedPlaylistId === playlist.id}
            <li>
              <a
                href={`/playlist/${encodeURIComponent(playlist.id)}`}
                class="group/row flex items-center gap-3 rounded-md px-2 py-2 transition-colors hover:bg-white/10 {isActive ? 'bg-white/5' : ''}"
              >
                <!-- Thumbnail -->
                <div class="group/cover relative size-10 shrink-0 flex-none overflow-hidden rounded-md">
                  {#if playlist.coverArtUrl}
                    <img src={playlist.coverArtUrl} alt={playlist.name} class="h-full w-full object-cover" />
                  {:else}
                    <div class="flex h-full w-full items-center justify-center bg-secondary">
                      <ListMusic class="size-4 text-muted-foreground" />
                    </div>
                  {/if}
                  <!-- Play overlay -->
                  <button
                    class="absolute inset-0 flex items-center justify-center rounded-md bg-black/60 opacity-0 transition-opacity group-hover/row:opacity-100 {isActive && $isPlaying ? '!opacity-100' : ''}"
                    onclick={(e) => { e.preventDefault(); e.stopPropagation(); playPlaylist(playlist.id); }}
                  >
                    {#if isActive && $isPlaying}
                      <Pause class="size-4 text-white" />
                    {:else}
                      <Play class="size-4 text-white" />
                    {/if}
                  </button>
                  <!-- Equalizer overlay when playing -->
                  {#if isActive && $isPlaying}
                    <div class="absolute inset-0 flex items-end justify-center gap-[2px] rounded-md bg-black/50 pb-1.5 pointer-events-none">
                      <span class="w-[3px] rounded-sm bg-primary" style="height:5px;animation:now-playing-bar 0.8s ease-in-out infinite alternate"></span>
                      <span class="w-[3px] rounded-sm bg-primary" style="height:9px;animation:now-playing-bar 0.8s ease-in-out 0.2s infinite alternate"></span>
                      <span class="w-[3px] rounded-sm bg-primary" style="height:6px;animation:now-playing-bar 0.8s ease-in-out 0.4s infinite alternate"></span>
                    </div>
                  {/if}
                </div>
                <!-- Text -->
                <div class="min-w-0 flex-1 group-data-[collapsible=icon]:hidden">
                  <p class="truncate text-sm font-medium leading-tight {isActive ? 'text-primary' : 'text-foreground'}">{playlist.name}</p>
                  <p class="mt-0.5 truncate text-xs text-muted-foreground">Playlist &bull; {playlist.songCount} songs</p>
                </div>
              </a>
            </li>
          {/each}

          {#if likedArtists.length > 0}
            <!-- Subtle divider before artists -->
            <li class="px-2 py-1 group-data-[collapsible=icon]:hidden"><div class="h-px bg-white/5"></div></li>
          {/if}

          <!-- Liked Artists -->
          {#each likedArtists as artist (artist)}
            <li>
              <a
                href={`/artist/${encodeURIComponent(artist)}`}
                class="flex items-center gap-3 rounded-md px-2 py-2 transition-colors hover:bg-white/10"
              >
                <!-- Thumbnail -->
                <div class="size-10 shrink-0 flex-none overflow-hidden rounded-full">
                  {#if artistPhotos[artist]}
                    <img src={artistPhotos[artist]} alt={artist} class="h-full w-full object-cover" />
                  {:else}
                    <div class="flex h-full w-full items-center justify-center bg-gradient-to-br from-slate-600 to-slate-800 text-xs font-bold text-white/70">{initials(artist)}</div>
                  {/if}
                </div>
                <!-- Text -->
                <div class="min-w-0 flex-1 group-data-[collapsible=icon]:hidden">
                  <p class="truncate text-sm font-medium leading-tight text-foreground">{artist}</p>
                  <p class="mt-0.5 text-xs text-muted-foreground">Artist</p>
                </div>
              </a>
            </li>
          {/each}

        </ul>
      </ScrollArea>
    </SidebarContent>

    <SidebarRail />
  </Sidebar>

  <!-- Main content area + right panel -->
  <SidebarInset class="flex h-full flex-col overflow-hidden">
    <!-- Body: page content + optional right Now Playing panel -->
    <div class="flex min-h-0 flex-1 overflow-hidden">
      <ScrollArea class="h-full flex-1 min-w-0" bind:viewportRef={lyricsScrollRef}>
        {#if $showLyrics}
          {@const track = $queue[$currentIndex] ?? null}
          <div
            class="relative min-h-full"
            in:fly={{ y: 20, duration: 380, easing: cubicOut }}
            out:fly={{ y: 20, duration: 250, easing: cubicIn }}
          >
            <!-- Blurred cover art backdrop -->
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
                        class="group relative mb-2 block w-full cursor-pointer select-none rounded-lg px-4 py-1.5 text-left
                          transition-[font-size,color,opacity,background-color] duration-300 ease-out
                          {i === currentLyricIdx
                            ? 'text-[28px] font-extrabold text-foreground bg-white/[0.03]'
                            : Math.abs(i - currentLyricIdx) === 1
                              ? 'text-xl font-semibold text-foreground/35'
                              : Math.abs(i - currentLyricIdx) === 2
                                ? 'text-lg font-medium text-foreground/20'
                                : 'text-base font-medium text-foreground/10 hover:text-foreground/30'}"
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

      <aside class="hidden h-full shrink-0 flex-col border-border/70 bg-background xl:flex overflow-hidden transition-[width] duration-300 ease-in-out {rightOpen ? 'w-72 border-l' : 'w-0'}">
          <div class="flex items-center justify-between border-b border-border/60 px-4 py-4 shrink-0 w-72">
            <!-- Context: where is this playing from? -->
            {#if $playingFrom.type}
              <div class="flex min-w-0 flex-1 flex-col">
                <span class="text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">
                  {$playingFrom.type === 'favorites' ? 'Playing from' : $playingFrom.type === 'playlist' ? 'Playing from playlist' : $playingFrom.type === 'artist' ? 'Playing from artist' : 'Playing from album'}
                </span>
                <a href={$playingFrom.href} class="truncate text-sm font-semibold hover:underline leading-snug">{$playingFrom.name}</a>
              </div>
            {:else}
              <span class="text-base font-semibold">Now Playing</span>
            {/if}
            <Button
              variant="ghost"
              size="icon"
              class="size-7 shrink-0 ml-2"
              onclick={() => { showQueue.update(v => !v); }}
              title="Close"
            >
              <PanelRight class="size-4" />
              <span class="sr-only">Close Now Playing</span>
            </Button>
          </div>
          <ScrollArea class="h-full flex-1 px-4 pt-4 pb-24 w-72">
            {#if $focusTrack}
              <div class="space-y-4">
                <!-- Album art with hover overlay showing playing-from context -->
                <div class="group/art relative aspect-square w-full">
                  {#if $focusTrack.imageUrl}
                    <img class="aspect-square w-full rounded-lg object-cover shadow-lg" src={$focusTrack.imageUrl} alt={$focusTrack.title} />
                  {:else}
                    <div class="grid aspect-square w-full place-items-center rounded-lg bg-gradient-to-br from-slate-500 to-slate-700 text-2xl font-bold">
                      {initials($focusTrack.title)}
                    </div>
                  {/if}
                  {#if $playingFrom.type}
                    <a
                      href={$playingFrom.href}
                      class="absolute inset-0 flex flex-col justify-end rounded-lg bg-gradient-to-t from-black/80 via-black/20 to-transparent p-3 opacity-0 transition-opacity duration-200 group-hover/art:opacity-100"
                    >
                      <span class="text-[10px] font-semibold uppercase tracking-wider text-white/70">
                        {$playingFrom.type === 'favorites' ? 'Playing from' : $playingFrom.type === 'playlist' ? 'Playing from playlist' : $playingFrom.type === 'artist' ? 'Playing from artist' : 'Playing from album'}
                      </span>
                      <span class="truncate text-sm font-bold text-white">{$playingFrom.name}</span>
                    </a>
                  {/if}
                </div>
                <div class="space-y-0.5">
                  <p class="text-base font-semibold leading-snug">{$focusTrack.title}</p>
                  <a
                    href={`/artist/${encodeURIComponent($focusTrack.artist)}`}
                    class="text-sm text-muted-foreground hover:text-foreground hover:underline transition-colors"
                  >{$focusTrack.artist}</a>
                </div>
              </div>
            {:else}
              <p class="text-sm text-muted-foreground">Pick a song to start listening.</p>
            {/if}

            <!-- Related Songs -->
            {#if relatedLoading || relatedSongs.length > 0}
              <div class="mt-6 border-t border-border/70 pt-5">
                <div class="mb-3 flex items-center justify-between">
                  <h3 class="text-xs font-semibold uppercase tracking-wider text-muted-foreground">Related Songs</h3>
                  {#if !relatedLoading && relatedSongs.length > 1}
                    <button
                      class="flex items-center gap-1 text-xs text-muted-foreground hover:text-foreground transition-colors"
                      onclick={() => { relatedExpanded = !relatedExpanded; }}
                    >
                      {relatedExpanded ? 'Show less' : `Show all ${relatedSongs.length}`}
                      {#if relatedExpanded}<ChevronUp class="size-3" />{:else}<ChevronDown class="size-3" />{/if}
                    </button>
                  {/if}
                </div>
                {#if relatedLoading}
                  <div class="space-y-2">
                    {#each Array(1) as _}
                      <div class="flex items-center gap-3 px-2 py-1.5">
                        <div class="size-9 shrink-0 rounded bg-secondary animate-pulse"></div>
                        <div class="flex-1 space-y-1.5">
                          <div class="h-3 w-3/4 rounded bg-secondary animate-pulse"></div>
                          <div class="h-2.5 w-1/2 rounded bg-secondary animate-pulse"></div>
                        </div>
                      </div>
                    {/each}
                  </div>
                {:else}
                  <div class="space-y-1">
                    {#each (relatedExpanded ? relatedSongs : relatedSongs.slice(0, 1)) as song (song.id)}
                      <button
                        class="flex w-full items-center gap-3 rounded-md px-2 py-1.5 text-left transition hover:bg-accent"
                        onclick={() => {
                          playQueue([song], 0);
                          focusTrack.set({ title: song.title, artist: song.artist, imageUrl: song.coverArtUrl, source: 'subsonic', album: song.album });
                          playingFrom.set({ type: 'artist', name: song.artist, href: `/artist/${encodeURIComponent(song.artist)}` });
                          shouldAutoplay.set(true);
                        }}
                      >
                        {#if song.coverArtUrl}
                          <img class="size-9 rounded object-cover shrink-0" src={song.coverArtUrl} alt={song.title} loading="lazy" />
                        {:else}
                          <span class="grid size-9 shrink-0 place-items-center rounded bg-secondary text-[10px] font-semibold">{initials(song.title)}</span>
                        {/if}
                        <span class="min-w-0 flex-1">
                          <span class="block truncate text-sm font-medium">{song.title}</span>
                          <span class="block truncate text-xs text-muted-foreground">{song.artist}</span>
                        </span>
                      </button>
                    {/each}
                  </div>
                {/if}
              </div>
            {/if}

            <!-- About Artist -->
            {#if $focusTrack}
              <div class="mt-6 border-t border-border/70 pt-5">
                <h3 class="mb-3 text-xs font-semibold uppercase tracking-wider text-muted-foreground">About the Artist</h3>
                {#if panelArtistLoading}
                  <div class="space-y-2">
                    <div class="h-20 w-full rounded-lg bg-secondary animate-pulse"></div>
                    <div class="h-3 w-2/3 rounded bg-secondary animate-pulse"></div>
                    <div class="h-3 w-full rounded bg-secondary animate-pulse"></div>
                  </div>
                {:else if panelArtistInfo}
                  <button
                    class="group/about w-full text-left"
                    onclick={() => { aboutDialogOpen = true; }}
                  >
                    {#if panelArtistInfo.imageUrl}
                      <div class="relative mb-3 h-28 w-full overflow-hidden rounded-lg">
                        <img src={panelArtistInfo.imageUrl} alt={panelArtistInfo.name} class="h-full w-full object-cover object-top shadow transition-transform duration-300 group-hover/about:scale-105" />
                        <div class="absolute inset-0 rounded-lg bg-black/0 transition-colors group-hover/about:bg-black/20"></div>
                      </div>
                    {/if}
                    <p class="mb-0.5 text-sm font-semibold group-hover/about:underline">{panelArtistInfo.name}</p>
                    {#if panelArtistInfo.listeners}
                      <p class="mb-2 text-xs text-muted-foreground">
                        {panelArtistInfo.listeners >= 1_000_000
                          ? `${(panelArtistInfo.listeners / 1_000_000).toFixed(1)}M monthly listeners`
                          : `${(panelArtistInfo.listeners / 1_000).toFixed(0)}K monthly listeners`}
                      </p>
                    {/if}
                    {#if panelArtistInfo.bio}
                      {@const bioText = panelArtistInfo.bio.replace(/<[^>]*>/g, '').trim()}
                      <p class="line-clamp-3 text-xs leading-relaxed text-muted-foreground">{bioText}</p>
                      <p class="mt-1 text-xs font-medium text-foreground/60 group-hover/about:text-foreground transition-colors">Read more…</p>
                    {/if}
                  </button>
                {:else}
                  <p class="text-xs text-muted-foreground">No artist info available.</p>
                {/if}
              </div>
            {/if}

            <!-- About Artist Dialog -->
            {#if panelArtistInfo && $focusTrack}
              <Dialog.Root bind:open={aboutDialogOpen}>
                <Dialog.Portal>
                  <Dialog.Overlay class="fixed inset-0 z-50 bg-black/60 backdrop-blur-sm" />
                  <Dialog.Content class="fixed left-1/2 top-1/2 z-50 w-full max-w-lg -translate-x-1/2 -translate-y-1/2 rounded-2xl bg-background shadow-2xl outline-none overflow-hidden">
                    {#if panelArtistInfo.imageUrl}
                      <div class="relative h-56 w-full">
                        <img src={panelArtistInfo.imageUrl} alt={panelArtistInfo.name} class="h-full w-full object-cover object-top" />
                        <div class="absolute inset-0 bg-gradient-to-t from-background via-background/30 to-transparent"></div>
                        <div class="absolute bottom-0 left-0 p-6">
                          <Dialog.Title class="text-2xl font-bold">{panelArtistInfo.name}</Dialog.Title>
                          {#if panelArtistInfo.listeners}
                            <p class="text-sm text-muted-foreground">
                              {panelArtistInfo.listeners >= 1_000_000
                                ? `${(panelArtistInfo.listeners / 1_000_000).toFixed(1)}M monthly listeners`
                                : `${(panelArtistInfo.listeners / 1_000).toFixed(0)}K monthly listeners`}
                            </p>
                          {/if}
                        </div>
                        <Dialog.Close class="absolute right-4 top-4 grid size-8 place-items-center rounded-full bg-black/50 text-white hover:bg-black/70 transition-colors">
                          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
                        </Dialog.Close>
                      </div>
                    {:else}
                      <div class="flex items-start justify-between p-6 pb-2">
                        <div>
                          <Dialog.Title class="text-xl font-bold">{panelArtistInfo.name}</Dialog.Title>
                          {#if panelArtistInfo.listeners}
                            <p class="text-sm text-muted-foreground">
                              {panelArtistInfo.listeners >= 1_000_000
                                ? `${(panelArtistInfo.listeners / 1_000_000).toFixed(1)}M monthly listeners`
                                : `${(panelArtistInfo.listeners / 1_000).toFixed(0)}K monthly listeners`}
                            </p>
                          {/if}
                        </div>
                        <Dialog.Close class="grid size-8 place-items-center rounded-full hover:bg-accent transition-colors">
                          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
                        </Dialog.Close>
                      </div>
                    {/if}
                    <div class="max-h-72 overflow-y-auto px-6 py-4">
                      {#if panelArtistInfo.tags?.length}
                        <div class="mb-4 flex flex-wrap gap-1.5">
                          {#each panelArtistInfo.tags as tag}
                            <span class="rounded-full border border-border px-2.5 py-0.5 text-xs text-muted-foreground">{tag}</span>
                          {/each}
                        </div>
                      {/if}
                      {#if panelArtistInfo.bio}
                        <p class="text-sm leading-relaxed text-muted-foreground">{panelArtistInfo.bio.replace(/<[^>]*>/g, '').trim()}</p>
                      {/if}
                    </div>
                    <div class="border-t border-border/60 px-6 py-4">
                      <a
                        href={`/artist/${encodeURIComponent($focusTrack.artist)}`}
                        onclick={() => { aboutDialogOpen = false; }}
                        class="text-sm font-medium hover:underline"
                      >Go to artist page →</a>
                    </div>
                  </Dialog.Content>
                </Dialog.Portal>
              </Dialog.Root>
            {/if}

            <!-- Credits -->
            {#if currentSong}
              <div class="mt-6 border-t border-border/70 pt-5 pb-2">
                <div class="mb-3 flex items-center justify-between">
                  <h3 class="text-xs font-semibold uppercase tracking-wider text-muted-foreground">Credits</h3>
                  <button
                    class="flex items-center gap-1 text-xs text-muted-foreground hover:text-foreground transition-colors"
                    onclick={() => { creditsExpanded = !creditsExpanded; }}
                  >
                    {creditsExpanded ? 'Show less' : 'Show all'}
                    {#if creditsExpanded}<ChevronUp class="size-3" />{:else}<ChevronDown class="size-3" />{/if}
                  </button>
                </div>
                <div class="space-y-3">
                  <div>
                    <p class="text-[10px] uppercase tracking-wider text-muted-foreground/70 mb-0.5">Performed by</p>
                    <a href={`/artist/${encodeURIComponent(currentSong.artist)}`} class="text-sm font-medium hover:underline">{currentSong.artist}</a>
                  </div>
                  {#if creditsExpanded}
                    <div>
                      <p class="text-[10px] uppercase tracking-wider text-muted-foreground/70 mb-0.5">Album</p>
                      <a href={`/album/${encodeURIComponent(currentSong.albumId)}`} class="text-sm font-medium hover:underline">{currentSong.album}</a>
                    </div>
                    {#if creditsAlbum?.year}
                      <div>
                        <p class="text-[10px] uppercase tracking-wider text-muted-foreground/70 mb-0.5">Year</p>
                        <p class="text-sm font-medium">{creditsAlbum.year}</p>
                      </div>
                    {/if}
                    {#if creditsAlbum?.genre}
                      <div>
                        <p class="text-[10px] uppercase tracking-wider text-muted-foreground/70 mb-0.5">Genre</p>
                        <p class="text-sm font-medium">{creditsAlbum.genre}</p>
                      </div>
                    {/if}
                    {#if currentSong.duration}
                      <div>
                        <p class="text-[10px] uppercase tracking-wider text-muted-foreground/70 mb-0.5">Duration</p>
                        <p class="text-sm font-medium">{Math.floor(currentSong.duration / 60)}:{String(currentSong.duration % 60).padStart(2, '0')}</p>
                      </div>
                    {/if}
                  {/if}
                </div>
              </div>
            {/if}

            <!-- Up Next -->
            {#if currentSong}
              <div class="mt-6 border-t border-border/70 pt-5 pb-2">
                <div class="mb-3 flex items-center justify-between">
                  <h3 class="text-xs font-semibold uppercase tracking-wider text-muted-foreground">Up Next</h3>
                  {#if upNext.length > 1}
                    <button
                      class="flex items-center gap-1 text-xs text-muted-foreground hover:text-foreground transition-colors"
                      onclick={() => { upNextExpanded = !upNextExpanded; }}
                    >
                      {upNextExpanded ? 'Show less' : `Show all ${upNext.length}`}
                      {#if upNextExpanded}<ChevronUp class="size-3" />{:else}<ChevronDown class="size-3" />{/if}
                    </button>
                  {/if}
                </div>
                {#if upNext.length === 0}
                  <p class="text-xs text-muted-foreground/60 px-2">Nothing queued</p>
                {:else}
                  <div class="space-y-1">
                    {#each (upNextExpanded ? upNext : upNext.slice(0, 1)) as item (item.song.id + '-' + item.index)}
                      <button
                        class="flex w-full items-center gap-3 rounded-md px-2 py-2 text-left transition hover:bg-accent"
                        onclick={() => playFromUpNext(item.index)}
                      >
                        {#if item.song.coverArtUrl}
                          <img class="size-9 rounded object-cover shrink-0" src={item.song.coverArtUrl} alt={item.song.title} loading="lazy" />
                        {:else}
                          <span class="grid size-9 shrink-0 place-items-center rounded bg-secondary text-[10px] font-semibold">{initials(item.song.title)}</span>
                        {/if}
                        <span class="min-w-0">
                          <span class="block truncate text-sm font-medium">{item.song.title}</span>
                          <span class="block truncate text-xs text-muted-foreground">{item.song.artist}</span>
                        </span>
                      </button>
                    {/each}
                  </div>
                {/if}
              </div>
            {/if}

          </ScrollArea>
        </aside>
    </div>

    <PlayerBar />
    <Toaster richColors />
  </SidebarInset>
</SidebarProvider>
