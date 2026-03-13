import { Hono } from 'hono';
import { prisma, getSettings } from '../store.js';

const stats = new Hono();

stats.get('/', async (c) => {
  const [count, settings] = await Promise.all([
    prisma.likedArtist.count(),
    getSettings()
  ]);
  return c.json({
    likedArtists: count,
    playlistCount: null,
    totalPlaylistSongs: null,
    starredSongs: null,
    lastFmConfigured: Boolean(settings.LASTFM_API_KEY)
  });
});

export default stats;
