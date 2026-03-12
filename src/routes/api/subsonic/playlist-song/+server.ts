import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';

import { getSubsonicClient } from '$lib/server/subsonic';

export const POST: RequestHandler = async ({ request }) => {
  const { playlistId, songId } = await request.json();
  if (!playlistId || !songId) {
    return json({ error: 'playlistId and songId are required.' }, { status: 400 });
  }

  try {
    const client = await getSubsonicClient();
    await client.addSongToPlaylist(String(playlistId), String(songId));
    return json({ ok: true });
  } catch (error) {
    const message = error instanceof Error ? error.message : 'Failed to add song to playlist.';
    return json({ error: message }, { status: 502 });
  }
};
