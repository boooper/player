import type { ServiceStatus } from '@player/shared';
import type { EqBandValues, EqPresetId } from '$lib/audio/equalizer';

export type { Song, Album, Playlist, AlbumDetail, PlaylistDetail } from '@player/shared';

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
  listenBrainzUsername: string;
  listenBrainzTokenConfigured: boolean;
  volume: number;
  crossfadeSeconds: number;
  eqEnabled: boolean;
  eqPreset: EqPresetId;
  eqBands: EqBandValues;
  discordRpcEnabled: boolean;
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

export type DesktopPlaybackStatus = {
  loaded: boolean;
  playing: boolean;
  position: number;
  duration: number;
  ended: boolean;
  trackId: string | null;
  eqEnabled: boolean;
  eqBands: EqBandValues;
  volume: number;
};

export type SearchBundlePayload = {
  songs: import('@player/shared').Song[];
  albums: import('@player/shared').Album[];
  recommendations: import('@player/shared').Song[];
};

export type ServiceHealthPayload = {
  subsonic: ServiceStatus;
  lastfm: ServiceStatus;
};

export type LastFmStatusPayload = {
  connected: boolean;
  username: string;
};

export type CastDeviceInfo = {
  name: string;
  addr: string;
  port: number;
};

export type CastSessionInfo = {
  deviceName: string;
  deviceAddr: string;
  devicePort: number;
  transportId: string;
  sessionId: string;
  mediaSessionId: number;
};

export type CastPlaybackStatus = {
  currentTime: number;
  playerState: 'PLAYING' | 'PAUSED' | 'IDLE' | 'BUFFERING';
};
