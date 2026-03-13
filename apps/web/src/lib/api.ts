import { apiFetch } from './http';
import type { ServiceStatus } from '@player/shared/contracts';

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

async function request<T>(input: string, init?: RequestInit): Promise<T> {
  const response = await apiFetch(input, {
    headers: {
      'Content-Type': 'application/json',
      ...(init?.headers ?? {})
    },
    ...init
  });

  if (!response.ok) {
    const json = await response.json().catch(() => ({}));
    const message = typeof json?.error === 'string' ? json.error : `Request failed (${response.status})`;
    throw new Error(message);
  }

  if (response.status === 204) return undefined as T;
  return response.json() as Promise<T>;
}

export type AppSettingsPayload = {
  lastFmApiKey: string;
  lastFmSharedSecretConfigured: boolean;
  recommendationProvider: string;
  metadataProvider: string;
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

type SettingsProfilesPlatformApi = {
  fetchAppSettings: () => Promise<AppSettingsPayload>;
  updateAppSettings: (data: {
    lastFmApiKey: string;
    recommendationProvider: string;
    metadataProvider: string;
    lastFmSharedSecret?: string;
  }) => Promise<void>;
  fetchProfiles: () => Promise<ProfilePayload[]>;
  createProfile: (data: ProfileDraftPayload) => Promise<ProfilePayload>;
  updateProfile: (id: number, data: ProfileDraftPayload) => Promise<ProfilePayload>;
  deleteProfile: (id: number) => Promise<void>;
  activateProfile: (id: number) => Promise<void>;
  fetchServiceHealth: () => Promise<{ subsonic: ServiceStatus; lastfm: ServiceStatus }>;
  fetchLibraryStats: () => Promise<LibraryStatsPayload>;
  fetchLastFmStatus: () => Promise<{ connected: boolean; username: string }>;
};

const httpSettingsProfilesApi: SettingsProfilesPlatformApi = {
  async fetchAppSettings() {
    const payload = await request<{
      LASTFM_API_KEY?: string;
      LASTFM_SHARED_SECRET_CONFIGURED?: string;
      RECOMMENDATION_PROVIDER?: string;
      METADATA_PROVIDER?: string;
    }>('/api/settings');

    return {
      lastFmApiKey: payload.LASTFM_API_KEY ?? '',
      lastFmSharedSecretConfigured: payload.LASTFM_SHARED_SECRET_CONFIGURED === 'true',
      recommendationProvider: payload.RECOMMENDATION_PROVIDER || 'lastfm',
      metadataProvider: payload.METADATA_PROVIDER || 'both'
    };
  },
  async updateAppSettings(data) {
    const body: Record<string, string> = {
      LASTFM_API_KEY: data.lastFmApiKey,
      RECOMMENDATION_PROVIDER: data.recommendationProvider,
      METADATA_PROVIDER: data.metadataProvider
    };
    if (data.lastFmSharedSecret?.trim()) {
      body.LASTFM_SHARED_SECRET = data.lastFmSharedSecret.trim();
    }
    await request<{ updated: string[] }>('/api/settings', {
      method: 'PUT',
      body: JSON.stringify(body)
    });
  },
  async fetchProfiles() {
    const payload = await request<{ profiles: ProfilePayload[] }>('/api/profiles');
    return payload.profiles;
  },
  async createProfile(data) {
    const payload = await request<{ profile: ProfilePayload }>('/api/profiles', {
      method: 'POST',
      body: JSON.stringify(data)
    });
    return payload.profile;
  },
  async updateProfile(id, data) {
    const payload = await request<{ profile: ProfilePayload }>(`/api/profiles/${id}`, {
      method: 'PUT',
      body: JSON.stringify(data)
    });
    return payload.profile;
  },
  async deleteProfile(id) {
    await request<{ success: boolean }>(`/api/profiles/${id}`, { method: 'DELETE' });
  },
  async activateProfile(id) {
    await request<{ success: boolean }>(`/api/profiles/${id}/activate`, { method: 'POST' });
  },
  async fetchServiceHealth() {
    return request<{ subsonic: ServiceStatus; lastfm: ServiceStatus }>('/api/health/services');
  },
  async fetchLibraryStats() {
    return request<LibraryStatsPayload>('/api/stats');
  },
  async fetchLastFmStatus() {
    const payload = await request<{ connected?: boolean; username?: string }>('/api/lastfm/user-taste');
    return {
      connected: Boolean(payload.connected),
      username: payload.username ?? ''
    };
  }
};

async function settingsProfilesApi(): Promise<SettingsProfilesPlatformApi> {
  return httpSettingsProfilesApi;
}

export async function fetchAppSettings(): Promise<AppSettingsPayload> {
  return (await settingsProfilesApi()).fetchAppSettings();
}

export async function updateAppSettings(data: {
  lastFmApiKey: string;
  recommendationProvider: string;
  metadataProvider: string;
  lastFmSharedSecret?: string;
}): Promise<void> {
  return (await settingsProfilesApi()).updateAppSettings(data);
}

export async function fetchProfiles(): Promise<ProfilePayload[]> {
  return (await settingsProfilesApi()).fetchProfiles();
}

export async function createProfile(data: ProfileDraftPayload): Promise<ProfilePayload> {
  return (await settingsProfilesApi()).createProfile(data);
}

export async function updateProfile(id: number, data: ProfileDraftPayload): Promise<ProfilePayload> {
  return (await settingsProfilesApi()).updateProfile(id, data);
}

export async function deleteProfile(id: number): Promise<void> {
  return (await settingsProfilesApi()).deleteProfile(id);
}

export async function activateProfile(id: number): Promise<void> {
  return (await settingsProfilesApi()).activateProfile(id);
}

export async function fetchServiceHealth(): Promise<{ subsonic: ServiceStatus; lastfm: ServiceStatus }> {
  return (await settingsProfilesApi()).fetchServiceHealth();
}

export async function fetchLibraryStats(): Promise<LibraryStatsPayload> {
  return (await settingsProfilesApi()).fetchLibraryStats();
}

export async function fetchLastFmStatus(): Promise<{ connected: boolean; username: string }> {
  return (await settingsProfilesApi()).fetchLastFmStatus();
}

export async function fetchLikedArtists(): Promise<StoredLikedArtist[]> {
  const payload = await request<{ artists: StoredLikedArtist[] }>('/api/liked-artists');
  return payload.artists;
}

export async function saveLikedArtist(name: string): Promise<StoredLikedArtist> {
  const payload = await request<{ artist: StoredLikedArtist }>('/api/liked-artists', {
    method: 'POST',
    body: JSON.stringify({
      name,
      source: 'lastfm'
    })
  });
  return payload.artist;
}

export async function removeLikedArtist(name: string): Promise<void> {
  await request<void>(`/api/liked-artists/${encodeURIComponent(name)}`, {
    method: 'DELETE'
  });
}

export async function searchSubsonicSongs(query: string, count = 20): Promise<SubsonicSong[]> {
  const payload = await request<{ songs: SubsonicSong[] }>(
    `/api/subsonic/search?q=${encodeURIComponent(query)}&count=${count}`
  );
  return payload.songs;
}

export async function fetchSubsonicSimilar(songId: string, count = 20): Promise<SubsonicSong[]> {
  const payload = await request<{ songs: SubsonicSong[] }>(
    `/api/subsonic/similar?songId=${encodeURIComponent(songId)}&count=${count}`
  );
  return payload.songs;
}

export async function fetchSubsonicPlaylists(): Promise<SubsonicPlaylist[]> {
  const payload = await request<{ playlists: SubsonicPlaylist[] }>('/api/subsonic/playlists');
  return payload.playlists;
}

export type SubsonicPlaylistDetail = {
  playlist: { id: string; name: string; songCount: number; duration: number; coverArtUrl: string };
  songs: SubsonicSong[];
};

export async function starSubsonicSong(id: string, artist?: string, title?: string): Promise<void> {
  await request<{ ok: boolean }>('/api/subsonic/star', { method: 'POST', body: JSON.stringify({ id, artist, title }) });
}

export async function unstarSubsonicSong(id: string, artist?: string, title?: string): Promise<void> {
  await request<{ ok: boolean }>('/api/subsonic/star', { method: 'POST', body: JSON.stringify({ id, unstar: true, artist, title }) });
}

export async function addSongToSubsonicPlaylist(playlistId: string, songId: string): Promise<void> {
  await request<{ ok: boolean }>('/api/subsonic/playlist-song', { method: 'POST', body: JSON.stringify({ playlistId, songId }) });
}

export async function fetchSubsonicStarredSongs(): Promise<SubsonicSong[]> {
  const payload = await request<{ songs: SubsonicSong[] }>('/api/subsonic/starred');
  return payload.songs;
}

export async function fetchSubsonicArtistAlbums(query: string, count = 20): Promise<SubsonicAlbum[]> {
  const payload = await request<{ albums: SubsonicAlbum[] }>(
    `/api/subsonic/artist-albums?q=${encodeURIComponent(query)}&count=${count}`
  );
  return payload.albums;
}

export async function fetchSubsonicAlbumSongs(albumId: string): Promise<SubsonicSong[]> {
  const payload = await request<{ songs: SubsonicSong[] }>(
    `/api/subsonic/album-songs?id=${encodeURIComponent(albumId)}`
  );
  return payload.songs;
}

export type SubsonicAlbumDetail = {
  album: SubsonicAlbum & { genre?: string };
  songs: SubsonicSong[];
};

export async function fetchSubsonicAlbumDetail(albumId: string): Promise<SubsonicAlbumDetail> {
  return request<SubsonicAlbumDetail>(`/api/subsonic/album?id=${encodeURIComponent(albumId)}`);
}

export async function fetchSubsonicAlbumList(
  type: 'newest' | 'random' | 'frequent' | 'recent' | 'highest' = 'newest',
  count = 20
): Promise<SubsonicAlbum[]> {
  const payload = await request<{ albums: SubsonicAlbum[] }>(
    `/api/subsonic/album-list?type=${type}&count=${count}`
  );
  return payload.albums;
}

export async function fetchSubsonicPlaylistSongs(playlistId: string): Promise<SubsonicSong[]> {
  const payload = await request<SubsonicPlaylistDetail>(
    `/api/subsonic/playlist?id=${encodeURIComponent(playlistId)}`
  );
  return payload.songs;
}

export async function fetchSubsonicPlaylistDetail(playlistId: string): Promise<SubsonicPlaylistDetail> {
  return request<SubsonicPlaylistDetail>(`/api/subsonic/playlist?id=${encodeURIComponent(playlistId)}`);
}

/**
 * Fetch up-next songs by combining Last.fm similar track recommendations with
 * Subsonic/octo-fiesta search (which proxies Deezer). Returns only tracks that
 * are actually streamable via the configured octo-fiesta instance.
 */
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
    const exact = candidates.find(
      (s) => key(s.artist, s.title) === key(recArtist, recTitle)
    );
    if (exact) return exact;
    const byArtist = candidates.filter(
      (s) => normalize(s.artist) === normalize(recArtist)
    );
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
    const candidates = await searchSubsonicSongs(
      `${rec.artist} ${rec.title}`,
      10
    ).catch(() => []);
    const match = matchingSong(candidates, rec.artist, rec.title);
    if (match && !seen.has(match.id)) {
      seen.add(match.id);
      results.push(match);
    }
  }

  return results;
}

export type LyricsResult = {
  plainLyrics: string | null;
  syncedLyrics: string | null;
  instrumental: boolean;
};

export async function fetchLyrics(
  artist: string,
  title: string,
  album: string,
  duration: number
): Promise<LyricsResult | null> {
  const params = new URLSearchParams({
    artist,
    title,
    album,
    duration: Math.round(duration).toString()
  });
  const resp = await apiFetch(`/api/lyrics?${params}`);
  if (!resp.ok) return null;
  const data = await resp.json();
  if (!data) return null;
  return {
    plainLyrics: data.plainLyrics ?? null,
    syncedLyrics: data.syncedLyrics ?? null,
    instrumental: data.instrumental ?? false
  };
}

// ─── Last.fm account ──────────────────────────────────────────────────────────

/** Begin OAuth — returns the token and URL the user should open in their browser */
export async function lfmBeginAuth(): Promise<{ token: string; authUrl: string }> {
  const res = await apiFetch('/api/lastfm/auth');
  const json = await res.json();
  if (!res.ok) throw new Error(json?.error ?? 'Failed to start Last.fm auth');
  return json as { token: string; authUrl: string };
}

/** Complete OAuth — exchange the authorized token for a session key */
export async function lfmCompleteAuth(token: string): Promise<{ username: string }> {
  const res = await apiFetch('/api/lastfm/auth', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ token })
  });
  const json = await res.json();
  if (!res.ok) throw new Error(json?.error ?? 'Failed to complete Last.fm auth');
  return json as { username: string };
}

export async function lfmDisconnect(): Promise<void> {
  await apiFetch('/api/lastfm/disconnect', { method: 'POST' });
}

/** Fire-and-forget: update Last.fm "now playing" — never throws */
export function lfmNowPlaying(artist: string, track: string, album?: string, duration?: number): void {
  apiFetch('/api/lastfm/now-playing', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ artist, track, album, duration })
  }).catch(() => undefined);
}

/** Fire-and-forget: scrobble a track — never throws */
export function lfmScrobble(
  artist: string,
  track: string,
  timestamp: number,
  album?: string,
  duration?: number
): void {
  apiFetch('/api/lastfm/scrobble', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ artist, track, timestamp, album, duration })
  }).catch(() => undefined);
}

/** Returns the user's top artists from Last.fm (empty array if not connected) */
export async function lfmUserTaste(): Promise<string[]> {
  const res = await apiFetch('/api/lastfm/user-taste');
  if (!res.ok) return [];
  const json = await res.json();
  return Array.isArray(json?.artists) ? json.artists : [];
}

