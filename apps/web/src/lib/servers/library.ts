import type { Song, Album, Playlist, AlbumDetail, PlaylistDetail, SearchBundlePayload, StoredLikedArtist } from './types';
import { normalizeAlbum, normalizeAlbumDetail, normalizePlaylist, normalizePlaylistDetail, normalizeSong } from './media';
import { invoke } from './shared';

export async function fetchLikedArtists(): Promise<StoredLikedArtist[]> {
  return invoke<StoredLikedArtist[]>('get_liked_artists');
}

export async function saveLikedArtist(name: string): Promise<StoredLikedArtist> {
  return invoke<StoredLikedArtist>('save_liked_artist', { name, source: 'lastfm', externalId: null });
}

export async function removeLikedArtist(name: string): Promise<void> {
  await invoke('remove_liked_artist', { name });
}

export async function searchSongs(query: string, count = 20): Promise<Song[]> {
  return (await invoke<Song[]>('library_search', { query, count })).map(normalizeSong);
}

export async function searchBundle(
  query: string,
  songCount = 24,
  albumCount = 12,
  recommendationCount = 16
): Promise<SearchBundlePayload> {
  const result = await invoke<SearchBundlePayload>('library_search_bundle', {
    query,
    songCount,
    albumCount,
    recommendationCount
  });
  return {
    songs: result.songs.map(normalizeSong),
    albums: result.albums.map(normalizeAlbum),
    recommendations: result.recommendations.map(normalizeSong)
  };
}

export async function searchAlbums(query: string, count = 20): Promise<Album[]> {
  return (await invoke<Album[]>('library_artist_albums', { query, count })).map(normalizeAlbum);
}

export async function fetchSimilarSongs(songId: string, count = 20): Promise<Song[]> {
  return (await invoke<Song[]>('library_similar', { songId, count })).map(normalizeSong);
}

export async function fetchPlaylists(): Promise<Playlist[]> {
  return (await invoke<Playlist[]>('library_playlists')).map(normalizePlaylist);
}

export async function fetchPlaylistDetail(playlistId: string): Promise<PlaylistDetail> {
  return normalizePlaylistDetail(await invoke<PlaylistDetail>('library_playlist', { id: playlistId }));
}

export async function fetchPlaylistSongs(playlistId: string): Promise<Song[]> {
  const result = await invoke<PlaylistDetail>('library_playlist', { id: playlistId });
  return result.songs;
}

export async function fetchArtistAlbums(query: string, count = 20): Promise<Album[]> {
  return (await invoke<Album[]>('library_artist_albums', { query, count })).map(normalizeAlbum);
}

export async function fetchAlbumSongs(albumId: string): Promise<Song[]> {
  return (await invoke<Song[]>('library_album_songs', { id: albumId })).map(normalizeSong);
}

export async function fetchAlbumDetail(albumId: string): Promise<AlbumDetail> {
  return normalizeAlbumDetail(await invoke<AlbumDetail>('library_album', { id: albumId }));
}

export async function fetchAlbumList(
  type: 'newest' | 'random' | 'frequent' | 'recent' | 'highest' = 'newest',
  count = 20
): Promise<Album[]> {
  return (await invoke<Album[]>('library_album_list', { kind: type, count })).map(normalizeAlbum);
}

export async function fetchStarredSongs(): Promise<Song[]> {
  return (await invoke<Song[]>('library_starred')).map(normalizeSong);
}

export async function starSong(id: string, artist?: string, title?: string, album?: string): Promise<void> {
  await invoke('library_star', { id, unstar: false, artist: artist ?? null, title: title ?? null, album: album ?? null });
}

export async function unstarSong(id: string, artist?: string, title?: string, album?: string): Promise<void> {
  await invoke('library_star', { id, unstar: true, artist: artist ?? null, title: title ?? null, album: album ?? null });
}

export async function addSongToPlaylist(playlistId: string, song: Song): Promise<void> {
  await invoke('library_add_to_playlist', {
    playlistId,
    songId: song.id,
    artist: song.artist,
    title: song.title,
    album: song.album
  });
}

export async function createPlaylist(name: string, songIds: string[]): Promise<Playlist> {
  return normalizePlaylist(await invoke<Playlist>('library_create_playlist', { name, songIds }));
}

export async function renamePlaylist(playlistId: string, name: string): Promise<void> {
  await invoke('library_rename_playlist', { playlistId, name });
}

export async function deletePlaylist(playlistId: string): Promise<void> {
  await invoke('library_delete_playlist', { playlistId });
}

export async function materializeSong(songId: string): Promise<void> {
  await invoke('library_materialize_song', { songId });
}
