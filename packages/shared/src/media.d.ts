/**
 * Canonical media types shared across all music providers (Subsonic, Jellyfin, etc.).
 * Add new provider support by mapping provider responses into these types.
 */

export declare type Song = {
  id: string;
  title: string;
  artist: string;
  album: string;
  albumId: string;
  /** Provider-internal cover art identifier (opaque string). */
  coverArt: string;
  /** Fully-resolved URL ready for display. */
  coverArtUrl: string;
  streamUrl: string;
  duration: number;
};

export declare type Album = {
  id: string;
  name: string;
  artist: string;
  artistId: string;
  coverArt: string;
  coverArtUrl: string;
  songCount: number;
  duration: number;
  year?: number;
};

export declare type Playlist = {
  id: string;
  name: string;
  songCount: number;
  duration: number;
  coverArt: string;
  coverArtUrl: string;
};

export declare type AlbumDetail = {
  album: Album & { genre?: string };
  songs: Song[];
};

export declare type PlaylistDetail = {
  playlist: {
    id: string;
    name: string;
    songCount: number;
    duration: number;
    coverArtUrl: string;
  };
  songs: Song[];
};
