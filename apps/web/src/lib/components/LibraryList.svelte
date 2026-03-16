<script lang="ts">
  import { Heart, ListMusic, Play, Pause, Music2, Disc3, Mic2, Plus, Pin } from '@lucide/svelte';
  import { page } from '$app/state';
  import * as Dialog from '$lib/components/ui/dialog';
  import { goto } from '$app/navigation';
  import { toast } from 'svelte-sonner';
  import { isPlaying, pinnedPlaylistIds, playingFrom, subsonicPlaylists, starredSongIds } from '$lib/stores/player';
  import { createPlaylist, fetchPlaylists, type Song } from '$lib/servers';
  import { requestLibraryRefresh } from '$lib/stores/ui-state';
  import PlaylistContextMenu from '$lib/components/PlaylistContextMenu.svelte';
  import ArtistContextMenu from '$lib/components/ArtistContextMenu.svelte';

  let {
    likedArtists,
    artistPhotos,
    starredSongs,
    selectedPlaylistId,
    onPlayPlaylist,
    compact = false,
  }: {
    likedArtists: string[];
    artistPhotos: Record<string, string>;
    starredSongs: Song[];
    selectedPlaylistId: string;
    onPlayPlaylist: (id: string) => void;
    compact?: boolean;
  } = $props();

  const starredSongCount = $derived($starredSongIds.size);
  const favoritesIsActive = $derived(page.url.pathname === '/favorites');
  const favoritesIsPlaying = $derived($playingFrom.href === '/favorites' && $isPlaying);
  const orderedPlaylists = $derived.by(() => {
    const pinned = $pinnedPlaylistIds;
    const playlists = $subsonicPlaylists;
    return [...playlists].sort((a, b) => {
      const aPinned = pinned.has(a.id);
      const bPinned = pinned.has(b.id);
      if (aPinned === bPinned) return 0;
      return aPinned ? -1 : 1;
    });
  });
  let createDialogOpen = $state(false);
  let playlistName = $state('');
  let creatingPlaylist = $state(false);

  function initials(name: string): string {
    return name.split(' ').filter(Boolean).slice(0, 2).map((p) => p[0]?.toUpperCase() ?? '').join('');
  }

  async function submitCreatePlaylist() {
    const nextName = playlistName.trim();
    if (!nextName) {
      toast.error('Playlist name is required');
      return;
    }

    creatingPlaylist = true;
    try {
      const playlist = await createPlaylist(nextName, []);
      subsonicPlaylists.update((lists) => [playlist, ...lists]);
      requestLibraryRefresh();
      fetchPlaylists()
        .then((lists) => subsonicPlaylists.set(lists))
        .catch(() => undefined);
      createDialogOpen = false;
      playlistName = '';
      toast.success('Playlist created', { description: playlist.name });
      goto(`/playlist/${encodeURIComponent(playlist.id)}`);
    } catch (err) {
      toast.error(err instanceof Error ? err.message : 'Failed to create playlist');
    } finally {
      creatingPlaylist = false;
    }
  }
</script>

<ul class="group/library m-0 list-none px-2 pb-4 pt-1 space-y-0.5 {compact ? 'pt-0.5' : ''}" data-compact={compact}>

  <!-- Browse nav links -->
  {#each [
    { href: '/songs',   label: 'Songs',   Icon: Music2 },
    { href: '/albums',  label: 'Albums',  Icon: Disc3 },
    { href: '/artists', label: 'Artists', Icon: Mic2 },
  ] as nav (nav.href)}
    <li>
      <a
        href={nav.href}
        class="flex items-center gap-3 rounded-xl px-3 py-2.5 transition-colors {compact ? 'justify-center px-2' : ''} {page.url.pathname === nav.href ? 'bg-white/[0.06] text-foreground' : 'text-muted-foreground hover:bg-white/[0.04] hover:text-foreground'}"
      >
        <div class="flex size-10 shrink-0 items-center justify-center rounded-xl bg-white/[0.04]">
          <nav.Icon class="size-4 text-muted-foreground" />
        </div>
        <div class="min-w-0 flex-1 group-data-[compact=true]/library:hidden group-data-[collapsible=icon]:hidden">
          <p class="truncate text-sm font-medium leading-tight text-foreground">{nav.label}</p>
          <p class="mt-0.5 text-xs text-muted-foreground">Browse library</p>
        </div>
      </a>
    </li>
  {/each}

  <li class="px-2 py-1 group-data-[compact=true]/library:hidden group-data-[collapsible=icon]:hidden"><div class="h-px bg-border"></div></li>

  <li class="group-data-[compact=true]/library:hidden group-data-[collapsible=icon]:hidden">
    <button
      class="flex w-full items-center gap-3 rounded-xl px-3 py-2.5 text-left transition-colors hover:bg-white/[0.04] {compact ? 'justify-center px-2' : ''}"
      onclick={() => { playlistName = ''; createDialogOpen = true; }}
      type="button"
    >
      <div class="flex size-10 shrink-0 items-center justify-center rounded-xl bg-white/[0.04]">
        <Plus class="size-4 text-muted-foreground" />
      </div>
      <div class="min-w-0 flex-1">
        <p class="truncate text-sm font-medium leading-tight text-foreground">Create playlist</p>
        <p class="mt-0.5 text-xs text-muted-foreground">Start with an empty playlist</p>
      </div>
    </button>
  </li>

  <!-- Liked Songs -->
  <li>
    <a
      href="/favorites"
      class="group/row flex items-center gap-3 rounded-xl px-3 py-2.5 transition-colors {compact ? 'justify-center px-2' : ''} {favoritesIsActive ? 'bg-white/[0.06]' : 'hover:bg-white/[0.04]'}"
    >
      <div class="group/cover relative size-10 shrink-0 flex-none overflow-hidden rounded-md">
        {#if starredSongs.length >= 4}
          <div class="grid h-full w-full grid-cols-2 grid-rows-2">
            {#each Array(4) as _, i (i)}
              <img src={starredSongs[i].coverArtUrl} alt="" class="h-full w-full object-cover" />
            {/each}
          </div>
        {:else}
          <div class="flex h-full w-full items-center justify-center rounded-md bg-white/[0.04]">
            <Heart class="size-4 text-muted-foreground" fill="currentColor" />
          </div>
        {/if}
        {#if favoritesIsPlaying}
          <div class="absolute inset-0 flex items-end justify-center gap-[2px] rounded-md bg-black/50 pb-1.5 pointer-events-none">
            <span class="w-[3px] rounded-sm bg-primary" style="height:5px;animation:now-playing-bar 0.8s ease-in-out infinite alternate"></span>
            <span class="w-[3px] rounded-sm bg-primary" style="height:9px;animation:now-playing-bar 0.8s ease-in-out 0.2s infinite alternate"></span>
            <span class="w-[3px] rounded-sm bg-primary" style="height:6px;animation:now-playing-bar 0.8s ease-in-out 0.4s infinite alternate"></span>
          </div>
        {/if}
      </div>
      <div class="min-w-0 flex-1 group-data-[compact=true]/library:hidden group-data-[collapsible=icon]:hidden">
        <p class="truncate text-sm font-medium leading-tight {favoritesIsActive ? 'text-primary' : 'text-foreground'}">Liked Songs</p>
        <p class="mt-0.5 truncate text-xs text-muted-foreground">Playlist &bull; {starredSongCount} songs</p>
      </div>
    </a>
  </li>

  <!-- Playlists -->
  {#each orderedPlaylists as playlist (playlist.id)}
    {@const isActive = selectedPlaylistId === playlist.id}
    {@const isPinned = $pinnedPlaylistIds.has(playlist.id)}
    <li>
      <PlaylistContextMenu {playlist} onplay={() => onPlayPlaylist(playlist.id)} triggerClass="block w-full">
        <a
          href={`/playlist/${encodeURIComponent(playlist.id)}`}
          class="group/row flex items-center gap-3 rounded-xl px-3 py-2.5 transition-colors {compact ? 'justify-center px-2' : ''} {isActive ? 'bg-white/[0.06]' : 'hover:bg-white/[0.04]'}"
        >
          <div class="group/cover relative size-10 shrink-0 flex-none overflow-hidden rounded-md">
            {#if playlist.coverArtUrl}
              <img src={playlist.coverArtUrl} alt={playlist.name} class="h-full w-full object-cover" />
            {:else}
              <div class="flex h-full w-full items-center justify-center rounded-md bg-white/[0.04]">
                <ListMusic class="size-4 text-muted-foreground" />
              </div>
            {/if}
            <button
              class="absolute inset-0 flex items-center justify-center rounded-md bg-black/60 opacity-0 transition-opacity group-hover/row:opacity-100 {isActive && $isPlaying ? '!opacity-100' : ''}"
              onclick={(e) => { e.preventDefault(); e.stopPropagation(); onPlayPlaylist(playlist.id); }}
            >
              {#if isActive && $isPlaying}
                <Pause class="size-4 text-white" />
              {:else}
                <Play class="size-4 text-white" />
              {/if}
            </button>
            {#if isActive && $isPlaying}
              <div class="absolute inset-0 flex items-end justify-center gap-[2px] rounded-md bg-black/50 pb-1.5 pointer-events-none">
                <span class="w-[3px] rounded-sm bg-primary" style="height:5px;animation:now-playing-bar 0.8s ease-in-out infinite alternate"></span>
                <span class="w-[3px] rounded-sm bg-primary" style="height:9px;animation:now-playing-bar 0.8s ease-in-out 0.2s infinite alternate"></span>
                <span class="w-[3px] rounded-sm bg-primary" style="height:6px;animation:now-playing-bar 0.8s ease-in-out 0.4s infinite alternate"></span>
              </div>
            {/if}
          </div>
          <div class="min-w-0 flex-1 group-data-[compact=true]/library:hidden group-data-[collapsible=icon]:hidden">
            <div class="flex items-center gap-1.5">
              {#if isPinned}
                <Pin class="size-3 text-primary" fill="currentColor" />
              {/if}
              <p class="truncate text-sm font-medium leading-tight {isActive ? 'text-primary' : 'text-foreground'}">{playlist.name}</p>
            </div>
            <p class="mt-0.5 truncate text-xs text-muted-foreground">{isPinned ? 'Pinned playlist' : 'Playlist'} &bull; {playlist.songCount} songs</p>
          </div>
        </a>
      </PlaylistContextMenu>
    </li>
  {/each}

  {#if likedArtists.length > 0}
    <li class="px-2 py-1 group-data-[compact=true]/library:hidden group-data-[collapsible=icon]:hidden"><div class="h-px bg-border"></div></li>
  {/if}

  <!-- Liked Artists -->
  {#each likedArtists as artist (artist)}
    <li>
      <ArtistContextMenu name={artist} triggerClass="block w-full">
        <a
          href={`/artist/${encodeURIComponent(artist)}`}
          class="flex items-center gap-3 rounded-xl px-3 py-2.5 transition-colors hover:bg-white/[0.04] {compact ? 'justify-center px-2' : ''}"
        >
          <div class="size-10 shrink-0 flex-none overflow-hidden rounded-full">
            {#if artistPhotos[artist]}
              <img src={artistPhotos[artist]} alt={artist} class="h-full w-full object-cover" />
            {:else}
              <div class="flex h-full w-full items-center justify-center bg-gradient-to-br from-slate-600 to-slate-800 text-xs font-bold text-white/70">{initials(artist)}</div>
            {/if}
          </div>
          <div class="min-w-0 flex-1 group-data-[compact=true]/library:hidden group-data-[collapsible=icon]:hidden">
            <p class="truncate text-sm font-medium leading-tight text-foreground">{artist}</p>
            <p class="mt-0.5 text-xs text-muted-foreground">Artist</p>
          </div>
        </a>
      </ArtistContextMenu>
    </li>
  {/each}

</ul>

<Dialog.Root bind:open={createDialogOpen}>
  <Dialog.Portal>
    <Dialog.Overlay class="fixed inset-0 z-50 bg-black/60 backdrop-blur-sm" />
    <Dialog.Content class="app-glass fixed left-1/2 top-1/2 z-50 w-full max-w-md -translate-x-1/2 -translate-y-1/2 rounded-[28px] p-6 outline-none">
      <Dialog.Header class="space-y-2">
        <Dialog.Title class="text-xl font-semibold text-foreground">Create playlist</Dialog.Title>
        <Dialog.Description class="text-sm text-muted-foreground">
          Add an empty playlist to your library and fill it later.
        </Dialog.Description>
      </Dialog.Header>
      <form class="mt-5 space-y-4" onsubmit={(event) => { event.preventDefault(); void submitCreatePlaylist(); }}>
        <label class="block space-y-2">
          <span class="text-sm font-medium text-foreground">Playlist name</span>
          <input
            bind:value={playlistName}
            class="h-11 w-full rounded-xl border border-border bg-background/70 px-3 text-sm text-foreground outline-none transition-colors placeholder:text-muted-foreground focus:border-primary"
            maxlength="120"
            placeholder="Road trip"
          />
        </label>
        <Dialog.Footer class="flex justify-end gap-2">
          <Dialog.Close class="app-round-button h-10 px-4 text-sm">Cancel</Dialog.Close>
          <button
            class="app-round-button h-10 px-4 text-sm text-foreground disabled:cursor-not-allowed disabled:opacity-50"
            disabled={creatingPlaylist}
            type="submit"
          >
            {creatingPlaylist ? 'Creating...' : 'Create'}
          </button>
        </Dialog.Footer>
      </form>
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>
