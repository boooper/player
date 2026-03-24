<script lang="ts">
  import type { Snippet } from 'svelte';
  import { Play, ListStart, ListEnd, ListMusic, Pencil, Pin, Trash2 } from '@lucide/svelte';
  import { toast } from 'svelte-sonner';
  import { goto } from '$app/navigation';
  import * as ContextMenu from '$lib/components/ui/context-menu';
  import * as Dialog from '$lib/components/ui/dialog';
  import { Button } from '$lib/components/ui/button/index.js';
  import { appendToQueue, playQueue, playingFrom, focusTrack, addRecentlyPlayed, queue, currentIndex, subsonicPlaylists, pinnedPlaylistIds, togglePlaylistPinned } from '$lib/stores/player';
  import { requestLibraryRefresh } from '$lib/stores/ui-state';
  import { get } from 'svelte/store';
  import { fetchPlaylistSongs, fetchPlaylists, renamePlaylist, deletePlaylist, type Playlist } from '$lib/servers';

  let { playlist, onplay, children, triggerClass }: {
    playlist: Playlist;
    onplay?: () => void;
    children: Snippet;
    triggerClass?: string;
  } = $props();

  let renameDialogOpen = $state(false);
  let renameValue = $state('');
  let renaming = $state(false);
  let deleteDialogOpen = $state(false);
  let deleting = $state(false);
  const isPinned = $derived($pinnedPlaylistIds.has(playlist.id));

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

  function openRenameDialog() {
    renameValue = playlist.name;
    renameDialogOpen = true;
  }

  async function submitRename() {
    const nextName = renameValue.trim();
    if (!nextName) {
      toast.error('Playlist name is required');
      return;
    }
    if (nextName === playlist.name) {
      renameDialogOpen = false;
      return;
    }

    renaming = true;
    try {
      await renamePlaylist(playlist.id, nextName);
      subsonicPlaylists.update((lists) =>
        lists.map((item) => (item.id === playlist.id ? { ...item, name: nextName } : item))
      );
      playingFrom.update((current) =>
        current.href === `/playlist/${encodeURIComponent(playlist.id)}`
          ? { ...current, name: nextName }
          : current
      );
      requestLibraryRefresh();
      fetchPlaylists()
        .then((lists) => subsonicPlaylists.set(lists))
        .catch(() => undefined);
      renameDialogOpen = false;
      toast.success('Playlist renamed', { description: nextName });
    } catch (err) {
      toast.error(err instanceof Error ? err.message : 'Failed to rename playlist');
    } finally {
      renaming = false;
    }
  }

  function togglePinnedState() {
    const pinned = togglePlaylistPinned(playlist.id);
    toast.success(pinned ? 'Playlist pinned' : 'Playlist unpinned', {
      description: playlist.name,
    });
  }

  async function submitDelete() {
    deleting = true;
    try {
      await deletePlaylist(playlist.id);
      subsonicPlaylists.update((lists) => lists.filter((p) => p.id !== playlist.id));
      deleteDialogOpen = false;
      toast.success('Playlist deleted', { description: playlist.name });
      goto('/songs');
    } catch (err) {
      toast.error(err instanceof Error ? err.message : 'Failed to delete playlist');
    } finally {
      deleting = false;
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
    <ContextMenu.Item onclick={togglePinnedState}><Pin />{isPinned ? 'Unpin playlist' : 'Pin playlist'}</ContextMenu.Item>
    <ContextMenu.Item onclick={openRenameDialog}><Pencil />Rename playlist</ContextMenu.Item>
    <ContextMenu.Item onclick={() => goto(`/playlist/${encodeURIComponent(playlist.id)}`)}><ListMusic />Go to playlist</ContextMenu.Item>
    <ContextMenu.Separator />
    <ContextMenu.Item onclick={() => { deleteDialogOpen = true; }} class="text-destructive focus:text-destructive"><Trash2 />Delete playlist</ContextMenu.Item>
  </ContextMenu.Content>
</ContextMenu.Root>

<Dialog.Root bind:open={deleteDialogOpen}>
  <Dialog.Portal>
    <Dialog.Overlay class="fixed inset-0 z-50 bg-black/60 backdrop-blur-sm" />
    <Dialog.Content class="app-glass fixed left-1/2 top-1/2 z-50 w-full max-w-md -translate-x-1/2 -translate-y-1/2 rounded-[28px] p-6 outline-none">
      <Dialog.Header class="space-y-2">
        <Dialog.Title class="text-xl font-semibold text-foreground">Delete playlist</Dialog.Title>
        <Dialog.Description class="text-sm text-muted-foreground">
          Are you sure you want to delete <span class="font-medium text-foreground">{playlist.name}</span>? This cannot be undone.
        </Dialog.Description>
      </Dialog.Header>
      <Dialog.Footer class="mt-6 flex justify-end gap-2">
        <Dialog.Close>
          <Button variant="outline" size="default">Cancel</Button>
        </Dialog.Close>
        <Button variant="destructive" size="default" disabled={deleting} onclick={() => void submitDelete()} type="button">
          {deleting ? 'Deleting...' : 'Delete'}
        </Button>
      </Dialog.Footer>
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>

<Dialog.Root bind:open={renameDialogOpen}>
  <Dialog.Portal>
    <Dialog.Overlay class="fixed inset-0 z-50 bg-black/60 backdrop-blur-sm" />
    <Dialog.Content class="app-glass fixed left-1/2 top-1/2 z-50 w-full max-w-md -translate-x-1/2 -translate-y-1/2 rounded-[28px] p-6 outline-none">
      <Dialog.Header class="space-y-2">
        <Dialog.Title class="text-xl font-semibold text-foreground">Rename playlist</Dialog.Title>
        <Dialog.Description class="text-sm text-muted-foreground">
          Update the playlist name in your library.
        </Dialog.Description>
      </Dialog.Header>
      <form class="mt-5 space-y-4" onsubmit={(event) => { event.preventDefault(); void submitRename(); }}>
        <label class="block space-y-2">
          <span class="text-sm font-medium text-foreground">Playlist name</span>
          <input
            bind:value={renameValue}
            class="h-11 w-full rounded-xl border border-border bg-background/70 px-3 text-sm text-foreground outline-none transition-colors placeholder:text-muted-foreground focus:border-primary"
            maxlength="120"
            placeholder="Late night mix"
          />
        </label>
        <Dialog.Footer class="flex justify-end gap-2">
          <Dialog.Close>
            <Button variant="outline" size="default">Cancel</Button>
          </Dialog.Close>
          <Button variant="default" size="default" disabled={renaming} type="submit">
            {renaming ? 'Saving...' : 'Save'}
          </Button>
        </Dialog.Footer>
      </form>
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>
