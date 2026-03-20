import { derived } from 'svelte/store';
import { queue, currentIndex, isPlaying } from './stores/player';
import { isTauri } from './tauri';
import type { Song } from './servers';
import { getArtistArtwork } from './discovery';

const APPLICATION_ID = '1189808615012450304'; // replace with your Discord app ID

const currentSong = derived([queue, currentIndex], ([$queue, $idx]) => $queue[$idx] ?? null);

let started = false;
let startPromise: Promise<void> | null = null;

async function drpcStart() {
  if (started) return;
  if (!startPromise) {
    startPromise = (async () => {
      const { start } = await import('tauri-plugin-drpc');
      try {
        await start(APPLICATION_ID);
      } catch (err) {
        // "already spawned" means the thread is running — treat as success.
        if (String(err).toLowerCase().includes('already spawned')) {
          started = true;
          return;
        }
        throw err;
      }
      started = true;
    })().finally(() => {
      startPromise = null;
    });
  }

  await startPromise;
}

async function syncActivity(
  song: Song | null,
  playing: boolean,
  startedAt: number | null,
  isAborted: () => boolean
) {
  if (!song) {
    // Skip clearActivity if we never successfully started (Discord not open).
    if (!started) return;
    const { clearActivity } = await import('tauri-plugin-drpc');
    await clearActivity();
    return;
  }

  const { setActivity } = await import('tauri-plugin-drpc');
  const { Activity, Assets, Timestamps } = await import('tauri-plugin-drpc/activity');

  // Discord requires a publicly accessible URL; local/Tauri asset URLs will fail silently.
  const isPublicUrl = (url: string | undefined | null): boolean =>
    !!url &&
    url.startsWith('https://') &&
    !url.includes('localhost') &&
    !url.includes('127.0.0.1');

  // Prefer artist artwork (likely Last.fm public URL) over cover art (often local).
  const artistImageUrl = await getArtistArtwork(song.artist).catch(() => '');
  // Bail if cleanup ran while we were fetching artwork.
  if (isAborted()) return;

  const largeImage =
    isPublicUrl(artistImageUrl) ? artistImageUrl :
    isPublicUrl(song.coverArtUrl) ? song.coverArtUrl :
    'appicon';

  const assets = new Assets()
    .setLargeImage(largeImage)
    .setLargeText(song.album || song.title);

  if (typeof assets.setSmallImage === 'function' && typeof assets.setSmallText === 'function') {
    assets
      .setSmallImage(playing ? 'playing' : 'paused')
      .setSmallText(playing ? 'Playing' : 'Paused');
  }

  const activity = new Activity()
    .setDetails(song.title)
    .setState(`by ${song.artist}${song.album ? ` — ${song.album}` : ''}`)
    .setAssets(assets);

  if (playing && startedAt !== null) {
    activity.setTimestamps(new Timestamps(startedAt));
  }

  await setActivity(activity);
}

/**
 * Call once from the root layout. Sets up Discord RPC and keeps it in sync
 * with the player. No-ops when running in the browser.
 */
export function initDrpc(): () => void {
  if (!isTauri()) return () => {};

  let aborted = false;
  let latestSong: Song | null = null;
  let latestPlaying = false;
  let latestStartedAt: number | null = null;
  let syncing = false;
  let pending = false;

  async function sync() {
    pending = true;
    if (syncing) return;

    syncing = true;
    while (pending && !aborted) {
      pending = false;
      try {
        await drpcStart();
        if (aborted) break;
        await syncActivity(latestSong, latestPlaying, latestStartedAt, () => aborted);
      } catch (error) {
        console.error(error);
      }
    }
    syncing = false;
  }

  const unsubSong = currentSong.subscribe((v) => {
    const previousId = latestSong?.id ?? null;
    latestSong = v;
    if (v?.id !== previousId) {
      latestStartedAt = latestPlaying && v ? Date.now() : null;
    }
    sync();
  });

  const unsubPlaying = isPlaying.subscribe((value) => {
    const wasPlaying = latestPlaying;
    latestPlaying = value;
    if (latestSong) {
      if (value && !wasPlaying) {
        latestStartedAt = Date.now();
      } else if (!value) {
        latestStartedAt = null;
      }
    }
    sync();
  });

  return () => {
    aborted = true;
    unsubPlaying();
    unsubSong();
    if (started || startPromise) {
      import('tauri-plugin-drpc')
        .then(async ({ clearActivity, stop }) => {
          await clearActivity().catch((e) => console.warn('[drpc] clearActivity failed:', e));
          await stop().catch((e) => console.warn('[drpc] stop failed:', e));
        })
        .catch((e) => console.warn('[drpc] cleanup import failed:', e));
    }
    started = false;
    startPromise = null;
  };
}
