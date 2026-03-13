import { writable, get } from 'svelte/store';

import type { Song, Playlist } from '$lib/api';

export const queue = writable<Song[]>([]);
export const currentIndex = writable(0);
export const shouldAutoplay = writable(false);
export const isPlaying = writable(false);
export const currentTime = writable(0);
export const duration = writable(0);
export const volume = writable(0.8);
export const shuffleEnabled = writable(false);
export const repeatMode = writable<'off' | 'all' | 'one'>('off');

export const focusTrack = writable<{
  title: string;
  artist: string;
  imageUrl: string;
  source: 'lastfm' | 'library';
  album?: string;
} | null>(null);

export function setFocusTrack(track: {
  title: string;
  artist: string;
  imageUrl: string;
  source: 'lastfm' | 'library';
  album?: string;
} | null): void {
  focusTrack.set(track);
}

export function playQueue(items: Song[], startIndex = 0): void {
  if (!items.length) return;

  queue.set(items);
  currentIndex.set(Math.max(0, Math.min(startIndex, items.length - 1)));
  shouldAutoplay.set(true);
}

export function nextTrack(): void {
  const items = get(queue);
  if (!items.length) return;
  const current = get(currentIndex);
  const repeat = get(repeatMode);
  const shuffle = get(shuffleEnabled);

  if (repeat === 'one') {
    shouldAutoplay.set(true);
    return;
  }

  if (shuffle && items.length > 1) {
    let next = current;
    while (next === current) {
      next = Math.floor(Math.random() * items.length);
    }
    currentIndex.set(next);
    shouldAutoplay.set(true);
    return;
  }

  const atEnd = current >= items.length - 1;
  if (atEnd) {
    if (repeat === 'all') {
      currentIndex.set(0);
      shouldAutoplay.set(true);
    } else {
      shouldAutoplay.set(false);
      isPlaying.set(false);
    }
    return;
  }

  currentIndex.set(current + 1);
  shouldAutoplay.set(true);
}

export function prevTrack(): void {
  const items = get(queue);
  if (!items.length) return;
  const current = get(currentIndex);
  const shuffle = get(shuffleEnabled);

  if (shuffle && items.length > 1) {
    let prev = current;
    while (prev === current) {
      prev = Math.floor(Math.random() * items.length);
    }
    currentIndex.set(prev);
    shouldAutoplay.set(true);
    return;
  }

  currentIndex.update((index) => (index - 1 + items.length) % items.length);
  shouldAutoplay.set(true);
}

export function toggleShuffle(): void {
  shuffleEnabled.update((value) => !value);
}

export function cycleRepeatMode(): void {
  repeatMode.update((value) => {
    if (value === 'off') return 'all';
    if (value === 'all') return 'one';
    return 'off';
  });
}

export const upNextEnabled = writable(true);
export const smartShuffleMode = writable(false);
export const showLyrics = writable(false);
export const seekRequest = writable<number | null>(null);
export const subsonicPlaylists = writable<Playlist[]>([]);
export const starredSongIds = writable<Set<string>>(new Set());
export const showQueue = writable(false);

export type PlayingFrom = {
  type: 'playlist' | 'favorites' | 'artist' | 'album' | 'search' | null;
  name: string;
  href: string;
};
export const playingFrom = writable<PlayingFrom>({ type: null, name: '', href: '' });

// ─── Recently Played ─────────────────────────────────────────────────────────

export type RecentItem = {
  id: string;
  name: string;
  coverArtUrl: string;
  href: string;
  type: 'album' | 'playlist' | 'artist';
};

const RECENT_KEY = 'naviarr_recently_played';

function loadRecent(): RecentItem[] {
  if (typeof localStorage === 'undefined') return [];
  try {
    return JSON.parse(localStorage.getItem(RECENT_KEY) ?? '[]');
  } catch {
    return [];
  }
}

export const recentlyPlayed = writable<RecentItem[]>(loadRecent());

export function addRecentlyPlayed(item: RecentItem): void {
  recentlyPlayed.update((list) => {
    const filtered = list.filter((i) => i.id !== item.id);
    const next = [item, ...filtered].slice(0, 8);
    try { localStorage.setItem(RECENT_KEY, JSON.stringify(next)); } catch {}
    return next;
  });
}

export function playNextInQueue(song: Song): void {
  const items = get(queue);
  if (!items.length) {
    playQueue([song], 0);
    return;
  }
  const idx = get(currentIndex);
  queue.update((current) => {
    const next = [...current];
    next.splice(idx + 1, 0, song);
    return next;
  });
}

export function appendToQueue(items: Song[]): void {
  if (!items.length) return;
  queue.update((current) => [...current, ...items]);
}

/**
 * Prune songs that have already been played from the front of the queue,
 * keeping `keepPrev` songs before the current index so "previous" still works.
 * Adjusts currentIndex accordingly.
 */
export function pruneQueueHistory(keepPrev = 1): void {
  const idx = get(currentIndex);
  const removeCount = Math.max(0, idx - keepPrev);
  if (removeCount === 0) return;
  queue.update((items) => items.slice(removeCount));
  currentIndex.update((i) => i - removeCount);
}

export async function startRadio(
  song: Song,
  apiKey: string,
  limit = 25
): Promise<{ queued: number }> {
  const { fetchUpNextSongs } = await import('$lib/api');
  const tracks = await fetchUpNextSongs({ apiKey, artist: song.artist, title: song.title, limit });
  if (!tracks.length) return { queued: 0 };
  const all = [song, ...tracks];
  queue.set(all);
  currentIndex.set(0);
  shouldAutoplay.set(true);
  return { queued: tracks.length };
}
