<script lang="ts">
  import type { Snippet } from 'svelte';
  import { Play, ListStart, ListEnd, ListMusic } from '@lucide/svelte';
  import { toast } from 'svelte-sonner';
  import { goto } from '$app/navigation';
  import * as ContextMenu from '$lib/components/ui/context-menu';
  import { appendToQueue, playQueue, playingFrom, focusTrack, addRecentlyPlayed, queue, currentIndex } from '$lib/stores/player';
  import { get } from 'svelte/store';
  import { fetchPlaylistSongs, type Playlist } from '$lib/api';

  let { playlist, onplay, children, triggerClass }: {
    playlist: Playlist;
    onplay?: () => void;
    children: Snippet;
    triggerClass?: string;
  } = $props();

  async function playSongs(mode: 'now' | 'next' | 'queue') {
    let songs: Awaited<ReturnType<typeof fetchPlaylistSongs>>;
    try {
      songs = await fetchPlaylistSongs(playlist.id);
    } catch {
      toast.error('Failed to load playlist tracks');
      return;
    }
    if (!songs.length) { toast.warning('Playlist has no tracks'); return; }

    if (mode === 'now') {
      focusTrack.set({ title: songs[0].title, artist: songs[0].artist, imageUrl: songs[0].coverArtUrl, source: 'library', album: songs[0].album });
      playQueue(songs, 0);
      playingFrom.set({ type: 'playlist', name: playlist.name, href: `/playlist/${encodeURIComponent(playlist.id)}` });
      addRecentlyPlayed({ id: playlist.id, name: playlist.name, coverArtUrl: playlist.coverArtUrl, href: `/playlist/${encodeURIComponent(playlist.id)}`, type: 'playlist' });
    } else if (mode === 'next') {
      if (!get(queue).length) {
        focusTrack.set({ title: songs[0].title, artist: songs[0].artist, imageUrl: songs[0].coverArtUrl, source: 'library', album: songs[0].album });
        playQueue(songs, 0);
        playingFrom.set({ type: 'playlist', name: playlist.name, href: `/playlist/${encodeURIComponent(playlist.id)}` });
      } else {
        const idx = get(currentIndex);
        queue.update((current) => {
          const next = [...current];
          next.splice(idx + 1, 0, ...songs);
          return next;
        });
        toast.success('Playing next', { description: playlist.name });
      }
    } else {
      appendToQueue(songs);
      toast.success('Added to queue', { description: `${songs.length} tracks from ${playlist.name}` });
    }
  }
</script>

<ContextMenu.Root>
  <ContextMenu.Trigger class={triggerClass ?? 'block w-full'}>
    {@render children()}
  </ContextMenu.Trigger>
  <ContextMenu.Content>
    {#if onplay}
      <ContextMenu.Item onclick={onplay}><Play />Play playlist</ContextMenu.Item>
    {:else}
      <ContextMenu.Item onclick={() => playSongs('now')}><Play />Play playlist</ContextMenu.Item>
    {/if}
    <ContextMenu.Item onclick={() => playSongs('next')}><ListStart />Play next</ContextMenu.Item>
    <ContextMenu.Item onclick={() => playSongs('queue')}><ListEnd />Add to queue</ContextMenu.Item>
    <ContextMenu.Separator />
    <ContextMenu.Item onclick={() => goto(`/playlist/${encodeURIComponent(playlist.id)}`)}><ListMusic />Go to playlist</ContextMenu.Item>
  </ContextMenu.Content>
</ContextMenu.Root>
