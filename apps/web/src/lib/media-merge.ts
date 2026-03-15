import type { Album, Song } from '$lib/api';

export type MergedAlbum = Album & {
  sourceIds: string[];
};

function normalize(value: string): string {
  return value.trim().toLowerCase().replace(/\s+/g, ' ');
}

function albumKey(album: Pick<Album, 'artist' | 'name'>): string {
  return `${normalize(album.artist)}::${normalize(album.name)}`;
}

function songKey(song: Pick<Song, 'artist' | 'title'>): string {
  return `${normalize(song.artist)}::${normalize(song.title)}`;
}

function isExternalId(id: string): boolean {
  return id.startsWith('ext-');
}

function pickPreferredAlbum(albums: Album[]): Album {
  return albums.find((album) => !isExternalId(album.id)) ?? albums[0];
}

function pickPreferredSong(current: Song, candidate: Song): Song {
  if (isExternalId(current.id) && !isExternalId(candidate.id)) return candidate;
  if (!isExternalId(current.id) && isExternalId(candidate.id)) return current;
  return current;
}

export function mergeAlbums(albums: Album[]): MergedAlbum[] {
  const grouped = new Map<string, Album[]>();

  for (const album of albums) {
    const key = albumKey(album);
    const existing = grouped.get(key);
    if (existing) {
      existing.push(album);
    } else {
      grouped.set(key, [album]);
    }
  }

  return Array.from(grouped.values()).map((group) => {
    const primary = pickPreferredAlbum(group);
    return {
      ...primary,
      sourceIds: group.map((album) => album.id),
      songCount: Math.max(...group.map((album) => album.songCount)),
      duration: Math.max(...group.map((album) => album.duration ?? 0))
    };
  });
}

export function mergeAlbumSongs(songs: Song[]): Song[] {
  const merged = new Map<string, Song>();

  for (const song of songs) {
    const key = songKey(song);
    const existing = merged.get(key);
    if (existing) {
      merged.set(key, pickPreferredSong(existing, song));
    } else {
      merged.set(key, song);
    }
  }

  return Array.from(merged.values());
}

export function findAlbumGroupIds(albums: Album[], target: Pick<Album, 'artist' | 'name'>): string[] {
  const targetKey = albumKey(target);
  return albums.filter((album) => albumKey(album) === targetKey).map((album) => album.id);
}
