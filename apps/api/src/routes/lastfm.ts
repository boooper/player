import { Hono } from 'hono';
import { prisma, getSettings } from '../store.js';
import { requireCredentials, sign, signedPost, signedGet, sessionKey } from '../lastfm.js';

const lastfm = new Hono();

lastfm.get('/auth', async (c) => {
  const settings = await getSettings();
  try {
    const { apiKey, secret } = requireCredentials(settings);
    const params = { method: 'auth.getToken', api_key: apiKey };
    const sig = sign(params, secret);
    const url = new URL('https://ws.audioscrobbler.com/2.0/');
    for (const [k, v] of Object.entries({ ...params, api_sig: sig, format: 'json' })) {
      url.searchParams.set(k, v);
    }
    const res = await fetch(url);
    const json = (await res.json()) as Record<string, unknown> & { error?: number; message?: string; token?: string };
    if (json.error) throw new Error(json.message ?? 'Failed to get auth token');
    return c.json({
      token: String(json.token),
      authUrl: `https://www.last.fm/api/auth/?api_key=${apiKey}&token=${json.token}`
    });
  } catch (error) {
    return c.json({ error: error instanceof Error ? error.message : 'Failed to start Last.fm auth' }, 502);
  }
});

lastfm.post('/auth', async (c) => {
  const settings = await getSettings();
  try {
    const body = await c.req.json<Record<string, unknown>>();
    const token = String(body?.token ?? '').trim();
    if (!token) return c.json({ error: 'token is required' }, 400);
    const json = await signedPost(settings, { method: 'auth.getSession', token });
    const session = json.session as Record<string, string>;
    const username = String(session?.name ?? '');
    const sk = String(session?.key ?? '');
    if (!sk) throw new Error('No session key returned from Last.fm');
    await prisma.$transaction([
      prisma.setting.upsert({ where: { key: 'LASTFM_SESSION_KEY' }, update: { value: sk }, create: { key: 'LASTFM_SESSION_KEY', value: sk } }),
      prisma.setting.upsert({ where: { key: 'LASTFM_USERNAME' }, update: { value: username }, create: { key: 'LASTFM_USERNAME', value: username } })
    ]);
    return c.json({ username });
  } catch (error) {
    return c.json({ error: error instanceof Error ? error.message : 'Failed to complete Last.fm auth' }, 502);
  }
});

lastfm.post('/disconnect', async (c) => {
  await prisma.$transaction([
    prisma.setting.upsert({ where: { key: 'LASTFM_SESSION_KEY' }, update: { value: '' }, create: { key: 'LASTFM_SESSION_KEY', value: '' } }),
    prisma.setting.upsert({ where: { key: 'LASTFM_USERNAME' }, update: { value: '' }, create: { key: 'LASTFM_USERNAME', value: '' } })
  ]);
  return c.json({ ok: true });
});

lastfm.post('/now-playing', async (c) => {
  const settings = await getSettings();
  try {
    const body = await c.req.json<Record<string, unknown>>();
    const artist = String(body?.artist ?? '').trim();
    const track = String(body?.track ?? '').trim();
    if (!artist || !track) return c.json({ error: 'artist and track are required' }, 400);
    const sk = sessionKey(settings);
    if (sk) {
      const params: Record<string, string> = { method: 'track.updateNowPlaying', sk, artist, track };
      if (body?.album) params.album = String(body.album);
      if (body?.duration) params.duration = String(Math.round(Number(body.duration)));
      await signedPost(settings, params);
    }
    return c.json({ ok: true });
  } catch (error) {
    return c.json({ error: error instanceof Error ? error.message : 'Now playing update failed' }, 502);
  }
});

lastfm.post('/scrobble', async (c) => {
  const settings = await getSettings();
  try {
    const body = await c.req.json<Record<string, unknown>>();
    const artist = String(body?.artist ?? '').trim();
    const track = String(body?.track ?? '').trim();
    const timestamp = Number(body?.timestamp);
    if (!artist || !track || Number.isNaN(timestamp)) {
      return c.json({ error: 'artist, track, and timestamp are required' }, 400);
    }
    const sk = sessionKey(settings);
    if (sk) {
      const params: Record<string, string> = {
        method: 'track.scrobble',
        sk,
        artist,
        track,
        timestamp: String(Math.floor(timestamp))
      };
      if (body?.album) params.album = String(body.album);
      if (body?.duration) params.duration = String(Math.round(Number(body.duration)));
      await signedPost(settings, params);
    }
    return c.json({ ok: true });
  } catch (error) {
    return c.json({ error: error instanceof Error ? error.message : 'Scrobble failed' }, 502);
  }
});

lastfm.get('/user-taste', async (c) => {
  const settings = await getSettings();
  const sk = sessionKey(settings);
  const username = String(settings.LASTFM_USERNAME ?? '');
  if (!sk) return c.json({ connected: false, username, artists: [] });
  try {
    const json = await signedGet(settings, { method: 'user.getTopArtists', sk, period: '3month', limit: '50' });
    const topartists = json?.topartists as Record<string, unknown> | undefined;
    const artists = Array.isArray(topartists?.artist)
      ? (topartists.artist as Record<string, unknown>[]).map((a) => String(a?.name ?? '')).filter(Boolean)
      : [];
    return c.json({ connected: true, username, artists });
  } catch {
    return c.json({ connected: true, username, artists: [] });
  }
});

export default lastfm;
