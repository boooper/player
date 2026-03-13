import { Hono } from 'hono';
import { prisma, sanitizeProfile } from '../store.js';

const profiles = new Hono();

profiles.get('/', async (c) => {
  const all = await prisma.profile.findMany({ orderBy: { id: 'asc' } });
  return c.json({ profiles: all.map(sanitizeProfile) });
});

profiles.post('/', async (c) => {
  const body = await c.req.json<Record<string, unknown>>();
  const name = String(body?.name ?? '').trim();
  const url = String(body?.url ?? '').trim();
  const username = String(body?.username ?? '').trim();
  const password = String(body?.password ?? '');
  const usePasswordAuth = body?.usePasswordAuth === true;

  if (!name) return c.json({ error: 'name is required' }, 400);
  if (!url) return c.json({ error: 'url is required' }, 400);
  if (!username) return c.json({ error: 'username is required' }, 400);
  if (!password) return c.json({ error: 'password is required' }, 400);

  const count = await prisma.profile.count();
  const profile = await prisma.profile.create({
    data: { name, url: url.replace(/\/$/, ''), username, password, usePasswordAuth, isActive: count === 0 }
  });
  return c.json({ profile: sanitizeProfile(profile) }, 201);
});

profiles.put('/:id', async (c) => {
  const id = Number(c.req.param('id'));
  const existing = await prisma.profile.findUnique({ where: { id } });
  if (!existing) return c.json({ error: 'Profile not found' }, 404);

  const body = await c.req.json<Record<string, unknown>>();
  const name = String(body?.name ?? '').trim();
  const url = String(body?.url ?? '').trim();
  const username = String(body?.username ?? '').trim();
  const password = String(body?.password ?? '').trim();

  const profile = await prisma.profile.update({
    where: { id },
    data: {
      ...(name && { name }),
      ...(url && { url: url.replace(/\/$/, '') }),
      ...(username && { username }),
      ...(password && { password }),
      ...(typeof body?.usePasswordAuth === 'boolean' && { usePasswordAuth: body.usePasswordAuth })
    }
  });
  return c.json({ profile: sanitizeProfile(profile) });
});

profiles.delete('/:id', async (c) => {
  const id = Number(c.req.param('id'));
  const profile = await prisma.profile.findUnique({ where: { id } });
  if (!profile) return c.json({ error: 'Profile not found' }, 404);

  const count = await prisma.profile.count();
  if (profile.isActive && count === 1) {
    return c.json({ error: 'Cannot delete the only server profile' }, 400);
  }

  await prisma.profile.delete({ where: { id } });

  if (profile.isActive) {
    const next = await prisma.profile.findFirst({ orderBy: { id: 'asc' } });
    if (next) await prisma.profile.update({ where: { id: next.id }, data: { isActive: true } });
  }
  return c.json({ success: true });
});

profiles.post('/:id/activate', async (c) => {
  const id = Number(c.req.param('id'));
  const target = await prisma.profile.findUnique({ where: { id } });
  if (!target) return c.json({ error: 'Profile not found' }, 404);
  await prisma.profile.updateMany({ data: { isActive: false } });
  await prisma.profile.update({ where: { id }, data: { isActive: true } });
  return c.json({ success: true });
});

export default profiles;
