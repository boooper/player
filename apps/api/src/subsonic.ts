import { createHash, randomBytes } from 'node:crypto';
import type { Profile } from './store.js';

const SUBSONIC_API_VERSION = '1.16.1';
const SUBSONIC_CLIENT_NAME = 'naviarr';
const SUBSONIC_RESPONSE_FORMAT = 'json';

type Params = Record<string, string | number | undefined>;
type R = Record<string, unknown>;

function authParams(profile: Profile): Params {
  const base: Params = {
    u: profile.username,
    v: SUBSONIC_API_VERSION,
    c: SUBSONIC_CLIENT_NAME,
    f: SUBSONIC_RESPONSE_FORMAT
  };
  if (profile.usePasswordAuth || String(profile.password).startsWith('enc:')) {
    return { ...base, p: profile.password };
  }
  const salt = randomBytes(6).toString('hex');
  const token = createHash('md5').update(`${profile.password}${salt}`).digest('hex');
  return { ...base, t: token, s: salt };
}

export class SubsonicClient {
  private profile: Profile;

  constructor(profile: Profile) {
    this.profile = { ...profile, url: String(profile.url).replace(/\/$/, '') };
  }

  buildUrl(path: string, params: Params): string {
    const url = new URL(`${this.profile.url}/rest/${path}`);
    for (const [k, v] of Object.entries({ ...authParams(this.profile), ...params })) {
      if (v !== undefined && v !== null) url.searchParams.set(k, String(v));
    }
    return url.toString();
  }

  async request(path: string, params: Params): Promise<R> {
    const response = await fetch(this.buildUrl(path, params));
    if (!response.ok) throw new Error(`Subsonic request failed with status ${response.status}`);
    const json = (await response.json()) as R;
    const body = json?.['subsonic-response'] as R | undefined;
    if (!body) throw new Error('Invalid Subsonic response payload');
    if (body.status !== 'ok') {
      throw new Error((body?.error as R)?.message as string ?? 'Subsonic request failed');
    }
    return body;
  }

  coverArtUrl(id: string, size = 240): string {
    if (!id) return '';
    return this.buildUrl('getCoverArt', { id, size });
  }

  streamUrl(id: string, maxBitRate = 320): string {
    if (!id) return '';
    return this.buildUrl('stream', { id, maxBitRate });
  }
}

export function asArray<T>(value: T | T[] | undefined | null): T[] {
  if (!value) return [];
  return Array.isArray(value) ? value : [value];
}

export function mapSong(song: R) {
  return {
    id: String(song?.id ?? ''),
    title: String(song?.title ?? ''),
    artist: String(song?.artist ?? ''),
    album: String(song?.album ?? ''),
    albumId: String(song?.albumId ?? ''),
    coverArt: String(song?.coverArt ?? song?.id ?? ''),
    duration: Number(song?.duration ?? 0)
  };
}

export function mapAlbum(album: R) {
  return {
    id: String(album?.id ?? ''),
    name: String(album?.name ?? ''),
    artist: String(album?.artist ?? ''),
    artistId: String(album?.artistId ?? ''),
    coverArt: String(album?.coverArt ?? album?.id ?? ''),
    songCount: Number(album?.songCount ?? 0),
    duration: Number(album?.duration ?? 0),
    year: album?.year ? Number(album.year) : undefined
  };
}
