import { json, error } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { prisma } from '$lib/server/prisma';

export const POST: RequestHandler = async ({ params }) => {
  const id = Number(params.id);
  if (!Number.isInteger(id)) throw error(400, 'Invalid id');

  const target = await prisma.subsonicProfile.findUnique({ where: { id } });
  if (!target) throw error(404, 'Profile not found');

  // Deactivate all, then activate the target — done in a transaction
  await prisma.$transaction([
    prisma.subsonicProfile.updateMany({ data: { isActive: false } }),
    prisma.subsonicProfile.update({ where: { id }, data: { isActive: true } })
  ]);

  return json({ success: true });
};
