import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';

import { getSubsonicClient } from '$lib/server/subsonic';

export const GET: RequestHandler = async () => {
  try {
    const client = await getSubsonicClient();
    const songs = await client.getStarredSongs();
    const hydrated = songs.map((song) => ({
      ...song,
      coverArtUrl: client.coverArtUrl(song.coverArt),
      streamUrl: client.streamUrl(song.id)
    }));

    return json({ songs: hydrated });
  } catch (error) {
    const message = error instanceof Error ? error.message : 'Failed to load starred songs.';
    return json({ error: message }, { status: 502 });
  }
};
