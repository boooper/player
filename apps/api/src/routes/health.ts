import { Hono } from 'hono';
import { prisma, getSettings } from '../store.js';

const health = new Hono();

health.get('/', (c) => c.json({ ok: true }));

health.get('/services', async (c) => {
  const [profile, settings] = await Promise.all([
    prisma.profile.findFirst({ where: { isActive: true } }),
    getSettings()
  ]);
  return c.json({
    subsonic: profile ? 'offline' : 'missing',
    lastfm: settings.LASTFM_API_KEY ? 'offline' : 'missing'
  });
});

export default health;
