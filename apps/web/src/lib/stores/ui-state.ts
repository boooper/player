import { writable } from 'svelte/store';

// Increment this to signal that library-backed UI data should be reloaded.
export const libraryRefresh = writable(0);
