import { writable } from 'svelte/store';
import { clearDiscoveryCaches } from '$lib/discovery';
import { clearLibraryCache } from '$lib/servers/library';

// Increment this to signal that library-backed UI data should be reloaded.
export const libraryRefresh = writable(0);

export function requestLibraryRefresh(): void {
  clearDiscoveryCaches();
  clearLibraryCache();
  libraryRefresh.update((value) => value + 1);
}
