import { Hono } from 'hono';
import { prisma, getSettings } from '../store.js';

const ALLOWED_KEYS = new Set([
  'LASTFM_API_KEY',
  'LASTFM_SHARED_SECRET',
  'RECOMMENDATION_PROVIDER',
  'METADATA_PROVIDER',
  'SUBSONIC_BASE_URL',
  'SUBSONIC_USERNAME',
  'SUBSONIC_USE_PASSWORD_AUTH',
  'SUBSONIC_PASSWORD'
]);

function settingsPayload(s: Record<string, string>) {
  return {
    LASTFM_API_KEY: s.LASTFM_API_KEY ?? '',
    LASTFM_SHARED_SECRET_CONFIGURED: s.LASTFM_SHARED_SECRET ? 'true' : 'false',
    RECOMMENDATION_PROVIDER: s.RECOMMENDATION_PROVIDER || 'lastfm',
    METADATA_PROVIDER: s.METADATA_PROVIDER || 'both',
    SUBSONIC_BASE_URL: s.SUBSONIC_BASE_URL ?? '',
    SUBSONIC_USERNAME: s.SUBSONIC_USERNAME ?? '',
    SUBSONIC_USE_PASSWORD_AUTH: s.SUBSONIC_USE_PASSWORD_AUTH ?? 'false'
  };
}

const settings = new Hono();

settings.get('/', async (c) => {
  const s = await getSettings();
  return c.json(settingsPayload(s));
});

settings.put('/', async (c) => {
  const body = await c.req.json<Record<string, unknown>>();
  const updates = Object.entries(body).filter(([key]) => ALLOWED_KEYS.has(key));
  if (updates.length === 0) return c.json({ error: 'No valid setting keys provided' }, 400);
  await prisma.$transaction(
    updates.map(([key, value]) =>
      prisma.setting.upsert({
        where: { key },
        update: { value: String(value ?? '') },
        create: { key, value: String(value ?? '') }
      })
    )
  );
  return c.json({ updated: updates.map(([key]) => key) });
});

export default settings;
