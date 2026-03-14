<script lang="ts">
  import { onMount } from 'svelte';
  import { Play, Disc3, RefreshCw } from '@lucide/svelte';

  import { fetchAlbumList, fetchAlbumSongs, type Album } from '$lib/api';
  import { focusTrack, playQueue, playingFrom, addRecentlyPlayed } from '$lib/stores/player';
  import AlbumContextMenu from '$lib/components/AlbumContextMenu.svelte';

  type TabType = 'newest' | 'random' | 'recent' | 'frequent';
  const TABS: { id: TabType; label: string }[] = [
    { id: 'newest', label: 'Newly Added' },
    { id: 'random', label: 'Random' },
    { id: 'recent', label: 'Recently Played' },
    { id: 'frequent', label: 'Most Played' },
  ];

  let activeTab = $state<TabType>('newest');
  let albums = $state<Album[]>([]);
  let loading = $state(false);
  let error = $state('');
  let albumLoadingId = $state<string | null>(null);

  async function loadAlbums(type: TabType) {
    loading = true;
    error = '';
    albums = [];
    try {
      albums = await fetchAlbumList(type, 50);
    } catch (err) {
      error = err instanceof Error ? err.message : 'Failed to load albums.';
    } finally {
      loading = false;
    }
  }

  async function playAlbum(album: Album) {
    albumLoadingId = album.id;
    try {
      const songs = await fetchAlbumSongs(album.id);
      if (!songs.length) return;
      focusTrack.set({ title: songs[0].title, artist: songs[0].artist, imageUrl: songs[0].coverArtUrl, source: 'library', album: songs[0].album });
      playQueue(songs, 0);
      playingFrom.set({ type: 'album', name: album.name, href: `/album/${encodeURIComponent(album.id)}` });
      addRecentlyPlayed({ id: album.id, name: album.name, coverArtUrl: album.coverArtUrl, href: `/album/${encodeURIComponent(album.id)}`, type: 'album' });
    } catch {
    } finally {
      albumLoadingId = null;
    }
  }

  function switchTab(tab: TabType) {
    if (tab === activeTab && !loading) {
      loadAlbums(tab);
      return;
    }
    activeTab = tab;
    loadAlbums(tab);
  }

  onMount(() => {
    loadAlbums('newest');
  });
</script>

<div class="space-y-6">
  <!-- Header -->
  <div class="flex items-end justify-between">
    <div>
      <h1 class="text-3xl font-bold">Albums</h1>
      <p class="mt-1 text-sm text-muted-foreground">
        {loading ? 'Loading…' : albums.length ? `${albums.length} albums` : ''}
      </p>
    </div>
    <button
      class="flex items-center gap-1.5 rounded-md px-3 py-1.5 text-sm text-muted-foreground hover:bg-white/10 hover:text-foreground transition-colors disabled:opacity-50"
      disabled={loading}
      onclick={() => loadAlbums(activeTab)}
    >
      <RefreshCw class="size-3.5 {loading ? 'animate-spin' : ''}" />
      Refresh
    </button>
  </div>

  <!-- Tabs -->
  <div class="flex gap-1 rounded-lg bg-secondary/60 p-1 w-fit">
    {#each TABS as tab (tab.id)}
      <button
        class="rounded-md px-4 py-1.5 text-sm font-medium transition-colors {activeTab === tab.id ? 'bg-background text-foreground shadow-sm' : 'text-muted-foreground hover:text-foreground'}"
        onclick={() => switchTab(tab.id)}
      >
        {tab.label}
      </button>
    {/each}
  </div>

  <!-- Error -->
  {#if error}
    <div class="rounded-lg border border-rose-500/30 bg-rose-500/10 px-4 py-3 text-sm text-rose-300">{error}</div>
  {/if}

  <!-- Grid -->
  <div class="grid grid-cols-[repeat(auto-fill,minmax(9rem,1fr))] gap-4">
    {#if loading}
      {#each Array(24) as _, i (i)}
        <div class="flex flex-col gap-2 p-3">
          <div class="aspect-square w-full animate-pulse rounded-md bg-muted"></div>
          <div class="h-3 w-3/4 animate-pulse rounded bg-muted"></div>
          <div class="h-3 w-1/2 animate-pulse rounded bg-muted"></div>
        </div>
      {/each}
    {:else}
      {#each albums as album (album.id)}
        <AlbumContextMenu {album} onplay={() => playAlbum(album)}>
        <div class="group relative flex flex-col gap-2 rounded-lg bg-secondary/60 p-3 transition hover:bg-accent">
          <a href={`/album/${encodeURIComponent(album.id)}`} class="absolute inset-0 rounded-lg" aria-label={album.name}></a>
          <div class="relative">
            {#if album.coverArtUrl}
              <img
                class="aspect-square w-full rounded-md object-cover shadow-md"
                src={album.coverArtUrl}
                alt={album.name}
                loading="lazy"
              />
            {:else}
              <div class="grid aspect-square w-full place-items-center rounded-md bg-gradient-to-br from-slate-600 to-slate-800 shadow-md">
                <Disc3 class="size-10 text-white/40" />
              </div>
            {/if}
            <button
              class="absolute bottom-2 right-2 z-10 grid size-9 translate-y-1 place-items-center rounded-full bg-primary text-primary-foreground opacity-0 shadow-lg transition group-hover:translate-y-0 group-hover:opacity-100"
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
            <p class="truncate text-sm font-semibold leading-tight">{album.name}</p>
            <p class="truncate text-xs text-muted-foreground">{album.artist}{album.year ? ` · ${album.year}` : ''}</p>
          </div>
        </div>
        </AlbumContextMenu>
      {/each}
    {/if}
  </div>
</div>
