import { invoke } from './shared';

export type ArtistStat = {
  artist: string;
  coverArtUrl: string | null;
  playCount: number;
  fullPlayCount: number;
  skipCount: number;
  totalListenSecs: number;
  avgCompletion: number;
};

export type GenreStat = {
  genre: string;
  playCount: number;
  fullPlayCount: number;
  skipCount: number;
  avgCompletion: number;
};

export type SongAffinity = {
  songId: string;
  playCount: number;
  fullPlayCount: number;
  skipCount: number;
  totalListenSecs: number;
  /** Fraction of plays that were skipped (< 20 % heard). 0–1. */
  skipRate: number;
  /** Normalised quality score 0–1. Higher = the user likes this song. */
  score: number;
};

export type SongStat = {
  songId: string;
  title: string;
  artist: string;
  album: string | null;
  coverArtUrl: string | null;
  playCount: number;
  fullPlayCount: number;
  skipCount: number;
  totalListenSecs: number;
  avgCompletion: number;
};

export type ListeningProfile = {
  totalPlays: number;
  totalListenSecs: number;
  uniqueArtists: number;
  topSongs: SongStat[];
  topArtists: ArtistStat[];
  topGenres: GenreStat[];
};

/** Record a play event. `completedFraction` is 0–1. Fire-and-forget. */
export function recordPlay(params: {
  songId: string;
  artist: string;
  title: string;
  album?: string | null;
  coverArtUrl?: string | null;
  durationSecs?: number | null;
  completedFraction: number;
}): void {
  invoke('record_play', {
    songId: params.songId,
    artist: params.artist,
    title: params.title,
    album: params.album ?? null,
    coverArtUrl: params.coverArtUrl ?? null,
    durationSecs: params.durationSecs ?? null,
    completedFraction: params.completedFraction,
  }).catch(() => undefined);
}

/** Cache a song's genre for genre-affinity scoring. Fire-and-forget. */
export function cacheSongGenre(songId: string, artist: string, genre: string): void {
  invoke('cache_song_genre', { songId, artist, genre }).catch(() => undefined);
}

/** 0–1 affinity score per artist (keyed by lowercase artist name). */
export async function getArtistAffinities(
  artists: string[]
): Promise<Record<string, number>> {
  if (!artists.length) return {};
  return invoke<Record<string, number>>('get_artist_affinities', { artists }).catch(() => ({}));
}

/** 0–1 affinity score per genre (keyed by lowercase genre name). */
export async function getGenreAffinities(
  genres: string[]
): Promise<Record<string, number>> {
  if (!genres.length) return {};
  return invoke<Record<string, number>>('get_genre_affinities', { genres }).catch(() => ({}));
}

/**
 * Per-song reputation for a list of song IDs.
 * Returns only songs that have at least one recorded play.
 */
export async function getSongAffinities(
  songIds: string[]
): Promise<SongAffinity[]> {
  if (!songIds.length) return [];
  return invoke<SongAffinity[]>('get_song_affinities', { songIds }).catch(() => []);
}

/** User's overall listening profile — useful for a stats/insight page. */
export async function getListeningProfile(): Promise<ListeningProfile> {
  return invoke<ListeningProfile>('get_listening_profile').catch(() => ({
    totalPlays: 0,
    totalListenSecs: 0,
    uniqueArtists: 0,
    topSongs: [],
    topArtists: [],
    topGenres: [],
  }));
}
