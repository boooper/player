<script lang="ts">
  import { onMount } from 'svelte';
  import { afterNavigate, goto } from '$app/navigation';

  import { Clock, X } from '@lucide/svelte';
  import { searchSongs as searchLastFmSongs, type Song as LastFmSong } from '$lib/metadata';
  import { getRecommendations, getTrackTopGenre, type TrackRecommendation } from '$lib/recommendation';
  import { fetchLikedArtists, fetchSimilarSongs, searchSongs, type Song } from '$lib/api';
  import { focusTrack, playQueue } from '$lib/stores/player';
  import { appSettings } from '$lib/stores/settings';
  import { Badge, Button } from '$lib/components/ui';
  import SongContextMenu from '$lib/components/SongContextMenu.svelte';

  let { data } = $props<{ data: { q: string } }>();

  const lastFmApiKey = $derived($appSettings.lastFmApiKey);
  const hasLastFmKey = $derived(Boolean(lastFmApiKey));

  const query = $derived((data.q ?? '').trim());
  let loading = $state(false);
  let error = $state('');

  let likedArtists = $state<string[]>([]);
  let lastfmSongs = $state<LastFmSong[]>([]);
  let subsonicSongs = $state<Song[]>([]);
  let subsonicSimilar = $state<Song[]>([])

  type HybridRecommendation = {
    id: string;
    title: string;
    artist: string;
    url: string;
    matchScore: number;
    genreScore: number;
    combinedScore: number;
    playable: boolean;
    subsonicSong: Song | null;
    source: 'hybrid';
  };

  type SelectedItem = {
    id: string;
    title: string;
    artist: string;
    imageUrl: string;
    source: 'lastfm' | 'library';
    subsonicSong?: Song | null;
  };

  let selected = $state<SelectedItem | null>(null);
  let recs = $state<TrackRecommendation[]>([]);
  let recError = $state('');
  let recLoading = $state(false);
  let hybridRecs = $state<HybridRecommendation[]>([])
  let lastExecutedQuery = '';

  const RECENT_KEY = 'naviarr_recent_searches';
  let recentSearches = $state<string[]>([]);

  function loadRecentSearches() {
    try {
      recentSearches = JSON.parse(localStorage.getItem(RECENT_KEY) ?? '[]');
    } catch {
      recentSearches = [];
    }
  }

  function removeRecentSearch(q: string) {
    const updated = recentSearches.filter((r) => r !== q);
    recentSearches = updated;
    localStorage.setItem(RECENT_KEY, JSON.stringify(updated));
  }

  const merged = $derived.by(() => {
    const items: Array<{
      id: string;
      title: string;
      artist: string;
      imageUrl: string;
      source: 'lastfm' | 'library';
      subsonicSong: Song | null;
    }> = [];
    const seen: Record<string, boolean> = {};

    subsonicSongs.forEach((song) => {
      const key = `${song.artist.toLowerCase()}::${song.title.toLowerCase()}`;
      if (seen[key]) return;
      seen[key] = true;
      items.push({
        id: `sub-${song.id}`,
        title: song.title,
        artist: song.artist,
        imageUrl: song.coverArtUrl,
        source: 'library',
        subsonicSong: song
      });
    });

    lastfmSongs.forEach((song) => {
      const key = `${song.artist.toLowerCase()}::${song.title.toLowerCase()}`;
      if (seen[key]) return;
      seen[key] = true;
      items.push({
        id: song.id,
        title: song.title,
        artist: song.artist,
        imageUrl: song.imageUrl,
        source: 'lastfm',
        subsonicSong: null
      });
    });

    return items;
  });

  onMount(async () => {
    loadRecentSearches();
    try {
      const artists = await fetchLikedArtists();
      likedArtists = artists.map((entry) => entry.name);
    } catch {
      likedArtists = [];
    }

    await runQuerySearch();
  });

  afterNavigate(() => {
    void runQuerySearch();
  });

  async function runQuerySearch() {
    if (!query || loading || lastExecutedQuery === query) return;
    lastExecutedQuery = query;
    await searchAll(query);
  }

  function initials(name: string): string {
    return name
      .split(' ')
      .filter(Boolean)
      .slice(0, 2)
      .map((part) => part[0]?.toUpperCase() ?? '')
      .join('');
  }

  async function searchAll(queryText: string) {
    if (!queryText.trim() || loading) return;

    loading = true;
    error = '';
    recs = [];
    hybridRecs = [];
    recError = '';

    try {
      const [lfm, sub] = await Promise.all([
        hasLastFmKey ? searchLastFmSongs(queryText, 24) : Promise.resolve([]),
        searchSongs(queryText, 24)
      ]);

      lastfmSongs = lfm;
      subsonicSongs = sub;
      subsonicSimilar = [];

      selected = merged[0] ?? null;
      if (selected) {
        focusTrack.set({
          title: selected.title,
          artist: selected.artist,
          imageUrl: selected.imageUrl,
          source: selected.source
        });
        await loadRecommendations(selected.title, selected.artist);
      }

      if (!lfm.length && !sub.length) {
        error = 'No songs found for this search.';
      }
    } catch (err) {
      error = err instanceof Error ? err.message : 'Search failed.';
      lastfmSongs = [];
      subsonicSongs = [];
    } finally {
      loading = false;
    }
  }

  function findBestMatch(candidates: Song[], recArtist: string, recTitle: string): Song | null {
    if (!candidates.length) return null;
    const norm = (s: string) => s.toLowerCase().trim().replace(/^the\s+/, '');

    // 1. Exact artist + exact title
    const exact = candidates.find(
      (s) => norm(s.artist) === norm(recArtist) && norm(s.title) === norm(recTitle)
    );
    if (exact) return exact;

    // 2. Artist contains/contained + title contains/contained (handles "The X" vs "X", remasters, feat. tags, etc.)
    const loose = candidates.find(
      (s) =>
        (norm(s.artist).includes(norm(recArtist)) || norm(recArtist).includes(norm(s.artist))) &&
        (norm(s.title).includes(norm(recTitle)) || norm(recTitle).includes(norm(s.title)))
    );
    if (loose) return loose;

    // 3. If the search was already targeted ("Artist Title"), trust the first result
    //    rather than declaring it un-playable.
    return candidates[0] ?? null;
  }

  async function loadRecommendations(title: string, artist: string) {
    if (!hasLastFmKey) return;
    recLoading = true;
    recError = '';
    try {
      const genre = await getTrackTopGenre(artist, title);
      const lastfmRecs = await getRecommendations({
        seedArtist: artist,
        seedSongTitle: title,
        seedGenre: genre,
        likedArtists,
        limit: 18
      });

      recs = lastfmRecs;

      // Search Subsonic/octo-fiesta (Deezer) directly for each recommendation.
      // This is the only reliable way to know if a track is streamable — the old
      // approach of matching against a pre-fetched pool missed anything not by
      // the seed artist.
      const subMatches = await Promise.all(
        lastfmRecs.map((rec) =>
          searchSongs(`${rec.artist} ${rec.title}`, 5)
            .then((songs) => findBestMatch(songs, rec.artist, rec.title))
            .catch(() => null)
        )
      );

      const hybrid = lastfmRecs.map((track, i) => {
        const subMatch = subMatches[i] ?? null;
        const subBoost = subMatch ? 0.3 : 0;
        const combinedScore = Math.min(1, track.score + subBoost);
        return {
          id: track.id,
          title: track.title,
          artist: track.artist,
          url: track.url,
          matchScore: track.matchScore,
          genreScore: track.genreScore,
          combinedScore,
          playable: Boolean(subMatch),
          subsonicSong: subMatch,
          source: 'hybrid' as const
        };
      });

      hybridRecs = hybrid.sort((a, b) => b.combinedScore - a.combinedScore);
    } catch (err) {
      recError = err instanceof Error ? err.message : 'Failed to load recommendations.';
      recs = [];
      hybridRecs = [];
    } finally {
      recLoading = false;
    }
  }

  async function pick(item: { id: string; title: string; artist: string; imageUrl: string; source: 'lastfm' | 'library' }) {
    selected = item;
    focusTrack.set({
      title: item.title,
      artist: item.artist,
      imageUrl: item.imageUrl,
      source: item.source
    });

    if (item.source === 'library') {
      const id = item.id.replace('sub-', '');
      const startIndex = subsonicSongs.findIndex((song) => song.id === id);
      if (startIndex >= 0) {
        playQueue(subsonicSongs, startIndex);
      }

      try {
        subsonicSimilar = await fetchSimilarSongs(id, 16);
      } catch {
        subsonicSimilar = [];
      }
    }

    await loadRecommendations(item.title, item.artist);
  }

  function playSimilar(index: number) {
    if (!subsonicSimilar.length) return;
    playQueue(subsonicSimilar, index);
  }

  function useRecommendation(rec: HybridRecommendation) {
    if (rec.subsonicSong) {
      focusTrack.set({
        title: rec.subsonicSong.title,
        artist: rec.subsonicSong.artist,
        imageUrl: rec.subsonicSong.coverArtUrl,
        source: 'library',
        album: rec.subsonicSong.album
      });
      playQueue([rec.subsonicSong], 0);
      return;
    }

    if (rec.url) {
      window.open(rec.url, '_blank', 'noopener,noreferrer');
    }
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
    <div class="mb-6">
      <h3 class="mb-3 text-sm font-semibold uppercase tracking-wide text-muted-foreground">Recent searches</h3>
      <div class="divide-y divide-border overflow-hidden rounded-lg border border-input">
        {#each recentSearches as term (term)}
          <div class="flex items-center">
            <button
              class="flex flex-1 items-center gap-3 bg-secondary px-3 py-2.5 text-left transition hover:bg-accent"
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

<div class="mb-6 divide-y divide-border overflow-hidden rounded-lg border border-input">
  {#each merged as item (item.id)}
    {#if item.subsonicSong}
      <SongContextMenu song={item.subsonicSong} onplay={() => pick(item)}>
        <button
          class={selected?.id === item.id
            ? 'flex w-full items-center gap-3 bg-accent px-3 py-2.5 text-left ring-1 ring-inset ring-primary'
            : 'flex w-full items-center gap-3 bg-secondary px-3 py-2.5 text-left transition hover:bg-accent'}
          onclick={() => pick(item)}
        >
          {#if item.imageUrl}
            <img class="h-9 w-9 shrink-0 rounded object-cover" src={item.imageUrl} alt={item.title} loading="lazy" />
          {:else}
            <div class="flex h-9 w-9 shrink-0 items-center justify-center rounded bg-gradient-to-br from-slate-500 to-slate-700 text-xs font-bold">
              {initials(item.title)}
            </div>
          {/if}
          <div class="min-w-0 flex-1">
            <p class="truncate text-sm font-semibold">{item.title}</p>
            <p class="truncate text-xs text-muted-foreground">{item.artist}</p>
          </div>
        </button>
      </SongContextMenu>
    {:else}
      <button
        class={selected?.id === item.id
          ? 'flex w-full items-center gap-3 bg-accent px-3 py-2.5 text-left ring-1 ring-inset ring-primary'
          : 'flex w-full items-center gap-3 bg-secondary px-3 py-2.5 text-left transition hover:bg-accent'}
        onclick={() => pick(item)}
      >
        {#if item.imageUrl}
          <img class="h-9 w-9 shrink-0 rounded object-cover" src={item.imageUrl} alt={item.title} loading="lazy" />
        {:else}
          <div class="flex h-9 w-9 shrink-0 items-center justify-center rounded bg-gradient-to-br from-slate-500 to-slate-700 text-xs font-bold">
            {initials(item.title)}
          </div>
        {/if}
        <div class="min-w-0 flex-1">
          <p class="truncate text-sm font-semibold">{item.title}</p>
          <p class="truncate text-xs text-muted-foreground">{item.artist}</p>
        </div>
      </button>
    {/if}
  {/each}
</div>

{#if subsonicSimilar.length > 0}
  <div class="mb-2 flex items-center justify-between">
    <h3 class="text-sm font-semibold uppercase tracking-wide text-muted-foreground">Subsonic similar</h3>
  </div>
  <div class="mb-6 divide-y divide-border overflow-hidden rounded-lg border border-input">
    {#each subsonicSimilar as song, index (song.id)}
      <SongContextMenu {song} onplay={() => playSimilar(index)}>
        <button class="flex w-full items-center gap-3 bg-secondary px-3 py-2.5 text-left transition hover:bg-accent" onclick={() => playSimilar(index)}>
          {#if song.coverArtUrl}
            <img class="h-9 w-9 shrink-0 rounded object-cover" src={song.coverArtUrl} alt={song.title} loading="lazy" />
          {:else}
            <div class="flex h-9 w-9 shrink-0 items-center justify-center rounded bg-gradient-to-br from-slate-500 to-slate-700 text-xs font-bold">
              {initials(song.title)}
            </div>
          {/if}
          <div class="min-w-0 flex-1">
            <p class="truncate text-sm font-semibold">{song.title}</p>
            <p class="truncate text-xs text-muted-foreground">{song.artist}</p>
          </div>
        </button>
      </SongContextMenu>
    {/each}
  </div>
{/if}

<div class="mb-2 flex items-center justify-between">
  <h3 class="text-sm font-semibold uppercase tracking-wide text-muted-foreground">Smart recommendations</h3>
  {#if recLoading}
    <p class="text-sm text-muted-foreground">Loading...</p>
  {/if}
</div>
{#if recError}
  <p class="mb-3 text-sm text-destructive">{recError}</p>
{/if}

<div class="divide-y divide-border overflow-hidden rounded-lg border border-input">
  {#each hybridRecs as track (track.id)}
    {#if track.playable && track.subsonicSong}
      <SongContextMenu song={track.subsonicSong} onplay={() => useRecommendation(track)}>
        <div class="flex items-center gap-3 bg-secondary px-3 py-2.5">
          <div class="min-w-0 flex-1">
            <p class="truncate text-sm font-semibold">{track.title}</p>
            <p class="truncate text-xs text-muted-foreground">{track.artist}</p>
          </div>
          <div class="flex shrink-0 items-center gap-2">
            <Badge class="bg-emerald-500/20 text-emerald-200">Playable</Badge>
            <Button size="sm" onclick={() => useRecommendation(track)}>Play</Button>
          </div>
        </div>
      </SongContextMenu>
    {:else}
      <div class="flex items-center gap-3 bg-secondary px-3 py-2.5">
        <div class="min-w-0 flex-1">
          <p class="truncate text-sm font-semibold">{track.title}</p>
          <p class="truncate text-xs text-muted-foreground">{track.artist}</p>
        </div>
        <div class="flex shrink-0 items-center gap-2">
          <Badge variant="outline">Last.fm only</Badge>
          <Button size="sm" variant="secondary" onclick={() => useRecommendation(track)}>Open</Button>
        </div>
      </div>
    {/if}
  {/each}
</div>
{/if}
