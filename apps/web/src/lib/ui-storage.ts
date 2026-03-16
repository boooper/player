import { isTauri } from '$lib/tauri';

type JsonValue = string | number | boolean | null | JsonValue[] | { [key: string]: JsonValue };

type StoreLike = {
  get<T>(key: string): Promise<T | null>;
  set(key: string, value: JsonValue): Promise<void>;
};

let storePromise: Promise<StoreLike | null> | null = null;

async function getStore(): Promise<StoreLike | null> {
  if (!isTauri()) return null;
  if (!storePromise) {
    storePromise = import('@tauri-apps/plugin-store')
      .then(({ load }) => load('ui-state.json', { autoSave: 100, defaults: {} }))
      .catch(() => null);
  }
  return storePromise;
}

function readBrowserJson<T>(keys: string[], fallback: T): T {
  if (typeof localStorage === 'undefined') return fallback;
  for (const key of keys) {
    const raw = localStorage.getItem(key);
    if (raw === null) continue;
    try {
      return JSON.parse(raw) as T;
    } catch {
      return fallback;
    }
  }
  return fallback;
}

function clearBrowserKeys(keys: string[]): void {
  if (typeof localStorage === 'undefined') return;
  for (const key of keys) {
    localStorage.removeItem(key);
  }
}

export async function readUiJson<T>(key: string, fallback: T, legacyKeys: string[] = []): Promise<T> {
  const store = await getStore();
  if (store) {
    const stored = await store.get<T>(key);
    if (stored !== null && stored !== undefined) return stored;
  }

  const browserKeys = [key, ...legacyKeys];
  const browserValue = readBrowserJson(browserKeys, fallback);
  if (store && browserValue !== fallback) {
    await store.set(key, browserValue as JsonValue);
    clearBrowserKeys(browserKeys);
  }
  return browserValue;
}

export async function writeUiJson<T extends JsonValue>(key: string, value: T, legacyKeys: string[] = []): Promise<void> {
  const store = await getStore();
  if (store) {
    await store.set(key, value);
  } else if (typeof localStorage !== 'undefined') {
    localStorage.setItem(key, JSON.stringify(value));
  }
  clearBrowserKeys(legacyKeys);
}
