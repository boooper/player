import { normalizeEqBands, normalizeEqFrequencies, type EqBandValues, type EqFrequencyValues } from '$lib/audio/equalizer';
import type { Song } from './types';
import type { DesktopPlaybackStatus } from './types';
import { invoke } from './shared';

export const DESKTOP_PLAYBACK_CACHE_UPDATED_EVENT = 'player:desktop-cache-updated';

function notifyDesktopPlaybackCacheUpdated(songId: string): void {
  if (typeof window === 'undefined' || !songId) return;
  window.dispatchEvent(
    new CustomEvent(DESKTOP_PLAYBACK_CACHE_UPDATED_EVENT, {
      detail: { songId },
    })
  );
}

export async function desktopPlaybackLoad(song: Song, autoplay = false, crossfadeMs?: number): Promise<void> {
  await invoke('playback_load', { song, autoplay, crossfadeMs });
  notifyDesktopPlaybackCacheUpdated(song.id);
}

export async function desktopPlaybackPreload(song: Song): Promise<void> {
  await invoke('playback_preload', { song });
  notifyDesktopPlaybackCacheUpdated(song.id);
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

export async function desktopPlaybackSetEq(enabled: boolean, frequencies: EqFrequencyValues, bands: EqBandValues): Promise<void> {
  await invoke('playback_set_eq', { enabled, frequencies: normalizeEqFrequencies(frequencies), bands: normalizeEqBands(bands) });
}

export async function desktopPlaybackSetNormalization(enabled: boolean): Promise<void> {
  await invoke('playback_set_normalization', { enabled });
}

export async function desktopPlaybackSetNormalizationMode(mode: 'lufs' | 'rms'): Promise<void> {
  await invoke('playback_set_normalization_mode', { mode });
}

export async function desktopPlaybackSetLoudnessCompensation(enabled: boolean): Promise<void> {
  await invoke('playback_set_loudness_compensation', { enabled });
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
