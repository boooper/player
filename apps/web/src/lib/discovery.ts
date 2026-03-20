import { fetchAudioDbArtist, fetchAudioDbArtistPhoto } from '$lib/providers/metadata/audiodb';
import {
  fetchTopArtists as lfmTopArtists,
  searchArtists as lfmSearchArtists,
  fetchTopSongs as lfmTopSongs,
  searchSongs as lfmSearchSongs,
  fetchArtistInfo as lfmArtistInfo,
  fetchArtistTopTracks as lfmArtistTopTracks,
  fetchTopTags as lfmTopTags,
  fetchLastFmRecommendations,
  fetchTrackTopGenre as lfmTrackTopGenre
} from '$lib/providers/metadata/lastfm';
import { fetchListenBrainzRecommendations } from '$lib/providers/recommendation/listenbrainz';
import { fetchArtistAlbums, searchSongs as searchLibrarySongs, type Album as LibraryAlbum, type Song as LibrarySong } from '$lib/servers';
import { getArtistAffinities, getGenreAffinities, getSongAffinities } from '$lib/servers/play-history';
import {
  getLastFmApiKey,
  getListenBrainzUsername,
  getMetadataProviderSetting,
  getRecommendationProviderSetting
} from '$lib/stores/backend-settings';

export type DiscoveryArtist = {
  id: string;
  name: string;
  imageUrl: string;
  listeners?: number;
  url?: string;
};

export type DiscoverySong = {
  id: string;
  title: string;
  artist: string;
  imageUrl: string;
  listeners?: number;
  url?: string;
};

export type DiscoveryArtistInfo = {
  name: string;
  imageUrl: string;
  listeners: number;
  playcount: number;
  bio: string;
  tags: string[];
  similarArtists: { name: string; imageUrl: string }[];
  genre: string;
  country: string;
  formedYear: string;
};

export type DiscoveryRecommendation = {
  id: string;
  title: string;
  artist: string;
  score: number;
  matchScore: number;
  artistLiked: boolean;
  genreScore: number;
  url: string;
};

type MetadataProvider = 'local' | 'lastfm' | 'audiodb' | 'both';
type RecommendationProvider = 'lastfm' | 'listenbrainz';
const artistArtworkCache = new Map<string, Promise<string>>();
const discoveryCache = new Map<string, { expiresAt: number; promise: Promise<unknown> }>();
const SHORT_CACHE_TTL_MS = 30_000;
const DEFAULT_CACHE_TTL_MS = 5 * 60_000;

function getCachedValue<T>(key: string, loader: () => Promise<T>, ttlMs = DEFAULT_CACHE_TTL_MS): Promise<T> {
  const now = Date.now();
  const cached = discoveryCache.get(key);
  if (cached && cached.expiresAt > now) {
    return cached.promise as Promise<T>;
  }

  const promise = loader().catch((error) => {
    const current = discoveryCache.get(key);
    if (current?.promise === promise) {
      discoveryCache.delete(key);
    }
    throw error;
  });

  discoveryCache.set(key, { expiresAt: now + ttlMs, promise });
  return promise;
}

export function clearDiscoveryCaches(): void {
  discoveryCache.clear();
  artistArtworkCache.clear();
}

function getMetadataProvider(): MetadataProvider {
  const value = getMetadataProviderSetting().trim().toLowerCase();
  if (value === 'local' || value === 'lastfm' || value === 'audiodb' || value === 'both') return value;
  return 'both';
}

function getRecommendationProvider(): RecommendationProvider {
  const value = getRecommendationProviderSetting().trim().toLowerCase();
  if (value === 'listenbrainz') return 'listenbrainz';
  return 'lastfm';
}

function getLfmKey(): string {
  return getLastFmApiKey();
}

function hasLfm(): boolean {
  return Boolean(getLfmKey());
}

function normalize(value: string): string {
  return value.trim().toLowerCase();
}

function uniqueBy<T>(items: T[], key: (item: T) => string): T[] {
  const seen = new Set<string>();
  const out: T[] = [];
  for (const item of items) {
    const id = key(item);
    if (seen.has(id)) continue;
    seen.add(id);
    out.push(item);
  }
  return out;
}

async function hydrateImages<T extends { imageUrl: string }>(items: T[], getName: (item: T) => string): Promise<T[]> {
  return Promise.all(
    items.map(async (item) => {
      const imageUrl = await getArtistArtwork(getName(item), item.imageUrl);
      if (imageUrl === item.imageUrl) return item;
      return { ...item, imageUrl };
    })
  );
}

function recommendationKey(artist: string, title: string): string {
  return `${normalize(artist)}::${normalize(title)}`;
}

function matchRecommendedSong(
  candidates: LibrarySong[],
  recArtist: string,
  recTitle: string
): LibrarySong | null {
  const exact = candidates.find((song) => recommendationKey(song.artist, song.title) === recommendationKey(recArtist, recTitle));
  if (exact) return exact;

  const byArtist = candidates.filter((song) => normalize(song.artist) === normalize(recArtist));
  return (
    byArtist.find(
      (song) =>
        normalize(song.title).includes(normalize(recTitle)) ||
        normalize(recTitle).includes(normalize(song.title))
    ) ?? null
  );
}

async function searchLocalSongs(query: string, limit = 12): Promise<LibrarySong[]> {
  return searchLibrarySongs(query, Math.max(limit, 24)).catch(() => []);
}

async function searchLocalArtists(query: string, limit = 12): Promise<DiscoveryArtist[]> {
  const songs = await searchLocalSongs(query, Math.max(limit * 4, 24));
  const filtered = songs.filter((song) => normalize(song.artist).includes(normalize(query)));
  const uniqueArtists = uniqueBy(filtered, (song) => normalize(song.artist)).slice(0, limit);
  return uniqueArtists.map((song) => ({
    id: `local-artist-${encodeURIComponent(song.artist)}`,
    name: song.artist,
    imageUrl: song.coverArtUrl || ''
  }));
}

async function getLocalArtistAlbums(artist: string): Promise<LibraryAlbum[]> {
  return getCachedValue(
    `local-artist-albums:${normalize(artist)}`,
    () => fetchArtistAlbums(artist, 24).catch(() => []),
    DEFAULT_CACHE_TTL_MS
  );
}

async function getLocalArtistSongs(artist: string, limit = 30): Promise<LibrarySong[]> {
  return getCachedValue(
    `local-artist-songs:${normalize(artist)}:${limit}`,
    async () => {
      const songs = await searchLocalSongs(artist, Math.max(limit * 2, 30));
      return songs.filter((song) => normalize(song.artist) === normalize(artist)).slice(0, limit);
    },
    DEFAULT_CACHE_TTL_MS
  );
}

async function getLocalArtistInfo(artist: string): Promise<DiscoveryArtistInfo | null> {
  const [albums, songs] = await Promise.all([getLocalArtistAlbums(artist), getLocalArtistSongs(artist, 24)]);
  if (!albums.length && !songs.length) return null;

  const imageUrl = albums.find((album) => album.coverArtUrl)?.coverArtUrl || songs.find((song) => song.coverArtUrl)?.coverArtUrl || '';
  const tags = uniqueBy(
    songs.map((song) => song.album).filter(Boolean).map((value) => ({ value })),
    (item) => normalize(item.value)
  )
    .slice(0, 5)
    .map((item) => item.value);

  return {
    name: artist,
    imageUrl,
    listeners: songs.length,
    playcount: 0,
    bio: '',
    tags,
    similarArtists: [],
    genre: '',
    country: '',
    formedYear: ''
  };
}

async function getLocalArtistTopTracks(artist: string, limit = 10): Promise<DiscoverySong[]> {
  const songs = await getLocalArtistSongs(artist, limit * 3);
  return songs.slice(0, limit).map((song) => ({
    id: `local-track-${song.id}`,
    title: song.title,
    artist: song.artist,
    imageUrl: song.coverArtUrl || ''
  }));
}

async function resolveLocalArtistArtwork(artist: string): Promise<string> {
  const [albums, songs] = await Promise.all([
    getLocalArtistAlbums(artist).catch(() => []),
    getLocalArtistSongs(artist, 12).catch(() => [])
  ]);
  return albums.find((album) => album.coverArtUrl)?.coverArtUrl || songs.find((song) => song.coverArtUrl)?.coverArtUrl || '';
}

async function resolveMetadataArtistArtwork(artist: string): Promise<string> {
  const provider = getMetadataProvider();
  if (provider === 'local') return '';

  if (provider === 'lastfm') {
    if (!hasLfm()) return '';
    return (await lfmArtistInfo({ apiKey: getLfmKey(), artist }))?.imageUrl || '';
  }

  if (provider === 'audiodb') {
    return fetchAudioDbArtistPhoto(artist);
  }

  const [lfmRes, adbRes] = await Promise.allSettled([
    hasLfm() ? lfmArtistInfo({ apiKey: getLfmKey(), artist }) : Promise.resolve(null),
    fetchAudioDbArtist(artist)
  ]);

  const lfmImage = lfmRes.status === 'fulfilled' ? lfmRes.value?.imageUrl || '' : '';
  const adbImage = adbRes.status === 'fulfilled'
    ? adbRes.value?.thumb || adbRes.value?.fanart || adbRes.value?.banner || ''
    : '';

  return lfmImage || adbImage;
}

export async function getArtistArtwork(artist: string, currentImageUrl = ''): Promise<string> {
  const normalized = normalize(artist);
  if (!normalized) return currentImageUrl;

  if (!artistArtworkCache.has(normalized)) {
    artistArtworkCache.set(
      normalized,
      (async () => {
        const localImage = await resolveLocalArtistArtwork(artist);
        if (localImage) return localImage;
        if (currentImageUrl) return currentImageUrl;
        return resolveMetadataArtistArtwork(artist);
      })()
    );
  }

  const resolved = await artistArtworkCache.get(normalized)!;
  if (resolved) return resolved;
  return currentImageUrl;
}

export async function getArtistArtworkMap(artists: string[]): Promise<Record<string, string>> {
  const uniqueArtists = [...new Set(artists.map((artist) => artist.trim()).filter(Boolean))];
  const entries = await Promise.all(
    uniqueArtists.map(async (artist) => [artist, await getArtistArtwork(artist)] as const)
  );
  return Object.fromEntries(entries);
}

export async function getTopArtists(limit = 24): Promise<DiscoveryArtist[]> {
  const provider = getMetadataProvider();
  if (provider === 'local' || provider === 'audiodb' || !hasLfm()) return [];
  return getCachedValue(
    `top-artists:${provider}:${limit}`,
    async () => hydrateImages(await lfmTopArtists({ apiKey: getLfmKey(), limit }), (artist) => artist.name),
    SHORT_CACHE_TTL_MS
  );
}

export async function searchArtists(query: string, limit = 12): Promise<DiscoveryArtist[]> {
  const provider = getMetadataProvider();
  if (provider === 'local') return searchLocalArtists(query, limit);
  if (provider === 'audiodb' || !hasLfm()) return [];

  const remote = await hydrateImages(await lfmSearchArtists({ apiKey: getLfmKey(), query, limit }), (artist) => artist.name);
  if (provider !== 'both') return remote;
  const local = await searchLocalArtists(query, limit);
  return uniqueBy([...local, ...remote], (artist) => normalize(artist.name)).slice(0, limit);
}

export async function getTopSongs(limit = 24): Promise<DiscoverySong[]> {
  const provider = getMetadataProvider();
  if (provider === 'local' || provider === 'audiodb' || !hasLfm()) return [];
  return getCachedValue(
    `top-songs:${provider}:${limit}`,
    async () => hydrateImages(await lfmTopSongs({ apiKey: getLfmKey(), limit }), (song) => song.artist),
    SHORT_CACHE_TTL_MS
  );
}

export async function searchMetadataSongs(query: string, limit = 12): Promise<DiscoverySong[]> {
  const provider = getMetadataProvider();
  const localSongs = (await searchLocalSongs(query, limit)).map((song) => ({
    id: `local-song-${song.id}`,
    title: song.title,
    artist: song.artist,
    imageUrl: song.coverArtUrl || ''
  }));

  if (provider === 'local') return localSongs.slice(0, limit);
  if (provider === 'audiodb' || !hasLfm()) return [];

  const remote = await hydrateImages(await lfmSearchSongs({ apiKey: getLfmKey(), query, limit }), (song) => song.artist);
  if (provider !== 'both') return remote;
  return uniqueBy([...localSongs, ...remote], (song) => `${normalize(song.artist)}::${normalize(song.title)}`).slice(0, limit);
}

export async function getArtistInfo(artist: string): Promise<DiscoveryArtistInfo | null> {
  const provider = getMetadataProvider();
  return getCachedValue(
    `artist-info:${provider}:${normalize(artist)}`,
    async () => {
      const local = await getLocalArtistInfo(artist);

      if (provider === 'local') return local;

      if (provider === 'audiodb') {
        const adb = await fetchAudioDbArtist(artist);
        if (!adb) return local;
        return {
          name: local?.name || adb.name || artist,
          imageUrl: local?.imageUrl || adb.thumb || adb.fanart || adb.banner || '',
          listeners: local?.listeners ?? 0,
          playcount: 0,
          bio: local?.bio || adb.biography,
          tags: local?.tags?.length ? local.tags : adb.genre ? [adb.genre] : [],
          similarArtists: [],
          genre: adb.genre,
          country: adb.country,
          formedYear: adb.formedYear
        };
      }

      if (provider === 'lastfm') {
        if (!hasLfm()) return local;
        const lfm = await lfmArtistInfo({ apiKey: getLfmKey(), artist });
        if (!lfm) return local;
        return {
          name: local?.name || lfm.name,
          imageUrl: local?.imageUrl || lfm.imageUrl,
          listeners: lfm.listeners ?? local?.listeners ?? 0,
          playcount: lfm.playcount ?? 0,
          bio: lfm.bio || local?.bio || '',
          tags: lfm.tags?.length ? lfm.tags : local?.tags ?? [],
          similarArtists: lfm.similarArtists,
          genre: lfm.tags[0] ?? '',
          country: '',
          formedYear: ''
        };
      }

      const [lfmRes, adbRes] = await Promise.allSettled([
        hasLfm() ? lfmArtistInfo({ apiKey: getLfmKey(), artist }) : Promise.resolve(null),
        fetchAudioDbArtist(artist)
      ]);
      const lfm = lfmRes.status === 'fulfilled' ? lfmRes.value : null;
      const adb = adbRes.status === 'fulfilled' ? adbRes.value : null;

      if (!local && !lfm && !adb) return null;

      const imageUrl = await getArtistArtwork(artist, local?.imageUrl || lfm?.imageUrl || (adb ? adb.thumb || adb.fanart || adb.banner : '') || '');
      const rawSimilar = lfm?.similarArtists ?? [];
      const similarArtists = await Promise.all(
        rawSimilar.map(async (item) => ({ ...item, imageUrl: await getArtistArtwork(item.name, item.imageUrl) }))
      );

      return {
        name: local?.name || lfm?.name || adb?.name || artist,
        imageUrl,
        listeners: lfm?.listeners ?? local?.listeners ?? 0,
        playcount: lfm?.playcount ?? 0,
        bio: local?.bio || lfm?.bio || adb?.biography || '',
        tags: local?.tags?.length ? local.tags : lfm?.tags?.length ? lfm.tags : adb?.genre ? [adb.genre] : [],
        similarArtists,
        genre: adb?.genre || lfm?.tags?.[0] || '',
        country: adb?.country || '',
        formedYear: adb?.formedYear || ''
      };
    },
    SHORT_CACHE_TTL_MS
  );
}

export async function getArtistTopTracks(artist: string, limit = 10): Promise<DiscoverySong[]> {
  const provider = getMetadataProvider();
  return getCachedValue(
    `artist-top-tracks:${provider}:${normalize(artist)}:${limit}`,
    async () => {
      const local = await getLocalArtistTopTracks(artist, limit);
      if (provider === 'local') return local;
      if (!hasLfm() || provider === 'audiodb') return local;
      const remote = await lfmArtistTopTracks({ apiKey: getLfmKey(), artist, limit });
      if (provider !== 'both') return remote;
      return uniqueBy([...local, ...remote], (song) => `${normalize(song.artist)}::${normalize(song.title)}`).slice(0, limit);
    },
    SHORT_CACHE_TTL_MS
  );
}

export async function getTopTags(limit = 40): Promise<string[]> {
  const provider = getMetadataProvider();
  if (!hasLfm() || provider === 'audiodb' || provider === 'local') return [];
  return lfmTopTags({ apiKey: getLfmKey(), limit });
}

export async function getRecommendations(params: {
  seedArtist: string;
  seedSongTitle: string;
  seedGenre?: string;
  likedArtists?: string[];
  limit?: number;
}): Promise<DiscoveryRecommendation[]> {
  const provider = getRecommendationProvider();
  if (provider === 'listenbrainz') {
    return fetchListenBrainzRecommendations({
      username: getListenBrainzUsername(),
      likedArtists: params.likedArtists,
      limit: params.limit
    });
  }
  if (!hasLfm()) return [];
  return fetchLastFmRecommendations({ apiKey: getLfmKey(), ...params });
}

export async function getTrackTopGenre(artist: string, track: string): Promise<string> {
  if (getRecommendationProvider() !== 'lastfm' || !hasLfm()) return '';
  return lfmTrackTopGenre({ apiKey: getLfmKey(), artist, track });
}

export async function getUpNextSongs(params: {
  artist: string;
  title: string;
  likedArtists?: string[];
  limit?: number;
}): Promise<LibrarySong[]> {
  const artist = params.artist.trim();
  const title = params.title.trim();
  const limit = params.limit ?? 5;

  if (!artist || !title || limit <= 0) return [];
  const likedKey = (params.likedArtists ?? []).map(normalize).sort().join('|');

  return getCachedValue(
    `up-next:${getRecommendationProvider()}:${normalize(artist)}:${normalize(title)}:${limit}:${likedKey}`,
    async () => {
      const recommendations = await getRecommendations({
        seedArtist: artist,
        seedSongTitle: title,
        likedArtists: params.likedArtists,
        limit: limit * 4
      }).catch(() => [] as DiscoveryRecommendation[]);

      if (!recommendations.length) return [];

      // Step 1 — Re-rank by artist + genre affinity before searching local library
      const uniqueArtists = [...new Set(recommendations.map((r) => r.artist.toLowerCase()))];
      const uniqueGenres  = [...new Set(
        recommendations.flatMap((r) => r.genreScore > 0 ? [normalize(r.artist)] : [])
      )];
      const [artistAffinities, genreAffinities] = await Promise.all([
        getArtistAffinities(uniqueArtists),
        uniqueGenres.length ? getGenreAffinities(uniqueGenres) : Promise.resolve({} as Record<string, number>)
      ]);

      const reranked = recommendations
        .map((rec) => {
          const artistBoost = (artistAffinities[rec.artist.toLowerCase()] ?? 0) * 0.35;
          const genreBoost  = (genreAffinities[normalize(rec.artist)] ?? 0) * 0.10;
          return { ...rec, score: Math.min(rec.score * 0.55 + artistBoost + genreBoost, 1) };
        })
        .sort((a, b) => b.score - a.score);

      // Step 2 — Find local library matches (collect more than needed so we can filter)
      const candidates: { song: LibrarySong; recScore: number }[] = [];
      const seen = new Set<string>();

      for (const rec of reranked) {
        if (candidates.length >= limit * 3) break;
        const hits = await searchLocalSongs(`${rec.artist} ${rec.title}`, 10);
        const match = matchRecommendedSong(hits, rec.artist, rec.title);
        if (!match || seen.has(match.id)) continue;
        seen.add(match.id);
        candidates.push({ song: match, recScore: rec.score });
      }

      if (!candidates.length) return [];

      // Step 3 — Song-level affinity: filter out songs the user consistently skips,
      //           and boost songs they've already enjoyed.
      const songAffinities = await getSongAffinities(candidates.map((c) => c.song.id));
      const affinityById = new Map(songAffinities.map((a) => [a.songId, a]));

      const scored = candidates
        .filter(({ song }) => {
          const aff = affinityById.get(song.id);
          // Exclude only if we have enough data AND the user skips it most of the time
          if (!aff || aff.playCount < 3) return true;
          return aff.skipRate < 0.6;
        })
        .map(({ song, recScore }) => {
          const aff = affinityById.get(song.id);
          // Boost songs the user has played fully before; no change for unknowns
          const songBoost = aff && aff.playCount >= 2 ? aff.score * 0.15 : 0;
          return { song, finalScore: Math.min(recScore + songBoost, 1) };
        })
        .sort((a, b) => b.finalScore - a.finalScore);

      return scored.slice(0, limit).map((s) => s.song);
    },
    SHORT_CACHE_TTL_MS
  );
}
