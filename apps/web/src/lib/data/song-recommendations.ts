/**
 * Enriched song recommendations seeded from a single track.
 *
 * Three signal sources, merged and re-ranked:
 *   1. Subsonic "similar songs"  — library-native acoustic/catalog similarity
 *   2. LastFM / ListenBrainz     — internet taste graph, already affinity-ranked
 *   3. Liked-artist fill         — ensures artists you explicitly liked surface
 *
 * Final ranking formula per candidate:
 *   base × 0.5  +  artistAffinity × 0.35  +  songAffinity × 0.15  +  likedBoost
 *   capped at 1.0 — songs with skipRate ≥ 0.6 (and ≥ 3 plays) are dropped.
 */

import { fetchSimilarSongs, fetchLikedArtists, lfmUserTaste } from '$lib/servers';
import { getArtistAffinities, getSongAffinities } from '$lib/servers/play-history';
import { getUpNextSongs } from './recommendations';
import { normalizeString as norm, ttlCache, uniqueBy } from '$lib/utils';
import type { Song } from '$lib/servers';

// ── Cache ─────────────────────────────────────────────────────────────────────

const recCache = ttlCache();
const TTL_MS = 30_000;

function cached<T>(key: string, loader: () => Promise<T>): Promise<T> {
  return recCache.get(key, loader, TTL_MS);
}

export function clearSongRecommendationCache(): void {
  recCache.clear();
}

// ── Types ─────────────────────────────────────────────────────────────────────

export type SongSeed = {
  id: string;
  title: string;
  artist: string;
};

// ── Public API ────────────────────────────────────────────────────────────────

/**
 * Returns up to `limit` library songs most relevant to the given seed track,
 * enriched with the user's liked artists, play history, and taste graph.
 *
 * Safe to call repeatedly — results are cached for 30 s per (artist, title, limit).
 */
export function getEnrichedSongRecommendations(params: {
  seed: SongSeed;
  limit?: number;
}): Promise<Song[]> {
  const { seed, limit = 10 } = params;
  const key = `enriched:${norm(seed.artist)}:${norm(seed.title)}:${limit}`;
  return cached(key, () => fetchEnrichedRecs(seed, limit));
}

// ── Implementation ────────────────────────────────────────────────────────────

async function fetchEnrichedRecs(seed: SongSeed, limit: number): Promise<Song[]> {
  // ── Signal 1 & context: Subsonic similarity + liked artists + taste ──────
  const [subsonicSongs, likedStored, taste] = await Promise.all([
    fetchSimilarSongs(seed.id, limit * 3).catch((): Song[] => []),
    fetchLikedArtists().catch(() => []),
    lfmUserTaste().catch((): string[] => []),
  ]);

  const likedArtistNames = likedStored.map((a) => a.name);
  const allLiked = [...new Set([...likedArtistNames, ...taste])];
  const likedSet = new Set(allLiked.map(norm));

  // ── Signal 2: taste-graph recs → matched to local library + affinity ─────
  const tasteSongs = await getUpNextSongs({
    artist: seed.artist,
    title: seed.title,
    likedArtists: allLiked,
    limit: limit * 2,
  }).catch((): Song[] => []);

  // ── Merge pools — Subsonic first so equal-score songs keep library order ──
  const pool = uniqueBy([...subsonicSongs, ...tasteSongs], (s) => s.id);

  if (!pool.length) return [];

  // ── Re-rank merged pool ───────────────────────────────────────────────────
  const uniqueArtists = [...new Set(pool.map((s) => s.artist.toLowerCase()))];

  const [artistAffinities, songAffinities] = await Promise.all([
    getArtistAffinities(uniqueArtists),
    getSongAffinities(pool.map((s) => s.id)),
  ]);

  const affinityById = new Map(songAffinities.map((a) => [a.songId, a]));

  // Position-based base score for taste songs: rank 0 → 0.9, rank N → ≥ 0.3
  const tasteRankMap = new Map(tasteSongs.map((s, i) => [s.id, i]));
  const tasteCount = Math.max(tasteSongs.length - 1, 1);

  const scored = pool
    .map((song) => {
      const songAff = affinityById.get(song.id);

      // Drop songs the user consistently skips
      if (songAff && songAff.playCount >= 3 && songAff.skipRate >= 0.6) return null;

      const rank = tasteRankMap.get(song.id);
      const base = rank !== undefined
        ? Math.max(0.3, 0.9 - rank * (0.6 / tasteCount)) // taste: position-ranked
        : 0.6;                                             // subsonic: trusted-similar

      const artistBoost = (artistAffinities[song.artist.toLowerCase()] ?? 0) * 0.35;
      const songBoost = songAff && songAff.playCount >= 2 ? songAff.score * 0.15 : 0;
      const likedBoost = likedSet.has(norm(song.artist)) ? 0.3 : 0;

      return { song, score: Math.min(base + artistBoost + songBoost + likedBoost, 1) };
    })
    .filter((x): x is { song: Song; score: number } => x !== null)
    .sort((a, b) => b.score - a.score);

  return scored.slice(0, limit).map((x) => x.song);
}

/**
 * Returns a ranked list of artist names relevant to the user's taste,
 * discovered by seeding `getEnrichedSongRecommendations` from their top-played songs.
 *
 * Artists that appear across multiple seed results rank highest — they are
 * broadly similar to everything the user listens to, not just one song.
 *
 * Use this to enrich album carousels on the home page with artists the user
 * hasn't necessarily played directly but would enjoy.
 */
export async function getPersonalisedArtists(params: {
  /** Top-played songs to seed from — only first 3 are used to keep it fast. */
  topSongs: SongSeed[];
  limit?: number;
}): Promise<string[]> {
  const { topSongs, limit = 12 } = params;
  const seeds = topSongs.slice(0, 3);
  if (!seeds.length) return [];

  const key = `personalised-artists:${seeds.map((s) => norm(s.artist)).join('|')}:${limit}`;
  return cached(key, async () => {
    const results = await Promise.allSettled(
      seeds.map((seed) => getEnrichedSongRecommendations({ seed, limit: 10 }))
    );

    // Count how many seed results each artist appears in.
    const artistCounts = new Map<string, { name: string; count: number }>();
    for (const result of results) {
      if (result.status !== 'fulfilled') continue;
      // One count per unique artist per seed (not per song) to avoid one seed dominating.
      const seedArtists = new Set(result.value.map((s) => norm(s.artist)));
      for (const normName of seedArtists) {
        const canonical = result.value.find((s) => norm(s.artist) === normName)!.artist;
        const entry = artistCounts.get(normName);
        if (entry) {
          entry.count++;
        } else {
          artistCounts.set(normName, { name: canonical, count: 1 });
        }
      }
    }

    return [...artistCounts.values()]
      .sort((a, b) => b.count - a.count)
      .slice(0, limit)
      .map((e) => e.name);
  });
}
