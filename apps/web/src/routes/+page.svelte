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
    fetchArtistAlbums,
    fetchLikedArtists,
    type Album,
    type Song,
  } from '$lib/servers';
  import { AlbumCard, ArtistCard, SongRow } from '$lib/components/media';
  import AlbumCarouselSection from '$lib/components/AlbumCarouselSection.svelte';
  import { getListeningProfile, type ArtistStat } from '$lib/servers/play-history';
  import { getPersonalisedArtists } from '$lib/data';
  import {
    addRecentlyPlayed,
    startQueue,
    currentIndex,
    isPlaying,
    playingFrom,
    queue,
    queueLoading,
    togglePlayRequest,
  } from '$lib/stores/player';
  import { initials } from '$lib/utils';
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
    const seen = new Set<string>();
    return albums.filter((a) => !seen.has(a.id) && !!seen.add(a.id));
  }

  // Precomputed O(1) lookup structures — rebuilt only when source data changes.
  const likedSet = $derived(new Set(likedArtists.map((a) => a.trim().toLowerCase())));
  const topArtistPlayCounts = $derived(new Map(topArtists.map((a) => [a.artist.trim().toLowerCase(), a.playCount])));

  function artistScore(name: string): number {
    const key = name.trim().toLowerCase();
    return (likedSet.has(key) ? 4 : 0) + (topArtistPlayCounts.get(key) ?? 0);
  }

  const spotlightAlbum = $derived.by(() => {
    const rankedNewest = [...newestAlbums].sort((a, b) => artistScore(b.artist) - artistScore(a.artist));
    return rankedNewest[0] ?? recentAlbums[0] ?? randomAlbums[0] ?? null;
  });

  const allRecentlyPlayedAlbums = $derived.by(() => {
    const pool = recentAlbums.length ? recentAlbums : randomAlbums;
    return uniqueAlbums(pool);
  });
  // "For You Mixes" — albums where the artist has any taste relationship (liked or played).
  // Sorted strongest-first so most relevant albums lead.
  const allFavoriteAlbums = $derived.by(() =>
    uniqueAlbums(newestAlbums.concat(randomAlbums))
      .filter((album) => artistScore(album.artist) > 0 && album.id !== spotlightAlbum?.id)
      .sort((a, b) => artistScore(b.artist) - artistScore(a.artist))
  );
  // "Fresh Finds" — albums where artistScore is 0: no liked/played relationship yet.
  // This is genuine discovery; enrichment only adds taste-relevant albums to randomAlbums
  // so zero-score albums are always truly unfamiliar artists.
  const allFreshFindsAlbums = $derived.by(() =>
    uniqueAlbums(randomAlbums.concat(newestAlbums))
      .filter((album) => artistScore(album.artist) === 0 && album.id !== spotlightAlbum?.id)
  );
  const allLateNightAlbums = $derived.by(() =>
    uniqueAlbums(recentAlbums.concat(randomAlbums, newestAlbums))
      .filter((album) => album.id !== spotlightAlbum?.id)
  );
  const allArtistHighlights = $derived.by(() =>
    topArtists
      .filter((artist) => artist.artist.trim().length > 0)
      .map((artist) => ({ name: artist.artist, image: artist.coverArtUrl ?? '' }))
  );

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
      loading = false;

      // Phase 2 — enrich "For You Mixes" with albums from liked + top-played artists.
      // Runs in the background after the page is already rendered.
      const artistNames = [
        ...liked.map((a) => a.name),
        ...profile.topArtists.slice(0, 6).map((a) => a.artist),
      ];
      // Dedupe by lowercase key, preserve first occurrence (liked artists take priority).
      const dedupedArtists = [
        ...new Map(artistNames.map((a) => [a.trim().toLowerCase(), a])).values(),
      ].slice(0, 10);

      if (dedupedArtists.length) {
        const artistAlbumResults = await Promise.allSettled(
          dedupedArtists.map((artist) => fetchArtistAlbums(artist, 6).catch(() => [] as Album[]))
        );

        if (version !== loadVersion) return;

        const artistAlbums = artistAlbumResults.flatMap((r) =>
          r.status === 'fulfilled' ? r.value : []
        );
        if (artistAlbums.length) {
          // Prepend so they bubble to the top after artistScore sorting.
          randomAlbums = uniqueAlbums([...artistAlbums, ...random]);
        }
      }

      // Phase 3 — discover similar artists via enriched recs seeded from top songs.
      // These are artists the user may not have played directly but whose music is
      // similar to what they already love — feeds "Releases for You" & all carousels.
      const topSongSeeds = profile.topSongs.slice(0, 3).map((s) => ({
        id: s.songId,
        title: s.title,
        artist: s.artist,
      }));

      if (topSongSeeds.length) {
        const similarArtists = await getPersonalisedArtists({
          topSongs: topSongSeeds,
          limit: 12,
        }).catch(() => [] as string[]);

        if (version !== loadVersion) return;

        if (similarArtists.length) {
          // Only fetch albums for artists not already covered by Phase 2.
          const phase2Keys = new Set(dedupedArtists.map((a) => a.trim().toLowerCase()));
          const newArtists = similarArtists
            .filter((a) => !phase2Keys.has(a.trim().toLowerCase()))
            .slice(0, 8);

          const similarAlbumResults = await Promise.allSettled(
            newArtists.map((artist) => fetchArtistAlbums(artist, 4).catch(() => [] as Album[]))
          );

          if (version !== loadVersion) return;

          const similarAlbums = similarAlbumResults.flatMap((r) =>
            r.status === 'fulfilled' ? r.value : []
          );
          if (similarAlbums.length) {
            randomAlbums = uniqueAlbums([...randomAlbums, ...similarAlbums]);
          }
        }
      }
    } catch (err) {
      if (version !== loadVersion) return;
      error = err instanceof Error ? err.message : 'Failed to load your home feed.';
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
      togglePlayRequest.update((n) => n + 1);
      return;
    }

    albumLoadingId = album.id;
    queueLoading.set(true);

    try {
      const songs = album.id === spotlightAlbumId && spotlightSongs.length
        ? spotlightSongs
        : await fetchAlbumSongs(album.id);

      if (!songs.length) return;

      startQueue(songs, 0, { type: 'album', name: album.name, href: `/album/${encodeURIComponent(album.id)}` });
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

    startQueue(songs, index, { type: 'album', name: spotlightAlbum.name, href: `/album/${encodeURIComponent(spotlightAlbum.id)}` });
    addRecentlyPlayed({
      id: spotlightAlbum.id,
      name: spotlightAlbum.name,
      coverArtUrl: spotlightAlbum.coverArtUrl,
      href: `/album/${encodeURIComponent(spotlightAlbum.id)}`,
      type: 'album',
    });
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
                {initials(spotlightAlbum.name)}
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

  <AlbumCarouselSection
    title="Recently Played"
    allItems={allRecentlyPlayedAlbums}
    expanded={expandedSection === 'recent'}
    {loading}
    onExpand={() => expandedSection = 'recent'}
    onCollapse={() => expandedSection = null}
  />

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
              {#each allArtistHighlights as artist (artist.name)}
                <CarouselItem class="basis-[90px] sm:basis-[100px]">
                  <ArtistCard name={artist.name} image={artist.image} />
                </CarouselItem>
              {/each}
            </CarouselContent>
            <CarouselPrevious class="carousel-nav" />
            <CarouselNext class="carousel-nav" />
            {#if allArtistHighlights.length > 12}
              <CarouselSeeAll onclick={() => expandedSection = 'artists'} />
            {/if}
          </Carousel>
        </div>
      {/if}
    </section>

    <AlbumCarouselSection
      title="For You Mixes"
      allItems={allFavoriteAlbums}
      expanded={expandedSection === 'favorites'}
      onExpand={() => expandedSection = 'favorites'}
      onCollapse={() => expandedSection = null}
    />

    <AlbumCarouselSection
      title="Fresh Finds"
      allItems={allFreshFindsAlbums}
      expanded={expandedSection === 'fresh'}
      onExpand={() => expandedSection = 'fresh'}
      onCollapse={() => expandedSection = null}
    />

    <AlbumCarouselSection
      title="Late Night Rotation"
      allItems={allLateNightAlbums}
      expanded={expandedSection === 'latenight'}
      onExpand={() => expandedSection = 'latenight'}
      onCollapse={() => expandedSection = null}
    />
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
