import { DESKTOP_PLAYBACK_CACHE_UPDATED_EVENT, desktopPlaybackCachedIds, type Song } from '$lib/servers';
import { isTauri } from '$lib/tauri';

/**
 * Manages the set of locally-cached song IDs for desktop (Tauri) playback.
 * Registers a window event listener to track newly cached songs, and exposes
 * a `load()` method to initialise the set from a batch of songs.
 *
 * Instantiate once per component – the internal `$effect` ties the listener
 * lifetime to the owning component's lifecycle.
 */
export class DesktopCache {
  ids = $state<Set<string>>(new Set());
  readonly enabled = isTauri();

  constructor() {
    $effect(() => {
      if (!this.enabled) return;

      const handler = (event: Event) => {
        const songId = (event as CustomEvent<{ songId?: string }>).detail?.songId;
        if (!songId) return;
        this.ids.add(songId);
      };

      window.addEventListener(DESKTOP_PLAYBACK_CACHE_UPDATED_EVENT, handler);
      return () => window.removeEventListener(DESKTOP_PLAYBACK_CACHE_UPDATED_EVENT, handler);
    });
  }

  reset(): void {
    this.ids = new Set();
  }

  async load(songs: Song[], loadVersion: number, getVersion: () => number): Promise<void> {
    if (!this.enabled) {
      this.ids = new Set();
      return;
    }
    try {
      const result = await desktopPlaybackCachedIds(songs);
      if (loadVersion !== getVersion()) return;
      this.ids = new Set(result);
    } catch {
      if (loadVersion !== getVersion()) return;
      this.ids = new Set();
    }
  }
}
