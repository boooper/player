import { invoke } from '@tauri-apps/api/core';
import type { ServiceStatus } from '@player/shared/contracts';

// ── Exported types (unchanged so all callers stay compatible) ─────────────────

export type StoredLikedArtist = {
  id: number;
  name: string;
  source: string | null;
  externalId: string | null;
  createdAt: string;
  updatedAt: string;
};

export type SubsonicSong = {
  id: string;
  title: string;
  artist: string;
  album: string;
  albumId: string;
  coverArt: string;
  coverArtUrl: string;
  streamUrl: string;
  duration: number;
};

export type SubsonicAlbum = {
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

export type SubsonicPlaylist = {
  id: string;
  name: string;
  songCount: number;
  duration: number;
  coverArt: string;
  coverArtUrl: string;
};

export type AppSettingsPayload = {
  lastFmApiKey: string;
  lastFmSharedSecretConfigured: boolean;
  recommendationProvider: string;
  metadataProvider: string;
  volume: number;
  shuffleEnabled: boolean;
  smartShuffleMode: boolean;
  repeatMode: 'off' | 'all' | 'one';
};

export type ProfilePayload = {
  id: number;
  name: string;
  url: string;
  username: string;
  usePasswordAuth: boolean;
  isActive: boolean;
};

export type ProfileDraftPayload = {
  name: string;
  url: string;
  username: string;
  password?: string;
  usePasswordAuth: boolean;
};

export type LibraryStatsPayload = {
  likedArtists: number;
  playlistCount: number | null;
  totalPlaylistSongs: number | null;
  starredSongs: number | null;
  lastFmConfigured: boolean;
};

export type SubsonicPlaylistDetail = {
  playlist: { id: string; name: string; songCount: number; duration: number; coverArtUrl: string };
  songs: SubsonicSong[];
};

export type SubsonicAlbumDetail = {
  album: SubsonicAlbum & { genre?: string };
  songs: SubsonicSong[];
};

export type LyricsResult = {
  plainLyrics: string | null;
  syncedLyrics: string | null;
  instrumental: boolean;
};

// ── Settings ──────────────────────────────────────────────────────────────────

export async function fetchAppSettings(): Promise<AppSettingsPayload> {
  const s = await invoke<Record<string, string>>('get_settings');
  const rawVol = parseFloat(s.VOLUME ?? '');
  const rawRepeat = s.REPEAT;
  return {
    lastFmApiKey: s.LASTFM_API_KEY ?? '',
    lastFmSharedSecretConfigured: Boolean(s.LASTFM_SHARED_SECRET),
    recommendationProvider: s.RECOMMENDATION_PROVIDER || 'lastfm',
    metadataProvider: s.METADATA_PROVIDER || 'both',
    volume: isNaN(rawVol) ? 0.8 : Math.max(0, Math.min(1, rawVol)),
    shuffleEnabled: s.SHUFFLE === 'true',
    smartShuffleMode: s.SMART_SHUFFLE === 'true',
    repeatMode: (rawRepeat === 'all' || rawRepeat === 'one') ? rawRepeat : 'off'
  };
}

export async function saveVolume(value: number): Promise<void> {
  await invoke('update_settings', { updates: { VOLUME: String(value) } });
}

export async function savePlaybackPrefs(shuffle: boolean, smartShuffle: boolean, repeat: string): Promise<void> {
  await invoke('update_settings', { updates: { SHUFFLE: String(shuffle), SMART_SHUFFLE: String(smartShuffle), REPEAT: repeat } });
}

export async function updateAppSettings(data: {
  lastFmApiKey: string;
  recommendationProvider: string;
  metadataProvider: string;
  lastFmSharedSecret?: string;
}): Promise<void> {
  const updates: Record<string, string> = {
    LASTFM_API_KEY: data.lastFmApiKey,
    RECOMMENDATION_PROVIDER: data.recommendationProvider,
    METADATA_PROVIDER: data.metadataProvider
  };
  if (data.lastFmSharedSecret?.trim()) {
    updates.LASTFM_SHARED_SECRET = data.lastFmSharedSecret.trim();
  }
  await invoke('update_settings', { updates });
}

// ── Profiles ──────────────────────────────────────────────────────────────────

export async function fetchProfiles(): Promise<ProfilePayload[]> {
  return invoke<ProfilePayload[]>('get_profiles');
}

export async function createProfile(data: ProfileDraftPayload): Promise<ProfilePayload> {
  return invoke<ProfilePayload>('create_profile', { data });
}

export async function updateProfile(id: number, data: ProfileDraftPayload): Promise<ProfilePayload> {
  return invoke<ProfilePayload>('update_profile', { id, data });
}

export async function deleteProfile(id: number): Promise<void> {
  await invoke('delete_profile', { id });
}

export async function activateProfile(id: number): Promise<void> {
  await invoke('activate_profile', { id });
}

// ── Stats & Health ────────────────────────────────────────────────────────────

export async function fetchServiceHealth(): Promise<{ subsonic: ServiceStatus; lastfm: ServiceStatus }> {
  return invoke('get_service_health');
}

export async function fetchLibraryStats(): Promise<LibraryStatsPayload> {
  return invoke('get_library_stats');
}

export async function fetchLastFmStatus(): Promise<{ connected: boolean; username: string }> {
  return invoke('lfm_status');
}

// ── Liked Artists ─────────────────────────────────────────────────────────────

export async function fetchLikedArtists(): Promise<StoredLikedArtist[]> {
  return invoke<StoredLikedArtist[]>('get_liked_artists');
}

export async function saveLikedArtist(name: string): Promise<StoredLikedArtist> {
  return invoke<StoredLikedArtist>('save_liked_artist', { name, source: 'lastfm', externalId: null });
}

export async function removeLikedArtist(name: string): Promise<void> {
  await invoke('remove_liked_artist', { name });
}

// ── Subsonic ──────────────────────────────────────────────────────────────────

export async function searchSubsonicSongs(query: string, count = 20): Promise<SubsonicSong[]> {
  return invoke<SubsonicSong[]>('subsonic_search', { query, count });
}

export async function fetchSubsonicSimilar(songId: string, count = 20): Promise<SubsonicSong[]> {
  return invoke<SubsonicSong[]>('subsonic_similar', { songId, count });
}

export async function fetchSubsonicPlaylists(): Promise<SubsonicPlaylist[]> {
  return invoke<SubsonicPlaylist[]>('subsonic_playlists');
}

export async function fetchSubsonicPlaylistDetail(playlistId: string): Promise<SubsonicPlaylistDetail> {
  return invoke<SubsonicPlaylistDetail>('subsonic_playlist', { id: playlistId });
}

export async function fetchSubsonicPlaylistSongs(playlistId: string): Promise<SubsonicSong[]> {
  const result = await invoke<SubsonicPlaylistDetail>('subsonic_playlist', { id: playlistId });
  return result.songs;
}

export async function fetchSubsonicArtistAlbums(query: string, count = 20): Promise<SubsonicAlbum[]> {
  return invoke<SubsonicAlbum[]>('subsonic_artist_albums', { query, count });
}

export async function fetchSubsonicAlbumSongs(albumId: string): Promise<SubsonicSong[]> {
  return invoke<SubsonicSong[]>('subsonic_album_songs', { id: albumId });
}

export async function fetchSubsonicAlbumDetail(albumId: string): Promise<SubsonicAlbumDetail> {
  return invoke<SubsonicAlbumDetail>('subsonic_album', { id: albumId });
}

export async function fetchSubsonicAlbumList(
  type: 'newest' | 'random' | 'frequent' | 'recent' | 'highest' = 'newest',
  count = 20
): Promise<SubsonicAlbum[]> {
  return invoke<SubsonicAlbum[]>('subsonic_album_list', { kind: type, count });
}

export async function fetchSubsonicStarredSongs(): Promise<SubsonicSong[]> {
  return invoke<SubsonicSong[]>('subsonic_starred');
}

export async function starSubsonicSong(id: string, artist?: string, title?: string): Promise<void> {
  await invoke('subsonic_star', { id, unstar: false, artist: artist ?? null, title: title ?? null });
}

export async function unstarSubsonicSong(id: string, artist?: string, title?: string): Promise<void> {
  await invoke('subsonic_star', { id, unstar: true, artist: artist ?? null, title: title ?? null });
}

export async function addSongToSubsonicPlaylist(playlistId: string, songId: string): Promise<void> {
  await invoke('subsonic_add_to_playlist', { playlistId, songId });
}

// ── Lyrics ────────────────────────────────────────────────────────────────────

export async function fetchLyrics(
  artist: string,
  title: string,
  album: string,
  duration: number
): Promise<LyricsResult | null> {
  return invoke<LyricsResult | null>('fetch_lyrics', { artist, title, album, duration });
}

// ── Last.fm account ───────────────────────────────────────────────────────────

export async function lfmBeginAuth(): Promise<{ token: string; authUrl: string }> {
  return invoke('lfm_begin_auth');
}

export async function lfmCompleteAuth(token: string): Promise<{ username: string }> {
  return invoke('lfm_complete_auth', { token });
}

export async function lfmDisconnect(): Promise<void> {
  await invoke('lfm_disconnect');
}

export function lfmNowPlaying(artist: string, track: string, album?: string, duration?: number): void {
  invoke('lfm_now_playing', { artist, track, album: album ?? null, duration: duration ?? null })
    .catch(() => undefined);
}

export function lfmScrobble(
  artist: string,
  track: string,
  timestamp: number,
  album?: string,
  duration?: number
): void {
  invoke('lfm_scrobble', { artist, track, timestamp, album: album ?? null, duration: duration ?? null })
    .catch(() => undefined);
}

export async function lfmUserTaste(): Promise<string[]> {
  const result = await invoke<{ connected: boolean; username: string; artists: string[] }>(
    'lfm_user_taste'
  ).catch(() => null);
  return result?.artists ?? [];
}

// ── Up-next recommendation (unchanged — combines Last.fm + Subsonic search) ──

export async function fetchUpNextSongs({
  apiKey,
  artist,
  title,
  likedArtists = [],
  limit = 5
}: {
  apiKey: string;
  artist: string;
  title: string;
  likedArtists?: string[];
  limit?: number;
}): Promise<SubsonicSong[]> {
  if (!apiKey || !artist.trim() || !title.trim()) return [];

  const { fetchLastFmRecommendations } = await import('./lastfm');

  const recs = await fetchLastFmRecommendations({
    apiKey,
    seedArtist: artist,
    seedSongTitle: title,
    likedArtists,
    limit: limit * 4
  });

  const normalize = (s: string) => s.trim().toLowerCase();
  const key = (a: string, t: string) => `${normalize(a)}::${normalize(t)}`;

  const matchingSong = (
    candidates: SubsonicSong[],
    recArtist: string,
    recTitle: string
  ): SubsonicSong | null => {
    const exact = candidates.find((s) => key(s.artist, s.title) === key(recArtist, recTitle));
    if (exact) return exact;
    const byArtist = candidates.filter((s) => normalize(s.artist) === normalize(recArtist));
    return (
      byArtist.find(
        (s) =>
          s.title.toLowerCase().includes(recTitle.toLowerCase()) ||
          recTitle.toLowerCase().includes(s.title.toLowerCase())
      ) ?? null
    );
  };

  const results: SubsonicSong[] = [];
  const seen = new Set<string>();

  for (const rec of recs) {
    if (results.length >= limit) break;
    const candidates = await searchSubsonicSongs(`${rec.artist} ${rec.title}`, 10).catch(() => []);
    const match = matchingSong(candidates, rec.artist, rec.title);
    if (match && !seen.has(match.id)) {
      seen.add(match.id);
      results.push(match);
    }
  }

  return results;
}
