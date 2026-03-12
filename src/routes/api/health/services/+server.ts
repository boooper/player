import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';

import { getSubsonicClient } from '$lib/server/subsonic';
import { getSetting } from '$lib/server/settings';

const LASTFM_API_URL = 'https://ws.audioscrobbler.com/2.0/';

async function checkLastFm(): Promise<'online' | 'offline' | 'missing'> {
  const apiKey = await getSetting('LASTFM_API_KEY');
  if (!apiKey) return 'missing';

  try {
    const url = new URL(LASTFM_API_URL);
    url.searchParams.set('method', 'chart.gettopartists');
    url.searchParams.set('api_key', apiKey);
    url.searchParams.set('format', 'json');
    url.searchParams.set('limit', '1');

    const response = await fetch(url, { signal: AbortSignal.timeout(5000) });
    if (!response.ok) return 'offline';

    const payload = await response.json();
    return payload?.error ? 'offline' : 'online';
  } catch {
    return 'offline';
  }
}

async function checkSubsonic(): Promise<'online' | 'offline' | 'missing'> {
  try {
    const client = await getSubsonicClient();
    const ok = await client.ping();
    return ok ? 'online' : 'offline';
  } catch (err) {
    const message = err instanceof Error ? err.message : '';
    if (message.includes('Missing required Subsonic configuration')) return 'missing';
    return 'offline';
  }
}

export const GET: RequestHandler = async () => {
  const [lastfm, subsonic] = await Promise.all([checkLastFm(), checkSubsonic()]);
  return json({ lastfm, subsonic });
};
