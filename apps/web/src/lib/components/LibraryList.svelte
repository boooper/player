<script lang="ts">
  import { Album, Disc3, Heart, Home, ListMusic, Mic2, Pause, Pin, Play } from '@lucide/svelte';
  import { page } from '$app/state';
  import * as Dialog from '$lib/components/ui/dialog';
  import { goto } from '$app/navigation';
  import { toast } from 'svelte-sonner';
  import { isPlaying, pinnedPlaylistIds, playingFrom, subsonicPlaylists, starredSongIds } from '$lib/stores/player';
  import { createPlaylist, fetchPlaylists, type Song } from '$lib/servers';
  import { requestLibraryRefresh } from '$lib/stores/ui-state';
  import PlaylistContextMenu from '$lib/components/PlaylistContextMenu.svelte';
  import { initials } from '$lib/utils';
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

  const favoritesIsActive = $derived(page.url.pathname === '/favorites');
  const favoritesIsPlaying = $derived($playingFrom.href === '/favorites' && $isPlaying);
  const homeIsActive = $derived(page.url.pathname === '/');
  const songsIsActive = $derived(page.url.pathname === '/songs');
  const albumsIsActive = $derived(page.url.pathname.startsWith('/albums'));
  const artistsIsActive = $derived(page.url.pathname.startsWith('/artists'));
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

  export function openCreatePlaylist() {
    playlistName = '';
    createDialogOpen = true;
  }


  const collectionLinks = [
    {
      href: '/',
      label: 'For You',
      meta: 'Home',
      icon: Home,
      active: () => homeIsActive,
    },
    {
      href: '/songs',
      label: 'Songs',
      meta: 'All tracks',
      icon: Disc3,
      active: () => songsIsActive,
    },
    {
      href: '/album',
      label: 'Albums',
      meta: 'Full releases',
      icon: Album,
      active: () => albumsIsActive,
    },
    {
      href: '/artist',
      label: 'Artists',
      meta: 'Saved and discovered',
      icon: Mic2,
      active: () => artistsIsActive,
    },
  ];

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

<ul class="group/library m-0 list-none px-1.5 pb-3 pt-1.5 {compact ? 'space-y-1 pt-1' : 'space-y-3'}" data-compact={compact}>
  <li class="group-data-[compact=true]/library:hidden">
    <div class="px-3">
      <p class="text-[10px] font-semibold uppercase tracking-[0.2em] text-muted-foreground/70">Collection</p>
    </div>
  </li>

  {#each collectionLinks as item (item.href)}
    <li>
      <a
        href={item.href}
        class="flex items-center gap-2.5 rounded-[18px] px-2.5 py-2 transition-colors {compact ? 'justify-center px-1.5' : ''} {item.active() ? 'bg-white/10 text-primary ring-1 ring-white/8' : 'hover:bg-white/6'}"
      >
        <div class="flex size-8 shrink-0 items-center justify-center rounded-lg bg-white/5 ring-1 ring-white/8">
          <item.icon class="size-3.5 {item.active() ? 'text-primary' : 'text-muted-foreground'}" />
        </div>
        <div class="min-w-0 flex-1 group-data-[compact=true]/library:hidden group-data-[collapsible=icon]:hidden">
          <p class="truncate text-[13px] font-medium leading-tight {item.active() ? 'text-primary' : 'text-foreground'}">{item.label}</p>
          <p class="mt-0.5 truncate text-[11px] text-muted-foreground">{item.meta}</p>
        </div>
      </a>
    </li>
  {/each}

  <li class="group-data-[compact=true]/library:hidden">
    <div class="px-3 pt-1">
      <p class="text-[10px] font-semibold uppercase tracking-[0.2em] text-muted-foreground/70">Playlists</p>
    </div>
  </li>

  <li>
    <a
      href="/favorites"
      class="group/row flex items-center gap-2.5 rounded-[18px] px-2.5 py-2 transition-colors {compact ? 'justify-center px-1.5' : ''} {favoritesIsActive ? 'bg-white/10 text-primary ring-1 ring-white/8' : 'hover:bg-white/6'}"
    >
      <div class="group/cover relative size-8 shrink-0 flex-none overflow-hidden rounded-lg bg-white/5 ring-1 ring-white/8">
        {#if starredSongs.length >= 4}
          <div class="grid h-full w-full grid-cols-2 grid-rows-2">
            {#each Array(4) as _, i (i)}
              <img src={starredSongs[i].coverArtUrl} alt="" class="h-full w-full object-cover" />
            {/each}
          </div>
        {:else}
          <div class="flex h-full w-full items-center justify-center rounded-lg bg-[linear-gradient(160deg,rgba(255,255,255,0.14),rgba(255,255,255,0.03))]">
            <Heart class="size-3.5 text-primary" fill="currentColor" />
          </div>
        {/if}
        {#if favoritesIsPlaying}
          <div class="pointer-events-none absolute inset-0 flex items-end justify-center gap-[2px] rounded-lg bg-black/50 pb-1">
            <span class="w-[3px] rounded-sm bg-primary" style="height:5px;animation:now-playing-bar 0.8s ease-in-out infinite alternate"></span>
            <span class="w-[3px] rounded-sm bg-primary" style="height:9px;animation:now-playing-bar 0.8s ease-in-out 0.2s infinite alternate"></span>
            <span class="w-[3px] rounded-sm bg-primary" style="height:6px;animation:now-playing-bar 0.8s ease-in-out 0.4s infinite alternate"></span>
          </div>
        {/if}
      </div>
      <div class="min-w-0 flex-1 group-data-[compact=true]/library:hidden group-data-[collapsible=icon]:hidden">
        <p class="truncate text-[13px] font-medium leading-tight {favoritesIsActive ? 'text-primary' : 'text-foreground'}">Liked Songs</p>
        <p class="mt-0.5 truncate text-[11px] text-muted-foreground">Playlist • {$starredSongIds.size} songs</p>
      </div>
    </a>
  </li>

  {#each orderedPlaylists as playlist (playlist.id)}
    {@const isActive = selectedPlaylistId === playlist.id}
    {@const isPinned = $pinnedPlaylistIds.has(playlist.id)}
    <li>
      <PlaylistContextMenu {playlist} onplay={() => onPlayPlaylist(playlist.id)} triggerClass="block w-full">
        <a
          href={`/playlist/${encodeURIComponent(playlist.id)}`}
          class="group/row flex items-center gap-2.5 rounded-[18px] px-2.5 py-2 transition-colors {compact ? 'justify-center px-1.5' : ''} {isActive ? 'bg-white/10 text-primary ring-1 ring-white/8' : 'hover:bg-white/6'}"
        >
          <div class="group/cover relative size-8 shrink-0 flex-none overflow-hidden rounded-lg bg-white/5 ring-1 ring-white/8">
            {#if playlist.coverArtUrl}
              <img src={playlist.coverArtUrl} alt={playlist.name} class="h-full w-full object-cover" />
            {:else}
              <div class="flex h-full w-full items-center justify-center rounded-lg bg-transparent">
                <ListMusic class="size-3.5 text-muted-foreground" />
              </div>
            {/if}
            <button
              class="absolute inset-0 flex items-center justify-center rounded-lg bg-black/60 opacity-0 transition-opacity group-hover/row:opacity-100 {isActive && $isPlaying ? '!opacity-100' : ''}"
              onclick={(e) => { e.preventDefault(); e.stopPropagation(); onPlayPlaylist(playlist.id); }}
            >
              {#if isActive && $isPlaying}
                <Pause class="size-3.5 text-white" />
              {:else}
                <Play class="size-3.5 text-white" />
              {/if}
            </button>
            {#if isActive && $isPlaying}
              <div class="pointer-events-none absolute inset-0 flex items-end justify-center gap-[2px] rounded-lg bg-black/50 pb-1">
                <span class="w-[3px] rounded-sm bg-primary" style="height:5px;animation:now-playing-bar 0.8s ease-in-out infinite alternate"></span>
                <span class="w-[3px] rounded-sm bg-primary" style="height:9px;animation:now-playing-bar 0.8s ease-in-out 0.2s infinite alternate"></span>
                <span class="w-[3px] rounded-sm bg-primary" style="height:6px;animation:now-playing-bar 0.8s ease-in-out 0.4s infinite alternate"></span>
              </div>
            {/if}
          </div>
          <div class="min-w-0 flex-1 group-data-[compact=true]/library:hidden group-data-[collapsible=icon]:hidden">
            <div class="flex items-center gap-1.5">
              {#if isPinned}
                <Pin class="size-2.5 text-primary" fill="currentColor" />
              {/if}
              <p class="truncate text-[13px] font-medium leading-tight {isActive ? 'text-primary' : 'text-foreground'}">{playlist.name}</p>
            </div>
            <p class="mt-0.5 truncate text-[11px] text-muted-foreground">{isPinned ? 'Pinned playlist' : 'Playlist'} &bull; {playlist.songCount} songs</p>
          </div>
        </a>
      </PlaylistContextMenu>
    </li>
  {/each}

  {#if likedArtists.length > 0}
    <li class="group-data-[compact=true]/library:hidden">
      <div class="px-3 pt-1">
        <p class="text-[10px] font-semibold uppercase tracking-[0.2em] text-muted-foreground/70">Favorite Artists</p>
      </div>
    </li>

    {#each likedArtists as artist (artist)}
      <li>
        <ArtistContextMenu name={artist} triggerClass="block w-full">
          <a
            href={`/artist/${encodeURIComponent(artist)}`}
            class="flex items-center gap-2.5 rounded-[18px] px-2.5 py-2 transition-colors hover:bg-white/6 {compact ? 'justify-center px-1.5' : ''}"
          >
            <div class="size-8 shrink-0 flex-none overflow-hidden rounded-full bg-white/5 ring-1 ring-white/8">
              {#if artistPhotos[artist]}
                <img src={artistPhotos[artist]} alt={artist} class="h-full w-full object-cover" />
              {:else}
                <div class="flex h-full w-full items-center justify-center rounded-full" style="background: linear-gradient(135deg, color-mix(in srgb, var(--color-muted) 12%, transparent) 0%, color-mix(in srgb, var(--color-muted) 6%, transparent) 100%);">
                  <span class="text-xs font-bold text-muted-foreground">{initials(artist)}</span>
                </div>
              {/if}
            </div>
            <div class="min-w-0 flex-1 group-data-[compact=true]/library:hidden group-data-[collapsible=icon]:hidden">
              <p class="truncate text-[13px] font-medium leading-tight text-foreground">{artist}</p>
              <p class="mt-0.5 text-[11px] text-muted-foreground">Artist</p>
            </div>
          </a>
        </ArtistContextMenu>
      </li>
    {/each}
  {/if}
</ul>

<Dialog.Root bind:open={createDialogOpen}>
  <Dialog.Portal>
    <Dialog.Overlay class="fixed inset-0 z-50 bg-black/60 backdrop-blur-sm" />
    <Dialog.Content class="liquid-glass border border-border/40 fixed left-1/2 top-1/2 z-50 w-full max-w-md -translate-x-1/2 -translate-y-1/2 rounded-[28px] p-6 outline-none">
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
