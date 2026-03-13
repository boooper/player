<script lang="ts">
  import type { Snippet } from 'svelte';
  import { Play, ListStart, ListEnd, User, Disc3 } from '@lucide/svelte';
  import { toast } from 'svelte-sonner';
  import { goto } from '$app/navigation';
  import * as ContextMenu from '$lib/components/ui/context-menu';
  import { appendToQueue, playQueue, playingFrom, focusTrack, addRecentlyPlayed, queue, currentIndex } from '$lib/stores/player';
  import { get } from 'svelte/store';
  import { fetchAlbumSongs, type Album } from '$lib/api';

  let { album, onplay, children, triggerClass }: {
    album: Album;
    onplay?: () => void;
    children: Snippet;
    triggerClass?: string;
  } = $props();

  async function playSongs(mode: 'now' | 'next' | 'queue') {
    let songs: Awaited<ReturnType<typeof fetchAlbumSongs>>;
    try {
      songs = await fetchAlbumSongs(album.id);
    } catch {
      toast.error('Failed to load album tracks');
      return;
    }
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
    <ContextMenu.Separator />
    <ContextMenu.Item onclick={() => goto(`/album/${encodeURIComponent(album.id)}`)}><Disc3 />Go to album</ContextMenu.Item>
    {#if album.artist}
      <ContextMenu.Item onclick={() => goto(`/artist/${encodeURIComponent(album.artist)}`)}><User />Go to artist</ContextMenu.Item>
    {/if}
  </ContextMenu.Content>
</ContextMenu.Root>
