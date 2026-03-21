import { invoke } from '@tauri-apps/api/core';

export type AudioDbArtist = {
  name: string;
  thumb: string;
  fanart: string;
  banner: string;
  biography: string;
  genre: string;
  country: string;
  formedYear: string;
};

export async function fetchAudioDbArtist(artistName: string): Promise<AudioDbArtist | null> {
  if (!artistName.trim()) return null;
  try {
    return await invoke<AudioDbArtist | null>('audiodb_artist', { name: artistName });
  } catch {
    return null;
  }
}

export async function fetchAudioDbArtistPhoto(artistName: string): Promise<string> {
  const artist = await fetchAudioDbArtist(artistName);
  if (!artist) return '';
  return artist.thumb || artist.fanart || artist.banner || '';
}
