import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { getAuthToken, getSession } from '$lib/server/lastfm';

/**
 * GET  — begin the OAuth flow: returns { token, authUrl }
 *         The client opens authUrl in the system browser, then calls POST.
 */
export const GET: RequestHandler = async () => {
  try {
    const result = await getAuthToken();
    return json(result);
  } catch (error) {
    const msg = error instanceof Error ? error.message : 'Failed to start Last.fm auth';
    return json({ error: msg }, { status: 502 });
  }
};

/**
 * POST { token } — exchange an authorized request token for a session key.
 *                  Saves the session key + username to the settings store.
 *                  Returns { username }.
 */
export const POST: RequestHandler = async ({ request }) => {
  const body = await request.json().catch(() => ({}));
  const token = String(body?.token ?? '').trim();
  if (!token) return json({ error: 'token is required' }, { status: 400 });

  try {
    const result = await getSession(token);
    return json({ username: result.username });
  } catch (error) {
    const msg = error instanceof Error ? error.message : 'Failed to complete Last.fm auth';
    return json({ error: msg }, { status: 502 });
  }
};
