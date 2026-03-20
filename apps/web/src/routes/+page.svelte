<script lang="ts">
  import { Play, Pause, Plus, Check, ChevronLeft, ChevronRight, Disc3, Music2, Compass } from '@lucide/svelte';

  import {
    fetchLikedArtists, saveLikedArtist, removeLikedArtist,
    fetchAlbumList, fetchAlbumSongs, fetchPlaylistSongs,
    getListeningProfile,
    type Album
  } from '$lib/servers';
  import type { ArtistStat } from '$lib/servers/play-history';
  import { getTopArtists, type DiscoveryArtist as Artist } from '$lib/discovery';
  import {
    recentlyPlayed, focusTrack, playQueue, playingFrom,
    addRecentlyPlayed, isPlaying, togglePlayRequest, queueLoading, type RecentItem
  } from '$lib/stores/player';
  import AlbumContextMenu from '$lib/components/AlbumContextMenu.svelte';
  import ArtistContextMenu from '$lib/components/ArtistContextMenu.svelte';
  import { backendSettings } from '$lib/stores/backend-settings';
  import { libraryRefresh, requestLibraryRefresh } from '$lib/stores/ui-state';

  let liked = $state<string[]>([]);
  let top = $state<Artist[]>([]);
  let newestAlbums = $state<Album[]>([]);
  let randomAlbums = $state<Album[]>([]);
  let recentAlbums = $state<Album[]>([]);
  let topArtistStats = $state<ArtistStat[]>([]);
  let loading = $state(false);
  let error = $state('');
  let togglingArtist = $state<string | null>(null);
  let albumLoadingId = $state<string | null>(null);
  let recentLoadingId = $state<string | null>(null);


  const activeHref = $derived($playingFrom.href);

  // Affinity score for an artist: higher = user listens to them more and skips them less.
  function artistScore(artist: string): number {
    const norm = artist.toLowerCase();
    const stat = topArtistStats.find((s) => s.artist.toLowerCase() === norm);
    if (stat) return stat.avgCompletion * Math.log1p(stat.playCount);
    if (liked.some((l) => l.toLowerCase() === norm)) return 0.1;
    return 0;
  }

  const topMixArtists = $derived(topArtistStats.slice(0, 10).map((a) => a.artist));

  // Newest albums filtered to artists the user knows, sorted by affinity.
  // Falls back to all newest if not enough matches (user has no history yet).
  const newlyReleasedAlbums = $derived.by(() => {
    const knownArtists = new Set([...topMixArtists, ...liked].map((a) => a.toLowerCase()));
    if (!knownArtists.size) return newestAlbums;
    const filtered = newestAlbums
      .filter((a) => knownArtists.has(a.artist.toLowerCase()))
      .sort((a, b) => artistScore(b.artist) - artistScore(a.artist));
    return filtered.length >= 4 ? filtered : newestAlbums;
  });

  // Random albums re-ranked by affinity: known artists bubble up, unknown artists stay
  // at the bottom as discovery picks. Stable sort preserves server randomness within tiers.
  const recommendedAlbums = $derived.by(() => {
    if (!topArtistStats.length && !liked.length) return randomAlbums;
    return [...randomAlbums].sort((a, b) => artistScore(b.artist) - artistScore(a.artist));
  });

  let forYouCarouselEl = $state<HTMLDivElement | null>(null);
  let jumpBackCarouselEl = $state<HTMLDivElement | null>(null);
  let topMixesCarouselEl = $state<HTMLDivElement | null>(null);
  let newlyReleasedCarouselEl = $state<HTMLDivElement | null>(null);
  let likedCarouselEl = $state<HTMLDivElement | null>(null);
  let trendingCarouselEl = $state<HTMLDivElement | null>(null);
  let randomCarouselEl = $state<HTMLDivElement | null>(null);
  let homeLoadVersion = 0;

  function greeting(): string {
    const h = new Date().getHours();
    if (h < 12) return 'Good morning';
    if (h < 18) return 'Good afternoon';
    return 'Good evening';
  }

  async function toggleArtist(name: string) {
    if (togglingArtist) return;
    togglingArtist = name;
    try {
      if (liked.includes(name)) {
        liked = liked.filter((a) => a !== name);
        await removeLikedArtist(name);
      } else {
        liked = [...liked, name];
        await saveLikedArtist(name);
      }
      requestLibraryRefresh();
    } catch {
      liked = liked.includes(name) ? liked.filter((a) => a !== name) : [...liked, name];
    } finally {
      togglingArtist = null;
    }
  }

  async function playRecentItem(item: RecentItem) {
    if (recentLoadingId) return;
    recentLoadingId = item.id;
    queueLoading.set(true);
    try {
      if (item.type === 'album') {
        const songs = await fetchAlbumSongs(item.id);
        if (!songs.length) return;
        focusTrack.set({ title: songs[0].title, artist: songs[0].artist, imageUrl: songs[0].coverArtUrl, source: 'library', album: songs[0].album });
        playQueue(songs, 0);
        playingFrom.set({ type: 'album', name: item.name, href: item.href });
      } else if (item.type === 'playlist') {
        const songs = await fetchPlaylistSongs(item.id);
        if (!songs.length) return;
        focusTrack.set({ title: songs[0].title, artist: songs[0].artist, imageUrl: songs[0].coverArtUrl, source: 'library', album: songs[0].album });
        playQueue(songs, 0);
        playingFrom.set({ type: 'playlist', name: item.name, href: item.href });
      }
    } finally {
      recentLoadingId = null;
      queueLoading.set(false);
    }
  }

  async function playAlbum(album: Album) {
    albumLoadingId = album.id;
    queueLoading.set(true);
    try {
      const songs = await fetchAlbumSongs(album.id);
      if (!songs.length) return;
      focusTrack.set({ title: songs[0].title, artist: songs[0].artist, imageUrl: songs[0].coverArtUrl, source: 'library', album: songs[0].album });
      playQueue(songs, 0);
      playingFrom.set({ type: 'album', name: album.name, href: `/album/${encodeURIComponent(album.id)}` });
      addRecentlyPlayed({ id: album.id, name: album.name, coverArtUrl: album.coverArtUrl, href: `/album/${encodeURIComponent(album.id)}`, type: 'album' });
    } finally {
      albumLoadingId = null;
      queueLoading.set(false);
    }
  }

  function scroll(el: HTMLDivElement | null, dir: -1 | 1) {
    el?.scrollBy({ left: dir * 240, behavior: 'smooth' });
  }

  async function loadHomeData() {
    const loadVersion = ++homeLoadVersion;
    loading = true;
    try {
      const [stored, topArtists, newest, random, recent, profile] = await Promise.all([
        fetchLikedArtists(),
        getTopArtists(24),
        fetchAlbumList('newest', 50),
        fetchAlbumList('random', 40),
        fetchAlbumList('recent', 20),
        getListeningProfile(),
      ]);
      if (loadVersion !== homeLoadVersion) return;
      liked = stored.map((entry) => entry.name);
      top = topArtists;
      newestAlbums = newest;
      randomAlbums = random;
      recentAlbums = recent;
      topArtistStats = profile.topArtists;
    } catch (err) {
      if (loadVersion !== homeLoadVersion) return;
      error = err instanceof Error ? err.message : 'Failed to load home feed.';
    } finally {
      if (loadVersion !== homeLoadVersion) return;
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
    loadHomeData();
  });
</script>

<!-- Greeting -->
<h1 class="app-section-title mb-6 text-3xl font-bold tracking-tight">{greeting()}</h1>

{#if error}
  <p class="mb-3 text-sm text-destructive">{error}</p>
{/if}

<!-- Quick access: last 8 played items -->
{#if $recentlyPlayed.length > 0}
  <div class="mb-10 grid grid-cols-2 gap-2 sm:grid-cols-4">
    {#each $recentlyPlayed.slice(0, 8) as item, index (item.id)}
      <div class="app-card app-hover stagger-item group relative flex items-center gap-3 overflow-hidden rounded-lg pr-3" style="--stagger-index:{index}">
        <a href={item.href} class="absolute inset-0 rounded-lg" aria-label={item.name}></a>
        {#if item.coverArtUrl}
          <img class="size-14 shrink-0 rounded-l-lg object-cover" src={item.coverArtUrl} alt={item.name} loading="lazy" />
        {:else}
          <div class="grid size-14 shrink-0 place-items-center rounded-l-lg bg-gradient-to-br from-slate-600 to-slate-800">
            <Disc3 class="size-6 text-white/50" />
          </div>
        {/if}
        <p class="min-w-0 flex-1 truncate text-sm font-semibold leading-tight">{item.name}</p>
        <button
          class="relative z-10 ml-auto grid size-9 shrink-0 place-items-center rounded-full bg-primary text-primary-foreground opacity-0 shadow-lg transition group-hover:opacity-100"
          onclick={(e) => { e.preventDefault(); activeHref === item.href ? togglePlayRequest.update(n => n + 1) : playRecentItem(item); }}
          aria-label="{activeHref === item.href && $isPlaying ? 'Pause' : 'Play'} {item.name}"
        >
          {#if recentLoadingId === item.id}
            <span class="block size-3.5 animate-spin rounded-full border-2 border-primary-foreground border-t-transparent"></span>
          {:else if activeHref === item.href && $isPlaying}
            <Pause class="size-3.5" fill="currentColor" />
          {:else}
            <Play class="size-3.5 translate-x-px" fill="currentColor" />
          {/if}
        </button>
      </div>
    {/each}
  </div>
{/if}

<!-- 1. Made for You carousel -->
<section class="page-section mb-10">
  <div class="mb-4 flex items-center justify-between">
    <h2 class="app-section-title text-xl font-bold">Made for You</h2>
    <div class="flex gap-1">
      <button class="app-round-button grid size-8 place-items-center rounded-full" onclick={() => scroll(forYouCarouselEl, -1)} aria-label="Scroll left"><ChevronLeft class="size-4" /></button>
      <button class="app-round-button grid size-8 place-items-center rounded-full" onclick={() => scroll(forYouCarouselEl, 1)} aria-label="Scroll right"><ChevronRight class="size-4" /></button>
    </div>
  </div>
  <div bind:this={forYouCarouselEl} class="flex gap-4 overflow-x-auto pb-2" style="scrollbar-width:none;-ms-overflow-style:none">
    <a href="/for-you/replay" class="app-card app-hover stagger-item group flex w-40 shrink-0 flex-col gap-2 rounded-2xl p-3" style="--stagger-index:0">
      <div class="grid aspect-square w-full place-items-center rounded-md bg-gradient-to-br from-violet-600 to-indigo-900 shadow-md">
        <Music2 class="size-10 text-white/70" />
      </div>
      <div>
        <p class="truncate text-sm font-semibold">Your Replay</p>
        <p class="text-xs text-muted-foreground">Your top tracks</p>
      </div>
    </a>
    <a href="/for-you/discover" class="app-card app-hover stagger-item group flex w-40 shrink-0 flex-col gap-2 rounded-2xl p-3" style="--stagger-index:1">
      <div class="grid aspect-square w-full place-items-center rounded-md bg-gradient-to-br from-emerald-600 to-cyan-800 shadow-md">
        <Compass class="size-10 text-white/70" />
      </div>
      <div>
        <p class="truncate text-sm font-semibold">Discover Mix</p>
        <p class="text-xs text-muted-foreground">New recommendations</p>
      </div>
    </a>
    {#each liked.slice(0, 8) as artist, i (artist)}
      <a href="/for-you/artist/{encodeURIComponent(artist)}" class="app-card app-hover stagger-item group flex w-40 shrink-0 flex-col gap-2 rounded-2xl p-3" style="--stagger-index:{i + 2}">
        <div class="grid aspect-square w-full place-items-center rounded-md bg-gradient-to-br from-rose-600 to-pink-900 shadow-md">
          <span class="text-2xl font-bold text-white/80">{artist.slice(0, 2).toUpperCase()}</span>
        </div>
        <div>
          <p class="truncate text-sm font-semibold">{artist} Mix</p>
          <p class="truncate text-xs text-muted-foreground">{artist}</p>
        </div>
      </a>
    {/each}
  </div>
</section>

<!-- 3. Jump back in (server-side recent albums) -->
{#if recentAlbums.length > 0 || loading}
  <section class="page-section mb-10">
    <div class="mb-4 flex items-center justify-between">
      <h2 class="app-section-title text-xl font-bold">Jump back in</h2>
      <div class="flex gap-1">
        <button class="app-round-button grid size-8 place-items-center rounded-full" onclick={() => scroll(jumpBackCarouselEl, -1)} aria-label="Scroll left"><ChevronLeft class="size-4" /></button>
        <button class="app-round-button grid size-8 place-items-center rounded-full" onclick={() => scroll(jumpBackCarouselEl, 1)} aria-label="Scroll right"><ChevronRight class="size-4" /></button>
      </div>
    </div>
    <div bind:this={jumpBackCarouselEl} class="flex gap-4 overflow-x-auto pb-2" style="scrollbar-width:none;-ms-overflow-style:none">
      {#if loading}
        {#each Array(8) as _, i (i)}
          <div class="flex w-40 shrink-0 flex-col gap-2 p-3">
            <div class="aspect-square w-full animate-pulse rounded-md bg-muted"></div>
            <div class="h-3 w-24 animate-pulse rounded bg-muted"></div>
          </div>
        {/each}
      {:else}
        {#each recentAlbums as album, index (album.id)}
          <AlbumContextMenu {album} onplay={() => playAlbum(album)} triggerClass="contents">
            <div class="app-card app-hover stagger-item group relative flex w-40 shrink-0 flex-col gap-2 rounded-2xl p-3" style="--stagger-index:{index}">
              <a href={`/album/${encodeURIComponent(album.id)}`} class="absolute inset-0 rounded-lg" aria-label={album.name}></a>
              <div class="relative">
                {#if album.coverArtUrl}
                  <img class="aspect-square w-full rounded-md object-cover shadow-md" src={album.coverArtUrl} alt={album.name} loading="lazy" />
                {:else}
                  <div class="grid aspect-square w-full place-items-center rounded-md bg-gradient-to-br from-slate-600 to-slate-800 shadow-md">
                    <Disc3 class="size-10 text-white/50" />
                  </div>
                {/if}
                <button
                  class="absolute bottom-2 right-2 z-10 grid size-10 translate-y-1 place-items-center rounded-full bg-primary text-primary-foreground opacity-0 shadow-lg transition group-hover:translate-y-0 group-hover:opacity-100"
                  onclick={(e) => { e.preventDefault(); playAlbum(album); }}
                  aria-label="Play {album.name}"
                >
                  {#if albumLoadingId === album.id}
                    <span class="block size-4 animate-spin rounded-full border-2 border-primary-foreground border-t-transparent"></span>
                  {:else}
                    <Play class="size-4 translate-x-px" fill="currentColor" />
                  {/if}
                </button>
              </div>
              <div>
                <p class="truncate text-sm font-semibold">{album.name}</p>
                <p class="truncate text-xs text-muted-foreground">{album.artist}</p>
              </div>
            </div>
          </AlbumContextMenu>
        {/each}
      {/if}
    </div>
  </section>
{/if}

<!-- 4. Your top mixes (from listening history) -->
{#if topMixArtists.length > 0}
  <section class="page-section mb-10">
    <div class="mb-4 flex items-center justify-between">
      <h2 class="app-section-title text-xl font-bold">Your top mixes</h2>
      <div class="flex gap-1">
        <button class="app-round-button grid size-8 place-items-center rounded-full" onclick={() => scroll(topMixesCarouselEl, -1)} aria-label="Scroll left"><ChevronLeft class="size-4" /></button>
        <button class="app-round-button grid size-8 place-items-center rounded-full" onclick={() => scroll(topMixesCarouselEl, 1)} aria-label="Scroll right"><ChevronRight class="size-4" /></button>
      </div>
    </div>
    <div bind:this={topMixesCarouselEl} class="flex gap-4 overflow-x-auto pb-2" style="scrollbar-width:none;-ms-overflow-style:none">
      {#each topMixArtists as artist, i (artist)}
        <a href="/for-you/artist/{encodeURIComponent(artist)}" class="app-card app-hover stagger-item group flex w-40 shrink-0 flex-col gap-2 rounded-2xl p-3" style="--stagger-index:{i}">
          <div class="grid aspect-square w-full place-items-center rounded-md bg-gradient-to-br from-amber-500 to-orange-800 shadow-md">
            <span class="text-2xl font-bold text-white/80">{artist.slice(0, 2).toUpperCase()}</span>
          </div>
          <div>
            <p class="truncate text-sm font-semibold">{artist} Mix</p>
            <p class="truncate text-xs text-muted-foreground">{artist}</p>
          </div>
        </a>
      {/each}
    </div>
  </section>
{/if}

<!-- 5. Newly released (newest albums from artists you listen to) -->
{#if newlyReleasedAlbums.length > 0 || loading}
  <section class="page-section mb-10">
    <div class="mb-4 flex items-center justify-between">
      <h2 class="app-section-title text-xl font-bold">Newly released</h2>
      <div class="flex gap-1">
        <button class="app-round-button grid size-8 place-items-center rounded-full" onclick={() => scroll(newlyReleasedCarouselEl, -1)} aria-label="Scroll left"><ChevronLeft class="size-4" /></button>
        <button class="app-round-button grid size-8 place-items-center rounded-full" onclick={() => scroll(newlyReleasedCarouselEl, 1)} aria-label="Scroll right"><ChevronRight class="size-4" /></button>
      </div>
    </div>
    <div bind:this={newlyReleasedCarouselEl} class="flex gap-4 overflow-x-auto pb-2" style="scrollbar-width:none;-ms-overflow-style:none">
      {#if loading}
        {#each Array(8) as _, i (i)}
          <div class="flex w-40 shrink-0 flex-col gap-2 p-3">
            <div class="aspect-square w-full animate-pulse rounded-md bg-muted"></div>
            <div class="h-3 w-24 animate-pulse rounded bg-muted"></div>
          </div>
        {/each}
      {:else}
        {#each newlyReleasedAlbums as album, index (album.id)}
          <AlbumContextMenu {album} onplay={() => playAlbum(album)} triggerClass="contents">
            <div class="app-card app-hover stagger-item group relative flex w-40 shrink-0 flex-col gap-2 rounded-2xl p-3" style="--stagger-index:{index}">
              <a href={`/album/${encodeURIComponent(album.id)}`} class="absolute inset-0 rounded-lg" aria-label={album.name}></a>
              <div class="relative">
                {#if album.coverArtUrl}
                  <img class="aspect-square w-full rounded-md object-cover shadow-md" src={album.coverArtUrl} alt={album.name} loading="lazy" />
                {:else}
                  <div class="grid aspect-square w-full place-items-center rounded-md bg-gradient-to-br from-slate-600 to-slate-800 shadow-md">
                    <Disc3 class="size-10 text-white/50" />
                  </div>
                {/if}
                <button
                  class="absolute bottom-2 right-2 z-10 grid size-10 translate-y-1 place-items-center rounded-full bg-primary text-primary-foreground opacity-0 shadow-lg transition group-hover:translate-y-0 group-hover:opacity-100"
                  onclick={(e) => { e.preventDefault(); playAlbum(album); }}
                  aria-label="Play {album.name}"
                >
                  {#if albumLoadingId === album.id}
                    <span class="block size-4 animate-spin rounded-full border-2 border-primary-foreground border-t-transparent"></span>
                  {:else}
                    <Play class="size-4 translate-x-px" fill="currentColor" />
                  {/if}
                </button>
              </div>
              <div>
                <p class="truncate text-sm font-semibold">{album.name}</p>
                <p class="truncate text-xs text-muted-foreground">{album.artist}{album.year ? ` · ${album.year}` : ''}</p>
              </div>
            </div>
          </AlbumContextMenu>
        {/each}
      {/if}
    </div>
  </section>
{/if}

<!-- You might like (random picks) -->
{#if recommendedAlbums.length > 0}
  <section class="page-section mb-10">
    <div class="mb-4 flex items-center justify-between">
      <h2 class="app-section-title text-xl font-bold">You might like</h2>
      <div class="flex gap-1">
        <button class="app-round-button grid size-8 place-items-center rounded-full" onclick={() => scroll(randomCarouselEl, -1)} aria-label="Scroll left"><ChevronLeft class="size-4" /></button>
        <button class="app-round-button grid size-8 place-items-center rounded-full" onclick={() => scroll(randomCarouselEl, 1)} aria-label="Scroll right"><ChevronRight class="size-4" /></button>
      </div>
    </div>
    <div bind:this={randomCarouselEl} class="flex gap-4 overflow-x-auto pb-2" style="scrollbar-width:none;-ms-overflow-style:none">
      {#each recommendedAlbums as album, index (album.id)}
        <AlbumContextMenu {album} onplay={() => playAlbum(album)} triggerClass="contents">
          <div class="app-card app-hover stagger-item group relative flex w-40 shrink-0 flex-col gap-2 rounded-2xl p-3" style={`--stagger-index:${index};`}>
            <a href={`/album/${encodeURIComponent(album.id)}`} class="absolute inset-0 rounded-lg" aria-label={album.name}></a>
            <div class="relative">
              {#if album.coverArtUrl}
                <img class="aspect-square w-full rounded-md object-cover shadow-md" src={album.coverArtUrl} alt={album.name} loading="lazy" />
              {:else}
                <div class="grid aspect-square w-full place-items-center rounded-md bg-gradient-to-br from-slate-600 to-slate-800 shadow-md">
                  <Disc3 class="size-10 text-white/50" />
                </div>
              {/if}
              <button
                class="absolute bottom-2 right-2 z-10 grid size-10 translate-y-1 place-items-center rounded-full bg-primary text-primary-foreground opacity-0 shadow-lg transition group-hover:translate-y-0 group-hover:opacity-100"
                onclick={(e) => { e.preventDefault(); playAlbum(album); }}
                aria-label="Play {album.name}"
              >
                {#if albumLoadingId === album.id}
                  <span class="block size-4 animate-spin rounded-full border-2 border-primary-foreground border-t-transparent"></span>
                {:else}
                  <Play class="size-4 translate-x-px" fill="currentColor" />
                {/if}
              </button>
            </div>
            <div>
              <p class="truncate text-sm font-semibold">{album.name}</p>
              <p class="truncate text-xs text-muted-foreground">{album.artist}{album.year ? ` · ${album.year}` : ''}</p>
            </div>
          </div>
        </AlbumContextMenu>
      {/each}
    </div>
  </section>
{/if}

<!-- Your artists carousel -->
{#if liked.length > 0}
  <section class="page-section mb-10">
    <div class="mb-4 flex items-center justify-between">
      <h2 class="app-section-title text-xl font-bold">Your artists</h2>
      <div class="flex gap-1">
        <button class="app-round-button grid size-8 place-items-center rounded-full" onclick={() => scroll(likedCarouselEl, -1)} aria-label="Scroll left"><ChevronLeft class="size-4" /></button>
        <button class="app-round-button grid size-8 place-items-center rounded-full" onclick={() => scroll(likedCarouselEl, 1)} aria-label="Scroll right"><ChevronRight class="size-4" /></button>
      </div>
    </div>
    <div bind:this={likedCarouselEl} class="flex gap-4 overflow-x-auto pb-2" style="scrollbar-width:none;-ms-overflow-style:none">
      {#each liked as artist, index (artist)}
        <ArtistContextMenu name={artist} triggerClass="contents">
          <a href={`/artist/${encodeURIComponent(artist)}`} class="app-card app-hover stagger-item group flex w-36 shrink-0 flex-col items-center gap-2 rounded-2xl p-3 text-center" style={`--stagger-index:${index};`}>
            <div class="grid size-20 place-items-center rounded-full bg-gradient-to-br from-slate-500 to-slate-700 text-lg font-bold">
              {artist.slice(0, 2).toUpperCase()}
            </div>
            <p class="w-full truncate text-xs font-semibold">{artist}</p>
            <p class="text-[11px] text-muted-foreground">Artist</p>
          </a>
        </ArtistContextMenu>
      {/each}
    </div>
  </section>
{/if}

<!-- Trending artists carousel -->
{#if top.length > 0}
  <section class="page-section mb-10">
    <div class="mb-4 flex items-center justify-between">
      <h2 class="app-section-title text-xl font-bold">Trending artists</h2>
      <div class="flex gap-1">
        <button class="app-round-button grid size-8 place-items-center rounded-full" onclick={() => scroll(trendingCarouselEl, -1)} aria-label="Scroll left"><ChevronLeft class="size-4" /></button>
        <button class="app-round-button grid size-8 place-items-center rounded-full" onclick={() => scroll(trendingCarouselEl, 1)} aria-label="Scroll right"><ChevronRight class="size-4" /></button>
      </div>
    </div>
    <div bind:this={trendingCarouselEl} class="flex gap-4 overflow-x-auto pb-2" style="scrollbar-width:none;-ms-overflow-style:none">
      {#each top as artist, index (artist.id)}
        <ArtistContextMenu name={artist.name} triggerClass="contents">
          <div class="app-card app-hover stagger-item group relative flex w-36 shrink-0 flex-col items-center gap-2 rounded-2xl p-3 text-center" style={`--stagger-index:${index};`}>
            <a href={`/artist/${encodeURIComponent(artist.name)}`} class="absolute inset-0 rounded-lg" aria-label={artist.name}></a>
            {#if artist.imageUrl}
              <img class="size-20 rounded-full object-cover" src={artist.imageUrl} alt={artist.name} loading="lazy" />
            {:else}
              <div class="grid size-20 place-items-center rounded-full bg-gradient-to-br from-slate-500 to-slate-700 text-lg font-bold">
                {artist.name.slice(0, 2).toUpperCase()}
              </div>
            {/if}
            <p class="w-full truncate text-xs font-semibold">{artist.name}</p>
            <p class="text-[11px] text-muted-foreground">Artist</p>
            <button
              class={`relative z-10 mt-0.5 grid size-7 place-items-center rounded-full border transition ${
                liked.includes(artist.name)
                  ? 'border-primary bg-primary text-primary-foreground'
                  : 'border-input bg-secondary text-muted-foreground hover:border-primary hover:text-primary'
              }`}
              onclick={(e) => { e.preventDefault(); toggleArtist(artist.name); }}
              disabled={togglingArtist === artist.name}
              aria-label={liked.includes(artist.name) ? `Remove ${artist.name}` : `Add ${artist.name}`}
            >
              {#if liked.includes(artist.name)}
                <Check class="size-3" />
              {:else}
                <Plus class="size-3" />
              {/if}
            </button>
          </div>
        </ArtistContextMenu>
      {/each}
    </div>
  </section>
{/if}
