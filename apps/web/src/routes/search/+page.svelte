<script lang="ts">
  import { goto } from '$app/navigation';
  import { Clock, X } from '@lucide/svelte';

  import { searchBundle, type SearchBundlePayload, type Song } from '$lib/servers';
  import { focusTrack, playQueue } from '$lib/stores/player';
  import { Badge, Button } from '$lib/components/ui';
  import SongContextMenu from '$lib/components/SongContextMenu.svelte';
  import AlbumContextMenu from '$lib/components/AlbumContextMenu.svelte';
  import SongTechBadge from '$lib/components/SongTechBadge.svelte';
  import { readUiJson, writeUiJson } from '$lib/ui-storage';
  import { backendSettings } from '$lib/stores/backend-settings';
  import { libraryRefresh } from '$lib/stores/ui-state';

  let { data } = $props<{ data: { q: string } }>();

  const query = $derived((data.q ?? '').trim());
  let loading = $state(false);
  let error = $state('');
  let results = $state<SearchBundlePayload>({ songs: [], albums: [], recommendations: [] });
  let selectedSongId = $state<string | null>(null);
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

  $effect(() => {
    loadRecentSearches();
  });

  async function runQuerySearch() {
    const refresh = $libraryRefresh;
    const metadataProvider = $backendSettings.metadataProvider;
    const recommendationProvider = $backendSettings.recommendationProvider;
    const requestKey = `${query}::${refresh}::${metadataProvider}::${recommendationProvider}`;
    if (!query || lastExecutedQuery === requestKey) return;
    lastExecutedQuery = requestKey;
    await loadSearch(query);
  }

  function initials(name: string): string {
    return name
      .split(' ')
      .filter(Boolean)
      .slice(0, 2)
      .map((part) => part[0]?.toUpperCase() ?? '')
      .join('');
  }

  async function loadSearch(queryText: string) {
    if (!queryText.trim()) return;
    const loadVersion = ++searchLoadVersion;

    loading = true;
    error = '';
    results = { songs: [], albums: [], recommendations: [] };
    selectedSongId = null;

    try {
      const nextResults = await searchBundle(queryText, 24, 12, 16);
      if (loadVersion !== searchLoadVersion) return;
      results = nextResults;

      if (results.songs[0]) {
        const song = results.songs[0];
        selectedSongId = song.id;
        focusTrack.set({
          title: song.title,
          artist: song.artist,
          imageUrl: song.coverArtUrl,
          source: 'library',
          album: song.album
        });
      }

      if (!results.songs.length && !results.albums.length && !results.recommendations.length) {
        error = 'No results found for this search.';
      }
    } catch (err) {
      if (loadVersion !== searchLoadVersion) return;
      error = err instanceof Error ? err.message : 'Search failed.';
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
    void queryText;
    void refresh;
    void metadataProvider;
    void recommendationProvider;
    if (!query) {
      lastExecutedQuery = '';
      results = { songs: [], albums: [], recommendations: [] };
      selectedSongId = null;
      error = '';
      return;
    }
    void runQuerySearch();
  });

  function pick(song: Song) {
    selectedSongId = song.id;
    focusTrack.set({
      title: song.title,
      artist: song.artist,
      imageUrl: song.coverArtUrl,
      source: 'library',
      album: song.album
    });
    const startIndex = results.songs.findIndex((item) => item.id === song.id);
    if (startIndex >= 0) {
      playQueue(results.songs, startIndex);
    } else {
      playQueue([song], 0);
    }
  }

  function playRecommendation(index: number) {
    if (!results.recommendations.length) return;
    const song = results.recommendations[index];
    focusTrack.set({
      title: song.title,
      artist: song.artist,
      imageUrl: song.coverArtUrl,
      source: 'library',
      album: song.album
    });
    playQueue(results.recommendations, index);
  }
</script>

<div class="mb-4 flex items-center justify-between gap-3">
  <h2 class="text-2xl font-bold tracking-tight">Search</h2>
  {#if query}
    <p class="text-sm text-muted-foreground">Results for: {query}</p>
  {/if}
</div>

{#if !query}
  {#if recentSearches.length > 0}
    <div class="page-section mb-6">
      <h3 class="mb-3 text-sm font-semibold uppercase tracking-wide text-muted-foreground">Recent searches</h3>
      <div class="divide-y divide-border overflow-hidden rounded-lg border border-input">
        {#each recentSearches as term, index (term)}
          <div class="flex items-center">
            <button
              class="stagger-row flex flex-1 items-center gap-3 bg-secondary px-3 py-2.5 text-left transition hover:bg-accent"
              style:--stagger-index={index}
              onclick={() => goto(`/search?q=${encodeURIComponent(term)}`)}
            >
              <Clock class="size-4 shrink-0 text-muted-foreground" />
              <span class="text-sm font-medium">{term}</span>
            </button>
            <button
              class="shrink-0 px-3 text-muted-foreground transition hover:text-foreground"
              aria-label="Remove"
              onclick={() => removeRecentSearch(term)}
            >
              <X class="size-4" />
            </button>
          </div>
        {/each}
      </div>
    </div>
  {:else}
    <p class="text-sm text-muted-foreground">Enter a query in the top bar to search.</p>
  {/if}
{:else}
  {#if error}
    <p class="mb-3 text-sm text-destructive">{error}</p>
  {/if}

  <div class="mb-2 flex items-center justify-between">
    <h3 class="text-sm font-semibold uppercase tracking-wide text-muted-foreground">Songs</h3>
    {#if loading}
      <p class="text-sm text-muted-foreground">Loading...</p>
    {/if}
  </div>
  <div class="page-section mb-6 divide-y divide-border overflow-hidden rounded-lg border border-input">
    {#each results.songs as song, index (song.id)}
      <SongContextMenu {song} onplay={() => pick(song)}>
        <button
          class={`stagger-row w-full ${selectedSongId === song.id
            ? 'flex items-center gap-3 bg-accent px-3 py-2.5 text-left ring-1 ring-inset ring-primary'
            : 'flex items-center gap-3 bg-secondary px-3 py-2.5 text-left transition hover:bg-accent'}`}
          style:--stagger-index={index}
          onclick={() => pick(song)}
        >
          {#if song.coverArtUrl}
            <img class="h-9 w-9 shrink-0 rounded object-cover" src={song.coverArtUrl} alt={song.title} loading="lazy" />
          {:else}
            <div class="flex h-9 w-9 shrink-0 items-center justify-center rounded bg-gradient-to-br from-slate-500 to-slate-700 text-xs font-bold">
              {initials(song.title)}
            </div>
          {/if}
          <div class="min-w-0 flex-1">
            <div class="flex items-center gap-2">
              <p class="truncate text-sm font-semibold">{song.title}</p>
              <SongTechBadge id={song.id} audioFormat={song.audioFormat} bitrateKbps={song.bitrateKbps} compact />
            </div>
            <p class="truncate text-xs text-muted-foreground">{song.artist}</p>
          </div>
        </button>
      </SongContextMenu>
    {/each}
  </div>

  {#if results.albums.length > 0}
    <div class="mb-2 flex items-center justify-between">
      <h3 class="text-sm font-semibold uppercase tracking-wide text-muted-foreground">Albums</h3>
    </div>
    <div class="page-section mb-6 divide-y divide-border overflow-hidden rounded-lg border border-input">
      {#each results.albums as album, index (album.id)}
        <AlbumContextMenu {album}>
          <button
            class="stagger-row flex w-full items-center gap-3 bg-secondary px-3 py-2.5 text-left transition hover:bg-accent"
            style:--stagger-index={index}
            onclick={() => goto(`/album/${encodeURIComponent(album.id)}`)}
          >
            {#if album.coverArtUrl}
              <img class="h-10 w-10 shrink-0 rounded object-cover" src={album.coverArtUrl} alt={album.name} loading="lazy" />
            {:else}
              <div class="flex h-10 w-10 shrink-0 items-center justify-center rounded bg-gradient-to-br from-slate-500 to-slate-700 text-xs font-bold">
                {initials(album.name)}
              </div>
            {/if}
            <div class="min-w-0 flex-1">
              <p class="truncate text-sm font-semibold">{album.name}</p>
              <p class="truncate text-xs text-muted-foreground">{album.artist}</p>
            </div>
            <Badge variant="outline">Album</Badge>
          </button>
        </AlbumContextMenu>
      {/each}
    </div>
  {/if}

  {#if results.recommendations.length > 0}
    <div class="mb-2 flex items-center justify-between">
      <h3 class="text-sm font-semibold uppercase tracking-wide text-muted-foreground">Recommendations</h3>
    </div>
    <div class="page-section divide-y divide-border overflow-hidden rounded-lg border border-input">
      {#each results.recommendations as song, index (song.id)}
        <SongContextMenu {song} onplay={() => playRecommendation(index)}>
          <div class="stagger-row flex items-center gap-3 bg-secondary px-3 py-2.5" style:--stagger-index={index}>
            {#if song.coverArtUrl}
              <img class="h-9 w-9 shrink-0 rounded object-cover" src={song.coverArtUrl} alt={song.title} loading="lazy" />
            {:else}
              <div class="flex h-9 w-9 shrink-0 items-center justify-center rounded bg-gradient-to-br from-slate-500 to-slate-700 text-xs font-bold">
                {initials(song.title)}
              </div>
            {/if}
            <div class="min-w-0 flex-1">
              <div class="flex items-center gap-2">
                <p class="truncate text-sm font-semibold">{song.title}</p>
                <SongTechBadge id={song.id} audioFormat={song.audioFormat} bitrateKbps={song.bitrateKbps} compact />
              </div>
              <p class="truncate text-xs text-muted-foreground">{song.artist}</p>
            </div>
            <Button size="sm" onclick={() => playRecommendation(index)}>Play</Button>
          </div>
        </SongContextMenu>
      {/each}
    </div>
  {/if}
{/if}
