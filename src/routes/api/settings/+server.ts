import { json, error } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { getSettings, setSetting } from '$lib/server/settings';

const PUBLIC_KEYS = ['LASTFM_API_KEY', 'RECOMMENDATION_PROVIDER', 'METADATA_PROVIDER'] as const;
const ALL_KEYS = [
  'LASTFM_API_KEY',
  'LASTFM_SHARED_SECRET',
  'RECOMMENDATION_PROVIDER',
  'METADATA_PROVIDER',
  'SUBSONIC_BASE_URL',
  'SUBSONIC_USERNAME',
  'SUBSONIC_USE_PASSWORD_AUTH'
] as const;

export const GET: RequestHandler = async () => {
  const settings = await getSettings([...ALL_KEYS]);
  // Mask password — never expose it via GET
  return json({
    LASTFM_API_KEY: settings.LASTFM_API_KEY,
    // Never expose the shared secret — only indicate whether it is set
    LASTFM_SHARED_SECRET_CONFIGURED: settings.LASTFM_SHARED_SECRET ? 'true' : 'false',
    RECOMMENDATION_PROVIDER: settings.RECOMMENDATION_PROVIDER || 'lastfm',
    METADATA_PROVIDER: settings.METADATA_PROVIDER || 'both',
    SUBSONIC_BASE_URL: settings.SUBSONIC_BASE_URL,
    SUBSONIC_USERNAME: settings.SUBSONIC_USERNAME,
    SUBSONIC_USE_PASSWORD_AUTH: settings.SUBSONIC_USE_PASSWORD_AUTH || 'false'
  });
};

export const PUT: RequestHandler = async ({ request }) => {
  let body: Record<string, string>;
  try {
    body = await request.json();
  } catch {
    error(400, 'Invalid JSON body');
  }

  const allowed = new Set([...ALL_KEYS, 'SUBSONIC_PASSWORD']);
  const updates = Object.entries(body).filter(([key]) => allowed.has(key));

  if (updates.length === 0) {
    error(400, 'No valid setting keys provided');
  }

  await Promise.all(updates.map(([key, value]) => setSetting(key, String(value))));
  return json({ updated: updates.map(([key]) => key) });
};
