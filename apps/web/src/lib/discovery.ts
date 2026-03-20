/**
 * @deprecated
 * This file is a backward-compatibility shim. Import from `$lib/data` instead.
 */

export {
  getArtistArtwork,
  getArtistArtworkMap,
  getTopArtists,
  searchArtists,
  getTopSongs,
  searchMetadataSongs,
  getArtistInfo,
  getArtistTopTracks,
  getTopTags,
} from '$lib/data/metadata';

export {
  getRecommendations,
  getTrackTopGenre,
  getUpNextSongs,
} from '$lib/data/recommendations';

export function clearDiscoveryCaches(): void {
  // Import inline to avoid circular reference at module evaluation time
  import('$lib/data/metadata').then(({ clearMetadataCaches }) => clearMetadataCaches());
  import('$lib/data/recommendations').then(({ clearRecommendationCaches }) => clearRecommendationCaches());
}

// Type aliases for backward compatibility
export type { UnifiedArtist as DiscoveryArtist } from '$lib/data/types';
export type { UnifiedSong as DiscoverySong } from '$lib/data/types';
export type { UnifiedArtistInfo as DiscoveryArtistInfo } from '$lib/data/types';
export type { UnifiedRecommendation as DiscoveryRecommendation } from '$lib/data/types';
