<script lang="ts">
  import { goto } from '$app/navigation';
  import { Play, Pause, Loader } from '@lucide/svelte';
  import { searchSongs } from '$lib/servers';
  import { focusTrack, playQueue, playingFrom, isPlaying, togglePlayRequest } from '$lib/stores/player';
  import { activeServerType } from '$lib/stores/ui-state';
  import { toast } from 'svelte-sonner';

  let {
    name,
    image = ''
  }: {
    name: string;
    image?: string;
  } = $props();

  let loading = $state(false);

  const artistHref = $derived(`/artist/${encodeURIComponent(name)}`);
  const isActive = $derived($playingFrom.href === artistHref);

  function initials(n: string) {
    return n.split(' ').filter(Boolean).slice(0, 2).map((p) => p[0]?.toUpperCase() ?? '').join('');
  }

  async function play(event: MouseEvent) {
    event.stopPropagation();
    if (isActive) {
      togglePlayRequest.update((n) => n + 1);
      return;
    }
    loading = true;
    try {
      const songs = await searchSongs(name, 50);
      const isOctoFiesta = $activeServerType === 'octo_fiesta';
      const playable = isOctoFiesta ? songs : songs.filter((s) => !s.id.startsWith('ext-'));
      const artistSongs = playable.filter((s) => s.artist?.toLowerCase().includes(name.toLowerCase()));
      const list = artistSongs.length ? artistSongs : playable;
      if (!list.length) { toast.warning('No locally available tracks for this artist'); return; }
      focusTrack.set({ title: list[0].title, artist: list[0].artist, imageUrl: list[0].coverArtUrl, source: 'library', album: list[0].album });
      playingFrom.set({ type: 'artist', name, href: artistHref });
      playQueue(list, 0);
    } finally {
      loading = false;
    }
  }
</script>

<div class="artist-card group relative flex w-full flex-col items-center gap-2 rounded-xl p-3">
  <div class="relative">
    <button class="block" onclick={() => goto(artistHref)} aria-label="Go to {name}">
      {#if image}
        <img class="h-16 w-16 rounded-full object-cover shadow-md ring-2 ring-white/5" src={image} alt={name} loading="lazy" />
      {:else}
        <div class="flex h-16 w-16 items-center justify-center rounded-full bg-card text-base font-bold text-muted-foreground ring-2 ring-border/10">
          {initials(name)}
        </div>
      {/if}
    </button>
    <!-- Play overlay -->
    <button
      class="absolute inset-0 flex items-center justify-center rounded-full bg-black/50 transition-opacity duration-150 {isActive ? 'opacity-100' : 'opacity-0 group-hover:opacity-100'}"
      onclick={play}
      aria-label={isActive && $isPlaying ? `Pause ${name}` : `Play ${name}`}
    >
      {#if loading}
        <Loader class="size-5 animate-spin text-white" />
      {:else if isActive && $isPlaying}
        <Pause class="size-5 text-white" fill="currentColor" />
      {:else}
        <Play class="size-5 translate-x-0.5 text-white" fill="currentColor" />
      {/if}
    </button>
  </div>
  <button class="w-full" onclick={() => goto(artistHref)}>
    <p class="w-full truncate text-center text-xs font-medium" class:text-primary={isActive}>{name}</p>
  </button>
</div>

<style>
  .artist-card {
    transition: background 150ms ease;
    cursor: default;
  }
  .artist-card:hover {
    background: color-mix(in srgb, var(--color-muted) 6%, transparent);
  }
</style>
