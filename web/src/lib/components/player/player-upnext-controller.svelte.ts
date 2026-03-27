import { fromStore } from 'svelte/store';
import {
  queue,
  currentIndex,
  repeatMode,
  upNextEnabled,
  smartShuffleMode,
  appendToQueue,
  pruneQueueHistory
} from '$lib/stores/player';
import { fetchLikedArtists } from '$lib/servers';
import { getUpNextSongs } from '$lib/discovery';

type PlayerUpNextControllerOptions = {
  getLastFmApiKey: () => string | null | undefined;
};

const queueRef = fromStore(queue);
const currentIndexRef = fromStore(currentIndex);
const repeatModeRef = fromStore(repeatMode);
const upNextEnabledRef = fromStore(upNextEnabled);
const smartShuffleModeRef = fromStore(smartShuffleMode);

const LIKED_ARTISTS_TTL = 60_000;
let likedArtistsCache: { names: string[]; expiresAt: number } | null = null;

function getCachedLikedArtists(): Promise<string[]> {
  const now = Date.now();
  if (likedArtistsCache && likedArtistsCache.expiresAt > now) {
    return Promise.resolve(likedArtistsCache.names);
  }
  return fetchLikedArtists()
    .then((stored) => {
      const names = stored.map((a) => a.name);
      likedArtistsCache = { names, expiresAt: Date.now() + LIKED_ARTISTS_TTL };
      return names;
    })
    .catch(() => [] as string[]);
}

export function createPlayerUpNextController(options: PlayerUpNextControllerOptions) {
  let upNextFetching = false;
  let lastUpNextSeed = '';

  $effect(() => {
    const items = queueRef.current;
    const index = currentIndexRef.current;
    const repeat = repeatModeRef.current;
    const isUpNextEnabled = upNextEnabledRef.current;
    const isSmartShuffle = smartShuffleModeRef.current;

    if (isUpNextEnabled && !isSmartShuffle && index > 1) {
      pruneQueueHistory(1);
    }

    const nearEnd = items.length > 0 && index >= items.length - 1;
    if (!nearEnd || !isUpNextEnabled || repeat !== 'off' || !options.getLastFmApiKey()) return;

    const track = items[index];
    if (!track) return;
    const seed = `${track.artist}::${track.title}`;
    if (seed === lastUpNextSeed || upNextFetching) return;
    lastUpNextSeed = seed;
    upNextFetching = true;

    getCachedLikedArtists()
      .then((liked) =>
        getUpNextSongs({ artist: track.artist, title: track.title, likedArtists: liked, limit: 5 })
      )
      .then((songs) => { if (songs.length) appendToQueue(songs); })
      .catch(() => undefined)
      .finally(() => { upNextFetching = false; });
  });
}

export type PlayerUpNextController = ReturnType<typeof createPlayerUpNextController>;
