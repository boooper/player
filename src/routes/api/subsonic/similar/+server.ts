import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';

import { getSubsonicClient } from '$lib/server/subsonic';

export const GET: RequestHandler = async ({ url }) => {
  const songId = String(url.searchParams.get('songId') ?? '').trim();
  const count = Number(url.searchParams.get('count') ?? 20);

  if (!songId) {
    return json({ error: 'songId is required.' }, { status: 400 });
  }

  try {
    const client = await getSubsonicClient();
    const songs = await client.getSimilarSongs(songId, count);
    const hydrated = songs.map((song) => ({
      ...song,
      coverArtUrl: client.coverArtUrl(song.coverArt),
      streamUrl: client.streamUrl(song.id)
    }));

    return json({ songs: hydrated });
  } catch (error) {
    const message = error instanceof Error ? error.message : 'Subsonic similar lookup failed.';
    return json({ error: message }, { status: 502 });
  }
};
