import { normalizeEqBands, type EqBandValues } from '$lib/audio/equalizer';
import type { Song } from './types';
import type { DesktopPlaybackStatus } from './types';
import { invoke } from './shared';

export async function desktopPlaybackLoad(song: Song, autoplay = false): Promise<void> {
  await invoke('playback_load', { song, autoplay });
}

export async function desktopPlaybackPreload(song: Song): Promise<void> {
  await invoke('playback_preload', { song });
}

export async function desktopPlaybackPlay(): Promise<void> {
  await invoke('playback_play');
}

export async function desktopPlaybackPause(): Promise<void> {
  await invoke('playback_pause');
}

export async function desktopPlaybackStop(): Promise<void> {
  await invoke('playback_stop');
}

export async function desktopPlaybackSeek(position: number): Promise<void> {
  await invoke('playback_seek', { position });
}

export async function desktopPlaybackSetVolume(volume: number): Promise<void> {
  await invoke('playback_set_volume', { volume });
}

export async function desktopPlaybackSetEq(enabled: boolean, bands: EqBandValues): Promise<void> {
  await invoke('playback_set_eq', { enabled, bands: normalizeEqBands(bands) });
}

export async function desktopPlaybackStatus(): Promise<DesktopPlaybackStatus> {
  return invoke<DesktopPlaybackStatus>('playback_status');
}

export async function desktopPlaybackIsCached(song: Song): Promise<boolean> {
  return invoke<boolean>('playback_is_cached', { song });
}

export async function desktopPlaybackCachedIds(songs: Song[]): Promise<string[]> {
  return invoke<string[]>('playback_cached_ids', { songs });
}
