/**
 * Server-side Last.fm module.
 * Handles authenticated calls (OAuth, scrobbling, user taste).
 * All signing happens here so the shared secret never leaves the server.
 */
import crypto from 'node:crypto';
import { getSetting, setSetting } from './settings';

const LASTFM_API_URL = 'https://ws.audioscrobbler.com/2.0/';

// ─── Signing ──────────────────────────────────────────────────────────────────

/**
 * Build the api_sig for a Last.fm authenticated call.
 * All params (excluding `format` and `callback`) are sorted alphabetically,
 * concatenated as key+value pairs, then the shared secret is appended and
 * the whole string is MD5-hashed.
 */
function sign(params: Record<string, string>, secret: string): string {
  const str =
    Object.entries(params)
      .filter(([k]) => k !== 'format' && k !== 'callback')
      .sort(([a], [b]) => a.localeCompare(b))
      .map(([k, v]) => `${k}${v}`)
      .join('') + secret;
  return crypto.createHash('md5').update(str, 'utf8').digest('hex');
}

async function getCredentials(): Promise<{ apiKey: string; secret: string }> {
  const apiKey = await getSetting('LASTFM_API_KEY');
  const secret = await getSetting('LASTFM_SHARED_SECRET');
  if (!apiKey || !secret) {
    throw new Error('Last.fm API key and shared secret are required');
  }
  return { apiKey, secret };
}

/** POST to Last.fm API (used for write operations and auth) */
async function signedPost(params: Record<string, string>): Promise<any> {
  const { apiKey, secret } = await getCredentials();
  const allParams = { ...params, api_key: apiKey };
  const sig = sign(allParams, secret);
  const body = new URLSearchParams({ ...allParams, api_sig: sig, format: 'json' });

  const response = await fetch(LASTFM_API_URL, {
    method: 'POST',
    headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
    body: body.toString()
  });
  if (!response.ok) throw new Error(`Last.fm request failed: ${response.status}`);
  const json = await response.json();
  if (json.error) throw new Error(json.message ?? `Last.fm error ${json.error}`);
  return json;
}

/** Signed GET — for read operations that need auth (e.g. user.getTopArtists) */
async function signedGet(params: Record<string, string>): Promise<any> {
  const { apiKey, secret } = await getCredentials();
  const allParams = { ...params, api_key: apiKey };
  const sig = sign(allParams, secret);

  const url = new URL(LASTFM_API_URL);
  Object.entries({ ...allParams, api_sig: sig, format: 'json' }).forEach(([k, v]) =>
    url.searchParams.set(k, v)
  );
  const response = await fetch(url);
  if (!response.ok) throw new Error(`Last.fm request failed: ${response.status}`);
  const json = await response.json();
  if (json.error) throw new Error(json.message ?? `Last.fm error ${json.error}`);
  return json;
}

// ─── Auth ─────────────────────────────────────────────────────────────────────

/**
 * Step 1 of the auth flow: obtain a short-lived request token and the URL
 * the user should visit to authorize the app on Last.fm.
 */
export async function getAuthToken(): Promise<{ token: string; authUrl: string }> {
  const { apiKey, secret } = await getCredentials();
  const params = { method: 'auth.getToken', api_key: apiKey };
  const sig = sign(params, secret);

  const url = new URL(LASTFM_API_URL);
  Object.entries({ ...params, api_sig: sig, format: 'json' }).forEach(([k, v]) =>
    url.searchParams.set(k, v)
  );
  const response = await fetch(url);
  const json = await response.json();
  if (json.error) throw new Error(json.message ?? 'Failed to get auth token');

  const token: string = json.token;
  return {
    token,
    authUrl: `https://www.last.fm/api/auth/?api_key=${apiKey}&token=${token}`
  };
}

/**
 * Step 2: exchange a user-authorized request token for a permanent session key.
 * Saves the session key and username to the settings store.
 */
export async function getSession(token: string): Promise<{ username: string }> {
  const json = await signedPost({ method: 'auth.getSession', token });
  const username: string = String(json.session?.name ?? '');
  const sessionKey: string = String(json.session?.key ?? '');
  if (!sessionKey) throw new Error('No session key returned from Last.fm');

  await Promise.all([
    setSetting('LASTFM_SESSION_KEY', sessionKey),
    setSetting('LASTFM_USERNAME', username)
  ]);
  return { username };
}

/** Clear stored session (disconnect account) */
export async function clearSession(): Promise<void> {
  await Promise.all([
    setSetting('LASTFM_SESSION_KEY', ''),
    setSetting('LASTFM_USERNAME', '')
  ]);
}

export async function isConnected(): Promise<boolean> {
  const sk = await getSetting('LASTFM_SESSION_KEY');
  return Boolean(sk);
}

export async function getStoredUsername(): Promise<string> {
  return getSetting('LASTFM_USERNAME');
}

// ─── Scrobbling ───────────────────────────────────────────────────────────────

export async function updateNowPlaying(
  artist: string,
  track: string,
  album?: string,
  duration?: number
): Promise<void> {
  const sk = await getSetting('LASTFM_SESSION_KEY');
  if (!sk) return; // Not connected — silently skip

  const params: Record<string, string> = {
    method: 'track.updateNowPlaying',
    sk,
    artist,
    track
  };
  if (album) params.album = album;
  if (duration) params.duration = String(Math.round(duration));

  await signedPost(params);
}

export async function scrobble(
  artist: string,
  track: string,
  timestamp: number,
  album?: string,
  duration?: number
): Promise<void> {
  const sk = await getSetting('LASTFM_SESSION_KEY');
  if (!sk) return; // Not connected — silently skip

  const params: Record<string, string> = {
    method: 'track.scrobble',
    sk,
    artist,
    track,
    timestamp: String(Math.floor(timestamp))
  };
  if (album) params.album = album;
  if (duration) params.duration = String(Math.round(duration));

  await signedPost(params);
}

// ─── Track love ───────────────────────────────────────────────────────────────

/**
 * Marks a track as "loved" on the connected Last.fm account.
 * Silently no-ops if no session key is stored.
 */
export async function loveTrack(artist: string, track: string): Promise<void> {
  const sk = await getSetting('LASTFM_SESSION_KEY');
  if (!sk) return;
  await signedPost({ method: 'track.love', sk, artist, track });
}

/**
 * Removes the "loved" status from a track on the connected Last.fm account.
 * Silently no-ops if no session key is stored.
 */
export async function unloveTrack(artist: string, track: string): Promise<void> {
  const sk = await getSetting('LASTFM_SESSION_KEY');
  if (!sk) return;
  await signedPost({ method: 'track.unlove', sk, artist, track });
}

// ─── User taste ───────────────────────────────────────────────────────────────

/**
 * Returns the user's top artists over the given period.
 * Used to seed / boost Smart Shuffle recommendations.
 */
export async function getUserTopArtists(
  period: 'overall' | '7day' | '1month' | '3month' | '6month' | '12month' = '3month',
  limit = 50
): Promise<string[]> {
  const sk = await getSetting('LASTFM_SESSION_KEY');
  if (!sk) return [];

  const json = await signedGet({
    method: 'user.getTopArtists',
    sk,
    period,
    limit: String(limit)
  });
  const artists = Array.isArray(json?.topartists?.artist) ? json.topartists.artist : [];
  return artists.map((a: any) => String(a?.name ?? '')).filter(Boolean);
}

/**
 * Returns the user's top tracks (artist + track pairs) over the given period.
 * Used as additional recommendation seeds beyond the currently-playing song.
 */
export async function getUserTopTracks(
  period: 'overall' | '7day' | '1month' | '3month' | '6month' | '12month' = '3month',
  limit = 20
): Promise<Array<{ artist: string; title: string }>> {
  const sk = await getSetting('LASTFM_SESSION_KEY');
  if (!sk) return [];

  const json = await signedGet({
    method: 'user.getTopTracks',
    sk,
    period,
    limit: String(limit)
  });
  const tracks = Array.isArray(json?.toptracks?.track) ? json.toptracks.track : [];
  return tracks
    .map((t: any) => ({
      artist: String(t?.artist?.name ?? ''),
      title: String(t?.name ?? '')
    }))
    .filter((t: { artist: string; title: string }) => t.artist && t.title);
}
