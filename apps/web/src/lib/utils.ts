import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]) {
	return twMerge(clsx(inputs));
}

export function formatClockDuration(seconds: number | null | undefined): string {
	if (!Number.isFinite(seconds) || !seconds || seconds <= 0) return "0:00";
	const safe = Math.max(0, Math.floor(seconds));
	const hours = Math.floor(safe / 3600);
	const minutes = Math.floor((safe % 3600) / 60);
	const secs = safe % 60;

	if (hours > 0) {
		return `${hours}:${String(minutes).padStart(2, "0")}:${String(secs).padStart(2, "0")}`;
	}

	return `${minutes}:${String(secs).padStart(2, "0")}`;
}

/** Creates an isolated TTL promise-cache. Entries are deduplicated and evicted on error. */
export function ttlCache() {
  const store = new Map<string, { expiresAt: number; promise: Promise<unknown> }>();
  return {
    get<T>(key: string, loader: () => Promise<T>, ttlMs: number): Promise<T> {
      const now = Date.now();
      const entry = store.get(key);
      if (entry && entry.expiresAt > now) return entry.promise as Promise<T>;
      const promise = loader().catch((err) => {
        if (store.get(key)?.promise === promise) store.delete(key);
        throw err;
      });
      store.set(key, { expiresAt: now + ttlMs, promise });
      return promise;
    },
    clear: () => store.clear(),
  };
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export type WithoutChild<T> = T extends { child?: any } ? Omit<T, "child"> : T;
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export type WithoutChildren<T> = T extends { children?: any } ? Omit<T, "children"> : T;
export type WithoutChildrenOrChild<T> = WithoutChildren<WithoutChild<T>>;
export type WithElementRef<T, U extends HTMLElement = HTMLElement> = T & { ref?: U | null };
