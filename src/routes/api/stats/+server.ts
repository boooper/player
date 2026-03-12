import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { prisma } from '$lib/server/prisma';
import { getSubsonicClient } from '$lib/server/subsonic';
import { getSetting } from '$lib/server/settings';

export const GET: RequestHandler = async () => {
  // Run all stats fetches in parallel — failures are tolerated
  const [likedArtistsResult, playlistsResult, starredResult, hasLastFm] = await Promise.allSettled([
    prisma.likedArtist.count(),
    getSubsonicClient().then((c) => c.getPlaylists()),
    getSubsonicClient().then((c) => c.getStarredSongs()),
    getSetting('LASTFM_API_KEY').then((k) => Boolean(k))
  ]);

  const likedArtists = likedArtistsResult.status === 'fulfilled' ? likedArtistsResult.value : 0;

  const playlistCount =
    playlistsResult.status === 'fulfilled' ? playlistsResult.value.length : null;

  const totalPlaylistSongs =
    playlistsResult.status === 'fulfilled'
      ? playlistsResult.value.reduce((sum, p) => sum + p.songCount, 0)
      : null;

  const starredSongs =
    starredResult.status === 'fulfilled' ? starredResult.value.length : null;

  const lastFmConfigured =
    hasLastFm.status === 'fulfilled' ? hasLastFm.value : false;

  return json({
    likedArtists,
    playlistCount,
    totalPlaylistSongs,
    starredSongs,
    lastFmConfigured
  });
};
