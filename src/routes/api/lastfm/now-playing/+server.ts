import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { updateNowPlaying } from '$lib/server/lastfm';

export const POST: RequestHandler = async ({ request }) => {
  const body = await request.json().catch(() => ({}));
  const { artist, track, album, duration } = body;

  if (!artist || !track) {
    return json({ error: 'artist and track are required' }, { status: 400 });
  }

  try {
    await updateNowPlaying(
      String(artist),
      String(track),
      album ? String(album) : undefined,
      duration ? Number(duration) : undefined
    );
    return json({ ok: true });
  } catch (error) {
    // Silently absorb scrobbling errors — they should never break playback
    const msg = error instanceof Error ? error.message : 'Now playing update failed';
    return json({ error: msg }, { status: 502 });
  }
};
