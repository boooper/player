import { DEFAULT_EQ_BANDS, type EqBandValues, type EqPresetId } from '$lib/audio/equalizer';
import { writable, get } from 'svelte/store';

export type AppSettings = {
  lastFmApiKey: string;
  recommendationProvider: string;
  metadataProvider: string;
  lastFmConnected: boolean;
  lastFmUsername: string;
  listenBrainzUsername: string;
  listenBrainzToken: string;
  crossfadeSeconds: number;
  eqEnabled: boolean;
  eqPreset: EqPresetId;
  eqBands: EqBandValues;
};

export const appSettings = writable<AppSettings>({
  lastFmApiKey: '',
  recommendationProvider: 'lastfm',
  metadataProvider: 'both',
  lastFmConnected: false,
  lastFmUsername: '',
  listenBrainzUsername: '',
  listenBrainzToken: '',
  crossfadeSeconds: 4,
  eqEnabled: false,
  eqPreset: 'flat',
  eqBands: DEFAULT_EQ_BANDS
});

export function getLastFmApiKey(): string {
  return get(appSettings).lastFmApiKey;
}

export function getRecommendationProviderSetting(): string {
  return get(appSettings).recommendationProvider;
}

export function getMetadataProviderSetting(): string {
  return get(appSettings).metadataProvider;
}

export function getListenBrainzUsername(): string {
  return get(appSettings).listenBrainzUsername;
}

export function getListenBrainzToken(): string {
  return get(appSettings).listenBrainzToken;
}

// Increment this to signal to the layout that library data should be reloaded
// (playlists, starred songs, status indicators, liked artists).
export const libraryRefresh = writable(0);
