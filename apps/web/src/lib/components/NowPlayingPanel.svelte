<script lang="ts">
  import { ChevronDown, ChevronUp, PanelRight } from '@lucide/svelte';
  import { flip } from 'svelte/animate';
  import { cubicOut } from 'svelte/easing';
  import { fade, fly } from 'svelte/transition';
  import * as Dialog from '$lib/components/ui/dialog';
  import { Button, ScrollArea } from '$lib/components/ui';
  import {
    focusTrack,
    playingFrom,
    queue,
    currentIndex,
    shouldAutoplay,
    showQueue,
    playQueue,
    smartShuffleTrackIds,
  } from '$lib/stores/player';
  import {
    fetchSimilarSongs,
    fetchAlbumDetail,
    type Song,
  } from '$lib/servers';
  import { getArtistInfo, type DiscoveryArtistInfo as ArtistInfo } from '$lib/discovery';
  import { primarySongArtist } from '$lib/song-artists';
  import SongArtistLinks from '$lib/components/SongArtistLinks.svelte';
  import { formatClockDuration } from '$lib/utils';

  let { open }: { open: boolean } = $props();

  const currentSong = $derived($queue[$currentIndex] ?? null);
  const displaySong = $derived(currentSong ?? ($focusTrack
    ? {
        id: '',
        title: $focusTrack.title,
        artist: $focusTrack.artist,
        album: $focusTrack.album ?? '',
        albumId: '',
        coverArtUrl: $focusTrack.imageUrl,
        duration: 0,
        streamUrl: '',
      } as Song
    : null));

  let relatedSongs = $state<Song[]>([])
  let relatedLoading = $state(false);
  let panelArtistInfo = $state<ArtistInfo | null>(null);
  let panelArtistLoading = $state(false);
  let aboutDialogOpen = $state(false);
  let upNextExpanded = $state(false);
  let relatedExpanded = $state(false);
  let creditsExpanded = $state(false);
  let creditsAlbum = $state<{ year?: number; genre?: string } | null>(null);

  let _panelSongId = '';
  let _panelArtist = '';

  $effect(() => {
    const song = currentSong;
    if (!song) return;

    if (song.id !== _panelSongId) {
      _panelSongId = song.id;
      relatedSongs = [];
      relatedExpanded = false;
      creditsExpanded = false;
      relatedLoading = true;
      fetchSimilarSongs(song.id, 6)
        .then((s) => { relatedSongs = s; })
        .catch(() => {})
        .finally(() => { relatedLoading = false; });

      creditsAlbum = null;
      if (song.albumId) {
        fetchAlbumDetail(song.albumId)
          .then((d) => { creditsAlbum = { year: d.album?.year, genre: d.album?.genre }; })
          .catch(() => {});
      }
    }

    if (song.artist && song.artist !== _panelArtist) {
      _panelArtist = primarySongArtist(song.artist);
      panelArtistInfo = null;
      panelArtistLoading = true;
      aboutDialogOpen = false;
      getArtistInfo(primarySongArtist(song.artist))
        .then((info) => { panelArtistInfo = info; })
        .catch(() => {})
        .finally(() => { panelArtistLoading = false; });
    }
  });

  const upNext = $derived.by(() => {
    const items = $queue;
    if (!items.length || $currentIndex >= items.length - 1) return [];
    return items.slice($currentIndex + 1).map((song, i) => ({ index: $currentIndex + 1 + i, song }));
  });

  function playFromUpNext(index: number) {
    const song = $queue[index];
    if (!song) return;
    currentIndex.set(index);
    shouldAutoplay.set(true);
    focusTrack.set({ title: song.title, artist: song.artist, imageUrl: song.coverArtUrl, source: 'library', album: song.album });
  }

  function initials(name: string): string {
    return name.split(' ').filter(Boolean).slice(0, 2).map((p) => p[0]?.toUpperCase() ?? '').join('');
  }
</script>

<aside class="app-shell-rail flex h-full w-full shrink-0 flex-col overflow-hidden border-l border-border/40">
  <div class="flex shrink-0 items-center justify-between border-b border-border/40 px-4 py-4">
    {#if $playingFrom.type}
      <div class="flex min-w-0 flex-1 flex-col">
        <span class="text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">
          {$playingFrom.type === 'favorites' ? 'Playing from' : $playingFrom.type === 'playlist' ? 'Playing from playlist' : $playingFrom.type === 'artist' ? 'Playing from artist' : 'Playing from album'}
        </span>
        <a href={$playingFrom.href} class="truncate text-sm font-semibold hover:underline leading-snug">{$playingFrom.name}</a>
      </div>
    {:else}
      <span class="text-base font-semibold">Now Playing</span>
    {/if}
    <Button
      variant="ghost"
      size="icon"
      class="size-7 shrink-0 ml-2"
      onclick={() => { showQueue.update((v) => !v); }}
      title="Close"
    >
      <PanelRight class="size-4" />
      <span class="sr-only">Close Now Playing</span>
    </Button>
  </div>

  <ScrollArea class="h-full flex-1 px-4 pt-4 pb-24">
    {#if displaySong}
      <div class="space-y-4">
        <div class="group/art relative aspect-square w-full">
          {#if displaySong.coverArtUrl}
            <img class="aspect-square w-full rounded-lg object-cover shadow-lg" src={displaySong.coverArtUrl} alt={displaySong.title} />
          {:else}
            <div class="grid aspect-square w-full place-items-center rounded-lg bg-gradient-to-br from-slate-500 to-slate-700 text-2xl font-bold">
              {initials(displaySong.title)}
            </div>
          {/if}
          {#if $playingFrom.type}
            <a
              href={$playingFrom.href}
              class="absolute inset-0 flex flex-col justify-end rounded-lg bg-gradient-to-t from-black/80 via-black/20 to-transparent p-3 opacity-0 transition-opacity duration-200 group-hover/art:opacity-100"
            >
              <span class="text-[10px] font-semibold uppercase tracking-wider text-white/70">
                {$playingFrom.type === 'favorites' ? 'Playing from' : $playingFrom.type === 'playlist' ? 'Playing from playlist' : $playingFrom.type === 'artist' ? 'Playing from artist' : 'Playing from album'}
              </span>
              <span class="truncate text-sm font-bold text-white">{$playingFrom.name}</span>
            </a>
          {/if}
        </div>
        <div class="space-y-0.5">
          <p class="text-base font-semibold leading-snug">{displaySong.title}</p>
          <SongArtistLinks
            artist={displaySong.artist}
            class="text-sm text-muted-foreground"
            linkClass="hover:text-foreground hover:underline transition-colors"
          />
        </div>
      </div>
    {:else}
      <p class="text-sm text-muted-foreground">Pick a song to start listening.</p>
    {/if}

    <!-- Related Songs -->
    {#if relatedLoading || relatedSongs.length > 0}
      <div class="mt-6 border-t border-border/70 pt-5">
        <div class="mb-3 flex items-center justify-between">
          <h3 class="text-xs font-semibold uppercase tracking-wider text-muted-foreground">Related Songs</h3>
          {#if !relatedLoading && relatedSongs.length > 1}
            <button
              class="flex items-center gap-1 text-xs text-muted-foreground hover:text-foreground transition-colors"
              onclick={() => { relatedExpanded = !relatedExpanded; }}
            >
              {relatedExpanded ? 'Show less' : `Show all ${relatedSongs.length}`}
              {#if relatedExpanded}<ChevronUp class="size-3" />{:else}<ChevronDown class="size-3" />{/if}
            </button>
          {/if}
        </div>
        {#if relatedLoading}
          <div class="flex items-center gap-3 px-2 py-1.5">
            <div class="size-9 shrink-0 rounded bg-secondary animate-pulse"></div>
            <div class="flex-1 space-y-1.5">
              <div class="h-3 w-3/4 rounded bg-secondary animate-pulse"></div>
              <div class="h-2.5 w-1/2 rounded bg-secondary animate-pulse"></div>
            </div>
          </div>
        {:else}
          <div class="space-y-1">
            {#each (relatedExpanded ? relatedSongs : relatedSongs.slice(0, 1)) as song (song.id)}
              <button
                class="flex w-full items-center gap-3 rounded-md px-2 py-1.5 text-left transition hover:bg-accent"
                onclick={() => {
                  playQueue([song], 0);
                  focusTrack.set({ title: song.title, artist: song.artist, imageUrl: song.coverArtUrl, source: 'library', album: song.album });
                  playingFrom.set({ type: 'artist', name: primarySongArtist(song.artist), href: `/artist/${encodeURIComponent(primarySongArtist(song.artist))}` });
                  shouldAutoplay.set(true);
                }}
              >
                {#if song.coverArtUrl}
                  <img class="size-9 rounded object-cover shrink-0" src={song.coverArtUrl} alt={song.title} loading="lazy" />
                {:else}
                  <span class="grid size-9 shrink-0 place-items-center rounded bg-secondary text-[10px] font-semibold">{initials(song.title)}</span>
                {/if}
                <span class="min-w-0 flex-1">
                  <span class="block truncate text-sm font-medium">{song.title}</span>
                  <SongArtistLinks
                    artist={song.artist}
                    class="block truncate text-xs text-muted-foreground"
                    linkClass="hover:text-foreground hover:underline transition-colors"
                  />
                </span>
              </button>
            {/each}
          </div>
        {/if}
      </div>
    {/if}

    <!-- About Artist -->
    {#if displaySong}
      <div class="mt-6 border-t border-border/70 pt-5">
        <h3 class="mb-3 text-xs font-semibold uppercase tracking-wider text-muted-foreground">About the Artist</h3>
        {#if panelArtistLoading}
          <div class="space-y-2">
            <div class="h-20 w-full rounded-lg bg-secondary animate-pulse"></div>
            <div class="h-3 w-2/3 rounded bg-secondary animate-pulse"></div>
            <div class="h-3 w-full rounded bg-secondary animate-pulse"></div>
          </div>
        {:else if panelArtistInfo}
          <button class="group/about w-full text-left" onclick={() => { aboutDialogOpen = true; }}>
            {#if panelArtistInfo.imageUrl}
            <div class="relative mb-3 h-28 w-full overflow-hidden rounded-2xl">
                <img src={panelArtistInfo.imageUrl} alt={panelArtistInfo.name} class="h-full w-full object-cover object-top shadow transition-transform duration-300 group-hover/about:scale-105" />
                <div class="absolute inset-0 rounded-lg bg-black/0 transition-colors group-hover/about:bg-black/20"></div>
              </div>
            {/if}
            <p class="mb-0.5 text-sm font-semibold group-hover/about:underline">{panelArtistInfo.name}</p>
            {#if panelArtistInfo.listeners}
              <p class="mb-2 text-xs text-muted-foreground">
                {panelArtistInfo.listeners >= 1_000_000
                  ? `${(panelArtistInfo.listeners / 1_000_000).toFixed(1)}M monthly listeners`
                  : `${(panelArtistInfo.listeners / 1_000).toFixed(0)}K monthly listeners`}
              </p>
            {/if}
            {#if panelArtistInfo.bio}
              {@const bioText = panelArtistInfo.bio.replace(/<[^>]*>/g, '').trim()}
              <p class="line-clamp-3 text-xs leading-relaxed text-muted-foreground">{bioText}</p>
              <p class="mt-1 text-xs font-medium text-foreground/60 group-hover/about:text-foreground transition-colors">Read more…</p>
            {/if}
          </button>
        {:else}
          <p class="text-xs text-muted-foreground">No artist info available.</p>
        {/if}
      </div>
    {/if}

    <!-- About Artist Dialog -->
    {#if panelArtistInfo && displaySong}
      <Dialog.Root bind:open={aboutDialogOpen}>
        <Dialog.Portal>
          <Dialog.Overlay class="fixed inset-0 z-50 bg-black/60 backdrop-blur-sm" />
          <Dialog.Content class="app-glass fixed left-1/2 top-1/2 z-50 w-full max-w-lg -translate-x-1/2 -translate-y-1/2 rounded-[28px] outline-none overflow-hidden">
            {#if panelArtistInfo.imageUrl}
              <div class="relative h-56 w-full">
                <img src={panelArtistInfo.imageUrl} alt={panelArtistInfo.name} class="h-full w-full object-cover object-top" />
                <div class="absolute inset-0 bg-gradient-to-t from-background via-background/30 to-transparent"></div>
                <div class="absolute bottom-0 left-0 p-6">
                  <Dialog.Title class="text-2xl font-bold">{panelArtistInfo.name}</Dialog.Title>
                  {#if panelArtistInfo.listeners}
                    <p class="text-sm text-muted-foreground">
                      {panelArtistInfo.listeners >= 1_000_000
                        ? `${(panelArtistInfo.listeners / 1_000_000).toFixed(1)}M monthly listeners`
                        : `${(panelArtistInfo.listeners / 1_000).toFixed(0)}K monthly listeners`}
                    </p>
                  {/if}
                </div>
                <Dialog.Close class="absolute right-4 top-4 grid size-8 place-items-center rounded-full bg-black/50 text-white hover:bg-black/70 transition-colors">
                  <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
                </Dialog.Close>
              </div>
            {:else}
              <div class="flex items-start justify-between p-6 pb-2">
                <div>
                  <Dialog.Title class="text-xl font-bold">{panelArtistInfo.name}</Dialog.Title>
                  {#if panelArtistInfo.listeners}
                    <p class="text-sm text-muted-foreground">
                      {panelArtistInfo.listeners >= 1_000_000
                        ? `${(panelArtistInfo.listeners / 1_000_000).toFixed(1)}M monthly listeners`
                        : `${(panelArtistInfo.listeners / 1_000).toFixed(0)}K monthly listeners`}
                    </p>
                  {/if}
                </div>
                <Dialog.Close class="grid size-8 place-items-center rounded-full hover:bg-accent transition-colors">
                  <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
                </Dialog.Close>
              </div>
            {/if}
            <div class="max-h-72 overflow-y-auto px-6 py-4">
              {#if panelArtistInfo.tags?.length}
                <div class="mb-4 flex flex-wrap gap-1.5">
                  {#each panelArtistInfo.tags as tag}
                    <span class="rounded-full border border-border px-2.5 py-0.5 text-xs text-muted-foreground">{tag}</span>
                  {/each}
                </div>
              {/if}
              {#if panelArtistInfo.bio}
                <p class="text-sm leading-relaxed text-muted-foreground">{panelArtistInfo.bio.replace(/<[^>]*>/g, '').trim()}</p>
              {/if}
            </div>
            <div class="border-t border-border/60 px-6 py-4">
              <a
                href={`/artist/${encodeURIComponent(primarySongArtist(displaySong.artist))}`}
                onclick={() => { aboutDialogOpen = false; }}
                class="text-sm font-medium hover:underline"
              >Go to artist page →</a>
            </div>
          </Dialog.Content>
        </Dialog.Portal>
      </Dialog.Root>
    {/if}

    <!-- Credits -->
    {#if currentSong}
      <div class="mt-6 border-t border-border/70 pt-5 pb-2">
        <div class="mb-3 flex items-center justify-between">
          <h3 class="text-xs font-semibold uppercase tracking-wider text-muted-foreground">Credits</h3>
          <button
            class="flex items-center gap-1 text-xs text-muted-foreground hover:text-foreground transition-colors"
            onclick={() => { creditsExpanded = !creditsExpanded; }}
          >
            {creditsExpanded ? 'Show less' : 'Show all'}
            {#if creditsExpanded}<ChevronUp class="size-3" />{:else}<ChevronDown class="size-3" />{/if}
          </button>
        </div>
        <div class="space-y-3">
          <div>
            <p class="text-[10px] uppercase tracking-wider text-muted-foreground/70 mb-0.5">Performed by</p>
            <SongArtistLinks
              artist={currentSong.artist}
              class="text-sm font-medium"
              linkClass="hover:underline"
            />
          </div>
          {#if creditsExpanded}
            <div>
              <p class="text-[10px] uppercase tracking-wider text-muted-foreground/70 mb-0.5">Album</p>
              <a href={`/album/${encodeURIComponent(currentSong.albumId)}`} class="text-sm font-medium hover:underline">{currentSong.album}</a>
            </div>
            {#if creditsAlbum?.year}
              <div>
                <p class="text-[10px] uppercase tracking-wider text-muted-foreground/70 mb-0.5">Year</p>
                <p class="text-sm font-medium">{creditsAlbum.year}</p>
              </div>
            {/if}
            {#if creditsAlbum?.genre}
              <div>
                <p class="text-[10px] uppercase tracking-wider text-muted-foreground/70 mb-0.5">Genre</p>
                <p class="text-sm font-medium">{creditsAlbum.genre}</p>
              </div>
            {/if}
            {#if currentSong.duration}
              <div>
                <p class="text-[10px] uppercase tracking-wider text-muted-foreground/70 mb-0.5">Duration</p>
                <p class="text-sm font-medium">{formatClockDuration(currentSong.duration)}</p>
              </div>
            {/if}
          {/if}
        </div>
      </div>
    {/if}

    <!-- Up Next -->
    {#if currentSong}
      <div class="mt-6 border-t border-border/70 pt-5 pb-2">
        <div class="mb-3 flex items-center justify-between">
          <h3 class="text-xs font-semibold uppercase tracking-wider text-muted-foreground">Up Next</h3>
          {#if upNext.length > 1}
            <button
              class="flex items-center gap-1 text-xs text-muted-foreground hover:text-foreground transition-colors"
              onclick={() => { upNextExpanded = !upNextExpanded; }}
            >
              {upNextExpanded ? 'Show less' : `Show all ${upNext.length}`}
              {#if upNextExpanded}<ChevronUp class="size-3" />{:else}<ChevronDown class="size-3" />{/if}
            </button>
          {/if}
        </div>
        {#if upNext.length === 0}
          <p
            class="text-xs text-muted-foreground/60 px-2"
            in:fade={{ duration: 180 }}
            out:fade={{ duration: 140 }}
          >Nothing queued</p>
        {:else}
          <div class="space-y-1">
            {#each (upNextExpanded ? upNext : upNext.slice(0, 1)) as item (item.song.id + '-' + item.index)}
              <button
                animate:flip={{ duration: 220, easing: cubicOut }}
                class="flex w-full items-center gap-3 rounded-md px-2 py-2 text-left transition hover:bg-accent"
                in:fly={{ y: 12, opacity: 0.15, duration: 220, easing: cubicOut }}
                out:fade={{ duration: 150 }}
                onclick={() => playFromUpNext(item.index)}
              >
                {#if item.song.coverArtUrl}
                  <img class="size-9 rounded object-cover shrink-0" src={item.song.coverArtUrl} alt={item.song.title} loading="lazy" />
                {:else}
                  <span class="grid size-9 shrink-0 place-items-center rounded bg-secondary text-[10px] font-semibold">{initials(item.song.title)}</span>
                {/if}
                <span class="min-w-0">
                  <span class="block truncate text-sm font-medium">{item.song.title}</span>
                  <SongArtistLinks
                    artist={item.song.artist}
                    class="block truncate text-xs text-muted-foreground"
                    linkClass="hover:text-foreground hover:underline transition-colors"
                  />
                  {#if $smartShuffleTrackIds.has(item.song.id)}
                    <span class="mt-1 inline-flex rounded-full border border-primary/30 bg-primary/10 px-1.5 py-0.5 text-[10px] font-semibold uppercase tracking-wide text-primary">
                      From Smart Shuffle
                    </span>
                  {/if}
                </span>
              </button>
            {/each}
          </div>
        {/if}
      </div>
    {/if}
  </ScrollArea>
</aside>
