import { writable, get } from 'svelte/store';

import type { Song, Playlist } from '$lib/servers';
import { readUiJson, writeUiJson } from '$lib/ui-storage';

export const queue = writable<Song[]>([]);
export const currentIndex = writable(0);
export const shouldAutoplay = writable(false);
export const isPlaying = writable(false);
export const currentTime = writable(0);
export const duration = writable(0);
export const volume = writable(0.8);
export const shuffleEnabled = writable(false);
export const repeatMode = writable<'off' | 'all' | 'one'>('off');
export const smartShuffleTrackIds = writable<Set<string>>(new Set());

function shuffleList<T>(items: T[]): T[] {
  const next = [...items];
  for (let i = next.length - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1));
    [next[i], next[j]] = [next[j], next[i]];
  }
  return next;
}

function shuffleUpcomingQueue(): void {
  const items = get(queue);
  const idx = get(currentIndex);
  const upcoming = items.slice(idx + 1);
  if (upcoming.length <= 1) return;
  queue.set([...items.slice(0, idx + 1), ...shuffleList(upcoming)]);
}

function syncSmartShuffleTrackIds(items: Song[]): void {
  const itemIds = new Set(items.map((song) => song.id));
  smartShuffleTrackIds.update((ids) => new Set([...ids].filter((id) => itemIds.has(id))));
}

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
  smartShuffleTrackIds.set(new Set());
  currentIndex.set(Math.max(0, Math.min(startIndex, items.length - 1)));
  shouldAutoplay.set(true);
}

export function nextTrack(): void {
  const items = get(queue);
  if (!items.length) return;
  const current = get(currentIndex);
  const repeat = get(repeatMode);

  if (repeat === 'one') {
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

  currentIndex.update((index) => (index - 1 + items.length) % items.length);
  shouldAutoplay.set(true);
}

export function enableShuffle(): void {
  const alreadyEnabled = get(shuffleEnabled);
  shuffleEnabled.set(true);
  if (!alreadyEnabled) shuffleUpcomingQueue();
}

export function enableSmartShuffle(): void {
  const alreadyEnabled = get(shuffleEnabled);
  shuffleEnabled.set(true);
  smartShuffleMode.set(true);
  if (!alreadyEnabled) shuffleUpcomingQueue();
}

export function disableShuffle(): void {
  shuffleEnabled.set(false);
  smartShuffleMode.set(false);
}

export function markSmartShuffleTracks(items: Song[]): void {
  if (!items.length) return;
  smartShuffleTrackIds.update((ids) => new Set([...ids, ...items.map((song) => song.id)]));
}

export function toggleShuffle(): void {
  if (get(shuffleEnabled)) {
    disableShuffle();
    return;
  }
  enableShuffle();
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
export const togglePlayRequest = writable(0);
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

const RECENT_KEY = 'madrify_recently_played';
const LEGACY_RECENT_KEY = 'naviarr_recently_played';

export const recentlyPlayed = writable<RecentItem[]>([]);

export function addRecentlyPlayed(item: RecentItem): void {
  recentlyPlayed.update((list) => {
    const filtered = list.filter((i) => i.id !== item.id);
    const next = [item, ...filtered].slice(0, 8);
    void writeUiJson(RECENT_KEY, next, [LEGACY_RECENT_KEY]);
    return next;
  });
}

// ─── Recently Played Songs ────────────────────────────────────────────────────

const RECENT_SONGS_KEY = 'madrify_recently_played_songs';
const LEGACY_RECENT_SONGS_KEY = 'naviarr_recently_played_songs';

export const recentlyPlayedSongs = writable<Song[]>([]);

export function addRecentlyPlayedSong(song: Song): void {
  recentlyPlayedSongs.update((list) => {
    const filtered = list.filter((s) => s.id !== song.id);
    const next = [song, ...filtered].slice(0, 20);
    void writeUiJson(RECENT_SONGS_KEY, next, [LEGACY_RECENT_SONGS_KEY]);
    return next;
  });
}

export async function hydratePlayerUiState(): Promise<void> {
  const [recentItems, recentSongs] = await Promise.all([
    readUiJson<RecentItem[]>(RECENT_KEY, [], [LEGACY_RECENT_KEY]),
    readUiJson<Song[]>(RECENT_SONGS_KEY, [], [LEGACY_RECENT_SONGS_KEY])
  ]);
  recentlyPlayed.set(recentItems);
  recentlyPlayedSongs.set(recentSongs);
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
  queue.update((current) => {
    const next = [...current, ...items];
    syncSmartShuffleTrackIds(next);
    return next;
  });
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
  queue.update((items) => {
    const next = items.slice(removeCount);
    syncSmartShuffleTrackIds(next);
    return next;
  });
  currentIndex.update((i) => i - removeCount);
}

export async function startRadio(
  song: Song,
  apiKey: string,
  limit = 25
): Promise<{ queued: number }> {
  const { getUpNextSongs } = await import('$lib/discovery');
  const tracks = await getUpNextSongs({ artist: song.artist, title: song.title, limit });
  if (!tracks.length) return { queued: 0 };
  const all = [song, ...tracks];
  queue.set(all);
  smartShuffleTrackIds.set(new Set());
  currentIndex.set(0);
  shouldAutoplay.set(true);
  return { queued: tracks.length };
}
