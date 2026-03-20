import { fetchLastFmRecommendations, fetchTrackTopGenre as lfmTrackTopGenre } from '$lib/providers/metadata/lastfm';
import { fetchListenBrainzRecommendations } from '$lib/providers/recommendation/listenbrainz';
import { searchSongs as searchLibrarySongs, type Song as LibrarySong } from '$lib/servers';
import { getArtistAffinities, getGenreAffinities, getSongAffinities } from '$lib/servers/play-history';
import {
  getLastFmApiKey,
  getListenBrainzUsername,
  getRecommendationProviderSetting,
} from '$lib/stores/backend-settings';
import type { RecommendationSource, UnifiedRecommendation } from './types';

// ── Cache ─────────────────────────────────────────────────────────────────────

const recCache = new Map<string, { expiresAt: number; promise: Promise<unknown> }>();
const SHORT_TTL = 30_000;

function cached<T>(key: string, loader: () => Promise<T>, ttlMs = SHORT_TTL): Promise<T> {
  const now = Date.now();
  const entry = recCache.get(key);
  if (entry && entry.expiresAt > now) return entry.promise as Promise<T>;
  const promise = loader().catch((err) => {
    if (recCache.get(key)?.promise === promise) recCache.delete(key);
    throw err;
  });
  recCache.set(key, { expiresAt: now + ttlMs, promise });
  return promise;
}

export function clearRecommendationCaches(): void {
  recCache.clear();
}

// ── Internal helpers ──────────────────────────────────────────────────────────

function getProvider(): RecommendationSource {
  const v = getRecommendationProviderSetting().trim().toLowerCase();
  return v === 'listenbrainz' ? 'listenbrainz' : 'lastfm';
}

function getLfmKey(): string { return getLastFmApiKey(); }
function hasLfm(): boolean { return Boolean(getLfmKey()); }

function norm(value: string): string { return value.trim().toLowerCase(); }

function recKey(artist: string, title: string): string {
  return `${norm(artist)}::${norm(title)}`;
}

function matchSong(candidates: LibrarySong[], artist: string, title: string): LibrarySong | null {
  const key = recKey(artist, title);
  const exact = candidates.find((s) => recKey(s.artist, s.title) === key);
  if (exact) return exact;
  const byArtist = candidates.filter((s) => norm(s.artist) === norm(artist));
  return (
    byArtist.find(
      (s) =>
        norm(s.title).includes(norm(title)) || norm(title).includes(norm(s.title))
    ) ?? null
  );
}

async function searchLocal(query: string, limit = 12): Promise<LibrarySong[]> {
  return searchLibrarySongs(query, Math.max(limit, 24)).catch(() => []);
}

// ── Public API ────────────────────────────────────────────────────────────────

export async function getRecommendations(params: {
  seedArtist: string;
  seedSongTitle: string;
  seedGenre?: string;
  likedArtists?: string[];
  limit?: number;
}): Promise<UnifiedRecommendation[]> {
  const provider = getProvider();
  if (provider === 'listenbrainz') {
    return fetchListenBrainzRecommendations({
      username: getListenBrainzUsername(),
      likedArtists: params.likedArtists,
      limit: params.limit,
    });
  }
  if (!hasLfm()) return [];
  return (await fetchLastFmRecommendations({ apiKey: getLfmKey(), ...params })).map((r) => ({
    ...r,
    source: 'lastfm' as const,
  }));
}

export async function getTrackTopGenre(artist: string, track: string): Promise<string> {
  if (getProvider() !== 'lastfm' || !hasLfm()) return '';
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

  const likedKey = (params.likedArtists ?? []).map(norm).sort().join('|');

  return cached(
    `up-next:${getProvider()}:${norm(artist)}:${norm(title)}:${limit}:${likedKey}`,
    async () => {
      const recommendations = await getRecommendations({
        seedArtist: artist,
        seedSongTitle: title,
        likedArtists: params.likedArtists,
        limit: limit * 4,
      }).catch(() => [] as UnifiedRecommendation[]);

      if (!recommendations.length) return [];

      // Step 1 — Re-rank by artist + genre affinity
      const uniqueArtists = [...new Set(recommendations.map((r) => r.artist.toLowerCase()))];
      const uniqueGenres = [
        ...new Set(recommendations.flatMap((r) => (r.genreScore > 0 ? [norm(r.artist)] : []))),
      ];
      const [artistAffinities, genreAffinities] = await Promise.all([
        getArtistAffinities(uniqueArtists),
        uniqueGenres.length
          ? getGenreAffinities(uniqueGenres)
          : Promise.resolve({} as Record<string, number>),
      ]);

      const reranked = recommendations
        .map((rec) => {
          const artistBoost = (artistAffinities[rec.artist.toLowerCase()] ?? 0) * 0.35;
          const genreBoost = (genreAffinities[norm(rec.artist)] ?? 0) * 0.1;
          return { ...rec, score: Math.min(rec.score * 0.55 + artistBoost + genreBoost, 1) };
        })
        .sort((a, b) => b.score - a.score);

      // Step 2 — Find local library matches
      const candidates: { song: LibrarySong; recScore: number }[] = [];
      const seen = new Set<string>();

      for (const rec of reranked) {
        if (candidates.length >= limit * 3) break;
        const hits = await searchLocal(`${rec.artist} ${rec.title}`, 10);
        const match = matchSong(hits, rec.artist, rec.title);
        if (!match || seen.has(match.id)) continue;
        seen.add(match.id);
        candidates.push({ song: match, recScore: rec.score });
      }

      if (!candidates.length) return [];

      // Step 3 — Filter and boost by song-level affinity
      const songAffinities = await getSongAffinities(candidates.map((c) => c.song.id));
      const affinityById = new Map(songAffinities.map((a) => [a.songId, a]));

      return candidates
        .filter(({ song }) => {
          const aff = affinityById.get(song.id);
          if (!aff || aff.playCount < 3) return true;
          return aff.skipRate < 0.6;
        })
        .map(({ song, recScore }) => {
          const aff = affinityById.get(song.id);
          const boost = aff && aff.playCount >= 2 ? aff.score * 0.15 : 0;
          return { song, finalScore: Math.min(recScore + boost, 1) };
        })
        .sort((a, b) => b.finalScore - a.finalScore)
        .slice(0, limit)
        .map((s) => s.song);
    }
  );
}
