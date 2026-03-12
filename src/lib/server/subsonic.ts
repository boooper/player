import crypto from 'node:crypto';
import { prisma } from './prisma';

const SUBSONIC_API_VERSION = '1.16.1';
const SUBSONIC_CLIENT_NAME = 'naviarr';
const SUBSONIC_RESPONSE_FORMAT = 'json';

type SubsonicConfig = {
  url: string;
  username: string;
  password: string;
  usePasswordAuth: boolean;
};

type QueryParams = Record<string, string | number | boolean | undefined>;

type SimilarSong = {
  id: string;
  title: string;
  artist: string;
  album: string;
  albumId: string;
  coverArt: string;
  duration: number;
};

type PlaylistSummary = {
  id: string;
  name: string;
  songCount: number;
  duration: number;
  coverArt: string;
};

type AlbumSummary = {
  id: string;
  name: string;
  artist: string;
  artistId: string;
  coverArt: string;
  songCount: number;
  duration: number;
  year?: number;
};

async function readConfig(): Promise<SubsonicConfig> {
  const profile = await prisma.subsonicProfile.findFirst({ where: { isActive: true } });
  if (!profile) {
    throw new Error('No active server profile configured. Add one in Settings.');
  }
  return {
    url: profile.url.replace(/\/$/, ''),
    username: profile.username,
    password: profile.password,
    usePasswordAuth: profile.usePasswordAuth
  };
}

function authParams(config: SubsonicConfig): Record<string, string> {
  const base = {
    u: config.username,
    v: SUBSONIC_API_VERSION,
    c: SUBSONIC_CLIENT_NAME,
    f: SUBSONIC_RESPONSE_FORMAT
  };

  // octo-fiesta accepts both Subsonic auth styles:
  // - token auth (t + s)
  // - password auth via `p` (plain or `enc:` hex)
  if (config.usePasswordAuth || config.password.startsWith('enc:')) {
    return {
      ...base,
      p: config.password
    };
  }

  const salt = crypto.randomBytes(6).toString('hex');
  const token = crypto.createHash('md5').update(`${config.password}${salt}`).digest('hex');

  return {
    ...base,
    t: token,
    s: salt
  };
}

function asArray<T>(value: T | T[] | undefined): T[] {
  if (!value) return [];
  return Array.isArray(value) ? value : [value];
}

export class SubsonicClient {
  private readonly config: SubsonicConfig;

  constructor(config: SubsonicConfig) {
    this.config = config;
  }

  private buildUrl(path: string, params: QueryParams): string {
    const url = new URL(`${this.config.url}/rest/${path}`);
    const merged = {
      ...authParams(this.config),
      ...params
    };

    Object.entries(merged).forEach(([key, value]) => {
      if (value === undefined) return;
      url.searchParams.set(key, String(value));
    });

    return url.toString();
  }

  private async request(path: string, params: QueryParams): Promise<any> {
    const url = this.buildUrl(path, params);
    const response = await fetch(url);

    if (!response.ok) {
      throw new Error(`Subsonic request failed with status ${response.status}`);
    }

    const json = await response.json();
    const body = json?.['subsonic-response'];

    if (!body) {
      throw new Error('Invalid Subsonic response payload');
    }

    if (body.status !== 'ok') {
      const message = body?.error?.message ?? 'Subsonic request failed';
      throw new Error(message);
    }

    return body;
  }

  async searchTracks(query: string, count = 20): Promise<SimilarSong[]> {
    const body = await this.request('search3', {
      query,
      songCount: count,
      artistCount: 0,
      albumCount: 0
    });

    const songs = asArray(body?.searchResult3?.song);
    return songs.map((song: any) => ({
      id: String(song?.id ?? ''),
      title: String(song?.title ?? ''),
      artist: String(song?.artist ?? ''),
      album: String(song?.album ?? ''),
      albumId: String(song?.albumId ?? ''),
      coverArt: String(song?.coverArt ?? song?.id ?? ''),
      duration: Number(song?.duration ?? 0)
    }));
  }

  async getSimilarSongs(songId: string, count = 20): Promise<SimilarSong[]> {
    const body = await this.request('getSimilarSongs2', {
      id: songId,
      count
    });

    const songs = asArray(body?.similarSongs2?.song);
    return songs.map((song: any) => ({
      id: String(song?.id ?? ''),
      title: String(song?.title ?? ''),
      artist: String(song?.artist ?? ''),
      album: String(song?.album ?? ''),
      albumId: String(song?.albumId ?? ''),
      coverArt: String(song?.coverArt ?? song?.id ?? ''),
      duration: Number(song?.duration ?? 0)
    }));
  }

  async getPlaylists(): Promise<PlaylistSummary[]> {
    const body = await this.request('getPlaylists', {});
    const playlists = asArray(body?.playlists?.playlist);

    return playlists.map((playlist: any) => ({
      id: String(playlist?.id ?? ''),
      name: String(playlist?.name ?? ''),
      songCount: Number(playlist?.songCount ?? 0),
      duration: Number(playlist?.duration ?? 0),
      coverArt: String(playlist?.coverArt ?? playlist?.id ?? '')
    }));
  }

  async getPlaylistSongs(playlistId: string): Promise<SimilarSong[]> {
    const detail = await this.getPlaylistDetail(playlistId);
    return detail.songs;
  }

  async getPlaylistDetail(playlistId: string): Promise<{
    id: string;
    name: string;
    songCount: number;
    duration: number;
    coverArt: string;
    songs: SimilarSong[];
  }> {
    const body = await this.request('getPlaylist', { id: playlistId });
    const pl = body?.playlist ?? {};
    const songs = asArray(pl?.entry);

    return {
      id: String(pl?.id ?? playlistId),
      name: String(pl?.name ?? ''),
      songCount: Number(pl?.songCount ?? songs.length),
      duration: Number(pl?.duration ?? 0),
      coverArt: String(pl?.coverArt ?? ''),
      songs: songs.map((song: any) => ({
        id: String(song?.id ?? ''),
        title: String(song?.title ?? ''),
        artist: String(song?.artist ?? ''),
        album: String(song?.album ?? ''),
        albumId: String(song?.albumId ?? ''),
        coverArt: String(song?.coverArt ?? song?.id ?? ''),
        duration: Number(song?.duration ?? 0)
      }))
    };
  }

  async getArtistAlbums(query: string, count = 20): Promise<AlbumSummary[]> {
    const body = await this.request('search3', {
      query,
      artistCount: 0,
      albumCount: count,
      songCount: 0
    });

    const albums = asArray(body?.searchResult3?.album);
    return albums.map((album: any) => ({
      id: String(album?.id ?? ''),
      name: String(album?.name ?? ''),
      artist: String(album?.artist ?? ''),
      artistId: String(album?.artistId ?? ''),
      coverArt: String(album?.coverArt ?? album?.id ?? ''),
      songCount: Number(album?.songCount ?? 0),
      duration: Number(album?.duration ?? 0),
      year: album?.year ? Number(album.year) : undefined
    }));
  }

  async getAlbumSongs(albumId: string): Promise<SimilarSong[]> {
    const body = await this.request('getAlbum', { id: albumId });
    const songs = asArray(body?.album?.song);
    return songs.map((song: any) => ({
      id: String(song?.id ?? ''),
      title: String(song?.title ?? ''),
      artist: String(song?.artist ?? ''),
      album: String(song?.album ?? ''),
      albumId: String(song?.albumId ?? albumId),
      coverArt: String(song?.coverArt ?? song?.id ?? ''),
      duration: Number(song?.duration ?? 0)
    }));
  }

  async getAlbumDetail(albumId: string): Promise<{
    id: string;
    name: string;
    artist: string;
    artistId: string;
    coverArt: string;
    songCount: number;
    duration: number;
    year?: number;
    genre?: string;
    songs: SimilarSong[];
  }> {
    const body = await this.request('getAlbum', { id: albumId });
    const al = body?.album ?? {};
    const songs = asArray(al?.song);
    return {
      id: String(al?.id ?? albumId),
      name: String(al?.name ?? ''),
      artist: String(al?.artist ?? ''),
      artistId: String(al?.artistId ?? ''),
      coverArt: String(al?.coverArt ?? al?.id ?? ''),
      songCount: Number(al?.songCount ?? songs.length),
      duration: Number(al?.duration ?? 0),
      year: al?.year ? Number(al.year) : undefined,
      genre: al?.genre ? String(al.genre) : undefined,
      songs: songs.map((song: any) => ({
        id: String(song?.id ?? ''),
        title: String(song?.title ?? ''),
        artist: String(song?.artist ?? ''),
        album: String(song?.album ?? al?.name ?? ''),
        albumId: String(song?.albumId ?? al?.id ?? ''),
        coverArt: String(song?.coverArt ?? al?.coverArt ?? song?.id ?? ''),
        duration: Number(song?.duration ?? 0)
      }))
    };
  }

  async starSong(id: string): Promise<void> {
    await this.request('star', { id });
  }

  async unstarSong(id: string): Promise<void> {
    await this.request('unstar', { id });
  }

  async addSongToPlaylist(playlistId: string, songId: string): Promise<void> {
    await this.request('updatePlaylist', { playlistId, songIdToAdd: songId });
  }

  async getAlbumList(type: 'newest' | 'random' | 'frequent' | 'recent' | 'highest' = 'newest', count = 20): Promise<AlbumSummary[]> {
    const body = await this.request('getAlbumList2', { type, size: count });
    const albums = asArray(body?.albumList2?.album);
    return albums.map((album: any) => ({
      id: String(album?.id ?? ''),
      name: String(album?.name ?? ''),
      artist: String(album?.artist ?? ''),
      artistId: String(album?.artistId ?? ''),
      coverArt: String(album?.coverArt ?? album?.id ?? ''),
      songCount: Number(album?.songCount ?? 0),
      duration: Number(album?.duration ?? 0),
      year: album?.year ? Number(album.year) : undefined
    }));
  }

  async getStarredSongs(): Promise<SimilarSong[]> {    const body = await this.request('getStarred2', {});
    const songs = asArray(body?.starred2?.song);
    return songs.map((song: any) => ({
      id: String(song?.id ?? ''),
      title: String(song?.title ?? ''),
      artist: String(song?.artist ?? ''),
      album: String(song?.album ?? ''),
      albumId: String(song?.albumId ?? ''),
      coverArt: String(song?.coverArt ?? song?.id ?? ''),
      duration: Number(song?.duration ?? 0)
    }));
  }

  async ping(): Promise<boolean> {
    try {
      await this.request('ping', {});
      return true;
    } catch {
      return false;
    }
  }

  coverArtUrl(coverArtId: string, size = 240): string {
    if (!coverArtId) return '';
    return this.buildUrl('getCoverArt', { id: coverArtId, size });
  }

  streamUrl(songId: string, maxBitRate = 320): string {
    if (!songId) return '';
    return this.buildUrl('stream', { id: songId, maxBitRate });
  }
}

export async function getSubsonicClient(): Promise<SubsonicClient> {
  const config = await readConfig();
  return new SubsonicClient(config);
}
