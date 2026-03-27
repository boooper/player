<script lang="ts">
  import { Play, Pause } from '@lucide/svelte';
  import type { Song } from '$lib/servers';
  import { queue, currentIndex, isPlaying, togglePlayRequest } from '$lib/stores/player';
  import SongContextMenu from '$lib/components/SongContextMenu.svelte';
  import { initials } from '$lib/utils';

  let {
    song,
    onplay
  }: {
    song: Song;
    onplay: () => void;
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

<SongContextMenu {song} {onplay}>
  <button class="song-card group w-full rounded-xl p-3 text-left" class:song-card-active={isActive} onclick={handleClick}>
    <div class="relative mb-3">
      {#if song.coverArtUrl}
        <img class="aspect-square w-full rounded-lg object-cover shadow-md" src={song.coverArtUrl} alt={song.title} loading="lazy" />
      {:else}
        <div class="flex aspect-square w-full items-center justify-center rounded-lg bg-gradient-to-br from-white/10 to-white/5 text-xl font-bold text-white/40">
          {initials(song.title)}
        </div>
      {/if}
      <div class="play-fab absolute bottom-2 right-2 flex h-9 w-9 items-center justify-center rounded-full shadow-lg" style="background: hsl(var(--primary))">
        {#if isActive && $isPlaying}
          <Pause class="size-4 fill-white text-white" />
        {:else}
          <Play class="size-4 translate-x-px fill-white text-white" />
        {/if}
      </div>
    </div>
    <p class="truncate text-sm font-semibold" class:text-primary={isActive}>{song.title}</p>
    <p class="truncate text-xs text-white/40">{song.artist}</p>
  </button>
</SongContextMenu>

<style>
  .song-card {
    background: var(--color-card);
    border: 1px solid var(--color-border);
    transition: background 150ms ease, transform 150ms ease;
  }
  .song-card:hover {
    background: color-mix(in srgb, var(--color-muted) 7%, transparent);
    transform: translateY(-2px);
  }
  .song-card-active {
    border-color: color-mix(in srgb, var(--color-primary) 30%, var(--color-border));
    background: color-mix(in srgb, var(--color-primary) 8%, transparent);
  }

  .play-fab {
    opacity: 0;
    transform: translateY(4px);
    transition: opacity 180ms ease, transform 180ms ease;
  }
  .song-card:hover .play-fab,
  .song-card-active .play-fab {
    opacity: 1;
    transform: translateY(0);
  }
</style>
