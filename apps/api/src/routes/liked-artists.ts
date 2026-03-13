import { Hono } from 'hono';
import { prisma } from '../store.js';

const likedArtists = new Hono();

likedArtists.get('/', async (c) => {
  const artists = await prisma.likedArtist.findMany({ orderBy: { name: 'asc' } });
  return c.json({ artists });
});

likedArtists.post('/', async (c) => {
  const body = await c.req.json<Record<string, unknown>>();
  const name = String(body?.name ?? '').trim();
  const source = String(body?.source ?? '').trim() || null;
  const externalId = String(body?.externalId ?? '').trim() || null;

  if (!name) return c.json({ error: 'Artist name is required.' }, 400);

  const artist = await prisma.likedArtist.upsert({
    where: { name },
    update: { source, externalId },
    create: { name, source, externalId }
  });
  return c.json({ artist }, 201);
});

likedArtists.delete('/:name', async (c) => {
  const name = decodeURIComponent(c.req.param('name')).trim();
  if (!name) return c.json({ error: 'Artist name is required.' }, 400);
  await prisma.likedArtist.deleteMany({ where: { name } });
  return new Response(null, { status: 204 });
});

export default likedArtists;
