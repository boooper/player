import { check, type Update } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';
import { isTauri } from '$lib/tauri';

export type AppUpdateInfo = {
  version: string;
  currentVersion: string;
  body?: string | null;
  date?: string | null;
};

// Cached update object so install doesn't need to re-check
let pendingUpdate: Update | null = null;

function normalizeUpdate(update: Update): AppUpdateInfo {
  return {
    version: update.version,
    currentVersion: update.currentVersion,
    body: update.body ?? null,
    date: update.date ?? null,
  };
}

export async function checkForAppUpdate(): Promise<AppUpdateInfo | null> {
  if (!isTauri()) return null;
  pendingUpdate = await check();
  return pendingUpdate ? normalizeUpdate(pendingUpdate) : null;
}

export async function installAppUpdate(): Promise<AppUpdateInfo | null> {
  if (!isTauri()) return null;

  // Reuse cached update from checkForAppUpdate; fall back to a fresh check
  if (!pendingUpdate) {
    pendingUpdate = await check();
  }
  if (!pendingUpdate) return null;

  const info = normalizeUpdate(pendingUpdate);
  await pendingUpdate.downloadAndInstall();
  pendingUpdate = null;
  await relaunch();
  return info;
}
