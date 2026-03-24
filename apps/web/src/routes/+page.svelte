<script lang="ts">
  import { goto } from '$app/navigation';
  import { Pause, Play } from '@lucide/svelte';

  import {
    Carousel,
    CarouselContent,
    CarouselItem,
    CarouselNext,
    CarouselPrevious,
    CarouselSeeAll,
  } from '$lib/components/ui/carousel';
  import {
    fetchAlbumList,
    fetchAlbumSongs,
    fetchLikedArtists,
    type Album,
    type Song,
  } from '$lib/servers';
  import { AlbumCard, ArtistCard, SongRow } from '$lib/components/media';
  import { getListeningProfile, type ArtistStat } from '$lib/servers/play-history';
  import {
    addRecentlyPlayed,
    currentIndex,
    focusTrack,
    isPlaying,
    playingFrom,
    playQueue,
    queue,
    queueLoading,
    togglePlayRequest,
  } from '$lib/stores/player';
  import { libraryRefresh } from '$lib/stores/ui-state';

  let loading = $state(true);
  let error = $state('');
  let likedArtists = $state<string[]>([]);
  let newestAlbums = $state<Album[]>([]);
  let randomAlbums = $state<Album[]>([]);
  let recentAlbums = $state<Album[]>([]);
  let topArtists = $state<ArtistStat[]>([]);

  let spotlightSongs = $state<Song[]>([]);
  let spotlightLoading = $state(false);
  let spotlightAlbumId = $state('');
  let albumLoadingId = $state<string | null>(null);
  let loadVersion = 0;
  let expandedSection = $state<string | null>(null);

  function uniqueAlbums(albums: Album[]) {
    return albums.filter((album, index, list) => list.findIndex((item) => item.id === album.id) === index);
  }

  function artistScore(name: string) {
    const key = name.trim().toLowerCase();
    const liked = likedArtists.some((artist) => artist.trim().toLowerCase() === key) ? 4 : 0;
    const history = topArtists.find((artist) => artist.artist.trim().toLowerCase() === key)?.playCount ?? 0;
    return liked + history;
  }

  const spotlightAlbum = $derived.by(() => {
    const rankedNewest = [...newestAlbums].sort((a, b) => artistScore(b.artist) - artistScore(a.artist));
    return rankedNewest[0] ?? recentAlbums[0] ?? randomAlbums[0] ?? null;
  });

  const recentlyPlayedAlbums = $derived.by(() => {
    const pool = recentAlbums.length ? recentAlbums : randomAlbums;
    return uniqueAlbums(pool).slice(0, 10);
  });

  const allRecentlyPlayedAlbums = $derived.by(() => {
    const pool = recentAlbums.length ? recentAlbums : randomAlbums;
    return uniqueAlbums(pool);
  });

  const favoriteAlbums = $derived.by(() => {
    return uniqueAlbums([...newestAlbums, ...randomAlbums])
      .sort((a, b) => artistScore(b.artist) - artistScore(a.artist))
      .filter((album) => album.id !== spotlightAlbum?.id)
      .slice(0, 12);
  });

  const allFavoriteAlbums = $derived.by(() => {
    return uniqueAlbums([...newestAlbums, ...randomAlbums])
      .sort((a, b) => artistScore(b.artist) - artistScore(a.artist))
      .filter((album) => album.id !== spotlightAlbum?.id);
  });

  const freshFindsAlbums = $derived.by(() => {
    return uniqueAlbums([...randomAlbums, ...newestAlbums])
      .filter((album) => !favoriteAlbums.some((item) => item.id === album.id))
      .slice(0, 12);
  });

  const allFreshFindsAlbums = $derived.by(() => {
    return uniqueAlbums([...randomAlbums, ...newestAlbums])
      .filter((album) => !favoriteAlbums.some((item) => item.id === album.id));
  });

  const lateNightAlbums = $derived.by(() => {
    return uniqueAlbums([...recentAlbums, ...randomAlbums, ...newestAlbums])
      .filter((album) => album.id !== spotlightAlbum?.id)
      .slice(3, 15);
  });

  const allLateNightAlbums = $derived.by(() => {
    return uniqueAlbums([...recentAlbums, ...randomAlbums, ...newestAlbums])
      .filter((album) => album.id !== spotlightAlbum?.id);
  });

  const artistHighlights = $derived.by(() => {
    return topArtists
      .filter((artist) => artist.artist.trim().length > 0)
      .slice(0, 12)
      .map((artist) => ({
        name: artist.artist,
        image: artist.coverArtUrl ?? '',
      }));
  });

  const allArtistHighlights = $derived.by(() => {
    return topArtists
      .filter((artist) => artist.artist.trim().length > 0)
      .map((artist) => ({
        name: artist.artist,
        image: artist.coverArtUrl ?? '',
      }));
  });

  const spotlightHref = $derived(
    spotlightAlbum ? `/album/${encodeURIComponent(spotlightAlbum.id)}` : ''
  );

  const spotlightIsActive = $derived($playingFrom.href === spotlightHref);

  async function loadHome() {
    const version = ++loadVersion;
    loading = true;
    error = '';

    try {
      const [liked, newest, random, recent, profile] = await Promise.all([
        fetchLikedArtists(),
        fetchAlbumList('newest', 24),
        fetchAlbumList('random', 24),
        fetchAlbumList('recent', 16),
        getListeningProfile(),
      ]);

      if (version !== loadVersion) return;

      likedArtists = liked.map((artist) => artist.name);
      newestAlbums = newest;
      randomAlbums = random;
      recentAlbums = recent;
      topArtists = profile.topArtists;
    } catch (err) {
      if (version !== loadVersion) return;
      error = err instanceof Error ? err.message : 'Failed to load your home feed.';
    } finally {
      if (version !== loadVersion) return;
      loading = false;
    }
  }

  async function loadSpotlightSongs(album: Album | null) {
    if (!album) {
      spotlightSongs = [];
      spotlightAlbumId = '';
      return;
    }

    if (spotlightAlbumId === album.id && spotlightSongs.length) return;

    spotlightLoading = true;
    try {
      const songs = await fetchAlbumSongs(album.id);
      if (spotlightAlbum?.id !== album.id) return;
      spotlightSongs = songs.slice(0, 5);
      spotlightAlbumId = album.id;
    } catch {
      if (spotlightAlbum?.id !== album.id) return;
      spotlightSongs = [];
      spotlightAlbumId = album.id;
    } finally {
      if (spotlightAlbum?.id === album.id) {
        spotlightLoading = false;
      }
    }
  }

  async function playAlbum(album: Album) {
    if (spotlightIsActive && spotlightAlbum?.id === album.id) {
      togglePlayRequest.update((value) => value + 1);
      return;
    }

    albumLoadingId = album.id;
    queueLoading.set(true);

    try {
      const songs = album.id === spotlightAlbumId && spotlightSongs.length
        ? spotlightSongs
        : await fetchAlbumSongs(album.id);

      if (!songs.length) return;

      focusTrack.set({
        title: songs[0].title,
        artist: songs[0].artist,
        imageUrl: songs[0].coverArtUrl,
        source: 'library',
        album: songs[0].album,
      });
      playQueue(songs, 0);
      playingFrom.set({ type: 'album', name: album.name, href: `/album/${encodeURIComponent(album.id)}` });
      addRecentlyPlayed({
        id: album.id,
        name: album.name,
        coverArtUrl: album.coverArtUrl,
        href: `/album/${encodeURIComponent(album.id)}`,
        type: 'album',
      });
    } finally {
      albumLoadingId = null;
      queueLoading.set(false);
    }
  }

  async function playSpotlightSong(song: Song, index: number) {
    if (!spotlightAlbum) return;

    const songs = spotlightSongs.length ? spotlightSongs : await fetchAlbumSongs(spotlightAlbum.id);
    if (!songs.length) return;

    focusTrack.set({
      title: song.title,
      artist: song.artist,
      imageUrl: song.coverArtUrl,
      source: 'library',
      album: song.album,
    });
    playQueue(songs, index);
    playingFrom.set({ type: 'album', name: spotlightAlbum.name, href: `/album/${encodeURIComponent(spotlightAlbum.id)}` });
    addRecentlyPlayed({
      id: spotlightAlbum.id,
      name: spotlightAlbum.name,
      coverArtUrl: spotlightAlbum.coverArtUrl,
      href: `/album/${encodeURIComponent(spotlightAlbum.id)}`,
      type: 'album',
    });
  }

  function coverFallback(name: string) {
    return name
      .split(' ')
      .filter(Boolean)
      .slice(0, 2)
      .map((part) => part[0]?.toUpperCase() ?? '')
      .join('');
  }

  $effect(() => {
    const refresh = $libraryRefresh;
    void refresh;
    loadHome();
  });

  $effect(() => {
    loadSpotlightSongs(spotlightAlbum);
  });
</script>

<div class="space-y-10 pb-8">
  <section class="page-section">
    <div class="mb-5 flex items-end justify-between gap-4">
      <div>
        <h1 class="text-[2rem] font-semibold tracking-[-0.03em] text-foreground">Releases for You</h1>
      </div>
    </div>

    {#if error}
      <div class="mb-6 rounded-[22px] border border-rose-500/20 bg-rose-500/8 px-4 py-3 text-sm text-rose-200">
        {error}
      </div>
    {/if}

    {#if loading}
      <div class="grid gap-6 xl:grid-cols-[280px_minmax(0,1fr)]">
        <div class="aspect-square animate-pulse rounded-[28px] bg-white/6"></div>
        <div class="space-y-3">
          {#each Array(5) as _, index (index)}
            <div class="h-[72px] animate-pulse rounded-[22px] bg-white/5"></div>
          {/each}
        </div>
      </div>
    {:else if spotlightAlbum}
      <div class="grid gap-6 xl:grid-cols-[280px_minmax(0,1fr)]">
        <div class="group relative overflow-hidden rounded-[28px]">
          <button
            type="button"
            class="block w-full text-left"
            onclick={() => goto(`/album/${encodeURIComponent(spotlightAlbum.id)}`)}
            aria-label={`Open ${spotlightAlbum.name}`}
          >
            {#if spotlightAlbum.coverArtUrl}
              <img
                src={spotlightAlbum.coverArtUrl}
                alt={spotlightAlbum.name}
                class="aspect-square w-full rounded-[28px] object-cover shadow-[0_26px_60px_rgba(0,0,0,0.38)] transition duration-300 group-hover:scale-[1.015]"
              />
            {:else}
              <div class="flex aspect-square w-full items-center justify-center rounded-[28px] bg-white/6 text-5xl font-semibold text-white/45">
                {coverFallback(spotlightAlbum.name)}
              </div>
            {/if}
          </button>
          <button
            type="button"
            class="absolute bottom-4 right-4 flex size-11 items-center justify-center rounded-full border border-white/10 bg-black/55 text-white shadow-lg backdrop-blur-md transition hover:scale-105"
            onclick={(event) => {
              event.stopPropagation();
              playAlbum(spotlightAlbum);
            }}
            aria-label={spotlightIsActive && $isPlaying ? `Pause ${spotlightAlbum.name}` : `Play ${spotlightAlbum.name}`}
          >
            {#if albumLoadingId === spotlightAlbum.id}
              <span class="block size-4 animate-spin rounded-full border-2 border-white/80 border-t-transparent"></span>
            {:else if spotlightIsActive && $isPlaying}
              <Pause class="size-4" fill="currentColor" />
            {:else}
              <Play class="size-4 translate-x-px" fill="currentColor" />
            {/if}
          </button>
        </div>

        <div class="flex min-w-0 flex-col justify-start pt-1">
          {#if spotlightLoading && !spotlightSongs.length}
            <div class="space-y-3">
              {#each Array(5) as _, index (index)}
                <div class="h-[72px] animate-pulse rounded-[22px] bg-white/5"></div>
              {/each}
            </div>
          {:else}
            <div class="space-y-1">
              {#each spotlightSongs as song, index (song.id)}
                <SongRow {song} {index} onplay={() => playSpotlightSong(song, index)} />
              {/each}
            </div>
          {/if}
        </div>
      </div>
    {/if}
  </section>

  <section class="page-section">
    <div class="mb-5 flex items-center justify-between">
      <h2 class="text-[1.6rem] font-semibold tracking-[-0.03em] text-foreground">Recently Played</h2>
      {#if expandedSection === 'recent'}
        <button
          class="text-[13px] font-medium text-muted-foreground transition-colors hover:text-foreground"
          onclick={() => expandedSection = null}
        >Show Less</button>
      {/if}
    </div>

    {#if loading}
      <div class="grid grid-cols-2 gap-4 sm:grid-cols-3 xl:grid-cols-5">
        {#each Array(5) as _, index (index)}
          <div class="space-y-3">
            <div class="aspect-square animate-pulse rounded-[24px] bg-white/6"></div>
            <div class="h-4 w-3/4 animate-pulse rounded-full bg-white/6"></div>
            <div class="h-3 w-1/2 animate-pulse rounded-full bg-white/5"></div>
          </div>
        {/each}
      </div>
    {:else if expandedSection === 'recent'}
      <div class="section-enter grid grid-cols-2 gap-4 sm:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 2xl:grid-cols-6">
        {#each allRecentlyPlayedAlbums as album (album.id)}
          <AlbumCard {album} />
        {/each}
      </div>
    {:else}
      <div class="section-enter">
        <Carousel opts={{ align: 'start', dragFree: true }}>
          <CarouselContent>
            {#each recentlyPlayedAlbums as album (album.id)}
              <CarouselItem class="basis-[36%] sm:basis-[24%] lg:basis-[18%] xl:basis-[14%]">
                <AlbumCard {album} />
              </CarouselItem>
            {/each}
          </CarouselContent>
          <CarouselPrevious class="carousel-nav" />
          <CarouselNext class="carousel-nav" />
          {#if !loading && allRecentlyPlayedAlbums.length > recentlyPlayedAlbums.length}
            <CarouselSeeAll onclick={() => expandedSection = 'recent'} />
          {/if}
        </Carousel>
      </div>
    {/if}
  </section>

  {#if !loading}
    <section class="page-section">
      <div class="mb-5 flex items-center justify-between">
        <h2 class="text-[1.6rem] font-semibold tracking-[-0.03em] text-foreground">Artists For You</h2>
        {#if expandedSection === 'artists'}
          <button
            class="text-[13px] font-medium text-muted-foreground transition-colors hover:text-foreground"
            onclick={() => expandedSection = null}
          >Show Less</button>
        {/if}
      </div>

      {#if expandedSection === 'artists'}
        <div class="section-enter grid grid-cols-3 gap-4 sm:grid-cols-4 lg:grid-cols-6 xl:grid-cols-8 2xl:grid-cols-10">
          {#each allArtistHighlights as artist (artist.name)}
            <ArtistCard name={artist.name} image={artist.image} />
          {/each}
        </div>
      {:else}
        <div class="section-enter">
          <Carousel opts={{ align: 'start', dragFree: true }}>
            <CarouselContent>
              {#each artistHighlights as artist (artist.name)}
                <CarouselItem class="basis-[90px] sm:basis-[100px]">
                  <ArtistCard name={artist.name} image={artist.image} />
                </CarouselItem>
              {/each}
            </CarouselContent>
            <CarouselPrevious class="carousel-nav" />
            <CarouselNext class="carousel-nav" />
            {#if allArtistHighlights.length > artistHighlights.length}
              <CarouselSeeAll onclick={() => expandedSection = 'artists'} />
            {/if}
          </Carousel>
        </div>
      {/if}
    </section>

    <section class="page-section">
      <div class="mb-5 flex items-center justify-between">
        <h2 class="text-[1.6rem] font-semibold tracking-[-0.03em] text-foreground">For You Mixes</h2>
        {#if expandedSection === 'favorites'}
          <button
            class="text-[13px] font-medium text-muted-foreground transition-colors hover:text-foreground"
            onclick={() => expandedSection = null}
          >Show Less</button>
        {/if}
      </div>

      {#if expandedSection === 'favorites'}
        <div class="section-enter grid grid-cols-2 gap-4 sm:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 2xl:grid-cols-6">
          {#each allFavoriteAlbums as album (album.id)}
            <AlbumCard {album} />
          {/each}
        </div>
      {:else}
        <div class="section-enter">
          <Carousel opts={{ align: 'start', dragFree: true }}>
            <CarouselContent>
              {#each favoriteAlbums as album (album.id)}
                <CarouselItem class="basis-[36%] sm:basis-[24%] lg:basis-[18%] xl:basis-[14%]">
                  <AlbumCard {album} />
                </CarouselItem>
              {/each}
            </CarouselContent>
            <CarouselPrevious class="carousel-nav" />
            <CarouselNext class="carousel-nav" />
            {#if allFavoriteAlbums.length > favoriteAlbums.length}
              <CarouselSeeAll onclick={() => expandedSection = 'favorites'} />
            {/if}
          </Carousel>
        </div>
      {/if}
    </section>

    <section class="page-section">
      <div class="mb-5 flex items-center justify-between">
        <h2 class="text-[1.6rem] font-semibold tracking-[-0.03em] text-foreground">Fresh Finds</h2>
        {#if expandedSection === 'fresh'}
          <button
            class="text-[13px] font-medium text-muted-foreground transition-colors hover:text-foreground"
            onclick={() => expandedSection = null}
          >Show Less</button>
        {/if}
      </div>

      {#if expandedSection === 'fresh'}
        <div class="section-enter grid grid-cols-2 gap-4 sm:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 2xl:grid-cols-6">
          {#each allFreshFindsAlbums as album (album.id)}
            <AlbumCard {album} />
          {/each}
        </div>
      {:else}
        <div class="section-enter">
          <Carousel opts={{ align: 'start', dragFree: true }}>
            <CarouselContent>
              {#each freshFindsAlbums as album (album.id)}
                <CarouselItem class="basis-[36%] sm:basis-[24%] lg:basis-[18%] xl:basis-[14%]">
                  <AlbumCard {album} />
                </CarouselItem>
              {/each}
            </CarouselContent>
            <CarouselPrevious class="carousel-nav" />
            <CarouselNext class="carousel-nav" />
            {#if allFreshFindsAlbums.length > freshFindsAlbums.length}
              <CarouselSeeAll onclick={() => expandedSection = 'fresh'} />
            {/if}
          </Carousel>
        </div>
      {/if}
    </section>

    <section class="page-section">
      <div class="mb-5 flex items-center justify-between">
        <h2 class="text-[1.6rem] font-semibold tracking-[-0.03em] text-foreground">Late Night Rotation</h2>
        {#if expandedSection === 'latenight'}
          <button
            class="text-[13px] font-medium text-muted-foreground transition-colors hover:text-foreground"
            onclick={() => expandedSection = null}
          >Show Less</button>
        {/if}
      </div>

      {#if expandedSection === 'latenight'}
        <div class="section-enter grid grid-cols-2 gap-4 sm:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 2xl:grid-cols-6">
          {#each allLateNightAlbums as album (album.id)}
            <AlbumCard {album} />
          {/each}
        </div>
      {:else}
        <div class="section-enter">
          <Carousel opts={{ align: 'start', dragFree: true }}>
            <CarouselContent>
              {#each lateNightAlbums as album (album.id)}
                <CarouselItem class="basis-[36%] sm:basis-[24%] lg:basis-[18%] xl:basis-[14%]">
                  <AlbumCard {album} />
                </CarouselItem>
              {/each}
            </CarouselContent>
            <CarouselPrevious class="carousel-nav" />
            <CarouselNext class="carousel-nav" />
            {#if allLateNightAlbums.length > lateNightAlbums.length}
              <CarouselSeeAll onclick={() => expandedSection = 'latenight'} />
            {/if}
          </Carousel>
        </div>
      {/if}
    </section>
  {/if}
</div>

<style>
  :global(.carousel-nav) {
    border-color: rgb(255 255 255 / 0.08);
    background: rgb(10 10 14 / 0.82);
    backdrop-filter: blur(14px);
  }

  .section-enter {
    animation: section-enter 240ms cubic-bezier(0.2, 0.9, 0.25, 1) both;
  }

  @keyframes section-enter {
    from {
      opacity: 0;
      transform: translateY(10px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }
</style>
