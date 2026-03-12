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
    const detail = await client.getAlbumDetail(albumId);
    return json({
      album: {
        ...detail,
        coverArtUrl: client.coverArtUrl(detail.coverArt, 400)
      },
      songs: detail.songs.map((song) => ({
        ...song,
        coverArtUrl: client.coverArtUrl(song.coverArt),
        streamUrl: client.streamUrl(song.id)
      }))
    });
  } catch (error) {
    const message = error instanceof Error ? error.message : 'Failed to fetch album.';
    return json({ error: message }, { status: 502 });
  }
};
