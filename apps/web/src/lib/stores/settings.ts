import { writable, get } from 'svelte/store';

export type AppSettings = {
  lastFmApiKey: string;
  recommendationProvider: string;
  metadataProvider: string;
  lastFmConnected: boolean;
  lastFmUsername: string;
};

export const appSettings = writable<AppSettings>({
  lastFmApiKey: '',
  recommendationProvider: 'lastfm',
  metadataProvider: 'both',
  lastFmConnected: false,
  lastFmUsername: ''
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

// Increment this to signal to the layout that library data should be reloaded
// (playlists, starred songs, status indicators, liked artists).
export const libraryRefresh = writable(0);
