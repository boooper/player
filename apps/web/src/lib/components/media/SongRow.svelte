<script lang="ts">
  import { Play, Pause } from '@lucide/svelte';
  import type { Song } from '$lib/servers';
  import { queue, currentIndex, isPlaying, togglePlayRequest } from '$lib/stores/player';
  import SongContextMenu from '$lib/components/SongContextMenu.svelte';
  import SongTechBadge from '$lib/components/SongTechBadge.svelte';
  import SongArtistLinks from '$lib/components/SongArtistLinks.svelte';

  let {
    song,
    index,
    onplay,
    onremove,
    showAlbum = false,
    staggerIndex,
    cached = null
  }: {
    song: Song;
    index?: number;
    onplay: () => void;
    onremove?: () => void;
    showAlbum?: boolean;
    staggerIndex?: number;
    cached?: boolean | null;
  } = $props();

  const isActive = $derived($queue[$currentIndex]?.id === song.id);

  function handleClick() {
    if (isActive) {
      togglePlayRequest.update((n) => n + 1);
    } else {
      onplay();
    }
  }

  function initials(name: string) {
    return name.split(' ').filter(Boolean).slice(0, 2).map((p) => p[0]?.toUpperCase() ?? '').join('');
  }

  function formatDuration(s: number) {
    const m = Math.floor(s / 60);
    return `${m}:${Math.floor(s % 60).toString().padStart(2, '0')}`;
  }
</script>

<SongContextMenu {song} {onplay} {onremove}>
  <button
    class="song-row group w-full rounded-lg px-3 py-2.5 text-left"
    class:song-row-active={isActive}
    style:--stagger-index={staggerIndex}
    style="grid-template-columns: {index !== undefined ? '2rem ' : ''}2.5rem 1fr{showAlbum ? ' 1fr' : ''} 4rem; display: grid; align-items: center; gap: 12px;"
    onclick={handleClick}
  >
    <!-- Track number / equalizer / play indicator -->
    {#if index !== undefined}
      <span class="relative flex h-6 w-6 shrink-0 items-center justify-center mx-auto">
        {#if isActive}
          <span class="absolute inset-0 flex items-center justify-center text-primary">
            {#if $isPlaying}<Pause class="size-3.5" fill="currentColor" />{:else}<Play class="size-3.5" fill="currentColor" />{/if}
          </span>
        {:else}
          <span class="absolute inset-0 flex items-center justify-center text-xs tabular-nums text-white/30 transition-all duration-150 group-hover:opacity-0">{index + 1}</span>
          <span class="absolute inset-0 flex items-center justify-center opacity-0 transition-all duration-150 group-hover:opacity-100 text-white/70">
            <Play class="size-3.5" fill="currentColor" />
          </span>
        {/if}
      </span>
    {/if}

    <!-- Cover art -->
    {#if song.coverArtUrl}
      <img class="h-10 w-10 shrink-0 rounded-md object-cover shadow-sm" src={song.coverArtUrl} alt={song.title} loading="lazy" />
    {:else}
      <div class="flex h-10 w-10 shrink-0 items-center justify-center rounded-md bg-gradient-to-br from-white/10 to-white/5 text-xs font-bold text-white/40">
        {initials(song.title)}
      </div>
    {/if}

    <!-- Title + artist -->
    <div class="min-w-0">
      <div class="flex items-center gap-1.5">
        <p class="truncate text-sm font-medium transition-colors" class:text-primary={isActive}>{song.title}</p>
        <SongTechBadge {cached} audioFormat={song.audioFormat} bitrateKbps={song.bitrateKbps} compact />
      </div>
      <SongArtistLinks
        artist={song.artist}
        class="truncate text-xs text-white/40"
        linkClass="hover:underline hover:text-white/70 transition-colors"
      />
    </div>

    <!-- Album (optional) -->
    {#if showAlbum}
      <span class="truncate text-xs text-white/30">{song.album}</span>
    {/if}

    <!-- Duration -->
    <span class="text-right text-xs tabular-nums text-white/30">{formatDuration(song.duration ?? 0)}</span>
  </button>
</SongContextMenu>

<style>
  .song-row {
    transition: background 150ms ease;
  }
  .song-row:hover {
    background: rgba(255, 255, 255, 0.05);
  }
  .song-row-active {
    background: hsl(var(--primary) / 0.1);
  }
  .song-row-active:hover {
    background: hsl(var(--primary) / 0.15);
  }

</style>
