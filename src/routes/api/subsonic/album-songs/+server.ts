import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { getSubsonicClient } from '$lib/server/subsonic';

export const GET: RequestHandler = async ({ url }) => {
  const albumId = String(url.searchParams.get('id') ?? '').trim();

  if (!albumId) {
    return json({ error: 'Query param `id` is required.' }, { status: 400 });
  }

  try {
    const client = await getSubsonicClient();
    const songs = await client.getAlbumSongs(albumId);
    const hydrated = songs.map((song) => ({
      ...song,
      coverArtUrl: client.coverArtUrl(song.coverArt),
      streamUrl: client.streamUrl(song.id)
    }));
    return json({ songs: hydrated });
  } catch (error) {
    const message = error instanceof Error ? error.message : 'Failed to fetch album songs.';
    return json({ error: message }, { status: 502 });
  }
};
