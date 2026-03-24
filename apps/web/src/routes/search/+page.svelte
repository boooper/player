<script lang="ts">
  import { goto } from '$app/navigation';
  import { Clock, X, Play } from '@lucide/svelte';

  import { searchBundle, type SearchBundlePayload, type Song } from '$lib/servers';
  import { focusTrack, playQueue, queue, currentIndex, isPlaying, togglePlayRequest } from '$lib/stores/player';
  import { SongRow, SongCard, AlbumCard, ArtistCard } from '$lib/components/media';
  import SongContextMenu from '$lib/components/SongContextMenu.svelte';
  import SongTechBadge from '$lib/components/SongTechBadge.svelte';
  import {
    Carousel, CarouselContent, CarouselItem, CarouselNext, CarouselPrevious
  } from '$lib/components/ui/carousel';
  import { readUiJson, writeUiJson } from '$lib/ui-storage';
  import { backendSettings } from '$lib/stores/backend-settings';
  import { libraryRefresh } from '$lib/stores/ui-state';

  let { data } = $props<{ data: { q: string } }>();

  const query = $derived((data.q ?? '').trim());
  let loading = $state(false);
  let error = $state('');
  let results = $state<SearchBundlePayload>({ songs: [], albums: [], recommendations: [] });
  let lastExecutedQuery = '';

  const RECENT_KEY = 'madrify_recent_searches';
  const LEGACY_RECENT_KEY = 'naviarr_recent_searches';
  let recentSearches = $state<string[]>([]);
  let searchLoadVersion = 0;

  async function loadRecentSearches() {
    recentSearches = await readUiJson<string[]>(RECENT_KEY, [], [LEGACY_RECENT_KEY]);
  }
  function removeRecentSearch(q: string) {
    const updated = recentSearches.filter((r) => r !== q);
    recentSearches = updated;
    void writeUiJson(RECENT_KEY, updated, [LEGACY_RECENT_KEY]);
  }
  $effect(() => { loadRecentSearches(); });

  async function runQuerySearch() {
    const refresh = $libraryRefresh;
    const metadataProvider = $backendSettings.metadataProvider;
    const recommendationProvider = $backendSettings.recommendationProvider;
    const requestKey = `${query}::${refresh}::${metadataProvider}::${recommendationProvider}`;
    if (!query || lastExecutedQuery === requestKey) return;
    lastExecutedQuery = requestKey;
    await loadSearch(query);
  }

  function initials(name: string) {
    return name.split(' ').filter(Boolean).slice(0, 2).map((p) => p[0]?.toUpperCase() ?? '').join('');
  }

  async function loadSearch(queryText: string) {
    if (!queryText.trim()) return;
    const loadVersion = ++searchLoadVersion;
    loading = true;
    error = '';
    results = { songs: [], albums: [], recommendations: [] };

    try {
      const nextResults = await searchBundle(queryText, 24, 12, 16);
      if (loadVersion !== searchLoadVersion) return;
      results = nextResults;

      if (results.songs[0]) {
        const song = results.songs[0];
        focusTrack.set({ title: song.title, artist: song.artist, imageUrl: song.coverArtUrl, source: 'library', album: song.album });
      }
      if (!results.songs.length && !results.albums.length && !results.recommendations.length) {
        error = 'No results found for this search.';
      }
    } catch (err) {
      if (loadVersion !== searchLoadVersion) return;
      error = err instanceof Error ? err.message : typeof err === 'string' ? err : 'Search failed.';
    } finally {
      if (loadVersion !== searchLoadVersion) return;
      loading = false;
    }
  }

  $effect(() => {
    const queryText = query;
    const refresh = $libraryRefresh;
    const metadataProvider = $backendSettings.metadataProvider;
    const recommendationProvider = $backendSettings.recommendationProvider;
    void queryText; void refresh; void metadataProvider; void recommendationProvider;
    if (!query) {
      lastExecutedQuery = '';
      results = { songs: [], albums: [], recommendations: [] };
      error = '';
      return;
    }
    void runQuerySearch();
  });

  function playSong(index: number) {
    playQueue(results.songs, index);
    focusTrack.set({ title: results.songs[index].title, artist: results.songs[index].artist, imageUrl: results.songs[index].coverArtUrl, source: 'library', album: results.songs[index].album });
  }

  function playRecommendation(index: number) {
    if (!results.recommendations.length) return;
    const song = results.recommendations[index];
    focusTrack.set({ title: song.title, artist: song.artist, imageUrl: song.coverArtUrl, source: 'library', album: song.album });
    playQueue(results.recommendations, index);
  }

  const topResult = $derived(results.songs[0] ?? null);
  const songRows = $derived(results.songs.slice(0, 5));
  const isTopActive = $derived($queue[$currentIndex]?.id === topResult?.id);

  const artists = $derived(() => {
    const seen = new Set<string>();
    const list: { name: string; image: string }[] = [];
    for (const song of [...results.songs, ...results.recommendations]) {
      if (!seen.has(song.artist)) {
        seen.add(song.artist);
        list.push({ name: song.artist, image: song.coverArtUrl });
      }
    }
    return list;
  });
</script>

<!-- ── No query ─────────────────────────────────────────────────────── -->
{#if !query}
  {#if recentSearches.length > 0}
    <div class="page-section mb-6">
      <p class="section-label">Recent</p>
      <div class="glass-panel overflow-hidden rounded-2xl">
        {#each recentSearches as term, index (term)}
          <div class="flex items-center border-b border-border/50 last:border-b-0">
            <button
              class="glass-row stagger-row flex flex-1 items-center gap-3 px-4 py-3 text-left"
              style:--stagger-index={index}
              onclick={() => goto(`/search?q=${encodeURIComponent(term)}`)}
            >
              <Clock class="size-3.5 shrink-0 text-muted-foreground" />
              <span class="text-sm font-medium">{term}</span>
            </button>
              <button
              class="shrink-0 px-4 text-muted-foreground/60 transition-colors hover:text-muted-foreground"
              aria-label="Remove"
              onclick={() => removeRecentSearch(term)}
            >
              <X class="size-3.5" />
            </button>
          </div>
        {/each}
      </div>
    </div>
  {:else}
    <p class="text-sm text-muted-foreground">Enter a query in the top bar to search.</p>
  {/if}

<!-- ── Has query ──────────────────────────────────────────────────────── -->
{:else}
  {#if error}
    <div class="mb-6 rounded-xl border border-red-500/20 px-4 py-3" style="background:rgba(239,68,68,0.08)">
      <p class="text-sm text-red-400">{error}</p>
    </div>
  {/if}

  <!-- Top result + Songs -->
  {#if loading || results.songs.length > 0}
    <div class="mb-8 grid gap-5" style="grid-template-columns: 220px 1fr">

      <!-- Top Result -->
      <div>
        <p class="section-label">Top result</p>
          {#if loading}
          <div class="glass-panel rounded-2xl p-5">
            <div class="mb-4 h-[88px] w-[88px] animate-pulse rounded-xl" style="background: color-mix(in srgb, var(--color-muted) 7%, transparent)"></div>
            <div class="mb-2 h-5 w-3/4 animate-pulse rounded-full" style="background: color-mix(in srgb, var(--color-muted) 7%, transparent)"></div>
            <div class="h-3 w-1/2 animate-pulse rounded-full" style="background: color-mix(in srgb, var(--color-muted) 4%, transparent)"></div>
          </div>
        {:else if topResult}
          <SongContextMenu song={topResult} onplay={() => playSong(0)}>
            <button
              class="top-result-card glass-panel w-full rounded-2xl p-5 text-left"
              class:top-result-active={isTopActive}
              onclick={() => isTopActive ? togglePlayRequest.update(n => n + 1) : playSong(0)}
            >
              {#if topResult.coverArtUrl}
                <img class="mb-4 h-[88px] w-[88px] rounded-xl object-cover shadow-xl" src={topResult.coverArtUrl} alt={topResult.title} />
              {:else}
                <div class="mb-4 flex h-[88px] w-[88px] items-center justify-center rounded-xl bg-gradient-to-br from-white/15 to-white/5 text-2xl font-bold text-muted-foreground/60 shadow-xl">
                  {initials(topResult.title)}
                </div>
              {/if}
              <p class="mb-0.5 line-clamp-2 text-xl font-bold leading-tight" class:text-primary={isTopActive}>{topResult.title}</p>
              <p class="truncate text-sm text-muted-foreground/80">{topResult.artist}</p>
              <span class="mt-3 inline-block rounded-full border border-border/40 px-2.5 py-0.5 text-xs text-muted-foreground/70" style="background: color-mix(in srgb, var(--color-muted) 5%, transparent)">Song</span>
              <div class="play-fab flex h-10 w-10 items-center justify-center rounded-full shadow-xl" style="background:hsl(var(--primary))">
                {#if isTopActive && $isPlaying}
                  <svg class="size-4 fill-white text-white" viewBox="0 0 24 24"><rect x="6" y="4" width="4" height="16"/><rect x="14" y="4" width="4" height="16"/></svg>
                {:else}
                  <Play class="size-4 translate-x-px fill-white text-white" />
                {/if}
              </div>
            </button>
          </SongContextMenu>
        {/if}
      </div>

      <!-- Songs list using SongRow -->
      <div>
        <p class="section-label">Songs</p>
        <div class="glass-panel overflow-hidden rounded-2xl px-1 py-1">
          {#if loading}
            {#each { length: 5 } as _, i}
              <div class="flex items-center gap-3 px-3 py-2.5">
                <div class="h-10 w-10 shrink-0 animate-pulse rounded-lg" style="background: color-mix(in srgb, var(--color-muted) 7%, transparent)"></div>
                <div class="flex-1 space-y-2">
                  <div class="h-2.5 animate-pulse rounded-full" style="width:{55+(i*13)%35}%;background: color-mix(in srgb, var(--color-muted) 7%, transparent)"></div>
                  <div class="h-2 animate-pulse rounded-full" style="width:{28+(i*11)%22}%;background: color-mix(in srgb, var(--color-muted) 4%, transparent)"></div>
                </div>
                <div class="h-2 w-7 shrink-0 animate-pulse rounded-full" style="background: color-mix(in srgb, var(--color-muted) 4%, transparent)"></div>
              </div>
            {/each}
          {:else}
            {#each songRows as song, i (song.id)}
              <SongRow {song} index={i} onplay={() => playSong(i)} staggerIndex={i} />
            {/each}
          {/if}
        </div>
      </div>
    </div>
  {/if}

  <!-- Artists carousel -->
  {#if artists().length > 0}
    <div class="mb-8">
      <p class="section-label">Artists</p>
      <Carousel opts={{ align: 'start', dragFree: true }}>
        <CarouselContent>
          {#each artists() as artist (artist.name)}
            <CarouselItem class="basis-[90px]">
              <ArtistCard name={artist.name} image={artist.image} />
            </CarouselItem>
          {/each}
        </CarouselContent>
        <CarouselPrevious />
        <CarouselNext />
      </Carousel>
    </div>
  {/if}

  <!-- Albums carousel -->
  {#if results.albums.length > 0}
    <div class="mb-8">
      <p class="section-label">Albums</p>
      <Carousel opts={{ align: 'start', dragFree: true }}>
        <CarouselContent>
          {#each results.albums as album (album.id)}
            <CarouselItem class="basis-[130px]">
              <AlbumCard {album} />
            </CarouselItem>
          {/each}
        </CarouselContent>
        <CarouselPrevious />
        <CarouselNext />
      </Carousel>
    </div>
  {/if}

  <!-- Recommendations carousel -->
  {#if results.recommendations.length > 0}
    <div class="mb-8">
      <p class="section-label">Recommended</p>
      <Carousel opts={{ align: 'start', dragFree: true }}>
        <CarouselContent>
          {#each results.recommendations as song, i (song.id)}
            <CarouselItem class="basis-[130px]">
              <SongCard {song} onplay={() => playRecommendation(i)} />
            </CarouselItem>
          {/each}
        </CarouselContent>
        <CarouselPrevious />
        <CarouselNext />
      </Carousel>
    </div>
  {/if}
{/if}

<style>
  .section-label {
    margin-bottom: 12px;
    font-size: 0.7rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    color: var(--color-muted-foreground);
  }
  .glass-panel {
    background: var(--color-card);
    border: 1px solid var(--color-border);
    backdrop-filter: blur(12px);
  }
  .glass-row {
    transition: background 150ms ease;
  }
  .glass-row:hover {
    background: color-mix(in srgb, var(--color-muted) 8%, transparent);
  }

  /* Top result */
  .top-result-card {
    position: relative;
    min-height: 200px;
    transition: background 150ms ease;
  }
  .top-result-card:hover {
    background: color-mix(in srgb, var(--color-muted) 10%, transparent);
  }
  .top-result-active {
    background: hsl(var(--primary) / 0.1);
    border-color: hsl(var(--primary) / 0.25);
  }
  .play-fab {
    position: absolute;
    bottom: 16px;
    right: 16px;
    opacity: 0;
    transform: translateY(6px);
    transition: opacity 200ms ease, transform 200ms ease;
  }
  .top-result-card:hover .play-fab,
  .top-result-active .play-fab {
    opacity: 1;
    transform: translateY(0);
  }
</style>
