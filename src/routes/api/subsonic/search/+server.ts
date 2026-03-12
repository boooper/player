import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';

import { getSubsonicClient } from '$lib/server/subsonic';

export const GET: RequestHandler = async ({ url }) => {
  const query = String(url.searchParams.get('q') ?? '').trim();
  const count = Number(url.searchParams.get('count') ?? 20);

  if (!query) {
    return json({ error: 'Search query `q` is required.' }, { status: 400 });
  }

  try {
    const client = await getSubsonicClient();
    const songs = await client.searchTracks(query, count);
    const hydrated = songs.map((song) => ({
      ...song,
      coverArtUrl: client.coverArtUrl(song.coverArt),
      streamUrl: client.streamUrl(song.id)
    }));

    return json({ songs: hydrated });
  } catch (error) {
    const message = error instanceof Error ? error.message : 'Subsonic search failed.';
    return json({ error: message }, { status: 502 });
  }
};
