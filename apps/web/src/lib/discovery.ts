/**
 * @deprecated Import from `$lib/data` instead.
 */

export {
  getArtistArtwork, getArtistArtworkMap,
  getTopArtists, searchArtists,
  getTopSongs, searchMetadataSongs,
  getArtistInfo, getArtistTopTracks, getTopTags,
  clearMetadataCaches,
} from '$lib/data/metadata';

export {
  getRecommendations, getTrackTopGenre, getUpNextSongs,
  clearRecommendationCaches,
} from '$lib/data/recommendations';

// Type aliases for backward compatibility
export type { UnifiedArtist as DiscoveryArtist } from '$lib/data/types';
export type { UnifiedSong as DiscoverySong } from '$lib/data/types';
export type { UnifiedArtistInfo as DiscoveryArtistInfo } from '$lib/data/types';
export type { UnifiedRecommendation as DiscoveryRecommendation } from '$lib/data/types';
