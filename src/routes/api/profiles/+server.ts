import { json, error } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { prisma } from '$lib/server/prisma';

// Strip password from profile before sending to client
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

export const GET: RequestHandler = async () => {
  const profiles = await prisma.subsonicProfile.findMany({
    orderBy: { createdAt: 'asc' }
  });
  return json({ profiles: profiles.map(sanitize) });
};

export const POST: RequestHandler = async ({ request }) => {
  let body: unknown;
  try {
    body = await request.json();
  } catch {
    throw error(400, 'Invalid JSON');
  }

  const { name, url, username, password, usePasswordAuth } = body as Record<string, unknown>;

  if (typeof name !== 'string' || !name.trim()) throw error(400, 'name is required');
  if (typeof url !== 'string' || !url.trim()) throw error(400, 'url is required');
  if (typeof username !== 'string' || !username.trim()) throw error(400, 'username is required');
  if (typeof password !== 'string' || !password) throw error(400, 'password is required');

  // First profile becomes active automatically
  const count = await prisma.subsonicProfile.count();
  const profile = await prisma.subsonicProfile.create({
    data: {
      name: name.trim(),
      url: url.trim().replace(/\/$/, ''),
      username: username.trim(),
      password,
      usePasswordAuth: usePasswordAuth === true,
      isActive: count === 0
    }
  });

  return json({ profile: sanitize(profile) }, { status: 201 });
};
