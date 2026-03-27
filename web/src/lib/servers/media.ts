import { convertFileSrc } from '@tauri-apps/api/core';
import type { Song, Album, Playlist, AlbumDetail, PlaylistDetail } from './types';

export function normalizeMediaUrl(url: string): string {
  if (!url || !url.startsWith('file://')) return url;

  try {
    const parsed = new URL(url);
    let filePath = decodeURIComponent(parsed.pathname);
    if (/^\/[A-Za-z]:\//.test(filePath)) {
      filePath = filePath.slice(1);
    }
    return convertFileSrc(filePath);
  } catch {
    return url;
  }
}

export function normalizeSong(song: Song): Song {
  return {
    ...song,
    coverArtUrl: normalizeMediaUrl(song.coverArtUrl),
    streamUrl: normalizeMediaUrl(song.streamUrl)
  };
}

export function normalizeAlbum(album: Album): Album {
  return {
    ...album,
    coverArtUrl: normalizeMediaUrl(album.coverArtUrl)
  };
}

export function normalizePlaylist(playlist: Playlist): Playlist {
  return {
    ...playlist,
    coverArtUrl: normalizeMediaUrl(playlist.coverArtUrl)
  };
}

export function normalizeAlbumDetail(detail: AlbumDetail): AlbumDetail {
  return {
    ...detail,
    album: {
      ...detail.album,
      coverArtUrl: normalizeMediaUrl(detail.album.coverArtUrl)
    },
    songs: detail.songs.map(normalizeSong)
  };
}

export function normalizePlaylistDetail(detail: PlaylistDetail): PlaylistDetail {
  return {
    ...detail,
    playlist: {
      ...detail.playlist,
      coverArtUrl: normalizeMediaUrl(detail.playlist.coverArtUrl)
    },
    songs: detail.songs.map(normalizeSong)
  };
}
