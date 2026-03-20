import { fetchLyrics as fetchLyricsIpc } from '$lib/servers/integrations';
import type { LyricsResult } from './types';

export type { LyricsResult };

// Session-scoped cache — keyed on artist::title::album.
// The backend already caches on disk; this layer avoids redundant Tauri IPC
// calls when the same track is re-requested within a session.
const lyricsCache = new Map<string, LyricsResult | null>();

function cacheKey(artist: string, title: string, album: string): string {
  return `${artist.trim().toLowerCase()}::${title.trim().toLowerCase()}::${album.trim().toLowerCase()}`;
}

export function clearLyricsCache(): void {
  lyricsCache.clear();
}

export async function getLyrics(
  artist: string,
  title: string,
  album: string,
  duration: number
): Promise<LyricsResult | null> {
  const key = cacheKey(artist, title, album);
  if (lyricsCache.has(key)) return lyricsCache.get(key)!;
  const result = await fetchLyricsIpc(artist, title, album, duration);
  lyricsCache.set(key, result);
  return result;
}
