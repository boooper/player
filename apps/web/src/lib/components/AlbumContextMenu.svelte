<script lang="ts">
  import type { Snippet } from 'svelte';
  import { Play, ListStart, ListEnd, User, Disc3, ListPlus } from '@lucide/svelte';
  import { toast } from 'svelte-sonner';
  import { goto } from '$app/navigation';
  import * as ContextMenu from '$lib/components/ui/context-menu';
  import { appendToQueue, playQueue, playingFrom, focusTrack, addRecentlyPlayed, queue, currentIndex, subsonicPlaylists, queueLoading } from '$lib/stores/player';
  import { requestLibraryRefresh } from '$lib/stores/ui-state';
  import { get } from 'svelte/store';
  import { createPlaylist, fetchAlbumSongs, fetchPlaylists, materializeSong, searchSongs, type Album } from '$lib/servers';

  let { album, onplay, children, triggerClass }: {
    album: Album;
    onplay?: () => void;
    children: Snippet;
    triggerClass?: string;
  } = $props();

  async function playSongs(mode: 'now' | 'next' | 'queue') {
    if (mode === 'now') queueLoading.set(true);
    let songs: Awaited<ReturnType<typeof fetchAlbumSongs>>;
    try {
      songs = await fetchAlbumSongs(album.id);
    } catch {
      toast.error('Failed to load album tracks');
      queueLoading.set(false);
      return;
    }
    queueLoading.set(false);
    if (!songs.length) { toast.warning('Album has no tracks'); return; }

    if (mode === 'now') {
      focusTrack.set({ title: songs[0].title, artist: songs[0].artist, imageUrl: songs[0].coverArtUrl, source: 'library', album: songs[0].album });
      playQueue(songs, 0);
      playingFrom.set({ type: 'album', name: album.name, href: `/album/${encodeURIComponent(album.id)}` });
      addRecentlyPlayed({ id: album.id, name: album.name, coverArtUrl: album.coverArtUrl, href: `/album/${encodeURIComponent(album.id)}`, type: 'album' });
    } else if (mode === 'next') {
      if (!get(queue).length) {
        focusTrack.set({ title: songs[0].title, artist: songs[0].artist, imageUrl: songs[0].coverArtUrl, source: 'library', album: songs[0].album });
        playQueue(songs, 0);
        playingFrom.set({ type: 'album', name: album.name, href: `/album/${encodeURIComponent(album.id)}` });
      } else {
        const idx = get(currentIndex);
        queue.update((current) => {
          const next = [...current];
          next.splice(idx + 1, 0, ...songs);
          return next;
        });
        toast.success('Playing next', { description: album.name });
      }
    } else {
      appendToQueue(songs);
      toast.success('Added to queue', { description: `${songs.length} tracks from ${album.name}` });
    }
  }

  async function createAlbumPlaylist() {
    let songs: Awaited<ReturnType<typeof fetchAlbumSongs>>;
    try {
      songs = await fetchAlbumSongs(album.id);
    } catch {
      toast.error('Failed to load album tracks');
      return;
    }

    if (!songs.length) {
      toast.warning('Album has no tracks');
      return;
    }

    const playlistName = album.artist ? `${album.artist} - ${album.name}` : album.name;
    const toastId = toast.loading('Preparing album playlist…', {
      description: `Creating playlist 0 / ${songs.length}`,
    });

    try {
      const playlistSongIds = await resolvePlaylistSongIds(songs, (current, total, songTitle) => {
        toast.loading('Preparing album playlist…', {
          id: toastId,
          description: `Creating playlist ${current} / ${total}${songTitle ? ` - ${songTitle}` : ''}`,
        });
      });
      const playlist = await createPlaylist(playlistName, playlistSongIds);
      subsonicPlaylists.update((lists) => [playlist, ...lists]);
      requestLibraryRefresh();
      fetchPlaylists()
        .then((lists) => subsonicPlaylists.set(lists))
        .catch(() => undefined);
      toast.success('Playlist created', { id: toastId, description: playlist.name });
      goto(`/playlist/${encodeURIComponent(playlist.id)}`);
    } catch (err) {
      toast.error(err instanceof Error ? err.message : 'Failed to create playlist', { id: toastId });
    }
  }

  async function resolvePlaylistSongIds(
    songs: Awaited<ReturnType<typeof fetchAlbumSongs>>,
    onProgress?: (current: number, total: number, songTitle?: string) => void,
  ) {
    const resolvedIds: string[] = [];
    const total = songs.length;

    for (const [index, song] of songs.entries()) {
      onProgress?.(index + 1, total, song.title);
      if (!song.id.startsWith('ext-')) {
        resolvedIds.push(song.id);
        continue;
      }

      await materializeSong(song.id).catch(() => undefined);
      const resolved = await findMaterializedSong(song);

      if (!resolved) {
        throw new Error(`Could not download "${song.title}" before creating the playlist`);
      }

      resolvedIds.push(resolved.id);
    }

    return resolvedIds;
  }

  async function findMaterializedSong(song: Awaited<ReturnType<typeof fetchAlbumSongs>>[number]) {
    for (let attempt = 0; attempt < 6; attempt += 1) {
      const candidates = await searchSongs(`${song.artist} ${song.title}`, 10).catch(() => []);
      const exact = candidates.find((candidate) =>
        !candidate.id.startsWith('ext-') &&
        normalize(candidate.title) === normalize(song.title) &&
        normalize(candidate.artist) === normalize(song.artist)
      );
      if (exact) return exact;

      const fallback = candidates.find((candidate) =>
        !candidate.id.startsWith('ext-') &&
        normalize(candidate.title).includes(normalize(song.title))
      );
      if (fallback) return fallback;

      await delay(1200);
    }

    return null;
  }

  function normalize(value: string) {
    return value.trim().toLowerCase();
  }

  function delay(ms: number) {
    return new Promise((resolve) => setTimeout(resolve, ms));
  }
</script>

<ContextMenu.Root>
  <ContextMenu.Trigger class={triggerClass ?? 'block w-full'}>
    {@render children()}
  </ContextMenu.Trigger>
  <ContextMenu.Content>
    {#if onplay}
      <ContextMenu.Item onclick={onplay}><Play />Play album</ContextMenu.Item>
    {:else}
      <ContextMenu.Item onclick={() => playSongs('now')}><Play />Play album</ContextMenu.Item>
    {/if}
    <ContextMenu.Item onclick={() => playSongs('next')}><ListStart />Play next</ContextMenu.Item>
    <ContextMenu.Item onclick={() => playSongs('queue')}><ListEnd />Add to queue</ContextMenu.Item>
    <ContextMenu.Item onclick={createAlbumPlaylist}><ListPlus />Create playlist from album</ContextMenu.Item>
    <ContextMenu.Separator />
    <ContextMenu.Item onclick={() => goto(`/album/${encodeURIComponent(album.id)}`)}><Disc3 />Go to album</ContextMenu.Item>
    {#if album.artist}
      <ContextMenu.Item onclick={() => goto(`/artist/${encodeURIComponent(album.artist)}`)}><User />Go to artist</ContextMenu.Item>
    {/if}
  </ContextMenu.Content>
</ContextMenu.Root>
