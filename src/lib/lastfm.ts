const LASTFM_API_URL = 'https://ws.audioscrobbler.com/2.0/';

export type LastFmTrackRecommendation = {
  id: string;
  title: string;
  artist: string;
  source: 'lastfm';
  score: number;
  matchScore: number;
  artistLiked: boolean;
  genreScore: number;
  url: string;
};

export type LastFmArtist = {
  id: string;
  name: string;
  imageUrl: string;
  listeners?: number;
  url?: string;
  source: 'top' | 'search';
};

export type LastFmSong = {
  id: string;
  title: string;
  artist: string;
  imageUrl: string;
  listeners?: number;
  url?: string;
  source: 'top' | 'search';
};

type LastFmImage = {
  '#text'?: string;
  size?: string;
};

const PLACEHOLDER_IMAGE_MARKERS = [
  '2a96cbd8b46e442fc41c2b86b821562f',
  'default_album',
  'default_artist',
  '/noimage/'
];

function normalize(value: string): string {
  return value.trim().toLowerCase();
}

function trackKey(artist: string, title: string): string {
  return `${normalize(artist)}::${normalize(title)}`;
}

function pickBestImage(images: LastFmImage[] | undefined): string {
  if (!images?.length) return '';
  const preferred = ['extralarge', 'large', 'medium', 'small'];
  for (const size of preferred) {
    const found = images.find((img) => img?.size === size && img['#text']);
    if (found?.['#text'] && !isPlaceholderImage(found['#text'])) return found['#text'];
  }
  const firstReal = images.find((img) => img?.['#text'] && !isPlaceholderImage(img['#text']));
  return firstReal?.['#text'] ?? '';
}

function isPlaceholderImage(url: string): boolean {
  const normalized = url.toLowerCase();
  return PLACEHOLDER_IMAGE_MARKERS.some((marker) => normalized.includes(marker));
}

async function callLastFm(
  method: string,
  params: Record<string, string | number>,
  apiKey: string
): Promise<any> {
  const url = new URL(LASTFM_API_URL);
  url.searchParams.set('method', method);
  url.searchParams.set('api_key', apiKey);
  url.searchParams.set('format', 'json');

  Object.entries(params).forEach(([key, value]) => {
    url.searchParams.set(key, String(value));
  });

  const response = await fetch(url);
  if (!response.ok) {
    throw new Error(`Last.fm request failed with status ${response.status}`);
  }

  const json = await response.json();
  if (json.error) {
    throw new Error(json.message || 'Last.fm returned an error');
  }

  return json;
}

export async function fetchLastFmRecommendations({
  apiKey,
  seedArtist,
  seedSongTitle,
  seedGenre,
  likedArtists = [],
  limit = 12
}: {
  apiKey: string;
  seedArtist: string;
  seedSongTitle: string;
  seedGenre?: string;
  likedArtists?: string[];
  limit?: number;
}): Promise<LastFmTrackRecommendation[]> {
  if (!apiKey) {
    throw new Error('Missing Last.fm API key');
  }

  const liked = new Set(likedArtists.map(normalize));
  const normalizedGenre = normalize(seedGenre ?? '');

  const [similarResult, genreResult] = await Promise.allSettled([
    callLastFm(
      'track.getSimilar',
      {
        artist: seedArtist,
        track: seedSongTitle,
        autocorrect: 1,
        limit: Math.max(limit * 3, 36)
      },
      apiKey
    ),
    normalizedGenre
      ? callLastFm('tag.gettoptracks', { tag: normalizedGenre, limit: 100 }, apiKey)
      : Promise.resolve({})
  ]);

  const similarPayload = similarResult.status === 'fulfilled' ? similarResult.value : {};
  const genrePayload = genreResult.status === 'fulfilled' ? genreResult.value : {};

  const similar = Array.isArray(similarPayload?.similartracks?.track) ? similarPayload.similartracks.track : [];
  const genreTracks = Array.isArray((genrePayload as any)?.tracks?.track) ? (genrePayload as any).tracks.track : [];
  const genreRank = new Map<string, number>();

  for (let index = 0; index < genreTracks.length; index += 1) {
    const track = genreTracks[index];
    const title = String(track?.name || '').trim();
    const artist = String(track?.artist?.name || track?.artist || '').trim();
    if (!title || !artist) continue;
    const key = trackKey(artist, title);
    if (!genreRank.has(key)) {
      genreRank.set(key, index + 1);
    }
  }

  const seen = new Set([trackKey(seedArtist, seedSongTitle)]);

  const ranked: LastFmTrackRecommendation[] = [];
  for (const track of similar) {
    const title = String(track?.name || '').trim();
    const artist = String(track?.artist?.name || track?.artist || '').trim();
    if (!title || !artist) continue;

    const dedupeKey = trackKey(artist, title);
    if (seen.has(dedupeKey)) continue;
    seen.add(dedupeKey);

    const baseMatch = Number.parseFloat(track.match ?? '0') || 0;
    const normalizedArtist = normalize(artist);
    const likedBoost = liked.has(normalizedArtist) ? 0.2 : 0;
    const genrePosition = genreRank.get(dedupeKey);
    const genreScore = genrePosition ? Math.max(0, 1 - (genrePosition - 1) / 100) : 0;
    const score = Math.min(baseMatch + likedBoost + genreScore * 0.2, 1);

    ranked.push({
      id: `lfm-${encodeURIComponent(`${artist}-${title}`)}`,
      title,
      artist,
      source: 'lastfm',
      score: Number(score.toFixed(3)),
      matchScore: Number(baseMatch.toFixed(3)),
      artistLiked: likedBoost > 0,
      genreScore: Number(genreScore.toFixed(3)),
      url: String(track.url || '')
    });
  }

  ranked.sort((a, b) => b.score - a.score);
  return ranked.slice(0, limit);
}

export async function fetchTopArtists({
  apiKey,
  limit = 24
}: {
  apiKey: string;
  limit?: number;
}): Promise<LastFmArtist[]> {
  const payload = await callLastFm('chart.gettopartists', { limit }, apiKey);
  const list = Array.isArray(payload?.artists?.artist) ? payload.artists.artist : [];

  const artists = list
    .map((artist: any) => ({
      id: `top-${encodeURIComponent(String(artist?.name || ''))}`,
      name: String(artist?.name || '').trim(),
      imageUrl: pickBestImage(artist?.image),
      listeners: Number.parseInt(String(artist?.listeners ?? '0'), 10) || 0,
      url: String(artist?.url || ''),
      source: 'top' as const
    }))
    .filter((artist: LastFmArtist) => Boolean(artist.name));

  return artists;
}

export async function searchArtists({
  apiKey,
  query,
  limit = 12
}: {
  apiKey: string;
  query: string;
  limit?: number;
}): Promise<LastFmArtist[]> {
  if (!query.trim()) return [];

  const payload = await callLastFm(
    'artist.search',
    {
      artist: query,
      limit
    },
    apiKey
  );

  const raw = payload?.results?.artistmatches?.artist;
  const list = Array.isArray(raw) ? raw : raw ? [raw] : [];

  const artists = list
    .map((artist: any) => ({
      id: `search-${encodeURIComponent(String(artist?.name || ''))}`,
      name: String(artist?.name || '').trim(),
      imageUrl: pickBestImage(artist?.image),
      listeners: Number.parseInt(String(artist?.listeners ?? '0'), 10) || 0,
      url: String(artist?.url || ''),
      source: 'search' as const
    }))
    .filter((artist: LastFmArtist) => Boolean(artist.name));

  return artists;
}

export async function fetchTopSongs({
  apiKey,
  limit = 24
}: {
  apiKey: string;
  limit?: number;
}): Promise<LastFmSong[]> {
  const payload = await callLastFm('chart.gettoptracks', { limit }, apiKey);
  const list = Array.isArray(payload?.tracks?.track) ? payload.tracks.track : [];

  const mapped = list
    .map((track: any) => ({
      id: `top-track-${encodeURIComponent(`${track?.artist?.name || ''}-${track?.name || ''}`)}`,
      title: String(track?.name || '').trim(),
      artist: String(track?.artist?.name || '').trim(),
      imageUrl: pickBestImage(track?.image),
      listeners: Number.parseInt(String(track?.listeners ?? '0'), 10) || 0,
      url: String(track?.url || ''),
      source: 'top' as const
    }))
    .filter((song: LastFmSong) => Boolean(song.title && song.artist));

  return mapped;
}

export async function searchSongs({
  apiKey,
  query,
  limit = 12
}: {
  apiKey: string;
  query: string;
  limit?: number;
}): Promise<LastFmSong[]> {
  if (!query.trim()) return [];

  // Run both a track-title search AND an artist top-tracks lookup in parallel.
  // This way searching "Nirvana" returns Nirvana's songs, not just tracks
  // whose title happens to contain "nirvana".
  const [trackResult, artistResult] = await Promise.allSettled([
    callLastFm('track.search', { track: query, limit }, apiKey),
    callLastFm('artist.getTopTracks', { artist: query, autocorrect: 1, limit }, apiKey)
  ]);

  const seen = new Set<string>();
  const songs: LastFmSong[] = [];

  const addTracks = (list: any[], source: 'top' | 'search', fallbackArtist = '') => {
    for (const track of list) {
      const title = String(track?.name || '').trim();
      const artist = String(track?.artist?.name || track?.artist || fallbackArtist).trim();
      if (!title || !artist) continue;
      const key = trackKey(artist, title);
      if (seen.has(key)) continue;
      seen.add(key);
      songs.push({
        id: `search-track-${encodeURIComponent(`${artist}-${title}`)}`,
        title,
        artist,
        imageUrl: pickBestImage(track?.image),
        listeners: Number.parseInt(String(track?.listeners ?? track?.playcount ?? '0'), 10) || 0,
        url: String(track?.url || ''),
        source
      });
    }
  };

  if (trackResult.status === 'fulfilled') {
    const raw = trackResult.value?.results?.trackmatches?.track;
    addTracks(Array.isArray(raw) ? raw : raw ? [raw] : [], 'search');
  }

  if (artistResult.status === 'fulfilled') {
    const raw = artistResult.value?.toptracks?.track;
    const attrArtist = artistResult.value?.toptracks?.['@attr']?.artist ?? '';
    addTracks(Array.isArray(raw) ? raw : raw ? [raw] : [], 'top', attrArtist);
  }

  const limited = songs.slice(0, limit);
  return limited;
}

export async function fetchTopTags({
  apiKey,
  limit = 40
}: {
  apiKey: string;
  limit?: number;
}): Promise<string[]> {
  const payload = await callLastFm('tag.getTopTags', { limit }, apiKey);
  const list = Array.isArray(payload?.toptags?.tag) ? payload.toptags.tag : [];

  return list
    .map((item: any) => String(item?.name || '').trim())
    .filter((name: string) => Boolean(name));
}

export async function fetchTrackTopGenre({
  apiKey,
  artist,
  track
}: {
  apiKey: string;
  artist: string;
  track: string;
}): Promise<string> {
  if (!artist.trim() || !track.trim()) return '';

  const payload = await callLastFm(
    'track.getTopTags',
    {
      artist,
      track,
      autocorrect: 1
    },
    apiKey
  );

  const tags = Array.isArray(payload?.toptags?.tag) ? payload.toptags.tag : [];
  const top = tags
    .map((item: any) => ({
      name: String(item?.name || '').trim(),
      count: Number.parseInt(String(item?.count ?? '0'), 10) || 0
    }))
    .filter((item: { name: string; count: number }) => Boolean(item.name))
    .sort((a: { count: number }, b: { count: number }) => b.count - a.count)[0];

  return top?.name ?? '';
}

export type LastFmArtistInfo = {
  name: string;
  imageUrl: string;
  listeners: number;
  playcount: number;
  bio: string;
  tags: string[];
  similarArtists: { name: string; imageUrl: string }[];
};

export async function fetchArtistInfo({
  apiKey,
  artist
}: {
  apiKey: string;
  artist: string;
}): Promise<LastFmArtistInfo | null> {
  try {
    const payload = await callLastFm('artist.getinfo', { artist, autocorrect: 1 }, apiKey);
    const a = payload?.artist;
    if (!a) return null;

    const imageUrl = pickBestImage(a.image);
    const tags = Array.isArray(a.tags?.tag)
      ? a.tags.tag.map((t: any) => String(t?.name ?? '')).filter(Boolean).slice(0, 5)
      : [];
    const similarArtists: Array<{ name: string; imageUrl: string }> = Array.isArray(a.similar?.artist)
      ? a.similar.artist
          .map((s: any) => ({
            name: String(s?.name ?? ''),
            imageUrl: pickBestImage(s?.image)
          }))
          .filter((s: { name: string }) => s.name)
          .slice(0, 6)
      : [];

    return {
      name: String(a.name ?? artist),
      imageUrl,
      listeners: Number(a.stats?.listeners ?? 0),
      playcount: Number(a.stats?.playcount ?? 0),
      bio: String(a.bio?.summary ?? '').replace(/<a[^>]*>.*?<\/a>/gi, '').trim(),
      tags,
      similarArtists
    };
  } catch {
    return null;
  }
}

export async function fetchArtistTopTracks({
  apiKey,
  artist,
  limit = 10
}: {
  apiKey: string;
  artist: string;
  limit?: number;
}): Promise<LastFmSong[]> {
  const payload = await callLastFm(
    'artist.getTopTracks',
    { artist, autocorrect: 1, limit },
    apiKey
  );
  const tracks = Array.isArray(payload?.toptracks?.track) ? payload.toptracks.track : [];
  const seen = new Set<string>();
  return tracks
    .map((track: any) => {
      const title = String(track?.name ?? '').trim();
      const artistName = String(track?.artist?.name ?? artist).trim();
      const id = `lfm-top-${artistName}-${title}`;
      if (seen.has(id)) return null;
      seen.add(id);
      return {
        id,
        title,
        artist: artistName,
        imageUrl: pickBestImage(track?.image),
        listeners: Number(track?.listeners ?? 0),
        url: String(track?.url ?? ''),
        source: 'top' as const
      };
    })
    .filter(Boolean) as LastFmSong[];
}

