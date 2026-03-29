<script lang="ts">
  import { page } from '$app/state';
  import { Play, Pause, Save, RefreshCw } from '@lucide/svelte';
  import { toast } from 'svelte-sonner';

  import type { Song } from '$lib/servers';
  import { fetchLikedArtists, searchSongs, createPlaylist } from '$lib/servers';
  import { getListeningProfile } from '$lib/servers/play-history';
  import { getUpNextSongs } from '$lib/discovery';
  import { startQueue, playingFrom, isPlaying, togglePlayRequest } from '$lib/stores/player';
  import { requestLibraryRefresh } from '$lib/stores/ui-state';
  import { formatClockDuration, normalizeString, sumDuration } from '$lib/utils';
  import { SongRow } from '$lib/components/media';

  const artist = $derived(decodeURIComponent(page.params.name));
  const HREF = $derived(`/for-you/artist/${page.params.name}`);

  let songs = $state<Song[]>([]);
  let loading = $state(true);
  let saving = $state(false);
  let error = $state('');
  let loadVersion = 0;

  const active = $derived($playingFrom.href === HREF);
  const totalDuration = $derived(formatClockDuration(sumDuration(songs)));

  async function load() {
    const ver = ++loadVersion;
    loading = true;
    error = '';
    songs = [];
    try {
      const [profile, likedArtistEntries] = await Promise.all([
        getListeningProfile(),
        fetchLikedArtists()
      ]);
      if (ver !== loadVersion) return;

      const likedArtists = likedArtistEntries.map((e) => e.name);
      const normArtist = normalizeString(artist);
      const artistSeed = profile.topSongs.find((s) => normalizeString(s.artist) === normArtist);

      let result: Song[];
      if (artistSeed) {
        result = await getUpNextSongs({
          artist: artistSeed.artist,
          title: artistSeed.title,
          likedArtists,
          limit: 20
        });
      } else {
        const hits = await searchSongs(artist, 40);
        result = hits.filter((s) => normalizeString(s.artist) === normArtist).slice(0, 20);
      }

      if (ver !== loadVersion) return;
      songs = result;
    } catch (err) {
      if (ver !== loadVersion) return;
      error = err instanceof Error ? err.message : 'Failed to load.';
    } finally {
      if (ver === loadVersion) loading = false;
    }
  }

  function playAll() {
    startQueue(songs, 0, { type: 'playlist', name: `${artist} Mix`, href: HREF });
  }

  function playSong(index: number) {
    startQueue(songs, index, { type: 'playlist', name: `${artist} Mix`, href: HREF });
  }

  async function save() {
    if (!songs.length || saving) return;
    saving = true;
    try {
      await createPlaylist(`${artist} Mix`, songs.map((s) => s.id));
      requestLibraryRefresh();
      toast.success(`Saved "${artist} Mix" to your library`);
    } catch {
      toast.error('Failed to save playlist');
    } finally {
      saving = false;
    }
  }

  $effect(() => {
    void artist;
    load();
  });
</script>

<div class="page-hero app-glass mb-6 flex gap-4 rounded-[2rem] p-5">
  <div class="relative h-36 w-36 shrink-0 overflow-hidden rounded-2xl shadow-lg">
    {#if songs.length >= 4}
      <div class="grid h-full w-full grid-cols-2 grid-rows-2">
        {#each songs.slice(0, 4) as song (song.id)}
          {#if song.coverArtUrl}
            <img class="h-full w-full object-cover" src={song.coverArtUrl} alt={song.title} />
          {:else}
            <div class="h-full w-full bg-gradient-to-br from-slate-600 to-slate-800"></div>
          {/if}
        {/each}
      </div>
      <div class="absolute inset-0 bg-gradient-to-br from-black/10 via-transparent to-black/45"></div>
    {:else if loading}
      <div class="h-full w-full animate-pulse bg-muted"></div>
    {:else}
      <div class="flex h-full w-full items-center justify-center bg-gradient-to-br from-rose-600 to-pink-900">
        <span class="text-3xl font-bold text-white/80">{artist.slice(0, 2).toUpperCase()}</span>
      </div>
    {/if}
  </div>

  <div class="flex flex-col justify-end gap-2">
    <p class="text-xs font-semibold uppercase tracking-widest text-muted-foreground">Playlist</p>
    <h2 class="app-section-title text-3xl font-bold tracking-tight">{artist} Mix</h2>
    <p class="text-sm text-muted-foreground">Songs from and inspired by {artist}</p>
    {#if !loading && songs.length > 0}
      <p class="text-sm text-muted-foreground">{songs.length} songs · {totalDuration}</p>
    {/if}
    <div class="mt-1 flex items-center gap-3">
      <button
        onclick={() => active ? togglePlayRequest.update((n) => n + 1) : playAll()}
        disabled={loading || !songs.length}
        class="flex size-14 shrink-0 items-center justify-center rounded-full bg-primary text-primary-foreground shadow-lg transition-all duration-150 hover:scale-105 hover:brightness-110 active:scale-95 disabled:cursor-not-allowed disabled:opacity-40"
      >
        {#if active && $isPlaying}
          <Pause class="size-6 text-muted-foreground" fill="currentColor" />
        {:else}
          <Play class="size-6 translate-x-0.5 text-muted-foreground" fill="currentColor" />
        {/if}
      </button>

      <button
        onclick={save}
        disabled={saving || loading || !songs.length}
        class="app-round-button flex items-center gap-2 rounded-full px-4 py-2 text-sm font-medium transition disabled:cursor-not-allowed disabled:opacity-40"
      >
        {#if saving}
          <span class="block size-3.5 animate-spin rounded-full border-2 border-current border-t-transparent"></span>
        {:else}
          <Save class="size-3.5" />
        {/if}
        Save to library
      </button>

      <button
        onclick={load}
        disabled={loading}
        class="app-round-button grid size-10 place-items-center rounded-full transition disabled:opacity-40"
        aria-label="Refresh"
      >
        <RefreshCw class="size-4 {loading ? 'animate-spin' : ''}" />
      </button>
    </div>
  </div>
</div>

{#if error}
  <p class="mb-3 text-sm text-destructive">{error}</p>
{/if}

{#if loading}
  <p class="mb-3 text-sm text-muted-foreground">Finding songs…</p>
{/if}

{#if songs.length}
  <div class="page-section mt-2">
    <div
      class="grid items-center gap-3 border-b border-border/40 px-3 pb-2 text-xs font-medium uppercase tracking-wider text-muted-foreground/50"
      style="grid-template-columns: 2rem 2.5rem 1fr 1fr 4rem"
    >
      <span class="text-center">#</span>
      <span></span>
      <span>Title</span>
      <span class="hidden md:block">Album</span>
      <span class="text-right">Duration</span>
    </div>
    <div class="mt-1 space-y-0.5">
      {#each songs as song, index (song.id)}
        <SongRow {song} {index} showAlbum onplay={() => playSong(index)} staggerIndex={index} />
      {/each}
    </div>
  </div>
{:else if !loading && !error}
  <p class="text-sm text-muted-foreground">No songs found for {artist}.</p>
{/if}
