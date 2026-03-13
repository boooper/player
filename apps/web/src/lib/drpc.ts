/**
 * Discord Rich Presence integration.
 * Only active when running inside Tauri (desktop).
 * Reacts to player store changes to keep activity in sync.
 */
import { derived } from 'svelte/store';
import { queue, currentIndex, isPlaying } from './stores/player';
import { isTauri } from './tauri';
import type { SubsonicSong } from './api';

const APPLICATION_ID = '1189808615012450304'; // replace with your Discord app ID

const currentSong = derived([queue, currentIndex], ([$queue, $idx]) => $queue[$idx] ?? null);

let started = false;

async function drpcStart() {
  const { start } = await import('tauri-plugin-drpc');
  await start(APPLICATION_ID);
  started = true;
}

async function syncActivity(song: SubsonicSong | null) {
  if (!song) {
    const { clearActivity } = await import('tauri-plugin-drpc');
    await clearActivity();
    return;
  }

  const { setActivity } = await import('tauri-plugin-drpc');
  const { Activity, Assets, Timestamps } = await import('tauri-plugin-drpc/activity');

  const assets = new Assets()
    .setLargeImage(song.coverArtUrl)
    .setLargeText(song.album || song.title);

  const activity = new Activity()
    .setDetails(song.title)
    .setState(`by ${song.artist}${song.album ? ` — ${song.album}` : ''}`)
    .setAssets(assets)
    .setTimestamps(new Timestamps(Date.now()));

  await setActivity(activity);
}

/**
 * Call once from the root layout. Sets up Discord RPC and keeps it in sync
 * with the player. No-ops when running in the browser.
 */
export function initDrpc(): () => void {
  if (!isTauri()) return () => {};

  drpcStart().catch(console.error);

  let latestSong: SubsonicSong | null = null;

  function sync() {
    if (!started) return;
    syncActivity(latestSong).catch(console.error);
  }

  const unsubSong = currentSong.subscribe((v) => {
    latestSong = v;
    sync();
  });

  const unsubPlaying = isPlaying.subscribe(() => {
    sync();
  });

  return () => {
    unsubPlaying();
    unsubSong();
    if (started) {
      import('tauri-plugin-drpc').then(({ stop }) => stop()).catch(() => {});
    }
  };
}
