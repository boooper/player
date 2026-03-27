import { check } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';
import { isTauri } from '$lib/tauri';

export type AppUpdateInfo = {
  version: string;
  currentVersion: string;
  body?: string | null;
  date?: string | null;
};

function normalizeUpdate(update: Awaited<ReturnType<typeof check>>): AppUpdateInfo | null {
  if (!update) return null;
  return {
    version: update.version,
    currentVersion: update.currentVersion,
    body: update.body ?? null,
    date: update.date ?? null,
  };
}

export async function checkForAppUpdate(): Promise<AppUpdateInfo | null> {
  if (!isTauri()) return null;
  return normalizeUpdate(await check());
}

export async function installAppUpdate(): Promise<AppUpdateInfo | null> {
  if (!isTauri()) return null;

  const update = await check();
  if (!update) return null;

  await update.downloadAndInstall();
  await relaunch();
  return normalizeUpdate(update);
}
