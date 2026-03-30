import { getContext, setContext } from 'svelte';
import { toast } from 'svelte-sonner';
import { platform } from '@tauri-apps/plugin-os';
import { backendSettings, defaultBackendSettings } from '$lib/stores/backend-settings';
import { requestLibraryRefresh } from '$lib/stores/ui-state';
import { DEFAULT_EQ_BANDS, DEFAULT_EQ_FREQUENCIES, type EqBandValues, type EqFrequencyValues, type EqPresetId } from '$lib/audio/equalizer';
import {
  fetchAppSettings,
  fetchLastFmStatus,
  fetchProfiles,
  getAutostart,
  updateAppSettings,
  type ProfilePayload,
} from '$lib/servers';

class SettingsController {
  profiles = $state<ProfilePayload[]>([]);
  lastFmApiKey = $state('');
  lastFmSharedSecret = $state('');
  lastFmSharedSecretConfigured = $state(false);
  lfmConnected = $state(false);
  lfmUsername = $state('');
  listenBrainzUsername = $state('');
  listenBrainzToken = $state('');
  listenBrainzTokenConfigured = $state(false);
  lyricsProvider = $state('auto');
  metadataProvider = $state('both');
  recommendationProvider = $state('lastfm');
  isMobile = $state(false);
  autostartEnabled = $state(false);
  crossfadeSeconds = $state(4);
  eqEnabled = $state(false);
  eqPreset = $state<EqPresetId>('flat');
  eqFrequencies = $state<EqFrequencyValues>(DEFAULT_EQ_FREQUENCIES);
  eqBands = $state<EqBandValues>(DEFAULT_EQ_BANDS);
  discordRpcEnabled = $state(true);
  saving = $state(false);
  statsRefreshKey = $state(0);

  initialize = async () => {
    let currentPlatform = 'unknown';
    try {
      currentPlatform = await platform();
    } catch {
      // Ignore platform checks outside Tauri.
    }

    this.isMobile = currentPlatform === 'android' || currentPlatform === 'ios';
    await this.loadInitialSettings();

    if (!this.isMobile) {
      this.autostartEnabled = await getAutostart().catch(() => false);
    }
  };

  loadInitialSettings = async () => {
    const profilesPromise = fetchProfiles()
      .then((profiles) => {
        this.profiles = profiles;
      })
      .catch(() => {
        toast.error('Failed to load servers');
      });

    const settingsPromise = (async () => {
      try {
        const [settings, lfmStatus] = await Promise.all([
          fetchAppSettings(),
          fetchLastFmStatus().catch(() => ({ connected: false, username: '' })),
        ]);

        this.lastFmApiKey = settings.lastFmApiKey;
        this.lastFmSharedSecretConfigured = settings.lastFmSharedSecretConfigured;
        this.recommendationProvider = settings.recommendationProvider;
        this.metadataProvider = settings.metadataProvider;
        this.listenBrainzUsername = settings.listenBrainzUsername;
        this.listenBrainzTokenConfigured = settings.listenBrainzTokenConfigured;
        this.lyricsProvider = settings.lyricsProvider ?? 'auto';
        this.crossfadeSeconds = settings.crossfadeSeconds;
        this.eqEnabled = settings.eqEnabled;
        this.eqPreset = settings.eqPreset;
        this.eqFrequencies = settings.eqFrequencies;
        this.eqBands = settings.eqBands;
        this.discordRpcEnabled = settings.discordRpcEnabled;
        this.lfmConnected = lfmStatus.connected;
        this.lfmUsername = lfmStatus.username;

        backendSettings.set({
          lastFmApiKey: this.lastFmApiKey,
          recommendationProvider: this.recommendationProvider,
          metadataProvider: this.metadataProvider,
          lastFmConnected: this.lfmConnected,
          lastFmUsername: this.lfmUsername,
          listenBrainzUsername: this.listenBrainzUsername,
          listenBrainzToken: '',
          crossfadeSeconds: this.crossfadeSeconds,
          eqEnabled: this.eqEnabled,
          eqPreset: this.eqPreset,
          eqFrequencies: this.eqFrequencies,
          eqBands: this.eqBands,
          discordRpcEnabled: this.discordRpcEnabled,
          lyricsProvider: this.lyricsProvider,
        });
      } catch {
        toast.error('Failed to load settings');
      }
    })();

    await Promise.all([profilesPromise, settingsPromise]);
  };

  save = async () => {
    this.saving = true;

    try {
      await updateAppSettings({
        lastFmApiKey: this.lastFmApiKey,
        recommendationProvider: this.recommendationProvider,
        metadataProvider: this.metadataProvider,
        lyricsProvider: this.lyricsProvider,
        listenBrainzUsername: this.listenBrainzUsername,
        crossfadeSeconds: this.crossfadeSeconds,
        eqEnabled: this.eqEnabled,
        eqPreset: this.eqPreset,
        eqFrequencies: this.eqFrequencies,
        eqBands: this.eqBands,
        discordRpcEnabled: this.discordRpcEnabled,
        listenBrainzToken: this.listenBrainzToken,
        lastFmSharedSecret: this.lastFmSharedSecret,
      });

      backendSettings.set({
        lastFmApiKey: this.lastFmApiKey,
        recommendationProvider: this.recommendationProvider,
        metadataProvider: this.metadataProvider,
        lastFmConnected: this.lfmConnected,
        lastFmUsername: this.lfmUsername,
        listenBrainzUsername: this.listenBrainzUsername,
        listenBrainzToken: this.listenBrainzToken,
        crossfadeSeconds: this.crossfadeSeconds,
        eqEnabled: this.eqEnabled,
        eqPreset: this.eqPreset,
        eqFrequencies: this.eqFrequencies,
        eqBands: this.eqBands,
        discordRpcEnabled: this.discordRpcEnabled,
        lyricsProvider: this.lyricsProvider,
      });

      requestLibraryRefresh();

      if (this.lastFmSharedSecret.trim()) {
        this.lastFmSharedSecretConfigured = true;
        this.lastFmSharedSecret = '';
      }

      if (this.listenBrainzToken.trim()) {
        this.listenBrainzTokenConfigured = true;
        this.listenBrainzToken = '';
      }

      toast.success('Settings saved');
      this.statsRefreshKey += 1;
    } catch (err) {
      toast.error(err instanceof Error ? err.message : String(err) || 'Failed to save settings');
    } finally {
      this.saving = false;
    }
  };

  refreshStats = () => {
    this.statsRefreshKey += 1;
  };

  handleCleared = () => {
    backendSettings.set(defaultBackendSettings);
    this.profiles = [];
    this.lastFmApiKey = defaultBackendSettings.lastFmApiKey;
    this.lastFmSharedSecret = '';
    this.lastFmSharedSecretConfigured = false;
    this.lfmConnected = false;
    this.lfmUsername = '';
    this.listenBrainzUsername = '';
    this.listenBrainzToken = '';
    this.listenBrainzTokenConfigured = false;
    this.recommendationProvider = defaultBackendSettings.recommendationProvider;
    this.metadataProvider = defaultBackendSettings.metadataProvider;
    this.crossfadeSeconds = defaultBackendSettings.crossfadeSeconds;
    this.eqEnabled = defaultBackendSettings.eqEnabled;
    this.eqPreset = defaultBackendSettings.eqPreset;
    this.eqFrequencies = defaultBackendSettings.eqFrequencies;
    this.eqBands = defaultBackendSettings.eqBands;
    this.discordRpcEnabled = defaultBackendSettings.discordRpcEnabled;
    this.lyricsProvider = defaultBackendSettings.lyricsProvider ?? 'auto';
    this.statsRefreshKey += 1;
  };
}

const SETTINGS_CONTEXT_KEY = Symbol.for('player.settings');

export function setSettingsContext() {
  return setContext(SETTINGS_CONTEXT_KEY, new SettingsController());
}

export function useSettingsContext(): SettingsController {
  return getContext(SETTINGS_CONTEXT_KEY);
}
