import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';

import { prisma } from '$lib/server/prisma';

export const GET: RequestHandler = async () => {
  const artists = await prisma.likedArtist.findMany({
    orderBy: { name: 'asc' }
  });

  return json({ artists });
};

export const POST: RequestHandler = async ({ request }) => {
  const body = (await request.json().catch(() => ({}))) as {
    name?: string;
    source?: string;
    externalId?: string;
  };

  const name = String(body?.name ?? '').trim();
  const source = String(body?.source ?? '').trim() || null;
  const externalId = String(body?.externalId ?? '').trim() || null;

  if (!name) {
    return json({ error: 'Artist name is required.' }, { status: 400 });
  }

  const artist = await prisma.likedArtist.upsert({
    where: { name },
    update: { source, externalId },
    create: { name, source, externalId }
  });

  return json({ artist }, { status: 201 });
};
