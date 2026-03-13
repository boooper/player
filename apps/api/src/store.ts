import 'dotenv/config';
import { PrismaLibSql } from '@prisma/adapter-libsql';
import { PrismaClient } from './generated/prisma/index.js';

const adapter = new PrismaLibSql({ url: process.env.DATABASE_URL ?? 'file:./dev.db' });
export const prisma = new PrismaClient({ adapter });

export type { Profile, Setting, LikedArtist } from './generated/prisma/index.js';

export function sanitizeProfile<T extends { password: string }>(profile: T): Omit<T, 'password'> {
  const { password: _pw, ...rest } = profile;
  return rest as Omit<T, 'password'>;
}

export async function getSettings(): Promise<Record<string, string>> {
  const rows = await prisma.setting.findMany();
  return Object.fromEntries(rows.map((r) => [r.key, r.value]));
}

export async function activeProfile() {
  return prisma.profile.findFirst({ where: { isActive: true } });
}

