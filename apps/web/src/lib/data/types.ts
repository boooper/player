// ── Unified data types ────────────────────────────────────────────────────────
// All provider-specific shapes (LastFmArtist, AudioDbArtist, etc.) are adapted
// to these types before leaving the data layer. Consumers only ever see these.

export type MetadataSource = 'local' | 'lastfm' | 'audiodb' | 'both';
export type RecommendationSource = 'lastfm' | 'listenbrainz';

/** A discoverable artist from any metadata provider. */
export type UnifiedArtist = {
  id: string;
  name: string;
  imageUrl: string;
  listeners?: number;
  url?: string;
  source: MetadataSource;
};

/** A discoverable song/track from any metadata provider. */
export type UnifiedSong = {
  id: string;
  title: string;
  artist: string;
  imageUrl: string;
  listeners?: number;
  url?: string;
  source: MetadataSource;
};

/** Full artist detail, merged from one or more providers. */
export type UnifiedArtistInfo = {
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
  source: MetadataSource;
};

/** A track recommendation from any recommendation provider. */
export type UnifiedRecommendation = {
  id: string;
  title: string;
  artist: string;
  score: number;
  matchScore: number;
  artistLiked: boolean;
  genreScore: number;
  url: string;
  source: RecommendationSource;
};

// Re-export LyricsResult from servers so consumers have a single import point.
export type { LyricsResult } from '$lib/servers/types';
