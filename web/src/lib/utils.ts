import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]) {
	return twMerge(clsx(inputs));
}

export function initials(name: string): string {
	return name.split(' ').filter(Boolean).slice(0, 2).map((p) => p[0]?.toUpperCase() ?? '').join('');
}

export function normalizeString(value: string): string {
	return value.trim().toLowerCase();
}

export function withTimeout<T>(promise: Promise<T>, ms: number, label = ''): Promise<T> {
	return new Promise<T>((resolve, reject) => {
		const timeout = window.setTimeout(() => reject(new Error(`${label} timed out.`)), ms);
		promise.then(
			(value) => { window.clearTimeout(timeout); resolve(value); },
			(reason) => { window.clearTimeout(timeout); reject(reason); }
		);
	});
}

export function shuffleArray<T>(items: T[]): T[] {
	const next = [...items];
	for (let i = next.length - 1; i > 0; i--) {
		const j = Math.floor(Math.random() * (i + 1));
		[next[i], next[j]] = [next[j], next[i]];
	}
	return next;
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

/** Sum of all song durations in seconds. */
export function sumDuration(songs: { duration?: number | null }[]): number {
	return songs.reduce((acc, s) => acc + (s.duration ?? 0), 0);
}

/** Promise that resolves after `ms` milliseconds. */
export function delay(ms: number): Promise<void> {
	return new Promise((resolve) => setTimeout(resolve, ms));
}

/** Human-readable listener count: "1.2M monthly listeners", "450K monthly listeners". */
export function formatListenerCount(listeners: number): string {
	return listeners >= 1_000_000
		? `${(listeners / 1_000_000).toFixed(1)}M monthly listeners`
		: `${(listeners / 1_000).toFixed(0)}K monthly listeners`;
}

/** Returns a new array with duplicates removed, based on the given key function. */
export function uniqueBy<T>(items: T[], key: (item: T) => string): T[] {
	const seen = new Set<string>();
	return items.filter((item) => {
		const k = key(item);
		if (seen.has(k)) return false;
		seen.add(k);
		return true;
	});
}

/** Human-readable duration: "1 hr 30 min", "45 min". Returns empty string for invalid input. */
export function formatDurationHuman(totalSeconds: number | null | undefined): string {
	if (!totalSeconds || !isFinite(totalSeconds) || totalSeconds <= 0) return '';
	const h = Math.floor(totalSeconds / 3600);
	const m = Math.floor((totalSeconds % 3600) / 60);
	if (h > 0) return `${h} hr ${m} min`;
	return `${m} min`;
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
