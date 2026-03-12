import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';

import { getSubsonicClient } from '$lib/server/subsonic';
import { loveTrack, unloveTrack } from '$lib/server/lastfm';

export const POST: RequestHandler = async ({ request }) => {
  const { id, unstar = false, artist, title } = await request.json();
  if (!id) return json({ error: 'Song id is required.' }, { status: 400 });

  try {
    const client = await getSubsonicClient();
    if (unstar) {
      await client.unstarSong(String(id));
    } else {
      await client.starSong(String(id));
    }

    // Mirror love/unlove to Last.fm in the background (fire-and-forget)
    if (artist && title) {
      const fn = unstar ? unloveTrack : loveTrack;
      fn(artist, title).catch(() => {/* intentionally silent */});
    }

    return json({ ok: true });
  } catch (error) {
    const message = error instanceof Error ? error.message : 'Failed to update favorite.';
    return json({ error: message }, { status: 502 });
  }
};
