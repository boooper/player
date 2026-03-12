/**
 * Unified recommendation facade.
 *
 * RECOMMENDATION_PROVIDER setting (stored in DB via /api/settings) controls where
 * track recommendations come from:
 *   'lastfm'  — Default. Uses Last.fm track.getSimilar (requires LASTFM_API_KEY setting)
 *   (future)  — 'spotify', 'deezer', etc. — implement the RecommendationProvider
 *               interface below, add it to RECOMMENDATION_PROVIDERS, and update the setting.
 *
 * All app code should import from this file, not from lastfm.ts or other providers directly.
 */

import {
  fetchLastFmRecommendations,
  fetchTrackTopGenre as lfmTrackTopGenre
} from '$lib/lastfm';
import { getLastFmApiKey, getRecommendationProviderSetting } from '$lib/stores/settings';

// ---------------------------------------------------------------------------
// Config helpers
// ---------------------------------------------------------------------------

function getLfmKey(): string {
  return getLastFmApiKey();
}

function hasLfm(): boolean {
  return Boolean(getLfmKey());
}

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export type TrackRecommendation = {
  id: string;
  title: string;
  artist: string;
  score: number;
  matchScore: number;
  artistLiked: boolean;
  genreScore: number;
  url: string;
};

/**
 * Interface every recommendation provider must implement.
 * To add a new provider (Spotify, Deezer, etc.):
 *   1. Create src/lib/<name>.ts with the API logic.
 *   2. Implement this interface and add it to RECOMMENDATION_PROVIDERS below.
 *   3. Set RECOMMENDATION_PROVIDER=<name> via PUT /api/settings.
 */
export type RecommendationProvider = {
  readonly name: string;
  getRecommendations(params: {
    seedArtist: string;
    seedSongTitle: string;
    seedGenre?: string;
    likedArtists?: string[];
    limit?: number;
  }): Promise<TrackRecommendation[]>;
  getTrackTopGenre(artist: string, track: string): Promise<string>;
};

// ---------------------------------------------------------------------------
// Provider implementations
// ---------------------------------------------------------------------------

const lastfmProvider: RecommendationProvider = {
  name: 'lastfm',
  async getRecommendations(params) {
    if (!hasLfm()) return [];
    return fetchLastFmRecommendations({ apiKey: getLfmKey(), ...params });
  },
  async getTrackTopGenre(artist, track) {
    if (!hasLfm()) return '';
    return lfmTrackTopGenre({ apiKey: getLfmKey(), artist, track });
  }
};

/**
 * Registry of all available recommendation providers.
 * Add new providers here once their module is implemented.
 */
const RECOMMENDATION_PROVIDERS: Record<string, RecommendationProvider> = {
  lastfm: lastfmProvider
  // spotify: spotifyProvider,
  // deezer: deezerProvider,
};

function getRecommendationProvider(): RecommendationProvider {
  const key = getRecommendationProviderSetting().trim().toLowerCase() || 'lastfm';
  return RECOMMENDATION_PROVIDERS[key] ?? lastfmProvider;
}

// ---------------------------------------------------------------------------
// Public functions
// ---------------------------------------------------------------------------

export async function getRecommendations(params: {
  seedArtist: string;
  seedSongTitle: string;
  seedGenre?: string;
  likedArtists?: string[];
  limit?: number;
}): Promise<TrackRecommendation[]> {
  return getRecommendationProvider().getRecommendations(params);
}

export async function getTrackTopGenre(artist: string, track: string): Promise<string> {
  return getRecommendationProvider().getTrackTopGenre(artist, track);
}
