import type { RequestHandler } from './$types';
import { json } from '@sveltejs/kit';

import { prisma } from '$lib/server/prisma';

export const DELETE: RequestHandler = async ({ params }) => {
  const name = decodeURIComponent(String(params.name ?? '')).trim();

  if (!name) {
    return json({ error: 'Artist name is required.' }, { status: 400 });
  }

  await prisma.likedArtist.deleteMany({
    where: { name }
  });

  return new Response(null, { status: 204 });
};
