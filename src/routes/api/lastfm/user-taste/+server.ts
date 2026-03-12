import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { getUserTopArtists, isConnected } from '$lib/server/lastfm';

/**
 * Returns the authenticated user's top artists (last 3 months, up to 50).
 * Used by Smart Shuffle to personalise recommendations with real listening
 * history rather than just the liked-artists list.
 */
export const GET: RequestHandler = async () => {
  if (!(await isConnected())) {
    return json({ artists: [] });
  }

  try {
    const artists = await getUserTopArtists('3month', 50);
    return json({ artists });
  } catch {
    // Don't fail — just return empty so recommendations still work
    return json({ artists: [] });
  }
};
