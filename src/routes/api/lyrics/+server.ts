import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';

export const GET: RequestHandler = async ({ url }) => {
  const artist = url.searchParams.get('artist') ?? '';
  const title = url.searchParams.get('title') ?? '';
  const album = url.searchParams.get('album') ?? '';
  const duration = url.searchParams.get('duration') ?? '';

  if (!artist || !title) {
    return json({ error: 'artist and title are required' }, { status: 400 });
  }

  // Try exact match first (with album + duration)
  if (album && duration) {
    const exactParams = new URLSearchParams({
      artist_name: artist,
      track_name: title,
      album_name: album,
      duration
    });
    try {
      const resp = await fetch(`https://lrclib.net/api/get?${exactParams}`, {
        headers: { 'Lrclib-Client': 'Naviarr' }
      });
      if (resp.ok) {
        const data = await resp.json();
        return json(data);
      }
    } catch {
      // fall through to search
    }
  }

  // Fall back to search (more lenient matching)
  const searchParams = new URLSearchParams({
    track_name: title,
    artist_name: artist
  });
  try {
    const resp = await fetch(`https://lrclib.net/api/search?${searchParams}`, {
      headers: { 'Lrclib-Client': 'Naviarr' }
    });
    if (!resp.ok) return json(null);
    const results: Array<{
      trackName: string;
      artistName: string;
      plainLyrics: string | null;
      syncedLyrics: string | null;
      instrumental: boolean;
      duration: number;
    }> = await resp.json();
    if (!results.length) return json(null);

    // Pick the result with synced lyrics if possible, otherwise the first
    const best = results.find((r) => r.syncedLyrics) ?? results[0];
    return json(best);
  } catch {
    return json(null);
  }
};
