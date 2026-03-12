import { json, error } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { prisma } from '$lib/server/prisma';

function sanitize(p: {
  id: number;
  name: string;
  url: string;
  username: string;
  password: string;
  usePasswordAuth: boolean;
  isActive: boolean;
  createdAt: Date;
  updatedAt: Date;
}) {
  const { password: _pw, ...rest } = p;
  return rest;
}

export const PUT: RequestHandler = async ({ params, request }) => {
  const id = Number(params.id);
  if (!Number.isInteger(id)) throw error(400, 'Invalid id');

  let body: unknown;
  try {
    body = await request.json();
  } catch {
    throw error(400, 'Invalid JSON');
  }

  const { name, url, username, password, usePasswordAuth } = body as Record<string, unknown>;

  const existing = await prisma.subsonicProfile.findUnique({ where: { id } });
  if (!existing) throw error(404, 'Profile not found');

  const updateData: Record<string, unknown> = {};
  if (typeof name === 'string' && name.trim()) updateData.name = name.trim();
  if (typeof url === 'string' && url.trim()) updateData.url = url.trim().replace(/\/$/, '');
  if (typeof username === 'string' && username.trim()) updateData.username = username.trim();
  if (typeof password === 'string' && password) updateData.password = password;
  if (typeof usePasswordAuth === 'boolean') updateData.usePasswordAuth = usePasswordAuth;

  const updated = await prisma.subsonicProfile.update({ where: { id }, data: updateData });
  return json({ profile: sanitize(updated) });
};

export const DELETE: RequestHandler = async ({ params }) => {
  const id = Number(params.id);
  if (!Number.isInteger(id)) throw error(400, 'Invalid id');

  const existing = await prisma.subsonicProfile.findUnique({ where: { id } });
  if (!existing) throw error(404, 'Profile not found');

  if (existing.isActive) {
    const count = await prisma.subsonicProfile.count();
    if (count === 1) throw error(400, 'Cannot delete the only server profile');
    // Activate the next available profile before deleting
    const next = await prisma.subsonicProfile.findFirst({
      where: { id: { not: id } },
      orderBy: { createdAt: 'asc' }
    });
    if (next) {
      await prisma.subsonicProfile.update({ where: { id: next.id }, data: { isActive: true } });
    }
  }

  await prisma.subsonicProfile.delete({ where: { id } });
  return json({ success: true });
};
