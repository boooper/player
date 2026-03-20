import type { CastDeviceInfo, CastPlaybackStatus } from './types';
import { invoke } from './shared';

export async function castDiscover(): Promise<CastDeviceInfo[]> {
  return invoke<CastDeviceInfo[]>('cast_discover');
}

export async function castConnect(params: {
  deviceName: string;
  deviceAddr: string;
  devicePort: number;
}): Promise<void> {
  return invoke<void>('cast_connect', {
    deviceName: params.deviceName,
    deviceAddr: params.deviceAddr,
    devicePort: params.devicePort,
  });
}

export async function castPlay(params: {
  deviceName: string;
  deviceAddr: string;
  devicePort: number;
  songId: string;
  streamUrl: string;
  title: string;
  artist: string;
  coverUrl: string;
}): Promise<void> {
  return invoke<void>('cast_play', {
    deviceName: params.deviceName,
    deviceAddr: params.deviceAddr,
    devicePort: params.devicePort,
    songId: params.songId,
    streamUrl: params.streamUrl,
    title: params.title,
    artist: params.artist,
    coverUrl: params.coverUrl
  });
}

export async function castPause(): Promise<void> {
  return invoke<void>('cast_pause');
}

export async function castResume(): Promise<void> {
  return invoke<void>('cast_resume');
}

export async function castStop(): Promise<void> {
  return invoke<void>('cast_stop');
}

export async function castGetSession(): Promise<CastDeviceInfo | null> {
  return invoke<CastDeviceInfo | null>('cast_get_session');
}

export async function castSetVolume(level: number): Promise<void> {
  return invoke<void>('cast_set_volume', { level });
}

export async function castSeek(position: number): Promise<void> {
  return invoke<void>('cast_seek', { position });
}

export async function castGetStatus(): Promise<CastPlaybackStatus> {
  return invoke<CastPlaybackStatus>('cast_get_status');
}
