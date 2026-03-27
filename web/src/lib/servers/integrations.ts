import type { LibraryStatsPayload, LastFmStatusPayload, LyricsResult, ServiceHealthPayload } from './types';
import { invoke } from './shared';

export async function fetchServiceHealth(): Promise<ServiceHealthPayload> {
  return invoke('get_service_health');
}

export async function fetchLibraryStats(): Promise<LibraryStatsPayload> {
  return invoke('get_library_stats');
}

export async function fetchLastFmStatus(): Promise<LastFmStatusPayload> {
  return invoke('lfm_status');
}

export async function fetchListenBrainzToken(): Promise<string> {
  const s = await invoke<Record<string, string>>('get_settings');
  return s.LISTENBRAINZ_TOKEN ?? '';
}

export async function fetchLyrics(
  artist: string,
  title: string,
  album: string,
  duration: number
): Promise<LyricsResult | null> {
  return invoke<LyricsResult | null>('fetch_lyrics', { artist, title, album, duration });
}

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
  invoke('lfm_now_playing', { artist, track, album: album ?? null, duration: duration ?? null }).catch(() => undefined);
}

export function lfmScrobble(
  artist: string,
  track: string,
  timestamp: number,
  album?: string,
  duration?: number
): void {
  invoke('lfm_scrobble', { artist, track, timestamp, album: album ?? null, duration: duration ?? null }).catch(() => undefined);
}

export async function lfmUserTaste(): Promise<string[]> {
  const result = await invoke<{ connected: boolean; username: string; artists: string[] }>('lfm_user_taste').catch(() => null);
  return result?.artists ?? [];
}
