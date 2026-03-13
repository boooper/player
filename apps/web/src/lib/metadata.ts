/**
 * Unified music metadata facade.
 *
 * VITE_METADATA_PROVIDER — controls artist/song info and image sources:
 *   'lastfm'  — Last.fm only  (requires VITE_LASTFM_API_KEY)
 *   'audiodb' — TheAudioDB only (no API key needed, but limited feature set)
 *   'both'    — Default. Fetches from both in parallel and merges the best data.
 *
 * All app code should import from this file, not from lastfm.ts or audiodb.ts directly.
 */

import { fetchAudioDbArtist, fetchAudioDbArtistPhoto } from '$lib/audiodb';
import {
  fetchTopArtists as lfmTopArtists,
  searchArtists as lfmSearchArtists,
  fetchTopSongs as lfmTopSongs,
  searchSongs as lfmSearchSongs,
  fetchArtistInfo as lfmArtistInfo,
  fetchArtistTopTracks as lfmArtistTopTracks,
  fetchTopTags as lfmTopTags
} from '$lib/lastfm';
import { getLastFmApiKey, getMetadataProviderSetting } from '$lib/stores/settings';

// ---------------------------------------------------------------------------
// Config helpers
// ---------------------------------------------------------------------------

export type Provider = 'lastfm' | 'audiodb' | 'both';

function getProvider(): Provider {
  const v = getMetadataProviderSetting().trim().toLowerCase();
  if (v === 'lastfm' || v === 'audiodb' || v === 'both') return v;
  return 'both';
}

function getLfmKey(): string {
  return getLastFmApiKey();
}

function hasLfm(): boolean {
  return Boolean(getLfmKey());
}

// ---------------------------------------------------------------------------
// Unified types
// ---------------------------------------------------------------------------

export type Artist = {
  id: string;
  name: string;
  imageUrl: string;
  listeners?: number;
  url?: string;
};

export type Song = {
  id: string;
  title: string;
  artist: string;
  imageUrl: string;
  listeners?: number;
  url?: string;
};

export type ArtistInfo = {
  name: string;
  imageUrl: string;
  listeners: number;
  playcount: number;
  bio: string;
  tags: string[];
  similarArtists: { name: string; imageUrl: string }[];
  genre: string;
  country: string;
  formedYear: string;
};

// ---------------------------------------------------------------------------
// Internal: image hydration via AudioDB
// Deduplicates by name so each unique artist is only fetched once.
// ---------------------------------------------------------------------------

async function hydrateImages<T extends { imageUrl: string }>(
  items: T[],
  getName: (item: T) => string
): Promise<T[]> {
  const cache = new Map<string, Promise<string>>();
  return Promise.all(
    items.map(async (item) => {
      if (item.imageUrl) return item;
      const key = getName(item).toLowerCase();
      if (!cache.has(key)) cache.set(key, fetchAudioDbArtistPhoto(getName(item)));
      return { ...item, imageUrl: await cache.get(key)! };
    })
  );
}

// ---------------------------------------------------------------------------
// Public functions
// ---------------------------------------------------------------------------

export async function getTopArtists(limit = 24): Promise<Artist[]> {
  if (getProvider() === 'audiodb' || !hasLfm()) return [];
  const artists = await lfmTopArtists({ apiKey: getLfmKey(), limit });
  return hydrateImages(artists, (a) => a.name);
}

export async function searchArtists(query: string, limit = 12): Promise<Artist[]> {
  if (getProvider() === 'audiodb' || !hasLfm()) return [];
  const artists = await lfmSearchArtists({ apiKey: getLfmKey(), query, limit });
  return hydrateImages(artists, (a) => a.name);
}

export async function getTopSongs(limit = 24): Promise<Song[]> {
  if (getProvider() === 'audiodb' || !hasLfm()) return [];
  const songs = await lfmTopSongs({ apiKey: getLfmKey(), limit });
  return hydrateImages(songs, (s) => s.artist);
}

export async function searchSongs(query: string, limit = 12): Promise<Song[]> {
  if (getProvider() === 'audiodb' || !hasLfm()) return [];
  const songs = await lfmSearchSongs({ apiKey: getLfmKey(), query, limit });
  return hydrateImages(songs, (s) => s.artist);
}

export async function getArtistInfo(artist: string): Promise<ArtistInfo | null> {
  const provider = getProvider();

  if (provider === 'audiodb') {
    const a = await fetchAudioDbArtist(artist);
    if (!a) return null;
    return {
      name: a.name || artist,
      imageUrl: a.thumb || a.fanart || a.banner || '',
      listeners: 0,
      playcount: 0,
      bio: a.biography,
      tags: a.genre ? [a.genre] : [],
      similarArtists: [],
      genre: a.genre,
      country: a.country,
      formedYear: a.formedYear
    };
  }

  if (provider === 'lastfm') {
    if (!hasLfm()) return null;
    const lfm = await lfmArtistInfo({ apiKey: getLfmKey(), artist });
    if (!lfm) return null;
    return { ...lfm, genre: lfm.tags[0] ?? '', country: '', formedYear: '' };
  }

  // 'both': run in parallel and merge the best of each source
  const [lfmRes, adbRes] = await Promise.allSettled([
    hasLfm() ? lfmArtistInfo({ apiKey: getLfmKey(), artist }) : Promise.resolve(null),
    fetchAudioDbArtist(artist)
  ]);

  const lfm = lfmRes.status === 'fulfilled' ? lfmRes.value : null;
  const adb = adbRes.status === 'fulfilled' ? adbRes.value : null;

  if (!lfm && !adb) return null;

  const imageUrl =
    lfm?.imageUrl ||
    (adb ? adb.thumb || adb.fanart || adb.banner : '') ||
    '';

  // Hydrate similar artist images from AudioDB where Last.fm returned nothing
  const rawSimilar = lfm?.similarArtists ?? [];
  const similarArtists = await Promise.all(
    rawSimilar.map(async (s) => {
      if (s.imageUrl) return s;
      return { ...s, imageUrl: await fetchAudioDbArtistPhoto(s.name) };
    })
  );

  return {
    name: lfm?.name || adb?.name || artist,
    imageUrl,
    listeners: lfm?.listeners ?? 0,
    playcount: lfm?.playcount ?? 0,
    bio: lfm?.bio || adb?.biography || '',
    tags: lfm?.tags?.length ? lfm.tags : adb?.genre ? [adb.genre] : [],
    similarArtists,
    genre: adb?.genre || lfm?.tags?.[0] || '',
    country: adb?.country || '',
    formedYear: adb?.formedYear || ''
  };
}

export async function getArtistTopTracks(artist: string, limit = 10): Promise<Song[]> {
  if (!hasLfm() || getProvider() === 'audiodb') return [];
  return lfmArtistTopTracks({ apiKey: getLfmKey(), artist, limit });
}

export async function getTopTags(limit = 40): Promise<string[]> {
  if (!hasLfm() || getProvider() === 'audiodb') return [];
  return lfmTopTags({ apiKey: getLfmKey(), limit });
}
