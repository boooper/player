import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { clearSession } from '$lib/server/lastfm';

export const POST: RequestHandler = async () => {
  await clearSession();
  return json({ ok: true });
};
