import { writable } from 'svelte/store';
import { clearDiscoveryCaches } from '$lib/discovery';

// Increment this to signal that library-backed UI data should be reloaded.
export const libraryRefresh = writable(0);

export function requestLibraryRefresh(): void {
  clearDiscoveryCaches();
  libraryRefresh.update((value) => value + 1);
}
