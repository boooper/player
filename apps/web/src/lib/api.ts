import { invoke } from '@tauri-apps/api/core';
import { enable as autostartEnable, disable as autostartDisable, isEnabled as autostartIsEnabled } from '@tauri-apps/plugin-autostart';
import type { ServiceStatus } from '@player/shared';

// ── Canonical media types (provider-agnostic) ─────────────────────────────────
export type { Song, Album, Playlist, AlbumDetail, PlaylistDetail } from '@player/shared';
import type { Song, Album, Playlist, AlbumDetail, PlaylistDetail } from '@player/shared';

// ── App-level types ───────────────────────────────────────────────────────────

export type StoredLikedArtist = {
  id: number;
  name: string;
  source: string | null;
  externalId: string | null;
  createdAt: string;
  updatedAt: string;
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
  serverType: string;
  isActive: boolean;
};

export type ProfileDraftPayload = {
  name: string;
  url: string;
  username: string;
  password?: string;
  serverType: string;
};

export type LibraryStatsPayload = {
  likedArtists: number;
  playlistCount: number | null;
  totalPlaylistSongs: number | null;
  starredSongs: number | null;
  lastFmConfigured: boolean;
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

// ── Library — routed by the active profile's server type ─────────────────────
// These functions are provider-agnostic. The Rust backend dispatches to the
// correct implementation (Subsonic, Jellyfin, etc.) based on the active profile.

export async function searchSongs(query: string, count = 20): Promise<Song[]> {
  return invoke<Song[]>('library_search', { query, count });
}

export async function fetchSimilarSongs(songId: string, count = 20): Promise<Song[]> {
  return invoke<Song[]>('library_similar', { songId, count });
}

export async function fetchPlaylists(): Promise<Playlist[]> {
  return invoke<Playlist[]>('library_playlists');
}

export async function fetchPlaylistDetail(playlistId: string): Promise<PlaylistDetail> {
  return invoke<PlaylistDetail>('library_playlist', { id: playlistId });
}

export async function fetchPlaylistSongs(playlistId: string): Promise<Song[]> {
  const result = await invoke<PlaylistDetail>('library_playlist', { id: playlistId });
  return result.songs;
}

export async function fetchArtistAlbums(query: string, count = 20): Promise<Album[]> {
  return invoke<Album[]>('library_artist_albums', { query, count });
}

export async function fetchAlbumSongs(albumId: string): Promise<Song[]> {
  return invoke<Song[]>('library_album_songs', { id: albumId });
}

export async function fetchAlbumDetail(albumId: string): Promise<AlbumDetail> {
  return invoke<AlbumDetail>('library_album', { id: albumId });
}

export async function fetchAlbumList(
  type: 'newest' | 'random' | 'frequent' | 'recent' | 'highest' = 'newest',
  count = 20
): Promise<Album[]> {
  return invoke<Album[]>('library_album_list', { kind: type, count });
}

export async function fetchStarredSongs(): Promise<Song[]> {
  return invoke<Song[]>('library_starred');
}

export async function starSong(id: string, artist?: string, title?: string): Promise<void> {
  await invoke('library_star', { id, unstar: false, artist: artist ?? null, title: title ?? null });
}

export async function unstarSong(id: string, artist?: string, title?: string): Promise<void> {
  await invoke('library_star', { id, unstar: true, artist: artist ?? null, title: title ?? null });
}

export async function addSongToPlaylist(playlistId: string, songId: string): Promise<void> {
  await invoke('library_add_to_playlist', { playlistId, songId });
}

// ── Autostart ─────────────────────────────────────────────────────────────────

export async function getAutostart(): Promise<boolean> {
  return autostartIsEnabled();
}

export async function setAutostart(enabled: boolean): Promise<void> {
  if (enabled) {
    await autostartEnable();
  } else {
    await autostartDisable();
  }
}

export async function clearDatabase(): Promise<void> {
  await invoke('clear_database');
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
}): Promise<Song[]> {
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
    candidates: Song[],
    recArtist: string,
    recTitle: string
  ): Song | null => {
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

  const results: Song[] = [];
  const seen = new Set<string>();

  for (const rec of recs) {
    if (results.length >= limit) break;
    const candidates = await searchSongs(`${rec.artist} ${rec.title}`, 10).catch(() => []);
    const match = matchingSong(candidates, rec.artist, rec.title);
    if (match && !seen.has(match.id)) {
      seen.add(match.id);
      results.push(match);
    }
  }

  return results;
}
