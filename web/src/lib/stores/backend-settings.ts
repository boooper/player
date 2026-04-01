import { get, writable } from 'svelte/store';
import { DEFAULT_EQ_BANDS, DEFAULT_EQ_FREQUENCIES, type EqBandValues, type EqFrequencyValues, type EqPresetId } from '$lib/audio/equalizer';

export type BackendSettings = {
  lastFmApiKey: string;
  recommendationProvider: string;
  metadataProvider: string;
  lastFmConnected: boolean;
  lastFmUsername: string;
  listenBrainzUsername: string;
  listenBrainzToken: string;
  crossfadeSeconds: number;
  gaplessEnabled: boolean;
  normalizationEnabled: boolean;
  normalizationMode: 'lufs' | 'rms';
  loudnessCompensationEnabled: boolean;
  smartCrossfadeEnabled: boolean;
  eqEnabled: boolean;
  eqPreset: EqPresetId;
  eqFrequencies: EqFrequencyValues;
  eqBands: EqBandValues;
  discordRpcEnabled: boolean;
  lyricsProvider?: string;
};

export const defaultBackendSettings: BackendSettings = {
  lastFmApiKey: '',
  recommendationProvider: 'lastfm',
  metadataProvider: 'both',
  lastFmConnected: false,
  lastFmUsername: '',
  listenBrainzUsername: '',
  listenBrainzToken: '',
  crossfadeSeconds: 4,
  gaplessEnabled: true,
  normalizationEnabled: false,
  normalizationMode: 'lufs' as const,
  loudnessCompensationEnabled: false,
  smartCrossfadeEnabled: false,
  eqEnabled: false,
  eqPreset: 'flat',
  eqFrequencies: DEFAULT_EQ_FREQUENCIES,
  eqBands: DEFAULT_EQ_BANDS,
  discordRpcEnabled: true,
  lyricsProvider: 'auto',
};

export const backendSettings = writable<BackendSettings>(defaultBackendSettings);

export function getLastFmApiKey(): string {
  return get(backendSettings).lastFmApiKey;
}

export function getRecommendationProviderSetting(): string {
  return get(backendSettings).recommendationProvider;
}

export function getMetadataProviderSetting(): string {
  return get(backendSettings).metadataProvider;
}

export function getListenBrainzUsername(): string {
  return get(backendSettings).listenBrainzUsername;
}

export function getListenBrainzToken(): string {
  return get(backendSettings).listenBrainzToken;
}
