<script lang="ts">
  import { Play, Pause } from '@lucide/svelte';
  import { goto } from '$app/navigation';
  import type { Song } from '$lib/servers';
  import { queue, currentIndex, isPlaying, togglePlayRequest } from '$lib/stores/player';
  import SongContextMenu from '$lib/components/SongContextMenu.svelte';
  import SongTechBadge from '$lib/components/SongTechBadge.svelte';
  import SongArtistLinks from '$lib/components/SongArtistLinks.svelte';
  import { initials, formatClockDuration } from '$lib/utils';

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
          <span class="absolute inset-0 flex items-center justify-center text-xs tabular-nums text-muted-foreground/60 transition-all duration-150 group-hover:opacity-0">{index + 1}</span>
          <span class="absolute inset-0 flex items-center justify-center opacity-0 transition-all duration-150 group-hover:opacity-100 text-muted-foreground">
            <Play class="size-3.5" fill="currentColor" />
          </span>
        {/if}
      </span>
    {/if}

    <!-- Cover art -->
    {#if song.coverArtUrl}
      <img class="h-10 w-10 shrink-0 rounded-md object-cover shadow-sm" src={song.coverArtUrl} alt={song.title} loading="lazy" />
      {:else}
      <div class="flex h-10 w-10 shrink-0 items-center justify-center rounded-md" style="background: linear-gradient(135deg, color-mix(in srgb, var(--color-muted) 10%, transparent) 0%, color-mix(in srgb, var(--color-muted) 5%, transparent) 100%);">
        <span class="text-xs font-bold text-muted-foreground/70">{initials(song.title)}</span>
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
        class="truncate text-xs text-muted-foreground/70"
        linkClass="hover:underline hover:text-muted-foreground transition-colors"
      />
    </div>

    <!-- Album (optional) -->
    {#if showAlbum}
      {#if song.albumId}
        <span
          role="link" tabindex="0"
          class="truncate text-xs text-muted-foreground/60 cursor-pointer hover:text-foreground hover:underline transition-colors"
          onclick={(e) => { e.stopPropagation(); goto(`/album/${encodeURIComponent(song.albumId)}`); }}
          onkeydown={(e) => { if (e.key === 'Enter') { e.stopPropagation(); goto(`/album/${encodeURIComponent(song.albumId)}`); } }}
        >{song.album}</span>
      {:else}
        <span class="truncate text-xs text-muted-foreground/60">{song.album}</span>
      {/if}
    {/if}

    <!-- Duration -->
    <span class="text-right text-xs tabular-nums text-muted-foreground/60">{formatClockDuration(song.duration ?? 0)}</span>
  </button>
</SongContextMenu>

<style>
  .song-row {
    transition: background 150ms ease;
  }
  .song-row:hover {
    background: color-mix(in srgb, var(--color-muted) 8%, transparent);
  }
  .song-row-active {
    background: color-mix(in srgb, var(--color-primary) 10%, transparent);
  }
  .song-row-active:hover {
    background: color-mix(in srgb, var(--color-primary) 14%, transparent);
  }

</style>
