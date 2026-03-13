import { Hono } from 'hono';
import { activeProfile, getSettings } from '../store.js';
import { SubsonicClient, asArray, mapSong, mapAlbum } from '../subsonic.js';
import { sessionKey, signedPost } from '../lastfm.js';

type R = Record<string, unknown>;

const subsonic = new Hono();

async function getClient() {
  const profile = await activeProfile();
  if (!profile) throw new Error('No active server profile configured. Add one in Settings.');
  return new SubsonicClient(profile);
}

subsonic.get('/search', async (c) => {
  try {
    const client = await getClient();
    const q = String(c.req.query('q') ?? '').trim();
    const count = Number(c.req.query('count') ?? 20);
    if (!q) return c.json({ error: 'Search query `q` is required.' }, 400);
    const body = await client.request('search3', { query: q, songCount: count, artistCount: 0, albumCount: 0 });
    const songs = asArray((body?.searchResult3 as R)?.song as R[]).map(mapSong).map((song) => ({
      ...song,
      coverArtUrl: client.coverArtUrl(song.coverArt),
      streamUrl: client.streamUrl(song.id)
    }));
    return c.json({ songs });
  } catch (error) {
    return c.json({ error: error instanceof Error ? error.message : 'Subsonic request failed.' }, 502);
  }
});

subsonic.get('/similar', async (c) => {
  try {
    const client = await getClient();
    const songId = String(c.req.query('songId') ?? '').trim();
    const count = Number(c.req.query('count') ?? 20);
    if (!songId) return c.json({ error: 'songId is required.' }, 400);
    const body = await client.request('getSimilarSongs2', { id: songId, count });
    const songs = asArray((body?.similarSongs2 as R)?.song as R[]).map(mapSong).map((song) => ({
      ...song,
      coverArtUrl: client.coverArtUrl(song.coverArt),
      streamUrl: client.streamUrl(song.id)
    }));
    return c.json({ songs });
  } catch (error) {
    return c.json({ error: error instanceof Error ? error.message : 'Subsonic request failed.' }, 502);
  }
});

subsonic.get('/playlists', async (c) => {
  try {
    const client = await getClient();
    const body = await client.request('getPlaylists', {});
    const playlists = asArray((body?.playlists as R)?.playlist as R[]).map((pl) => ({
      id: String(pl?.id ?? ''),
      name: String(pl?.name ?? ''),
      songCount: Number(pl?.songCount ?? 0),
      duration: Number(pl?.duration ?? 0),
      coverArt: String(pl?.coverArt ?? pl?.id ?? ''),
      coverArtUrl: client.coverArtUrl(String(pl?.coverArt ?? pl?.id ?? ''))
    }));
    return c.json({ playlists });
  } catch (error) {
    return c.json({ error: error instanceof Error ? error.message : 'Subsonic request failed.' }, 502);
  }
});

subsonic.get('/playlist', async (c) => {
  try {
    const client = await getClient();
    const id = String(c.req.query('id') ?? '').trim();
    if (!id) return c.json({ error: 'Playlist id is required.' }, 400);
    const body = await client.request('getPlaylist', { id });
    const pl = (body?.playlist as R) ?? {};
    const songs = asArray(pl?.entry as R[]).map(mapSong).map((song) => ({
      ...song,
      coverArtUrl: client.coverArtUrl(song.coverArt),
      streamUrl: client.streamUrl(song.id)
    }));
    return c.json({
      playlist: {
        id: String(pl?.id ?? id),
        name: String(pl?.name ?? ''),
        songCount: Number(pl?.songCount ?? songs.length),
        duration: Number(pl?.duration ?? 0),
        coverArtUrl: client.coverArtUrl(String(pl?.coverArt ?? ''))
      },
      songs
    });
  } catch (error) {
    return c.json({ error: error instanceof Error ? error.message : 'Subsonic request failed.' }, 502);
  }
});

subsonic.get('/artist-albums', async (c) => {
  try {
    const client = await getClient();
    const q = String(c.req.query('q') ?? '').trim();
    const count = Number(c.req.query('count') ?? 20);
    if (!q) return c.json({ error: 'Query param `q` is required.' }, 400);
    const body = await client.request('search3', { query: q, artistCount: 0, albumCount: count, songCount: 0 });
    const albums = asArray((body?.searchResult3 as R)?.album as R[]).map(mapAlbum).map((album) => ({
      ...album,
      coverArtUrl: client.coverArtUrl(album.coverArt, 300)
    }));
    return c.json({ albums });
  } catch (error) {
    return c.json({ error: error instanceof Error ? error.message : 'Subsonic request failed.' }, 502);
  }
});

subsonic.get('/album-songs', async (c) => {
  try {
    const client = await getClient();
    const id = String(c.req.query('id') ?? '').trim();
    if (!id) return c.json({ error: 'Query param `id` is required.' }, 400);
    const body = await client.request('getAlbum', { id });
    const songs = asArray((body?.album as R)?.song as R[]).map(mapSong).map((song) => ({
      ...song,
      albumId: song.albumId || id,
      coverArtUrl: client.coverArtUrl(song.coverArt),
      streamUrl: client.streamUrl(song.id)
    }));
    return c.json({ songs });
  } catch (error) {
    return c.json({ error: error instanceof Error ? error.message : 'Subsonic request failed.' }, 502);
  }
});

subsonic.get('/album', async (c) => {
  try {
    const client = await getClient();
    const id = String(c.req.query('id') ?? '').trim();
    if (!id) return c.json({ error: 'Query param `id` is required.' }, 400);
    const body = await client.request('getAlbum', { id });
    const al = (body?.album as R) ?? {};
    const songs = asArray(al?.song as R[]).map(mapSong).map((song) => ({
      ...song,
      album: song.album || String(al?.name ?? ''),
      albumId: song.albumId || String(al?.id ?? ''),
      coverArtUrl: client.coverArtUrl(song.coverArt || String(al?.coverArt ?? '')),
      streamUrl: client.streamUrl(song.id)
    }));
    return c.json({
      album: {
        id: String(al?.id ?? id),
        name: String(al?.name ?? ''),
        artist: String(al?.artist ?? ''),
        artistId: String(al?.artistId ?? ''),
        coverArt: String(al?.coverArt ?? al?.id ?? ''),
        songCount: Number(al?.songCount ?? songs.length),
        duration: Number(al?.duration ?? 0),
        year: al?.year ? Number(al.year) : undefined,
        genre: al?.genre ? String(al.genre) : undefined,
        coverArtUrl: client.coverArtUrl(String(al?.coverArt ?? al?.id ?? ''), 400)
      },
      songs
    });
  } catch (error) {
    return c.json({ error: error instanceof Error ? error.message : 'Subsonic request failed.' }, 502);
  }
});

subsonic.get('/album-list', async (c) => {
  try {
    const client = await getClient();
    const type = String(c.req.query('type') ?? 'newest');
    const count = Math.min(Number(c.req.query('count') ?? 20), 100);
    const body = await client.request('getAlbumList2', { type, size: count });
    const albums = asArray((body?.albumList2 as R)?.album as R[]).map(mapAlbum).map((album) => ({
      ...album,
      coverArtUrl: client.coverArtUrl(album.coverArt, 240)
    }));
    return c.json({ albums });
  } catch (error) {
    return c.json({ error: error instanceof Error ? error.message : 'Subsonic request failed.' }, 502);
  }
});

subsonic.get('/starred', async (c) => {
  try {
    const client = await getClient();
    const body = await client.request('getStarred2', {});
    const songs = asArray((body?.starred2 as R)?.song as R[]).map(mapSong).map((song) => ({
      ...song,
      coverArtUrl: client.coverArtUrl(song.coverArt),
      streamUrl: client.streamUrl(song.id)
    }));
    return c.json({ songs });
  } catch (error) {
    return c.json({ error: error instanceof Error ? error.message : 'Subsonic request failed.' }, 502);
  }
});

subsonic.post('/star', async (c) => {
  try {
    const [client, settings] = await Promise.all([getClient(), getSettings()]);
    const body = await c.req.json<Record<string, unknown>>();
    const id = String(body?.id ?? '').trim();
    const unstar = body?.unstar === true;
    if (!id) return c.json({ error: 'Song id is required.' }, 400);
    await client.request(unstar ? 'unstar' : 'star', { id });
    const artist = String(body?.artist ?? '').trim();
    const title = String(body?.title ?? '').trim();
    if (artist && title) {
      try {
        const sk = sessionKey(settings);
        if (sk) {
          await signedPost(settings, {
            method: unstar ? 'track.unlove' : 'track.love',
            sk,
            artist,
            track: title
          });
        }
      } catch {
        // intentionally silent
      }
    }
    return c.json({ ok: true });
  } catch (error) {
    return c.json({ error: error instanceof Error ? error.message : 'Subsonic request failed.' }, 502);
  }
});

subsonic.post('/playlist-song', async (c) => {
  try {
    const client = await getClient();
    const body = await c.req.json<Record<string, unknown>>();
    const playlistId = String(body?.playlistId ?? '').trim();
    const songId = String(body?.songId ?? '').trim();
    if (!playlistId || !songId) return c.json({ error: 'playlistId and songId are required.' }, 400);
    await client.request('updatePlaylist', { playlistId, songIdToAdd: songId });
    return c.json({ ok: true });
  } catch (error) {
    return c.json({ error: error instanceof Error ? error.message : 'Subsonic request failed.' }, 502);
  }
});

export default subsonic;
