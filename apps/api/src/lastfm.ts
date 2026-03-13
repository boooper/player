import { createHash } from 'node:crypto';

const LASTFM_API_URL = 'https://ws.audioscrobbler.com/2.0/';

type R = Record<string, unknown>;

export function requireCredentials(settings: Record<string, string>): { apiKey: string; secret: string } {
  const apiKey = String(settings.LASTFM_API_KEY ?? '').trim();
  const secret = String(settings.LASTFM_SHARED_SECRET ?? '').trim();
  if (!apiKey || !secret) throw new Error('Last.fm API key and shared secret are required');
  return { apiKey, secret };
}

export function sign(params: Record<string, string>, secret: string): string {
  const str =
    Object.entries(params)
      .filter(([k]) => k !== 'format' && k !== 'callback')
      .sort(([a], [b]) => a.localeCompare(b))
      .map(([k, v]) => `${k}${v}`)
      .join('') + secret;
  return createHash('md5').update(str, 'utf8').digest('hex');
}

export async function signedPost(settings: Record<string, string>, params: Record<string, string>): Promise<R> {
  const { apiKey, secret } = requireCredentials(settings);
  const all = { ...params, api_key: apiKey };
  const sig = sign(all, secret);
  const body = new URLSearchParams({ ...all, api_sig: sig, format: 'json' });
  const response = await fetch(LASTFM_API_URL, {
    method: 'POST',
    headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
    body: body.toString()
  });
  if (!response.ok) throw new Error(`Last.fm request failed: ${response.status}`);
  const json = (await response.json()) as R & { error?: number; message?: string };
  if (json.error) throw new Error(json.message ?? `Last.fm error ${json.error}`);
  return json;
}

export async function signedGet(settings: Record<string, string>, params: Record<string, string>): Promise<R> {
  const { apiKey, secret } = requireCredentials(settings);
  const all = { ...params, api_key: apiKey };
  const sig = sign(all, secret);
  const url = new URL(LASTFM_API_URL);
  for (const [k, v] of Object.entries({ ...all, api_sig: sig, format: 'json' })) {
    url.searchParams.set(k, String(v));
  }
  const response = await fetch(url);
  if (!response.ok) throw new Error(`Last.fm request failed: ${response.status}`);
  const json = (await response.json()) as R & { error?: number; message?: string };
  if (json.error) throw new Error(json.message ?? `Last.fm error ${json.error}`);
  return json;
}

export function sessionKey(settings: Record<string, string>): string {
  return String(settings.LASTFM_SESSION_KEY ?? '').trim();
}
