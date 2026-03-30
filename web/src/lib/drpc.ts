import { derived, get } from 'svelte/store';
import { queue, currentIndex, isPlaying, currentTime, duration } from './stores/player';
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

  // Only fetch artist artwork if cover art isn't available.
  const artistImageUrl = isPublicUrl(song.coverArtUrl)
    ? ''
    : await getArtistArtwork(song.artist).catch(() => '');
  // Bail if cleanup ran while we were fetching artwork.
  if (isAborted()) return;

  const largeImage =
    isPublicUrl(song.coverArtUrl) ? song.coverArtUrl :
    isPublicUrl(artistImageUrl) ? artistImageUrl :
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

  if (playing) {
    const positionSec = get(currentTime);
    const totalSec = song.duration > 0 ? song.duration : get(duration);
    const now = Date.now();
    const start = now - positionSec * 1000;
    const end = totalSec > 0 ? start + totalSec * 1000 : undefined;
    activity.setTimestamps(new Timestamps(start, end));
  }

  if (isAborted()) return;
  await setActivity(activity);
}

export async function stopDrpc(): Promise<void> {
  if (!started) return;
  const { clearActivity, stop } = await import('tauri-plugin-drpc');
  await clearActivity().catch((e) => console.warn('[drpc] clearActivity failed:', e));
  await stop().catch((e) => console.warn('[drpc] stop failed:', e));
  started = false;
  startPromise = null;
}

/**
 * Call once from the root layout. Sets up Discord RPC and keeps it in sync
 * with the player. No-ops when running in the browser.
 */
export function initDrpc(): () => void {
  if (!isTauri()) return () => {};
  // Discord RPC plugin is desktop-only — not bundled on Android/iOS
  if (/android|iphone|ipad/i.test(navigator.userAgent)) return () => {};

  let aborted = false;
  let latestSong: Song | null = null;
  let latestPlaying = false;
  let lastObservedTime = 0;
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
        await syncActivity(latestSong, latestPlaying, () => aborted);
      } catch (error) {
        console.error(error);
      }
    }
    syncing = false;
  }

  const unsubSong = currentSong.subscribe((v) => {
    latestSong = v;
    sync();
  });

  const unsubPlaying = isPlaying.subscribe((value) => {
    latestPlaying = value;
    sync();
  });

  const unsubTime = currentTime.subscribe((t) => {
    if (!latestPlaying) {
      lastObservedTime = t;
      return;
    }
    const delta = t - lastObservedTime;
    // Backward seek or forward jump >3s = user seeked
    if (delta < -0.5 || delta > 3) {
      sync();
    }
    lastObservedTime = t;
  });

  return () => {
    aborted = true;
    unsubPlaying();
    unsubSong();
    unsubTime();
    if (started || startPromise) {
      import('tauri-plugin-drpc')
        .then(async ({ clearActivity, stop }) => {
          await clearActivity().catch((e) => console.warn('[drpc] clearActivity failed:', e));
          await stop().catch((e) => console.warn('[drpc] stop failed:', e));
        })
        .catch((e) => console.warn('[drpc] cleanup import failed:', e));
      started = false;
      startPromise = null;
    }
  };
}
