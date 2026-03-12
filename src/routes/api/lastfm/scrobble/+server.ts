import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { scrobble } from '$lib/server/lastfm';

export const POST: RequestHandler = async ({ request }) => {
  const body = await request.json().catch(() => ({}));
  const { artist, track, timestamp, album, duration } = body;

  if (!artist || !track || timestamp == null) {
    return json({ error: 'artist, track, and timestamp are required' }, { status: 400 });
  }

  try {
    await scrobble(
      String(artist),
      String(track),
      Number(timestamp),
      album ? String(album) : undefined,
      duration ? Number(duration) : undefined
    );
    return json({ ok: true });
  } catch (error) {
    // Silently absorb scrobbling errors — they should never break playback
    const msg = error instanceof Error ? error.message : 'Scrobble failed';
    return json({ error: msg }, { status: 502 });
  }
};
