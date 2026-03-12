import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';

import { getSubsonicClient } from '$lib/server/subsonic';

export const GET: RequestHandler = async () => {
  try {
    const client = await getSubsonicClient();
    const playlists = await client.getPlaylists();
    const hydrated = playlists.map((playlist) => ({
      ...playlist,
      coverArtUrl: client.coverArtUrl(playlist.coverArt)
    }));

    return json({ playlists: hydrated });
  } catch (error) {
    const message = error instanceof Error ? error.message : 'Failed to load playlists.';
    return json({ error: message }, { status: 502 });
  }
};
