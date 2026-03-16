import { enable as autostartEnable, disable as autostartDisable, isEnabled as autostartIsEnabled } from '@tauri-apps/plugin-autostart';
import { DEFAULT_EQ_BANDS, findEqPresetId, normalizeEqBands, type EqBandValues, type EqPresetId } from '$lib/audio/equalizer';
import type { AppSettingsPayload } from './types';
import { invoke } from './shared';

export async function fetchAppSettings(): Promise<AppSettingsPayload> {
  const s = await invoke<Record<string, string>>('get_settings');
  const rawVol = parseFloat(s.VOLUME ?? '');
  const rawCrossfade = parseFloat(s.CROSSFADE_SECONDS ?? '');
  const rawRepeat = s.REPEAT;
  let rawEqBands: number[] = DEFAULT_EQ_BANDS;
  try {
    rawEqBands = JSON.parse(s.EQ_BANDS ?? JSON.stringify(DEFAULT_EQ_BANDS));
  } catch {
    rawEqBands = DEFAULT_EQ_BANDS;
  }
  const eqBands = normalizeEqBands(rawEqBands);
  const eqPreset = (s.EQ_PRESET as EqPresetId | undefined) ?? findEqPresetId(eqBands);
  return {
    lastFmApiKey: s.LASTFM_API_KEY ?? '',
    lastFmSharedSecretConfigured: Boolean(s.LASTFM_SHARED_SECRET),
    recommendationProvider: s.RECOMMENDATION_PROVIDER || 'lastfm',
    metadataProvider: s.METADATA_PROVIDER || 'both',
    listenBrainzUsername: s.LISTENBRAINZ_USERNAME ?? '',
    listenBrainzTokenConfigured: Boolean(s.LISTENBRAINZ_TOKEN),
    volume: isNaN(rawVol) ? 0.8 : Math.max(0, Math.min(1, rawVol)),
    crossfadeSeconds: isNaN(rawCrossfade) ? 4 : Math.max(0, Math.min(12, rawCrossfade)),
    eqEnabled: s.EQ_ENABLED === 'true',
    eqPreset,
    eqBands,
    discordRpcEnabled: s.DISCORD_RPC_ENABLED !== 'false',
    shuffleEnabled: s.SHUFFLE === 'true',
    smartShuffleMode: s.SMART_SHUFFLE === 'true',
    repeatMode: rawRepeat === 'all' || rawRepeat === 'one' ? rawRepeat : 'off'
  };
}

export async function saveVolume(value: number): Promise<void> {
  await invoke('update_settings', { updates: { VOLUME: String(value) } });
}

export async function savePlaybackPrefs(shuffle: boolean, smartShuffle: boolean, repeat: string): Promise<void> {
  await invoke('update_settings', {
    updates: { SHUFFLE: String(shuffle), SMART_SHUFFLE: String(smartShuffle), REPEAT: repeat }
  });
}

export async function updateAppSettings(data: {
  lastFmApiKey: string;
  recommendationProvider: string;
  metadataProvider: string;
  listenBrainzUsername: string;
  crossfadeSeconds: number;
  eqEnabled: boolean;
  eqPreset: EqPresetId;
  eqBands: EqBandValues;
  discordRpcEnabled: boolean;
  listenBrainzToken?: string;
  lastFmSharedSecret?: string;
}): Promise<void> {
  const updates: Record<string, string> = {
    LASTFM_API_KEY: data.lastFmApiKey,
    RECOMMENDATION_PROVIDER: data.recommendationProvider,
    METADATA_PROVIDER: data.metadataProvider,
    LISTENBRAINZ_USERNAME: data.listenBrainzUsername,
    CROSSFADE_SECONDS: String(Math.max(0, Math.min(12, data.crossfadeSeconds))),
    EQ_ENABLED: String(data.eqEnabled),
    EQ_PRESET: data.eqPreset,
    EQ_BANDS: JSON.stringify(normalizeEqBands(data.eqBands)),
    DISCORD_RPC_ENABLED: String(data.discordRpcEnabled)
  };
  if (data.lastFmSharedSecret?.trim()) {
    updates.LASTFM_SHARED_SECRET = data.lastFmSharedSecret.trim();
  }
  if (data.listenBrainzToken?.trim()) {
    updates.LISTENBRAINZ_TOKEN = data.listenBrainzToken.trim();
  }
  await invoke('update_settings', { updates });
}

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

export async function clearCache(): Promise<void> {
  await invoke('clear_cache');
}
