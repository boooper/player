<script lang="ts">
  import { onMount } from 'svelte';
  import { Play, Plus, Check, ChevronLeft, ChevronRight, Disc3, ListMusic } from '@lucide/svelte';

  import {
    fetchLikedArtists, saveLikedArtist, removeLikedArtist,
    fetchSubsonicStarredSongs, fetchSubsonicAlbumList, fetchSubsonicAlbumSongs,
    type SubsonicSong, type SubsonicAlbum
  } from '$lib/api';
  import { getTopArtists, type Artist } from '$lib/metadata';
  import {
    recentlyPlayed, subsonicPlaylists, focusTrack, playQueue, playingFrom,
    addRecentlyPlayed, type RecentItem
  } from '$lib/stores/player';

  let liked = $state<string[]>([]);
  let top = $state<Artist[]>([]);
  let starredSongs = $state<SubsonicSong[]>([]);
  let newestAlbums = $state<SubsonicAlbum[]>([]);
  let randomAlbums = $state<SubsonicAlbum[]>([]);
  let loading = $state(false);
  let error = $state('');
  let togglingArtist = $state<string | null>(null);
  let albumLoadingId = $state<string | null>(null);

  let likedCarouselEl = $state<HTMLDivElement | null>(null);
  let trendingCarouselEl = $state<HTMLDivElement | null>(null);
  let newestCarouselEl = $state<HTMLDivElement | null>(null);
  let randomCarouselEl = $state<HTMLDivElement | null>(null);
  let playlistCarouselEl = $state<HTMLDivElement | null>(null);

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
    } catch {
      liked = liked.includes(name) ? liked.filter((a) => a !== name) : [...liked, name];
    } finally {
      togglingArtist = null;
    }
  }

  async function playAlbum(album: SubsonicAlbum) {
    albumLoadingId = album.id;
    try {
      const songs = await fetchSubsonicAlbumSongs(album.id);
      if (!songs.length) return;
      focusTrack.set({ title: songs[0].title, artist: songs[0].artist, imageUrl: songs[0].coverArtUrl, source: 'subsonic', album: songs[0].album });
      playQueue(songs, 0);
      playingFrom.set({ type: 'album', name: album.name, href: `/album/${encodeURIComponent(album.id)}` });
      addRecentlyPlayed({ id: album.id, name: album.name, coverArtUrl: album.coverArtUrl, href: `/album/${encodeURIComponent(album.id)}`, type: 'album' });
    } finally {
      albumLoadingId = null;
    }
  }

  function playSong(songs: SubsonicSong[], index: number) {
    const song = songs[index];
    focusTrack.set({ title: song.title, artist: song.artist, imageUrl: song.coverArtUrl, source: 'subsonic', album: song.album });
    playingFrom.set({ type: 'album', name: song.album, href: `/album/${encodeURIComponent(song.albumId)}` });
    playQueue(songs, index);
  }

  function scroll(el: HTMLDivElement | null, dir: -1 | 1) {
    el?.scrollBy({ left: dir * 240, behavior: 'smooth' });
  }

  onMount(async () => {
    loading = true;
    try {
      const [stored, topArtists, starred, newest, random] = await Promise.all([
        fetchLikedArtists(),
        getTopArtists(24),
        fetchSubsonicStarredSongs(),
        fetchSubsonicAlbumList('newest', 20),
        fetchSubsonicAlbumList('random', 20)
      ]);
      liked = stored.map((entry) => entry.name);
      top = topArtists;
      starredSongs = starred;
      newestAlbums = newest;
      randomAlbums = random;
    } catch (err) {
      error = err instanceof Error ? err.message : 'Failed to load home feed.';
    } finally {
      loading = false;
    }
  });
</script>

<!-- Greeting -->
<h1 class="mb-8 text-3xl font-bold tracking-tight">{greeting()}</h1>

{#if error}
  <p class="mb-3 text-sm text-destructive">{error}</p>
{/if}

<!-- Recently played grid -->
{#if $recentlyPlayed.length > 0}
  <section class="mb-10">
    <h2 class="mb-4 text-xl font-bold">Recently played</h2>
    <div class="grid grid-cols-2 gap-3 sm:grid-cols-4">
      {#each $recentlyPlayed as item (item.id)}
        <a
          href={item.href}
          class="flex items-center gap-3 overflow-hidden rounded-md bg-secondary/80 transition hover:bg-accent"
        >
          {#if item.coverArtUrl}
            <img class="size-16 shrink-0 object-cover" src={item.coverArtUrl} alt={item.name} loading="lazy" />
          {:else}
            <div class="grid size-16 shrink-0 place-items-center bg-gradient-to-br from-slate-500 to-slate-700 text-sm font-bold">
              {item.name.slice(0, 2).toUpperCase()}
            </div>
          {/if}
          <span class="min-w-0 flex-1 truncate pr-3 text-sm font-semibold">{item.name}</span>
        </a>
      {/each}
    </div>
  </section>
{/if}

<!-- Playlists carousel -->
{#if $subsonicPlaylists.length > 0}
  <section class="mb-10">
    <div class="mb-4 flex items-center justify-between">
      <h2 class="text-xl font-bold">Your playlists</h2>
      <div class="flex gap-1">
        <button class="grid size-8 place-items-center rounded-full bg-secondary hover:bg-accent" onclick={() => scroll(playlistCarouselEl, -1)} aria-label="Scroll left"><ChevronLeft class="size-4" /></button>
        <button class="grid size-8 place-items-center rounded-full bg-secondary hover:bg-accent" onclick={() => scroll(playlistCarouselEl, 1)} aria-label="Scroll right"><ChevronRight class="size-4" /></button>
      </div>
    </div>
    <div bind:this={playlistCarouselEl} class="flex gap-4 overflow-x-auto pb-2" style="scrollbar-width:none;-ms-overflow-style:none">
      {#each $subsonicPlaylists as pl (pl.id)}
        <a
          href={`/playlist/${encodeURIComponent(pl.id)}`}
          class="group flex w-40 shrink-0 flex-col gap-2 rounded-lg bg-secondary/60 p-3 transition hover:bg-accent"
        >
          {#if pl.coverArtUrl}
            <img class="aspect-square w-full rounded-md object-cover shadow-md" src={pl.coverArtUrl} alt={pl.name} loading="lazy" />
          {:else}
            <div class="grid aspect-square w-full place-items-center rounded-md bg-gradient-to-br from-violet-600 to-indigo-800 shadow-md">
              <ListMusic class="size-10 text-white/70" />
            </div>
          {/if}
          <div>
            <p class="truncate text-sm font-semibold">{pl.name}</p>
            <p class="text-xs text-muted-foreground">{pl.songCount} songs</p>
          </div>
        </a>
      {/each}
    </div>
  </section>
{/if}

<!-- Newly added albums carousel -->
{#if newestAlbums.length > 0 || loading}
  <section class="mb-10">
    <div class="mb-4 flex items-center justify-between">
      <h2 class="text-xl font-bold">Newly added</h2>
      <div class="flex gap-1">
        <button class="grid size-8 place-items-center rounded-full bg-secondary hover:bg-accent" onclick={() => scroll(newestCarouselEl, -1)} aria-label="Scroll left"><ChevronLeft class="size-4" /></button>
        <button class="grid size-8 place-items-center rounded-full bg-secondary hover:bg-accent" onclick={() => scroll(newestCarouselEl, 1)} aria-label="Scroll right"><ChevronRight class="size-4" /></button>
      </div>
    </div>
    <div bind:this={newestCarouselEl} class="flex gap-4 overflow-x-auto pb-2" style="scrollbar-width:none;-ms-overflow-style:none">
      {#if loading}
        {#each Array(8) as _, i (i)}
          <div class="flex w-40 shrink-0 flex-col gap-2 p-3">
            <div class="aspect-square w-full animate-pulse rounded-md bg-muted"></div>
            <div class="h-3 w-24 animate-pulse rounded bg-muted"></div>
          </div>
        {/each}
      {:else}
        {#each newestAlbums as album (album.id)}
          <div class="group relative flex w-40 shrink-0 flex-col gap-2 rounded-lg bg-secondary/60 p-3 transition hover:bg-accent">
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
                class="absolute bottom-2 right-2 z-10 grid size-10 translate-y-1 place-items-center rounded-full bg-primary text-background opacity-0 shadow-lg transition group-hover:translate-y-0 group-hover:opacity-100"
                onclick={(e) => { e.preventDefault(); playAlbum(album); }}
                aria-label="Play {album.name}"
              >
                {#if albumLoadingId === album.id}
                  <span class="block size-4 animate-spin rounded-full border-2 border-background border-t-transparent"></span>
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
        {/each}
      {/if}
    </div>
  </section>
{/if}

<!-- Starred songs strip -->
{#if starredSongs.length > 0}
  <section class="mb-10">
    <div class="mb-4 flex items-center justify-between">
      <h2 class="text-xl font-bold">Liked songs</h2>
      <a href="/favorites" class="text-sm text-muted-foreground hover:text-foreground">See all</a>
    </div>
    <!-- Column headers -->
    <div class="grid items-center gap-4 border-b border-border/40 px-4 pb-2 text-xs font-medium uppercase tracking-wider text-muted-foreground/50"
         style="grid-template-columns: 2.5rem 1fr 1fr 4rem">
      <span class="text-center">#</span>
      <span>Title</span>
      <span class="hidden md:block">Album</span>
      <span class="text-right">Duration</span>
    </div>
    <div class="mt-1 space-y-0.5">
      {#each starredSongs.slice(0, 6) as song, i (song.id)}
        <button
          class="group grid w-full items-center gap-4 rounded-md px-4 py-2.5 text-left transition-colors duration-150 hover:bg-white/5"
          style="grid-template-columns: 2.5rem 1fr 1fr 4rem"
          onclick={() => playSong(starredSongs, i)}
        >
          <!-- Track # / Play icon crossfade -->
          <span class="relative flex h-7 w-7 shrink-0 items-center justify-center mx-auto">
            <span class="absolute inset-0 flex items-center justify-center text-sm tabular-nums text-muted-foreground transition-all duration-150 group-hover:scale-50 group-hover:opacity-0">{i + 1}</span>
            <span class="absolute inset-0 flex items-center justify-center scale-50 opacity-0 transition-all duration-150 group-hover:scale-100 group-hover:opacity-100">
              <Play class="size-4" fill="currentColor" />
            </span>
          </span>
          <!-- Title + cover art -->
          <div class="flex min-w-0 items-center gap-3">
            {#if song.coverArtUrl}
              <img class="size-10 shrink-0 rounded-md object-cover shadow-md" src={song.coverArtUrl} alt={song.title} loading="lazy" />
            {:else}
              <div class="grid size-10 shrink-0 place-items-center rounded-md bg-secondary text-xs font-bold">{song.title.slice(0, 2).toUpperCase()}</div>
            {/if}
            <div class="min-w-0">
              <p class="truncate text-sm font-medium group-hover:text-foreground transition-colors duration-150">{song.title}</p>
              <p class="truncate text-xs text-muted-foreground">{song.artist}</p>
            </div>
          </div>
          <!-- Album -->
          <span class="hidden truncate text-sm text-muted-foreground md:block">{song.album}</span>
          <!-- Duration -->
          <span class="text-right text-xs tabular-nums text-muted-foreground">{song.duration ? `${Math.floor(song.duration / 60)}:${String(song.duration % 60).padStart(2, '0')}` : '—'}</span>
        </button>
      {/each}
    </div>
  </section>
{/if}

<!-- Random picks carousel -->
{#if randomAlbums.length > 0}
  <section class="mb-10">
    <div class="mb-4 flex items-center justify-between">
      <h2 class="text-xl font-bold">You might like</h2>
      <div class="flex gap-1">
        <button class="grid size-8 place-items-center rounded-full bg-secondary hover:bg-accent" onclick={() => scroll(randomCarouselEl, -1)} aria-label="Scroll left"><ChevronLeft class="size-4" /></button>
        <button class="grid size-8 place-items-center rounded-full bg-secondary hover:bg-accent" onclick={() => scroll(randomCarouselEl, 1)} aria-label="Scroll right"><ChevronRight class="size-4" /></button>
      </div>
    </div>
    <div bind:this={randomCarouselEl} class="flex gap-4 overflow-x-auto pb-2" style="scrollbar-width:none;-ms-overflow-style:none">
      {#each randomAlbums as album (album.id)}
        <div class="group relative flex w-40 shrink-0 flex-col gap-2 rounded-lg bg-secondary/60 p-3 transition hover:bg-accent">
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
              class="absolute bottom-2 right-2 z-10 grid size-10 translate-y-1 place-items-center rounded-full bg-primary text-background opacity-0 shadow-lg transition group-hover:translate-y-0 group-hover:opacity-100"
              onclick={(e) => { e.preventDefault(); playAlbum(album); }}
              aria-label="Play {album.name}"
            >
              {#if albumLoadingId === album.id}
                <span class="block size-4 animate-spin rounded-full border-2 border-background border-t-transparent"></span>
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
      {/each}
    </div>
  </section>
{/if}

<!-- Your Artists carousel -->
{#if liked.length > 0}
  <section class="mb-10">
    <div class="mb-4 flex items-center justify-between">
      <h2 class="text-xl font-bold">Your artists</h2>
      <div class="flex gap-1">
        <button class="grid size-8 place-items-center rounded-full bg-secondary hover:bg-accent" onclick={() => scroll(likedCarouselEl, -1)} aria-label="Scroll left"><ChevronLeft class="size-4" /></button>
        <button class="grid size-8 place-items-center rounded-full bg-secondary hover:bg-accent" onclick={() => scroll(likedCarouselEl, 1)} aria-label="Scroll right"><ChevronRight class="size-4" /></button>
      </div>
    </div>
    <div bind:this={likedCarouselEl} class="flex gap-4 overflow-x-auto pb-2" style="scrollbar-width:none;-ms-overflow-style:none">
      {#each liked as artist (artist)}
        <a
          href={`/artist/${encodeURIComponent(artist)}`}
          class="group flex w-36 shrink-0 flex-col items-center gap-2 rounded-lg p-3 text-center transition hover:bg-accent"
        >
          <div class="grid size-20 place-items-center rounded-full bg-gradient-to-br from-slate-500 to-slate-700 text-lg font-bold">
            {artist.slice(0, 2).toUpperCase()}
          </div>
          <p class="w-full truncate text-xs font-semibold">{artist}</p>
          <p class="text-[11px] text-muted-foreground">Artist</p>
        </a>
      {/each}
    </div>
  </section>
{/if}

<!-- Trending Artists carousel -->
{#if top.length > 0}
  <section class="mb-10">
    <div class="mb-4 flex items-center justify-between">
      <h2 class="text-xl font-bold">Trending artists</h2>
      <div class="flex gap-1">
        <button class="grid size-8 place-items-center rounded-full bg-secondary hover:bg-accent" onclick={() => scroll(trendingCarouselEl, -1)} aria-label="Scroll left"><ChevronLeft class="size-4" /></button>
        <button class="grid size-8 place-items-center rounded-full bg-secondary hover:bg-accent" onclick={() => scroll(trendingCarouselEl, 1)} aria-label="Scroll right"><ChevronRight class="size-4" /></button>
      </div>
    </div>
    <div bind:this={trendingCarouselEl} class="flex gap-4 overflow-x-auto pb-2" style="scrollbar-width:none;-ms-overflow-style:none">
      {#each top as artist (artist.id)}
        <div class="group relative flex w-36 shrink-0 flex-col items-center gap-2 rounded-lg p-3 text-center transition hover:bg-accent">
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
      {/each}
    </div>
  </section>
{/if}

