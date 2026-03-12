import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { getSubsonicClient } from '$lib/server/subsonic';

export const GET: RequestHandler = async ({ url }) => {
  const query = String(url.searchParams.get('q') ?? '').trim();
  const count = Number(url.searchParams.get('count') ?? 20);

  if (!query) {
    return json({ error: 'Query param `q` is required.' }, { status: 400 });
  }

  try {
    const client = await getSubsonicClient();
    const albums = await client.getArtistAlbums(query, count);
    const hydrated = albums.map((album) => ({
      ...album,
      coverArtUrl: client.coverArtUrl(album.coverArt, 300)
    }));
    return json({ albums: hydrated });
  } catch (error) {
    const message = error instanceof Error ? error.message : 'Failed to fetch albums.';
    return json({ error: message }, { status: 502 });
  }
};
