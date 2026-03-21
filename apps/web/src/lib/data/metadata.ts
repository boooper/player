import { fetchAudioDbArtist, fetchAudioDbArtistPhoto } from '$lib/providers/metadata/audiodb';
import {
  fetchTopArtists as lfmTopArtists,
  searchArtists as lfmSearchArtists,
  fetchTopSongs as lfmTopSongs,
  searchSongs as lfmSearchSongs,
  fetchArtistInfo as lfmArtistInfo,
  fetchArtistTopTracks as lfmArtistTopTracks,
  fetchTopTags as lfmTopTags,
} from '$lib/providers/metadata/lastfm';
import { fetchArtistAlbums, searchSongs as searchLibrarySongs, type Album as LibraryAlbum, type Song as LibrarySong } from '$lib/servers';
import {
  getLastFmApiKey,
  getMetadataProviderSetting,
} from '$lib/stores/backend-settings';
import type { MetadataSource, UnifiedArtist, UnifiedArtistInfo, UnifiedSong } from './types';

// ── Cache ─────────────────────────────────────────────────────────────────────

const artworkCache = new Map<string, Promise<string>>();
const metadataCache = new Map<string, { expiresAt: number; promise: Promise<unknown> }>();
const SHORT_TTL = 30_000;
const DEFAULT_TTL = 5 * 60_000;

function cached<T>(key: string, loader: () => Promise<T>, ttlMs = DEFAULT_TTL): Promise<T> {
  const now = Date.now();
  const entry = metadataCache.get(key);
  if (entry && entry.expiresAt > now) return entry.promise as Promise<T>;
  const promise = loader().catch((err) => {
    if (metadataCache.get(key)?.promise === promise) metadataCache.delete(key);
    throw err;
  });
  metadataCache.set(key, { expiresAt: now + ttlMs, promise });
  return promise;
}

export function clearMetadataCaches(): void {
  metadataCache.clear();
  artworkCache.clear();
}

// ── Internal helpers ──────────────────────────────────────────────────────────

function getProvider(): MetadataSource {
  const v = getMetadataProviderSetting().trim().toLowerCase();
  if (v === 'local' || v === 'lastfm' || v === 'audiodb' || v === 'both') return v;
  return 'both';
}

function getLfmKey(): string { return getLastFmApiKey(); }
function hasLfm(): boolean { return Boolean(getLfmKey()); }

function norm(value: string): string { return value.trim().toLowerCase(); }

function uniqueBy<T>(items: T[], key: (item: T) => string): T[] {
  const seen = new Set<string>();
  return items.filter((item) => {
    const k = key(item);
    if (seen.has(k)) return false;
    seen.add(k);
    return true;
  });
}

async function hydrateImages<T extends { imageUrl: string }>(
  items: T[],
  getName: (item: T) => string
): Promise<T[]> {
  return Promise.all(
    items.map(async (item) => {
      const imageUrl = await getArtistArtwork(getName(item), item.imageUrl);
      return imageUrl === item.imageUrl ? item : { ...item, imageUrl };
    })
  );
}

async function searchLocalSongs(query: string, limit = 12): Promise<LibrarySong[]> {
  return searchLibrarySongs(query, Math.max(limit, 24)).catch(() => []);
}

async function searchLocalArtists(query: string, limit = 12): Promise<UnifiedArtist[]> {
  const songs = await searchLocalSongs(query, Math.max(limit * 4, 24));
  const filtered = songs.filter((s) => norm(s.artist).includes(norm(query)));
  return uniqueBy(filtered, (s) => norm(s.artist))
    .slice(0, limit)
    .map((s) => ({
      id: `local-artist-${encodeURIComponent(s.artist)}`,
      name: s.artist,
      imageUrl: s.coverArtUrl || '',
      source: 'local' as const,
    }));
}

async function getLocalArtistAlbums(artist: string): Promise<LibraryAlbum[]> {
  return cached(
    `local-albums:${norm(artist)}`,
    () => fetchArtistAlbums(artist, 24).catch(() => [])
  );
}

async function getLocalArtistSongs(artist: string, limit = 30): Promise<LibrarySong[]> {
  return cached(
    `local-songs:${norm(artist)}:${limit}`,
    async () => {
      const songs = await searchLocalSongs(artist, Math.max(limit * 2, 30));
      return songs.filter((s) => norm(s.artist) === norm(artist)).slice(0, limit);
    }
  );
}

async function getLocalArtistInfo(artist: string): Promise<UnifiedArtistInfo | null> {
  const [albums, songs] = await Promise.all([
    getLocalArtistAlbums(artist),
    getLocalArtistSongs(artist, 24),
  ]);
  if (!albums.length && !songs.length) return null;

  const imageUrl =
    albums.find((a) => a.coverArtUrl)?.coverArtUrl ||
    songs.find((s) => s.coverArtUrl)?.coverArtUrl ||
    '';
  const tags = uniqueBy(
    songs.map((s) => s.album).filter(Boolean).map((v) => ({ v })),
    (item) => norm(item.v)
  )
    .slice(0, 5)
    .map((item) => item.v);

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
    formedYear: '',
    source: 'local',
  };
}

async function getLocalArtistTopTracks(artist: string, limit = 10): Promise<UnifiedSong[]> {
  const songs = await getLocalArtistSongs(artist, limit * 3);
  return songs.slice(0, limit).map((s) => ({
    id: `local-track-${s.id}`,
    title: s.title,
    artist: s.artist,
    imageUrl: s.coverArtUrl || '',
    source: 'local' as const,
  }));
}

async function resolveLocalArtwork(artist: string): Promise<string> {
  const [albums, songs] = await Promise.all([
    getLocalArtistAlbums(artist).catch(() => [] as LibraryAlbum[]),
    getLocalArtistSongs(artist, 12).catch(() => [] as LibrarySong[]),
  ]);
  return (
    albums.find((a) => a.coverArtUrl)?.coverArtUrl ||
    songs.find((s) => s.coverArtUrl)?.coverArtUrl ||
    ''
  );
}

async function resolveMetadataArtwork(artist: string): Promise<string> {
  const provider = getProvider();
  if (provider === 'local') return '';
  if (provider === 'lastfm') {
    if (!hasLfm()) return '';
    return (await lfmArtistInfo({ artist }))?.imageUrl || '';
  }
  if (provider === 'audiodb') return fetchAudioDbArtistPhoto(artist);

  const [lfmRes, adbRes] = await Promise.allSettled([
    hasLfm() ? lfmArtistInfo({ artist }) : Promise.resolve(null),
    fetchAudioDbArtist(artist),
  ]);
  const lfmImage = lfmRes.status === 'fulfilled' ? lfmRes.value?.imageUrl || '' : '';
  const adb = adbRes.status === 'fulfilled' ? adbRes.value : null;
  const adbImage = adb ? adb.thumb || adb.fanart || adb.banner || '' : '';
  return lfmImage || adbImage;
}

// ── Public API ────────────────────────────────────────────────────────────────

export async function getArtistArtwork(artist: string, currentImageUrl = ''): Promise<string> {
  const key = norm(artist);
  if (!key) return currentImageUrl;
  if (!artworkCache.has(key)) {
    artworkCache.set(
      key,
      (async () => {
        if (currentImageUrl) return currentImageUrl;
        const local = await resolveLocalArtwork(artist);
        if (local) return local;
        return resolveMetadataArtwork(artist);
      })()
    );
  }
  return (await artworkCache.get(key)!) || currentImageUrl;
}

export async function getArtistArtworkMap(artists: string[]): Promise<Record<string, string>> {
  const unique = [...new Set(artists.map((a) => a.trim()).filter(Boolean))];
  const entries = await Promise.all(
    unique.map(async (a) => [a, await getArtistArtwork(a)] as const)
  );
  return Object.fromEntries(entries);
}

export async function getTopArtists(limit = 24): Promise<UnifiedArtist[]> {
  const provider = getProvider();
  if (provider === 'local' || provider === 'audiodb' || !hasLfm()) return [];
  return cached(
    `top-artists:${provider}:${limit}`,
    async () =>
      hydrateImages(
        (await lfmTopArtists({ limit })).map((a) => ({
          ...a,
          source: 'lastfm' as const,
        })),
        (a) => a.name
      ),
    SHORT_TTL
  );
}

export async function searchArtists(query: string, limit = 12): Promise<UnifiedArtist[]> {
  const provider = getProvider();
  if (provider === 'local') return searchLocalArtists(query, limit);
  if (provider === 'audiodb' || !hasLfm()) return [];

  const remote = await hydrateImages(
    (await lfmSearchArtists({ query, limit })).map((a) => ({
      ...a,
      source: 'lastfm' as const,
    })),
    (a) => a.name
  );
  if (provider !== 'both') return remote;
  const local = await searchLocalArtists(query, limit);
  return uniqueBy([...local, ...remote], (a) => norm(a.name)).slice(0, limit);
}

export async function getTopSongs(limit = 24): Promise<UnifiedSong[]> {
  const provider = getProvider();
  if (provider === 'local' || provider === 'audiodb' || !hasLfm()) return [];
  return cached(
    `top-songs:${provider}:${limit}`,
    async () =>
      hydrateImages(
        (await lfmTopSongs({ limit })).map((s) => ({
          ...s,
          source: 'lastfm' as const,
        })),
        (s) => s.artist
      ),
    SHORT_TTL
  );
}

export async function searchMetadataSongs(query: string, limit = 12): Promise<UnifiedSong[]> {
  const provider = getProvider();
  const localSongs = (await searchLocalSongs(query, limit)).map((s) => ({
    id: `local-song-${s.id}`,
    title: s.title,
    artist: s.artist,
    imageUrl: s.coverArtUrl || '',
    source: 'local' as const,
  }));

  if (provider === 'local') return localSongs.slice(0, limit);
  if (provider === 'audiodb' || !hasLfm()) return [];

  const remote = await hydrateImages(
    (await lfmSearchSongs({ query, limit })).map((s) => ({
      ...s,
      source: 'lastfm' as const,
    })),
    (s) => s.artist
  );
  if (provider !== 'both') return remote;
  return uniqueBy([...localSongs, ...remote], (s) => `${norm(s.artist)}::${norm(s.title)}`).slice(0, limit);
}

export async function getArtistInfo(artist: string): Promise<UnifiedArtistInfo | null> {
  const provider = getProvider();
  return cached(
    `artist-info:${provider}:${norm(artist)}`,
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
          formedYear: adb.formedYear,
          source: 'audiodb' as const,
        };
      }

      if (provider === 'lastfm') {
        if (!hasLfm()) return local;
        const lfm = await lfmArtistInfo({ artist });
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
          formedYear: '',
          source: 'lastfm' as const,
        };
      }

      // 'both'
      const [lfmRes, adbRes] = await Promise.allSettled([
        hasLfm() ? lfmArtistInfo({ artist }) : Promise.resolve(null),
        fetchAudioDbArtist(artist),
      ]);
      const lfm = lfmRes.status === 'fulfilled' ? lfmRes.value : null;
      const adb = adbRes.status === 'fulfilled' ? adbRes.value : null;
      if (!local && !lfm && !adb) return null;

      const imageUrl = await getArtistArtwork(
        artist,
        local?.imageUrl || lfm?.imageUrl || (adb ? adb.thumb || adb.fanart || adb.banner : '') || ''
      );
      const similarArtists = await Promise.all(
        (lfm?.similarArtists ?? []).map(async (item) => ({
          ...item,
          imageUrl: await getArtistArtwork(item.name, item.imageUrl),
        }))
      );

      return {
        name: local?.name || lfm?.name || adb?.name || artist,
        imageUrl,
        listeners: lfm?.listeners ?? local?.listeners ?? 0,
        playcount: lfm?.playcount ?? 0,
        bio: local?.bio || lfm?.bio || adb?.biography || '',
        tags: local?.tags?.length
          ? local.tags
          : lfm?.tags?.length
            ? lfm.tags
            : adb?.genre
              ? [adb.genre]
              : [],
        similarArtists,
        genre: adb?.genre || lfm?.tags?.[0] || '',
        country: adb?.country || '',
        formedYear: adb?.formedYear || '',
        source: 'both' as const,
      };
    }
  );
}

export async function getArtistTopTracks(artist: string, limit = 10): Promise<UnifiedSong[]> {
  const provider = getProvider();
  return cached(
    `artist-top-tracks:${provider}:${norm(artist)}:${limit}`,
    async () => {
      const local = await getLocalArtistTopTracks(artist, limit);
      if (provider === 'local') return local;
      if (!hasLfm() || provider === 'audiodb') return local;
      const remote = (await lfmArtistTopTracks({ artist, limit })).map(
        (s) => ({ ...s, source: 'lastfm' as const })
      );
      if (provider !== 'both') return remote;
      return uniqueBy([...local, ...remote], (s) => `${norm(s.artist)}::${norm(s.title)}`).slice(0, limit);
    }
  );
}

export async function getTopTags(limit = 40): Promise<string[]> {
  const provider = getProvider();
  if (!hasLfm() || provider === 'audiodb' || provider === 'local') return [];
  return lfmTopTags({ limit });
}
