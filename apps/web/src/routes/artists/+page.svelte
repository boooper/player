<script lang="ts">
  import { onMount } from 'svelte';
  import { Mic2, Search, X } from '@lucide/svelte';

  import { fetchAlbumList } from '$lib/api';
  import { fetchAudioDbArtistPhoto } from '$lib/audiodb';
  import ArtistContextMenu from '$lib/components/ArtistContextMenu.svelte';

  type ArtistEntry = { name: string; imageUrl: string };

  let allArtists = $state<ArtistEntry[]>([]);
  let query = $state('');
  let loading = $state(false);
  let error = $state('');

  // Client-side filter.
  const artists = $derived(
    query.trim()
      ? allArtists.filter((a) => a.name.toLowerCase().includes(query.toLowerCase()))
      : allArtists
  );

  function clearSearch() { query = ''; }

  function initials(name: string): string {
    return name.split(' ').filter(Boolean).slice(0, 2).map((p) => p[0]?.toUpperCase() ?? '').join('');
  }

  onMount(async () => {
    loading = true;
    try {
      // Fetch a large album list and extract unique artist names.
      const albums = await fetchAlbumList('newest', 500);
      const seen = new Set<string>();
      const entries: ArtistEntry[] = [];
      for (const album of albums) {
        if (album.artist && !seen.has(album.artist)) {
          seen.add(album.artist);
          entries.push({ name: album.artist, imageUrl: '' });
        }
      }
      // Sort alphabetically.
      entries.sort((a, b) => a.name.localeCompare(b.name));
      allArtists = entries;

      // Hydrate photos progressively in the background.
      entries.forEach((entry, i) => {
        fetchAudioDbArtistPhoto(entry.name).then((url) => {
          if (url) allArtists[i] = { ...allArtists[i], imageUrl: url };
        });
      });
    } catch (err) {
      error = err instanceof Error ? err.message : 'Failed to load artists.';
    } finally {
      loading = false;
    }
  });
</script>

<div class="space-y-6">
  <!-- Header -->
  <div>
    <h1 class="text-3xl font-bold">Artists</h1>
    <p class="mt-1 text-sm text-muted-foreground">
      {#if loading}
        Loading…
      {:else if query.trim()}
        {artists.length} of {allArtists.length} artist{allArtists.length !== 1 ? 's' : ''}
      {:else}
        {allArtists.length} artist{allArtists.length !== 1 ? 's' : ''}
      {/if}
    </p>
  </div>

  <!-- Search bar -->
  <div class="relative">
    <Search class="absolute left-3 top-1/2 size-4 -translate-y-1/2 text-muted-foreground pointer-events-none" />
    <input
      type="search"
      placeholder="Filter artists…"
      bind:value={query}
      class="w-full rounded-lg border border-border/50 bg-secondary/60 py-2.5 pl-9 pr-10 text-sm placeholder:text-muted-foreground focus:border-border focus:outline-none focus:ring-1 focus:ring-ring"
    />
    {#if query}
      <button class="absolute right-3 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground transition-colors" onclick={clearSearch}>
        <X class="size-4" />
      </button>
    {/if}
  </div>

  <!-- Error -->
  {#if error}
    <div class="rounded-lg border border-rose-500/30 bg-rose-500/10 px-4 py-3 text-sm text-rose-300">{error}</div>
  {/if}

  <!-- Grid -->
  {#if artists.length > 0}
    <div class="grid grid-cols-[repeat(auto-fill,minmax(9rem,1fr))] gap-4">
      {#each artists as artist (artist.name)}
        <ArtistContextMenu name={artist.name}>
        <a
          href={`/artist/${encodeURIComponent(artist.name)}`}
          class="group flex flex-col items-center gap-3 rounded-xl p-4 transition-colors hover:bg-white/5"
        >
          <div class="size-24 shrink-0 overflow-hidden rounded-full shadow-md ring-1 ring-white/10">
            {#if artist.imageUrl}
              <img src={artist.imageUrl} alt={artist.name} class="h-full w-full object-cover transition-transform duration-300 group-hover:scale-105" loading="lazy" />
            {:else}
              <div class="flex h-full w-full items-center justify-center bg-gradient-to-br from-slate-600 to-slate-800">
                {#if loading}
                  <span class="block size-6 animate-pulse rounded-full bg-white/20"></span>
                {:else}
                  <span class="text-base font-bold text-white/70">{initials(artist.name)}</span>
                {/if}
              </div>
            {/if}
          </div>
          <div class="w-full text-center">
            <p class="truncate text-sm font-semibold leading-tight">{artist.name}</p>
            <p class="mt-0.5 text-xs text-muted-foreground">Artist</p>
          </div>
        </a>
        </ArtistContextMenu>
      {/each}
    </div>
  {:else if !loading && !error}
    <div class="flex flex-col items-center justify-center gap-3 py-20 text-center">
      <Mic2 class="size-12 text-muted-foreground/30" />
      <p class="text-sm text-muted-foreground">
        {query.trim() ? `No artists matched "${query}".` : 'No artists found in your library.'}
      </p>
    </div>
  {/if}
</div>
