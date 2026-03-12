<script lang="ts">
  import { Play, Shuffle, ChevronLeft, ChevronRight, Sparkles, Mic2 } from '@lucide/svelte';

  import {
    fetchSubsonicArtistAlbums,
    fetchSubsonicAlbumSongs,
    searchSubsonicSongs,
    type SubsonicAlbum,
    type SubsonicSong
  } from '$lib/api';
  import {
    getArtistInfo,
    getArtistTopTracks,
    type ArtistInfo,
    type Song
  } from '$lib/metadata';
  import { focusTrack, playQueue, playingFrom, shuffleEnabled, addRecentlyPlayed, smartShuffleMode } from '$lib/stores/player';
  import { appSettings } from '$lib/stores/settings';
  import { toast } from 'svelte-sonner';
  import SongContextMenu from '$lib/components/SongContextMenu.svelte';
  import {
    DropdownMenu,
    DropdownMenuTrigger,
    DropdownMenuContent,
    DropdownMenuItem,
    DropdownMenuSeparator,
  } from '$lib/components/ui/dropdown-menu';

  let { data } = $props<{ data: { name: string } }>();

  let loading = $state(false);
  let error = $state('');

  let artistInfo = $state<ArtistInfo | null>(null);
  let topTracks = $state<Song[]>([]);
  let subsonicSongs = $state<SubsonicSong[]>([]);
  let albums = $state<SubsonicAlbum[]>([]);
  let showAllTracks = $state(false);

  let albumSongs = $state<Record<string, SubsonicSong[]>>({});
  let albumLoading = $state<Record<string, boolean>>({});

  let carouselEl = $state<HTMLDivElement | null>(null);

  async function loadArtist(name: string) {
    loading = true;
    error = '';
    artistInfo = null;
    topTracks = [];
    subsonicSongs = [];
    albums = [];
    albumSongs = {};
    albumLoading = {};
    showAllTracks = false;
    try {
      const [info, top, sub, albs] = await Promise.all([
        getArtistInfo(name),
        getArtistTopTracks(name, 10),
        searchSubsonicSongs(name, 30),
        fetchSubsonicArtistAlbums(name, 24)
      ]);
      artistInfo = info;
      topTracks = top;
      subsonicSongs = sub;
      albums = albs;
    } catch (err) {
      error = err instanceof Error ? err.message : 'Failed to load artist.';
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    loadArtist(data.name);
  });

  function findSubsonicMatch(title: string): SubsonicSong | null {
    const needle = title.toLowerCase();
    return subsonicSongs.find((s) => s.title.toLowerCase() === needle) ?? null;
  }

  const playableTopTracks = $derived(
    topTracks
      .map((t) => ({ lfm: t, sub: findSubsonicMatch(t.title) }))
      .filter((r) => r.sub !== null) as Array<{ lfm: Song; sub: SubsonicSong }>
  );

  const visibleTopTracks = $derived(
    showAllTracks ? playableTopTracks : playableTopTracks.slice(0, 5)
  );

  function playTopTrack(index: number) {
    const list = playableTopTracks.map((r) => r.sub!);
    const song = list[index];
    focusTrack.set({ title: song.title, artist: song.artist, imageUrl: song.coverArtUrl, source: 'subsonic', album: song.album });
    playQueue(list, index);
    playingFrom.set({ type: 'artist', name: data.name, href: `/artist/${encodeURIComponent(data.name)}` });
  }

  function playAllTopTracks() {
    const list = playableTopTracks.map((r) => r.sub!);
    if (!list.length) return;
    const ordered = ($shuffleEnabled || $smartShuffleMode) ? [...list].sort(() => Math.random() - 0.5) : list;
    focusTrack.set({ title: ordered[0].title, artist: ordered[0].artist, imageUrl: ordered[0].coverArtUrl, source: 'subsonic', album: ordered[0].album });
    playQueue(ordered, 0);
    playingFrom.set({ type: 'artist', name: data.name, href: `/artist/${encodeURIComponent(data.name)}` });
  }

  async function loadAndPlayAlbum(albumId: string, startIndex = 0) {
    if (!albumSongs[albumId]) {
      albumLoading = { ...albumLoading, [albumId]: true };
      try {
        albumSongs = { ...albumSongs, [albumId]: await fetchSubsonicAlbumSongs(albumId) };
      } catch {
        albumSongs = { ...albumSongs, [albumId]: [] };
      } finally {
        albumLoading = { ...albumLoading, [albumId]: false };
      }
    }
    const songs = albumSongs[albumId];
    if (!songs?.length) return;
    const song = songs[startIndex];
    focusTrack.set({ title: song.title, artist: song.artist, imageUrl: song.coverArtUrl, source: 'subsonic', album: song.album });
    playQueue(songs, startIndex);
    const album = albums.find((a) => a.id === albumId);
    if (album) {
      addRecentlyPlayed({ id: album.id, name: album.name, coverArtUrl: album.coverArtUrl, href: `/album/${encodeURIComponent(album.id)}`, type: 'album' });
      playingFrom.set({ type: 'album', name: album.name, href: `/album/${encodeURIComponent(album.id)}` });
    } else {
      playingFrom.set({ type: 'artist', name: data.name, href: `/artist/${encodeURIComponent(data.name)}` });
    }
  }

  // Shuffle dropdown
  let smartShuffleFetching = $state(false);
  let shuffleAllArtist = $state(false);
  const lastFmApiKey = $derived($appSettings.lastFmApiKey);

  function activateShuffle() {
    smartShuffleMode.set(false);
    shuffleEnabled.set(true);
    shuffleAllArtist = false;
  }

  function activateSmartShuffle() {
    shuffleEnabled.set(true);
    smartShuffleMode.set(true);
    shuffleAllArtist = false;
  }

  function activateShuffleAll() {
    smartShuffleMode.set(false);
    shuffleEnabled.set(true);
    shuffleAllArtist = true;
  }

  function deactivateShuffle() {
    shuffleEnabled.set(false);
    smartShuffleMode.set(false);
    shuffleAllArtist = false;
  }

  async function playOrShuffleAll() {
    if (shuffleAllArtist) {
      const toastId = toast.loading(`Loading all ${data.name} songs…`);
      try {
        const allSongs = (await Promise.all(albums.map(a => fetchSubsonicAlbumSongs(a.id)))).flat();
        if (!allSongs.length) throw new Error('No songs found');
        for (let i = allSongs.length - 1; i > 0; i--) {
          const j = Math.floor(Math.random() * (i + 1));
          [allSongs[i], allSongs[j]] = [allSongs[j], allSongs[i]];
        }
        playQueue(allSongs, 0);
        playingFrom.set({ type: 'artist', name: data.name, href: `/artist/${encodeURIComponent(data.name)}` });
        toast.success(`Shuffling all ${data.name} songs`, { id: toastId });
      } catch {
        toast.error('Failed to load artist songs', { id: toastId });
      }
    } else {
      playAllTopTracks();
    }
  }

  function scrollCarousel(dir: -1 | 1) {
    if (!carouselEl) return;
    carouselEl.scrollBy({ left: dir * 220, behavior: 'smooth' });
  }

  function fmtListeners(n: number): string {
    if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M listeners`;
    if (n >= 1_000) return `${(n / 1_000).toFixed(0)}K listeners`;
    return `${n} listeners`;
  }

  function fmt(seconds: number): string {
    if (!isFinite(seconds) || !seconds) return '';
    return `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, '0')}`;
  }

  function initials(name: string): string {
    return name.split(' ').filter(Boolean).slice(0, 2).map((p) => p[0]?.toUpperCase() ?? '').join('');
  }
</script>

<!-- Hero -->
<div class="relative -mx-4 -mt-4 mb-6 overflow-hidden">
  {#if artistInfo?.imageUrl}
    <div class="absolute inset-0">
      <img class="h-full w-full object-cover object-top" src={artistInfo.imageUrl} alt="" aria-hidden="true" />
      <div class="absolute inset-0 bg-gradient-to-b from-black/40 via-black/60 to-background"></div>
    </div>
  {:else}
    <div class="absolute inset-0 bg-gradient-to-b from-slate-700 via-slate-800/80 to-background"></div>
  {/if}

  <div class="relative px-6 pb-6 pt-10">
    {#if loading}
      <div class="h-16 w-56 animate-pulse rounded bg-white/10"></div>
    {:else}
      <p class="mb-1 text-xs font-semibold uppercase tracking-widest text-white/70">Artist</p>
      <h1 class="text-5xl font-black tracking-tight text-white drop-shadow-lg sm:text-6xl">{data.name}</h1>
      {#if artistInfo?.listeners}
        <p class="mt-2 text-sm text-white/60">{fmtListeners(artistInfo.listeners)}</p>
      {/if}
      {#if artistInfo?.tags?.length}
        <div class="mt-2 flex flex-wrap gap-1.5">
          {#each artistInfo.tags as tag (tag)}
            <span class="rounded-full bg-white/10 px-2.5 py-0.5 text-xs text-white/80">{tag}</span>
          {/each}
        </div>
      {/if}
    {/if}
    <div class="mt-5 flex items-center gap-3">
      <button
        class="grid size-14 shrink-0 place-items-center rounded-full bg-primary text-background shadow-lg transition hover:scale-105 disabled:opacity-40"
        onclick={playOrShuffleAll}
        disabled={playableTopTracks.length === 0 && !shuffleAllArtist}
        aria-label="Play"
      >
        <Play class="size-6 translate-x-0.5" fill="currentColor" />
      </button>
      <!-- Shuffle dropdown -->
      <DropdownMenu>
        <DropdownMenuTrigger>
          {#snippet child({ props })}
            <button
              {...props}
              class="flex size-10 items-center justify-center rounded-md transition-colors {$smartShuffleMode || $shuffleEnabled ? 'text-primary' : 'text-white/70 hover:text-white'}"
              aria-label="Shuffle options"
              title={$smartShuffleMode ? 'Smart Shuffle on' : $shuffleEnabled ? 'Shuffle on' : 'Shuffle off'}
              disabled={playableTopTracks.length === 0 && albums.length === 0}
            >
              {#if $smartShuffleMode}
                <Sparkles class="size-5 {smartShuffleFetching ? 'animate-pulse' : ''}" />
              {:else}
                <Shuffle class="size-5" />
              {/if}
            </button>
          {/snippet}
        </DropdownMenuTrigger>
        <DropdownMenuContent align="start" class="min-w-56">
          <DropdownMenuItem onclick={activateShuffle} class="gap-3 {$shuffleEnabled && !$smartShuffleMode ? 'text-primary' : ''}">
            <Shuffle class="size-4 shrink-0" />
            <div>
              <p class="font-medium">Shuffle</p>
              <p class="text-xs text-muted-foreground">Play popular tracks in random order</p>
            </div>
            {#if $shuffleEnabled && !$smartShuffleMode}
              <span class="ml-auto size-1.5 rounded-full bg-primary"></span>
            {/if}
          </DropdownMenuItem>
          <DropdownMenuItem onclick={activateSmartShuffle} class="gap-3 {$smartShuffleMode ? 'text-primary' : ''}">
            <Sparkles class="size-4 shrink-0" />
            <div>
              <p class="font-medium">Smart Shuffle</p>
              <p class="text-xs text-muted-foreground">Weaves in Last.fm recommendations</p>
            </div>
            {#if $smartShuffleMode}
              <span class="ml-auto size-1.5 rounded-full bg-primary"></span>
            {/if}
          </DropdownMenuItem>
          <DropdownMenuSeparator />
          <DropdownMenuItem onclick={activateShuffleAll} disabled={albums.length === 0} class="gap-3 {shuffleAllArtist ? 'text-primary' : ''}">
            <Mic2 class="size-4 shrink-0" />
            <div>
              <p class="font-medium">Shuffle All Songs</p>
              <p class="truncate max-w-40 text-xs text-muted-foreground">{data.name} · entire discography</p>
            </div>
            {#if shuffleAllArtist}<span class="ml-auto size-1.5 rounded-full bg-primary"></span>{/if}
          </DropdownMenuItem>
          <DropdownMenuSeparator />
          <DropdownMenuItem onclick={deactivateShuffle} class="gap-3 {!$shuffleEnabled && !$smartShuffleMode ? 'text-primary' : 'text-muted-foreground'}">
            <span class="size-4 shrink-0 flex items-center justify-center text-xs font-bold">—</span>
            Off
            {#if !$shuffleEnabled && !$smartShuffleMode}<span class="ml-auto size-1.5 rounded-full bg-primary"></span>{/if}
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
    </div>
  </div>
</div>

{#if error}
  <p class="mb-4 text-sm text-destructive">{error}</p>
{/if}

<!-- Popular -->
{#if loading}
  <div class="mb-6 space-y-2">
    {#each Array(5) as _, i (i)}
      <div class="flex h-12 items-center gap-3 rounded-md px-3">
        <div class="h-4 w-4 animate-pulse rounded bg-muted"></div>
        <div class="size-10 animate-pulse rounded bg-muted"></div>
        <div class="h-4 w-48 animate-pulse rounded bg-muted"></div>
      </div>
    {/each}
  </div>
{:else if playableTopTracks.length > 0}
  <section class="mb-8">
    <h2 class="mb-3 text-xl font-bold">Popular</h2>
    <div class="space-y-1">
      {#each visibleTopTracks as { lfm, sub }, i (lfm.id)}
        <SongContextMenu song={sub} onplay={() => playTopTrack(i)}>
          <button
            class="group flex w-full items-center gap-3 rounded-md px-3 py-2 text-left transition hover:bg-accent"
            onclick={() => playTopTrack(i)}
          >
            <span class="w-5 shrink-0 text-center text-sm tabular-nums text-muted-foreground group-hover:hidden">{i + 1}</span>
            <span class="hidden w-5 shrink-0 place-items-center group-hover:grid">
              <Play class="size-3.5" fill="currentColor" />
            </span>
            {#if sub.coverArtUrl}
              <img class="size-10 shrink-0 rounded object-cover" src={sub.coverArtUrl} alt={sub.title} loading="lazy" />
            {:else}
              <div class="grid size-10 shrink-0 place-items-center rounded bg-secondary text-xs font-bold">{initials(sub.title)}</div>
            {/if}
            <div class="min-w-0 flex-1">
              <p class="truncate text-sm font-medium">{sub.title}</p>
              {#if lfm.listeners}
                <p class="text-xs text-muted-foreground">{lfm.listeners.toLocaleString()} plays</p>
              {/if}
            </div>
            <span class="text-xs tabular-nums text-muted-foreground">{fmt(sub.duration)}</span>
          </button>
        </SongContextMenu>
      {/each}
    </div>
    {#if playableTopTracks.length > 5}
      <button
        class="mt-2 px-3 text-sm font-semibold text-muted-foreground hover:text-foreground"
        onclick={() => (showAllTracks = !showAllTracks)}
      >{showAllTracks ? 'Show less' : `See ${playableTopTracks.length - 5} more`}</button>
    {/if}
  </section>
{/if}

<!-- Albums carousel -->
{#if albums.length > 0}
  <section class="mb-8">
    <div class="mb-3 flex items-center justify-between">
      <h2 class="text-xl font-bold">Discography</h2>
      <div class="flex gap-1">
        <button class="grid size-8 place-items-center rounded-full bg-secondary hover:bg-accent" onclick={() => scrollCarousel(-1)} aria-label="Scroll left">
          <ChevronLeft class="size-4" />
        </button>
        <button class="grid size-8 place-items-center rounded-full bg-secondary hover:bg-accent" onclick={() => scrollCarousel(1)} aria-label="Scroll right">
          <ChevronRight class="size-4" />
        </button>
      </div>
    </div>
    <div bind:this={carouselEl} class="flex gap-4 overflow-x-auto pb-3" style="scrollbar-width:none;-ms-overflow-style:none">
      {#each albums as album (album.id)}
        <div class="group relative flex w-44 shrink-0 flex-col gap-2 rounded-lg bg-secondary/60 p-3 text-left transition hover:bg-accent">
          <a
            href="/album/{encodeURIComponent(album.id)}"
            class="absolute inset-0 z-10 rounded-lg"
            aria-label="Open {album.name}"
          ></a>
          <div class="relative w-full">
            {#if album.coverArtUrl}
              <img class="aspect-square w-full rounded-md object-cover shadow-md" src={album.coverArtUrl} alt={album.name} loading="lazy" />
            {:else}
              <div class="grid aspect-square w-full place-items-center rounded-md bg-gradient-to-br from-slate-600 to-slate-800 text-xl font-bold">{initials(album.name)}</div>
            {/if}
            <div class="absolute bottom-2 right-2 z-20 grid size-10 translate-y-1 place-items-center rounded-full bg-primary text-background opacity-0 shadow-lg transition group-hover:translate-y-0 group-hover:opacity-100">
              <button
                onclick={(e) => { e.preventDefault(); loadAndPlayAlbum(album.id); }}
                aria-label="Play {album.name}"
                class="grid size-full place-items-center rounded-full"
              >
                {#if albumLoading[album.id]}
                  <span class="block size-4 animate-spin rounded-full border-2 border-background border-t-transparent"></span>
                {:else}
                  <Play class="size-4 translate-x-px" fill="currentColor" />
                {/if}
              </button>
            </div>
          </div>
          <div class="min-w-0">
            <p class="truncate text-sm font-semibold">{album.name}</p>
            <p class="text-xs text-muted-foreground">{album.year ? `${album.year} · ` : ''}{album.songCount} songs</p>
          </div>
        </div>
      {/each}
    </div>
  </section>
{/if}

<!-- Fans also like -->
{#if artistInfo?.similarArtists?.length}
  <section class="mb-8">
    <h2 class="mb-3 text-xl font-bold">Fans also like</h2>
    <div class="flex flex-wrap gap-3">
      {#each artistInfo.similarArtists as artist (artist.name)}
        <a
          href="/artist/{encodeURIComponent(artist.name)}"
          class="flex flex-col items-center gap-2 rounded-lg p-3 text-center transition hover:bg-accent"
        >
          {#if artist.imageUrl}
            <img class="size-16 rounded-full object-cover" src={artist.imageUrl} alt={artist.name} loading="lazy" />
          {:else}
            <div class="grid size-16 place-items-center rounded-full bg-gradient-to-br from-slate-500 to-slate-700 text-sm font-bold">{initials(artist.name)}</div>
          {/if}
          <p class="max-w-[80px] truncate text-xs font-medium">{artist.name}</p>
        </a>
      {/each}
    </div>
  </section>
{/if}
