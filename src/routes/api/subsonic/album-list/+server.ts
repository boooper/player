import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { getSubsonicClient } from '$lib/server/subsonic';

export const GET: RequestHandler = async ({ url }) => {
  const type = (url.searchParams.get('type') ?? 'newest') as 'newest' | 'random' | 'frequent' | 'recent' | 'highest';
  const count = Math.min(Number(url.searchParams.get('count') ?? '20'), 100);

  try {
    const client = await getSubsonicClient();
    const albums = await client.getAlbumList(type, count);
    return json({
      albums: albums.map((a) => ({
        ...a,
        coverArtUrl: client.coverArtUrl(a.coverArt, 240)
      }))
    });
  } catch (error) {
    const message = error instanceof Error ? error.message : 'Failed to fetch album list.';
    return json({ error: message }, { status: 502 });
  }
};
