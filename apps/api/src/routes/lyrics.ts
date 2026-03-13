import { Hono } from 'hono';

const lyrics = new Hono();

lyrics.get('/', async (c) => {
  const artist = String(c.req.query('artist') ?? '').trim();
  const title = String(c.req.query('title') ?? '').trim();
  const album = String(c.req.query('album') ?? '').trim();
  const duration = String(c.req.query('duration') ?? '').trim();

  if (!artist || !title) return c.json({ error: 'artist and title are required' }, 400);

  if (album && duration) {
    const params = new URLSearchParams({ artist_name: artist, track_name: title, album_name: album, duration });
    try {
      const res = await fetch(`https://lrclib.net/api/get?${params}`, {
        headers: { 'Lrclib-Client': 'Naviarr' }
      });
      if (res.ok) return c.json(await res.json());
    } catch {
      // fall through to search
    }
  }

  try {
    const params = new URLSearchParams({ track_name: title, artist_name: artist });
    const res = await fetch(`https://lrclib.net/api/search?${params}`, {
      headers: { 'Lrclib-Client': 'Naviarr' }
    });
    if (!res.ok) return c.json(null);
    const results = (await res.json()) as Record<string, unknown>[];
    if (!Array.isArray(results) || results.length === 0) return c.json(null);
    const best = results.find((r) => r?.syncedLyrics) ?? results[0];
    return c.json(best);
  } catch {
    return c.json(null);
  }
});

export default lyrics;
