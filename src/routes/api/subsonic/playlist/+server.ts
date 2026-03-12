import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';

import { getSubsonicClient } from '$lib/server/subsonic';

export const GET: RequestHandler = async ({ url }) => {
  const playlistId = String(url.searchParams.get('id') ?? '').trim();
  if (!playlistId) {
    return json({ error: 'Playlist id is required.' }, { status: 400 });
  }

  try {
    const client = await getSubsonicClient();
    const detail = await client.getPlaylistDetail(playlistId);
    const hydrated = detail.songs.map((song) => ({
      ...song,
      coverArtUrl: client.coverArtUrl(song.coverArt),
      streamUrl: client.streamUrl(song.id)
    }));

    return json({
      playlist: {
        id: detail.id,
        name: detail.name,
        songCount: detail.songCount,
        duration: detail.duration,
        coverArtUrl: client.coverArtUrl(detail.coverArt)
      },
      songs: hydrated
    });
  } catch (error) {
    const message = error instanceof Error ? error.message : 'Failed to load playlist songs.';
    return json({ error: message }, { status: 502 });
  }
};
