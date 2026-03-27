<script lang="ts">
  import { goto } from '$app/navigation';
  import { Play, Pause, Loader } from '@lucide/svelte';
  import type { Album } from '$lib/servers';
  import { fetchAlbumSongs } from '$lib/servers';
  import AlbumContextMenu from '$lib/components/AlbumContextMenu.svelte';
  import { startQueue, playingFrom, isPlaying, togglePlayRequest } from '$lib/stores/player';
  import { activeServerType } from '$lib/stores/ui-state';
  import { initials } from '$lib/utils';
  import { toast } from 'svelte-sonner';

  let { album }: { album: Album } = $props();

  let loading = $state(false);

  const albumHref = $derived(`/album/${encodeURIComponent(album.id)}`);
  const isActive = $derived($playingFrom.href === albumHref);

  async function play(event: MouseEvent) {
    event.stopPropagation();
    if (isActive) {
      togglePlayRequest.update((n) => n + 1);
      return;
    }
    loading = true;
    try {
      const all = await fetchAlbumSongs(album.id);
      const isOctoFiesta = $activeServerType === 'octo_fiesta';
      const songs = isOctoFiesta ? all : all.filter((s) => !s.id.startsWith('ext-'));
      if (!songs.length) { toast.warning('No locally available tracks for this album'); return; }
      startQueue(songs, 0, { type: 'album', name: album.name, href: albumHref });
    } finally {
      loading = false;
    }
  }
</script>

<AlbumContextMenu {album}>
  <div class="album-card group relative w-full rounded-xl text-left">
    <!-- Cover art with play overlay -->
    <div class="relative mb-3">
      <button class="block w-full" onclick={() => goto(albumHref)} aria-label="Open {album.name}">
        {#if album.coverArtUrl}
          <img class="aspect-square w-full rounded-lg object-cover shadow-md" src={album.coverArtUrl} alt={album.name} loading="lazy" />
        {:else}
          <div class="flex aspect-square w-full items-center justify-center rounded-lg bg-card text-xl font-bold text-muted-foreground ring-1 ring-border/6">
            {initials(album.name)}
          </div>
        {/if}
      </button>
      <!-- Play button overlay -->
      <button
        class="absolute bottom-2 right-2 flex size-9 items-center justify-center rounded-full bg-primary text-primary-foreground shadow-lg transition-all duration-150 hover:scale-105 hover:brightness-110 active:scale-95 {isActive ? 'opacity-100 translate-y-0' : 'opacity-0 translate-y-1 group-hover:opacity-100 group-hover:translate-y-0'}"
        onclick={play}
        aria-label={isActive && $isPlaying ? `Pause ${album.name}` : `Play ${album.name}`}
      >
        {#if loading}
          <Loader class="size-4 animate-spin" />
        {:else if isActive && $isPlaying}
          <Pause class="size-4" fill="currentColor" />
        {:else}
          <Play class="size-4 translate-x-0.5" fill="currentColor" />
        {/if}
      </button>
    </div>
    <button class="w-full text-left" onclick={() => goto(albumHref)}>
      <p class="truncate text-sm font-semibold" class:text-primary={isActive}>{album.name}</p>
      <p class="truncate text-xs text-white/40">{album.year ? `${album.year} · ` : ''}{album.artist}</p>
    </button>
  </div>
</AlbumContextMenu>

<style>
  .album-card {
    background: transparent;
    border: 1px solid transparent;
    transition: background 150ms ease;
  }
  .album-card:hover {
    background: color-mix(in srgb, var(--color-muted) 7%, transparent);
  }
</style>
