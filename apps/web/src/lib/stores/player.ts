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
const SHOW_QUEUE_KEY = 'madrify_show_now_playing';
const PLAYBACK_SESSION_KEY = 'madrify_playback_session';
let hasHydratedPlayerUiState = false;

export type PlayingFrom = {
  type: 'playlist' | 'favorites' | 'artist' | 'album' | 'search' | null;
  name: string;
  href: string;
};
export const playingFrom = writable<PlayingFrom>({ type: null, name: '', href: '' });
export const restorePlaybackRequest = writable<{ songId: string; position: number } | null>(null);

type PlaybackSessionSnapshot = {
  queue: Song[];
  currentIndex: number;
  currentTime: number;
  playingFrom: PlayingFrom;
  focusTrack: {
    title: string;
    artist: string;
    imageUrl: string;
    source: 'lastfm' | 'library';
    album?: string;
  } | null;
};

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
export const pinnedPlaylistIds = writable<Set<string>>(new Set());

const PINNED_PLAYLISTS_KEY = 'madrify_pinned_playlists';

export function setPlaylistPinned(playlistId: string, pinned: boolean): void {
  if (!playlistId) return;
  pinnedPlaylistIds.update((ids) => {
    const next = new Set(ids);
    if (pinned) {
      next.add(playlistId);
    } else {
      next.delete(playlistId);
    }
    void writeUiJson(PINNED_PLAYLISTS_KEY, [...next]);
    return next;
  });
}

export function togglePlaylistPinned(playlistId: string): boolean {
  let nextPinned = false;
  pinnedPlaylistIds.update((ids) => {
    const next = new Set(ids);
    if (next.has(playlistId)) {
      next.delete(playlistId);
      nextPinned = false;
    } else {
      next.add(playlistId);
      nextPinned = true;
    }
    void writeUiJson(PINNED_PLAYLISTS_KEY, [...next]);
    return next;
  });
  return nextPinned;
}

export function addRecentlyPlayedSong(song: Song): void {
  recentlyPlayedSongs.update((list) => {
    const filtered = list.filter((s) => s.id !== song.id);
    const next = [song, ...filtered].slice(0, 20);
    void writeUiJson(RECENT_SONGS_KEY, next, [LEGACY_RECENT_SONGS_KEY]);
    return next;
  });
}

let persistPlaybackSessionTimer = 0;
let hasInstalledPlaybackSessionPersistence = false;

function buildPlaybackSessionSnapshot(): PlaybackSessionSnapshot | null {
  const items = get(queue);
  if (!items.length) return null;

  const index = Math.max(0, Math.min(get(currentIndex), items.length - 1));
  return {
    queue: items,
    currentIndex: index,
    currentTime: Math.max(0, get(currentTime)),
    playingFrom: get(playingFrom),
    focusTrack: get(focusTrack),
  };
}

function flushPersistPlaybackSession(): void {
  if (!hasHydratedPlayerUiState) return;
  clearTimeout(persistPlaybackSessionTimer);
  const snapshot = buildPlaybackSessionSnapshot();
  void writeUiJson(PLAYBACK_SESSION_KEY, snapshot);
}

function schedulePersistPlaybackSession(): void {
  if (!hasHydratedPlayerUiState) return;
  clearTimeout(persistPlaybackSessionTimer);
  persistPlaybackSessionTimer = window.setTimeout(() => {
    flushPersistPlaybackSession();
  }, 350);
}

function installPlaybackSessionPersistence(): void {
  if (hasInstalledPlaybackSessionPersistence || typeof window === 'undefined') return;
  hasInstalledPlaybackSessionPersistence = true;

  const flushIfHidden = () => {
    if (document.visibilityState === 'hidden') flushPersistPlaybackSession();
  };

  window.addEventListener('pagehide', flushPersistPlaybackSession);
  window.addEventListener('beforeunload', flushPersistPlaybackSession);
  document.addEventListener('visibilitychange', flushIfHidden);
}

export async function hydratePlayerUiState(): Promise<void> {
  installPlaybackSessionPersistence();
  const [recentItems, recentSongs, pinnedPlaylists, persistedShowQueue, playbackSession] = await Promise.all([
    readUiJson<RecentItem[]>(RECENT_KEY, [], [LEGACY_RECENT_KEY]),
    readUiJson<Song[]>(RECENT_SONGS_KEY, [], [LEGACY_RECENT_SONGS_KEY]),
    readUiJson<string[]>(PINNED_PLAYLISTS_KEY, []),
    readUiJson<boolean>(SHOW_QUEUE_KEY, false),
    readUiJson<PlaybackSessionSnapshot | null>(PLAYBACK_SESSION_KEY, null),
  ]);
  recentlyPlayed.set(recentItems);
  recentlyPlayedSongs.set(recentSongs);
  pinnedPlaylistIds.set(new Set(pinnedPlaylists.filter(Boolean)));
  showQueue.set(persistedShowQueue);
  if (playbackSession?.queue?.length) {
    const nextQueue = playbackSession.queue.filter((song) => song?.id && song.streamUrl);
    if (nextQueue.length) {
      const nextIndex = Math.max(0, Math.min(playbackSession.currentIndex ?? 0, nextQueue.length - 1));
      queue.set(nextQueue);
      currentIndex.set(nextIndex);
      currentTime.set(Math.max(0, playbackSession.currentTime ?? 0));
      isPlaying.set(false);
      shouldAutoplay.set(false);
      focusTrack.set(playbackSession.focusTrack ?? null);
      playingFrom.set(playbackSession.playingFrom ?? { type: null, name: '', href: '' });

      const activeSong = nextQueue[nextIndex];
      const position = Math.max(0, playbackSession.currentTime ?? 0);
      restorePlaybackRequest.set(
        activeSong && position > 0
          ? { songId: activeSong.id, position }
          : null
      );
    }
  }
  hasHydratedPlayerUiState = true;
}

showQueue.subscribe((open) => {
  if (!hasHydratedPlayerUiState) return;
  void writeUiJson(SHOW_QUEUE_KEY, open);
});

queue.subscribe(() => {
  schedulePersistPlaybackSession();
});

currentIndex.subscribe(() => {
  schedulePersistPlaybackSession();
});

currentTime.subscribe(() => {
  schedulePersistPlaybackSession();
});

playingFrom.subscribe(() => {
  schedulePersistPlaybackSession();
});

focusTrack.subscribe(() => {
  schedulePersistPlaybackSession();
});

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
