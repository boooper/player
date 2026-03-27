import { writable } from 'svelte/store';
import { clearMetadataCaches } from '$lib/data/metadata';
import { clearRecommendationCaches } from '$lib/data/recommendations';
import { clearLibraryCache } from '$lib/servers/library';

// Increment this to signal that library-backed UI data should be reloaded.
export const libraryRefresh = writable(0);

/** The server_type of the currently active profile (e.g. 'subsonic', 'octo_fiesta'). */
export const activeServerType = writable<string>('');

export function requestLibraryRefresh(): void {
  clearMetadataCaches();
  clearRecommendationCaches();
  clearLibraryCache();
  libraryRefresh.update((v) => v + 1);
}
