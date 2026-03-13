<script lang="ts">
  import type { Snippet } from 'svelte';
  import { Heart, HeartOff, ListStart, ListEnd, ListMusic, Play, Radio, User, Disc3 } from '@lucide/svelte';
  import { toast } from 'svelte-sonner';
  import { goto } from '$app/navigation';
  import * as ContextMenu from '$lib/components/ui/context-menu';
  import {
    appendToQueue,
    playNextInQueue,
    startRadio,
    subsonicPlaylists,
    starredSongIds
  } from '$lib/stores/player';
  import {
    starSong,
    unstarSong,
    addSongToPlaylist,
    type Song
  } from '$lib/api';
  import { appSettings } from '$lib/stores/settings';

  let { song, onplay, children, triggerClass }: {
    song: Song;
    onplay?: () => void;
    children: Snippet;
    triggerClass?: string;
  } = $props();

  const lastFmApiKey = $derived($appSettings.lastFmApiKey);

  const isStarred = $derived($starredSongIds.has(song.id));

  async function launchRadio() {
    if (!lastFmApiKey) {
      toast.error('Last.fm API key not configured');
      return;
    }
    const toastId = toast.loading(`Building radio for "${song.title}"…`);
    try {
      const { queued } = await startRadio(song, lastFmApiKey);
      if (queued === 0) {
        toast.warning('No similar tracks found', { id: toastId, description: 'Try a more popular song.' });
      } else {
        toast.success(`Radio started`, { id: toastId, description: `${song.title} + ${queued} similar tracks` });
      }
    } catch {
      toast.error('Failed to build radio', { id: toastId });
    }
  }

  async function toggleFavorite() {
    if (isStarred) {
      starredSongIds.update((ids) => { const s = new Set(ids); s.delete(song.id); return s; });
      try {
        await unstarSong(song.id, song.artist, song.title);
        toast.success('Removed from favorites', { description: song.title });
      } catch {
        starredSongIds.update((ids) => new Set([...ids, song.id]));
        toast.error('Failed to remove from favorites');
      }
    } else {
      starredSongIds.update((ids) => new Set([...ids, song.id]));
      try {
        await starSong(song.id, song.artist, song.title);
        toast.success('Added to favorites', { description: song.title });
      } catch {
        starredSongIds.update((ids) => { const s = new Set(ids); s.delete(song.id); return s; });
        toast.error('Failed to add to favorites');
      }
    }
  }

  async function addToPlaylist(playlistId: string, playlistName: string) {
    try {
      await addSongToPlaylist(playlistId, song.id);
      // Optimistically update the sidebar playlist song count
      subsonicPlaylists.update(lists =>
        lists.map(pl => pl.id === playlistId ? { ...pl, songCount: pl.songCount + 1 } : pl)
      );
      toast.success(`Added to ${playlistName}`, { description: song.title });
    } catch {
      toast.error('Failed to add to playlist');
    }
  }
</script>

<ContextMenu.Root>
  <ContextMenu.Trigger class={triggerClass ?? 'block w-full'}>
    {@render children()}
  </ContextMenu.Trigger>
  <ContextMenu.Content>
    {#if onplay}
      <ContextMenu.Item onclick={onplay}><Play />Play now</ContextMenu.Item>
    {/if}
    <ContextMenu.Item onclick={launchRadio}><Radio />Start radio</ContextMenu.Item>
    <ContextMenu.Item onclick={() => playNextInQueue(song)}><ListStart />Play next</ContextMenu.Item>
    <ContextMenu.Item onclick={() => appendToQueue([song])}><ListEnd />Add to queue</ContextMenu.Item>
    <ContextMenu.Separator />
    <ContextMenu.Item onclick={() => goto(`/artist/${encodeURIComponent(song.artist)}`)}><User />Go to artist</ContextMenu.Item>
    {#if song.albumId}
      <ContextMenu.Item onclick={() => goto(`/album/${encodeURIComponent(song.albumId)}`)}><Disc3 />Go to album</ContextMenu.Item>
    {/if}
    <ContextMenu.Separator />
    <ContextMenu.Item onclick={toggleFavorite}>
      {#if isStarred}<HeartOff />Remove from favorites{:else}<Heart />Add to favorites{/if}
    </ContextMenu.Item>
    {#if $subsonicPlaylists.length}
      <ContextMenu.Sub>
        <ContextMenu.SubTrigger><ListMusic />Add to playlist</ContextMenu.SubTrigger>
        <ContextMenu.SubContent>
          {#each $subsonicPlaylists as pl (pl.id)}
            <ContextMenu.Item onclick={() => addToPlaylist(pl.id, pl.name)}>{pl.name}</ContextMenu.Item>
          {/each}
        </ContextMenu.SubContent>
      </ContextMenu.Sub>
    {/if}
  </ContextMenu.Content>
</ContextMenu.Root>
