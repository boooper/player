import { fromStore } from 'svelte/store';
import {
  shuffleEnabled,
  smartShuffleMode,
  queue,
  currentIndex,
  enableShuffle,
  enableSmartShuffle,
  disableShuffle,
  playQueue,
  pruneQueueHistory,
  markSmartShuffleTracks
} from '$lib/stores/player';
import {
  fetchSimilarSongs,
  fetchArtistAlbums,
  fetchAlbumSongs,
  type Song
} from '$lib/servers';
import { fetchLikedArtists, lfmUserTaste } from '$lib/servers';
import { getUpNextSongs } from '$lib/discovery';
import { primarySongArtist } from '$lib/song-artists';
import { toast } from 'svelte-sonner';

type SongLike = Song & {
  id: string;
  title: string;
  artist: string;
  album?: string;
  albumId?: string | null;
};

type PlayerShuffleControllerOptions = {
  getCurrentTrack: () => SongLike | null;
  getLastFmApiKey: () => string | null | undefined;
};

const shuffleEnabledRef = fromStore(shuffleEnabled);
const smartShuffleModeRef = fromStore(smartShuffleMode);
const queueRef = fromStore(queue);
const currentIndexRef = fromStore(currentIndex);

const SMART_SHUFFLE_MIN_GAP = 2;
const SMART_SHUFFLE_MAX_GAP = 5;
const SMART_SHUFFLE_MIN_FETCH = 3;
const SMART_SHUFFLE_MAX_FETCH = 6;
const SMART_SHUFFLE_MAX_INSERT = 4;
const SMART_SHUFFLE_MIN_OFFSET = 1;
const SMART_SHUFFLE_MAX_OFFSET = 3;
const SMART_SHUFFLE_MIN_SPACING = 1;
const SMART_SHUFFLE_MAX_SPACING = 3;

function randomInt(min: number, max: number): number {
  return Math.floor(Math.random() * (max - min + 1)) + min;
}

function shuffleItems<T>(items: T[]): T[] {
  const next = [...items];
  for (let i = next.length - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1));
    [next[i], next[j]] = [next[j], next[i]];
  }
  return next;
}

export function createPlayerShuffleController(options: PlayerShuffleControllerOptions) {
  let smartShuffleFetching = $state(false);

  let smartShufflePlayCount = 0;
  let smartShuffleLastIdx = -1;
  let smartShuffleInflight = false;
  let smartShuffleNextInjectAfter = randomInt(SMART_SHUFFLE_MIN_GAP, SMART_SHUFFLE_MAX_GAP);

  const shuffleButtonClass = $derived(
    smartShuffleModeRef.current || shuffleEnabledRef.current
      ? 'text-primary'
      : 'text-muted-foreground hover:text-foreground'
  );

  function activateShuffle() {
    smartShuffleMode.set(false);
    enableShuffle();
  }

  function activateSmartShuffle() {
    enableSmartShuffle();
  }

  function deactivateShuffle() {
    disableShuffle();
  }

  async function shuffleArtist() {
    const currentTrack = options.getCurrentTrack();
    if (!currentTrack) return;

    const artist = primarySongArtist(currentTrack.artist);
    const toastId = toast.loading(`Loading ${artist}…`);

    try {
      const albums = await fetchArtistAlbums(artist, 50);
      const allSongs = (await Promise.all(albums.map((a) => fetchAlbumSongs(a.id)))).flat();

      if (!allSongs.length) throw new Error('No songs found');

      for (let i = allSongs.length - 1; i > 0; i--) {
        const j = Math.floor(Math.random() * (i + 1));
        [allSongs[i], allSongs[j]] = [allSongs[j], allSongs[i]];
      }

      smartShuffleMode.set(false);
      shuffleEnabled.set(true);
      playQueue(allSongs, 0);
      toast.success(`Shuffling ${artist}`, { id: toastId });
    } catch {
      toast.error('Failed to load artist songs', { id: toastId });
    }
  }

  async function shuffleAlbum() {
    const currentTrack = options.getCurrentTrack();
    if (!currentTrack?.albumId) return;

    const albumName = currentTrack.album;
    const toastId = toast.loading(`Loading ${albumName}…`);

    try {
      const songs = await fetchAlbumSongs(currentTrack.albumId);
      if (!songs.length) throw new Error('No songs found');

      for (let i = songs.length - 1; i > 0; i--) {
        const j = Math.floor(Math.random() * (i + 1));
        [songs[i], songs[j]] = [songs[j], songs[i]];
      }

      smartShuffleMode.set(false);
      shuffleEnabled.set(true);
      playQueue(songs, 0);
      toast.success(`Shuffling ${albumName}`, { id: toastId });
    } catch {
      toast.error('Failed to load album songs', { id: toastId });
    }
  }

  $effect(() => {
    const items = queueRef.current;
    const idx = currentIndexRef.current;
    const track = items[idx];
    const lastFmApiKey = options.getLastFmApiKey();

    if (!smartShuffleModeRef.current) {
      smartShufflePlayCount = 0;
      smartShuffleLastIdx = -1;
      smartShuffleNextInjectAfter = randomInt(SMART_SHUFFLE_MIN_GAP, SMART_SHUFFLE_MAX_GAP);
      return;
    }

    if (!track) return;
    if (idx === smartShuffleLastIdx) return;

    smartShuffleLastIdx = idx;
    smartShufflePlayCount++;

    pruneQueueHistory(1);

    if (smartShufflePlayCount < smartShuffleNextInjectAfter || smartShuffleInflight) return;

    smartShufflePlayCount = 0;
    smartShuffleNextInjectAfter = randomInt(SMART_SHUFFLE_MIN_GAP, SMART_SHUFFLE_MAX_GAP);
    smartShuffleInflight = true;
    smartShuffleFetching = true;

    const existingIds = new Set(items.map((s) => s.id));
    const capturedIdx = idx;
    const { artist, title, id } = track;

    const fetchLimit = randomInt(SMART_SHUFFLE_MIN_FETCH, SMART_SHUFFLE_MAX_FETCH);

    const doFetch: Promise<SongLike[]> = lastFmApiKey
      ? Promise.all([
          fetchLikedArtists().then((stored) => stored.map((a) => a.name)).catch((): string[] => []),
          lfmUserTaste().catch((): string[] => [])
        ]).then(([liked, taste]) => {
          const merged = [...new Set([...liked, ...taste])];
          return getUpNextSongs({ artist, title, likedArtists: merged, limit: fetchLimit }) as Promise<SongLike[]>;
        })
      : fetchSimilarSongs(id, fetchLimit) as Promise<SongLike[]>;

    doFetch
      .then((songs) => {
        const freshPool = shuffleItems(songs.filter((s) => !existingIds.has(s.id)));
        const fresh = freshPool.slice(
          0,
          randomInt(1, Math.min(SMART_SHUFFLE_MAX_INSERT, freshPool.length))
        );

        if (!fresh.length) return;

        markSmartShuffleTracks(fresh);

        queue.update((items) => {
          const next = [...items];
          let insertAt = Math.min(
            capturedIdx + randomInt(SMART_SHUFFLE_MIN_OFFSET, SMART_SHUFFLE_MAX_OFFSET),
            next.length
          );

          fresh.forEach((song) => {
            next.splice(insertAt, 0, song);
            insertAt = Math.min(
              insertAt + randomInt(SMART_SHUFFLE_MIN_SPACING, SMART_SHUFFLE_MAX_SPACING),
              next.length
            );
          });

          return next;
        });
      })
      .catch(() => undefined)
      .finally(() => {
        smartShuffleInflight = false;
        smartShuffleFetching = false;
      });
  });

  return {
    get smartShuffleFetching() {
      return smartShuffleFetching;
    },
    get shuffleButtonClass() {
      return shuffleButtonClass;
    },
    activateShuffle,
    activateSmartShuffle,
    deactivateShuffle,
    shuffleArtist,
    shuffleAlbum
  };
}

export type PlayerShuffleController = ReturnType<typeof createPlayerShuffleController>;