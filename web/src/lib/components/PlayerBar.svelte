<script lang="ts">
  import { untrack } from 'svelte';
  import { Pause, Play, Repeat, Repeat1, SkipBack, SkipForward, MicVocal, Heart, Sparkles, ChevronDown, Shuffle, ListMusic } from '@lucide/svelte';
  import { fly } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import { IsMobile } from '$lib/hooks/is-mobile.svelte';
  import {
    currentIndex, currentTime, duration, isPlaying,
    nextTrack, prevTrack, queue, repeatMode, shouldAutoplay,
    cycleRepeatMode, showLyrics, seekRequest, togglePlayRequest,
    starredSongIds, smartShuffleTrackIds, showQueue, queueLoading,
    shuffleEnabled, startQueue, playingFrom,
  } from '$lib/stores/player';
  import { starSong, unstarSong, desktopPlaybackPause, type CastDeviceInfo } from '$lib/servers';
  import { normalizeEqBands } from '$lib/audio/equalizer';
  import { formatClockDuration, initials } from '$lib/utils';
  import { toast } from 'svelte-sonner';
  import SongArtistLinks from '$lib/components/SongArtistLinks.svelte';
  import { Button, Slider } from '$lib/components/ui';
  import PlayerVolume from '$lib/components/player/PlayerVolume.svelte';
  import PlayerCastMenu from '$lib/components/player/PlayerCastMenu.svelte';
  import PlayerShuffleMenu from '$lib/components/player/PlayerShuffleMenu.svelte';
  import { createPlayerCastController } from '$lib/components/player/player-cast-controller.svelte';
  import { createPlayerShuffleController } from '$lib/components/player/player-shuffle-controller.svelte';
  import { createPlayerScrobbleController } from '$lib/components/player/player-scrobble-controller.svelte';
  import { createPlayerUpNextController } from '$lib/components/player/player-upnext-controller.svelte';
  import { createPlayerEqController } from '$lib/components/player/player-eq-controller.svelte';
  import { createPlayerDesktopController } from '$lib/components/player/player-desktop-controller.svelte';
  import { backendSettings } from '$lib/stores/backend-settings';
  import SongTechBadge from '$lib/components/SongTechBadge.svelte';
  import { Tooltip, TooltipTrigger, TooltipContent } from '$lib/components/ui/tooltip';
  import { goto } from '$app/navigation';
  import { getExternalSourceLabel } from '$lib/external-source';
  import SongContextMenu from '$lib/components/SongContextMenu.svelte';

  // ── Mobile state ──────────────────────────────────────────────────────────
  const isMobile = new IsMobile();
  let fullPlayerOpen = $state(false);
  let mobileQueueOpen = $state(false);

  // Swipe handling for mini player (prev/next)
  let miniSwipeTouchStartX = 0;
  let miniSwipeTouchStartY = 0;
  function onMiniTouchStart(e: TouchEvent) {
    miniSwipeTouchStartX = e.touches[0].clientX;
    miniSwipeTouchStartY = e.touches[0].clientY;
  }
  function onMiniTouchEnd(e: TouchEvent) {
    const dx = e.changedTouches[0].clientX - miniSwipeTouchStartX;
    const dy = e.changedTouches[0].clientY - miniSwipeTouchStartY;
    if (Math.abs(dy) > Math.abs(dx) || Math.abs(dx) < 50) return;
    if (dx < 0) nextTrack(); else prevTrack();
  }

  // Swipe-down to close full player
  let fullPlayerTouchStartY = 0;
  let fullPlayerDragY = $state(0);
  let fullPlayerDragging = $state(false);
  function onFullPlayerTouchStart(e: TouchEvent) {
    // Only drag from the artwork/info area, not interactive controls
    fullPlayerTouchStartY = e.touches[0].clientY;
    fullPlayerDragging = true;
    fullPlayerDragY = 0;
  }
  function onFullPlayerTouchMove(e: TouchEvent) {
    if (!fullPlayerDragging) return;
    const dy = e.touches[0].clientY - fullPlayerTouchStartY;
    if (dy > 0) fullPlayerDragY = dy;
  }
  function onFullPlayerTouchEnd() {
    if (fullPlayerDragY > 100) {
      fullPlayerOpen = false;
    }
    fullPlayerDragging = false;
    fullPlayerDragY = 0;
  }

  // ── UI state ──────────────────────────────────────────────────────────────
  let castActive = $state(false);
  let castPlaying = $state(false);
  let castDevice = $state<CastDeviceInfo | null>(null);
  let castVolume = $state<number | null>(null);
  let seekVal = $state<number[]>([0]);
  let seekDragging = $state(false);
  let isBuffering = $state(false);

  // ── Settings derivations ──────────────────────────────────────────────────
  const lastFmApiKey = $derived($backendSettings.lastFmApiKey);
  const lastFmConnected = $derived($backendSettings.lastFmConnected);
  const eqEnabled = $derived($backendSettings.eqEnabled ?? false);
  const eqBands = $derived(normalizeEqBands($backendSettings.eqBands));

  // ── Track derivations ─────────────────────────────────────────────────────
  const currentTrack = $derived($queue[$currentIndex] ?? null);
  const isStarred = $derived(currentTrack ? $starredSongIds.has(currentTrack.id) : false);
  const isSmartShuffleTrack = $derived(currentTrack ? $smartShuffleTrackIds.has(currentTrack.id) : false);

  // ── UI class derivations ──────────────────────────────────────────────────
  const favoriteButtonClass = $derived(
    `player-favorite-button shrink-0 inline-flex items-center justify-center rounded-full border p-1 transition-colors ${isStarred ? 'border-rose-500/30 bg-rose-500/10 text-rose-500' : 'border-border/50 bg-transparent text-muted-foreground/60 hover:text-rose-400'}`
  );
  const repeatButtonClass = $derived(
    `player-transport-button ${$repeatMode !== 'off' ? 'text-primary' : 'text-muted-foreground hover:text-foreground'}`
  );
  const lyricsButtonClass = $derived(
    `player-transport-button ${$showLyrics ? 'text-primary is-active' : 'text-muted-foreground hover:text-foreground'}`
  );
  const repeatTooltip = $derived(
    $repeatMode === 'one' ? 'Repeat: One' : $repeatMode === 'all' ? 'Repeat: All' : 'Repeat: Off'
  );
  const externalSourceLabel = $derived(
    (isBuffering || $queueLoading) && currentTrack ? getExternalSourceLabel(currentTrack.id) : null
  );

  // ── Controllers ───────────────────────────────────────────────────────────
  createPlayerEqController({
    getCastActive: () => castActive,
    getEqEnabled: () => eqEnabled,
    getEqBands: () => eqBands
  });

  const desktopController = createPlayerDesktopController({
    getCurrentTrack: () => currentTrack,
    getCastActive: () => castActive,
    getSeekDragging: () => seekDragging,
    getIsBuffering: () => isBuffering,
    getCrossfadeSeconds: () => $backendSettings.crossfadeSeconds ?? 4,
    setIsBuffering: (v) => { isBuffering = v; },
    onRestoreCastSession: () => castController.restoreSession()
  });

  const castController = createPlayerCastController({
    getCurrentTrack: () => currentTrack,
    getSeekDragging: () => seekDragging,
    setCurrentTime: (value) => currentTime.set(value),
    setDuration: (value) => duration.set(value),
    setCastVolume: (value) => { castVolume = value; },
    setCastPlaying: (value) => { castPlaying = value; },
    setCastActive: (value) => { castActive = value; },
    setCastDevice: (value) => { castDevice = value; },
    onPauseLocalPlayback: async () => {
      if ($isPlaying) await desktopPlaybackPause().catch(() => undefined);
    },
    onAdvanceTrack: () => { nextTrack(); }
  });

  const shuffleController = createPlayerShuffleController({
    getCurrentTrack: () => currentTrack,
  });

  createPlayerScrobbleController({
    getCurrentTrack: () => currentTrack,
    getLastFmConnected: () => lastFmConnected,
  });

  createPlayerUpNextController({ getLastFmApiKey: () => lastFmApiKey });

  // ── Transport UI derivations ──────────────────────────────────────────────
  const transportLocked = $derived(desktopController.loadPending || isBuffering || $queueLoading);
  const showPauseButton = $derived(castActive ? castPlaying : ($isPlaying && !isBuffering && !desktopController.loadPending && !$queueLoading));

  // ── Global event listener ─────────────────────────────────────────────────
  $effect(() => {
    function handleTogglePlay() { togglePlay(); }
    window.addEventListener('player:toggle-play', handleTogglePlay);
    return () => window.removeEventListener('player:toggle-play', handleTogglePlay);
  });

  // ── Seek bar sync ─────────────────────────────────────────────────────────
  $effect(() => {
    if (seekDragging) return;
    const t = $currentTime;
    seekVal = [Number.isFinite(t) && t >= 0 ? t : 0];
  });

  // ── Autoplay dispatcher ───────────────────────────────────────────────────
  $effect(() => {
    if (!$shouldAutoplay) return;
    if (castActive && castDevice) {
      shouldAutoplay.set(false);
      if (currentTrack) castController.playTrackOnCast(currentTrack);
      return;
    }
    desktopController.handleAutoplay();
  });

  // ── Seek request dispatcher ───────────────────────────────────────────────
  $effect(() => {
    const t = $seekRequest;
    if (t === null) return;
    desktopController.seek(t);
    currentTime.set(t);
    seekRequest.set(null);
  });

  // ── Toggle play request ───────────────────────────────────────────────────
  $effect(() => {
    if ($togglePlayRequest === 0) return;
    untrack(() => togglePlay());
  });

  // ── Functions ─────────────────────────────────────────────────────────────
  function togglePlay() {
    if (transportLocked) return;
    if (castActive) { castController.togglePlayPause(); return; }
    if (!currentTrack) return;
    desktopController.togglePlay();
  }

  function seek(values: number[]) {
    const value = Math.max(0, Number(values[0] ?? 0));
    currentTime.set(value);
    seekVal = [value];
    seekDragging = false;
    if (castActive) castController.seek(value);
    else desktopController.seek(value);
  }

  async function toggleFavorite() {
    if (!currentTrack) return;
    const id = currentTrack.id;
    if (isStarred) {
      starredSongIds.update((ids) => { const s = new Set(ids); s.delete(id); return s; });
      try {
        await unstarSong(id, currentTrack.artist, currentTrack.title, currentTrack.album);
      } catch {
        starredSongIds.update((ids) => new Set([...ids, id]));
        toast.error('Failed to remove from favorites');
      }
    } else {
      starredSongIds.update((ids) => new Set([...ids, id]));
      try {
        await starSong(id, currentTrack.artist, currentTrack.title, currentTrack.album);
      } catch {
        starredSongIds.update((ids) => { const s = new Set(ids); s.delete(id); return s; });
        toast.error('Failed to add to favorites');
      }
    }
  }

</script>

{#if isMobile.current}
<!-- ─── MOBILE: Mini player + Full screen player ──────────────────────── -->

{#if currentTrack}
  <!-- Mini player bar — fixed above bottom nav -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="fixed left-2 right-2 z-30 flex items-center gap-3 rounded-2xl border border-white/[0.08] bg-card/[0.97] px-3 py-2.5 shadow-2xl backdrop-blur-xl"
    style="bottom: calc(var(--sab) + 3.75rem); -webkit-backdrop-filter:blur(24px)"
    ontouchstart={onMiniTouchStart}
    ontouchend={onMiniTouchEnd}
  >
    <button
      class="flex min-w-0 flex-1 items-center gap-3 text-left"
      onclick={() => { fullPlayerOpen = true; }}
      aria-label="Open player"
    >
      {#if currentTrack.coverArtUrl}
        <img class="size-11 shrink-0 rounded-xl object-cover shadow-sm" src={currentTrack.coverArtUrl} alt={currentTrack.title} />
      {:else}
        <div class="grid size-11 shrink-0 place-items-center rounded-xl bg-gradient-to-br from-muted to-muted/60 text-xs font-bold text-muted-foreground">
          {initials(currentTrack.title)}
        </div>
      {/if}
      <div class="min-w-0">
        <p class="truncate text-sm font-semibold leading-tight">{currentTrack.title}</p>
        <p class="truncate text-xs text-muted-foreground">{currentTrack.artist}</p>
      </div>
    </button>
    <button
      onclick={togglePlay}
      disabled={!currentTrack || transportLocked}
      class="flex size-9 items-center justify-center rounded-full transition-colors active:bg-white/10 disabled:opacity-40"
      aria-label={showPauseButton ? 'Pause' : 'Play'}
    >
      {#if showPauseButton}
        <Pause class="size-5" fill="currentColor" />
      {:else}
        <Play class="size-5 translate-x-px" fill="currentColor" />
      {/if}
    </button>
    <button
      onclick={nextTrack}
      class="flex size-9 items-center justify-center rounded-full text-muted-foreground transition-colors active:bg-white/10"
      aria-label="Next track"
    >
      <SkipForward class="size-5" />
    </button>
  </div>
{/if}

<!-- Full screen player overlay -->
{#if fullPlayerOpen}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="fixed inset-0 z-50 flex flex-col"
    style="transform: translateY({fullPlayerDragY}px); transition: {fullPlayerDragging ? 'none' : 'transform 0.3s ease'};"
    transition:fly={{ y: 900, duration: 340, easing: cubicOut }}
    ontouchstart={onFullPlayerTouchStart}
    ontouchmove={onFullPlayerTouchMove}
    ontouchend={onFullPlayerTouchEnd}
  >
    <div class="absolute inset-0 bg-[#0b0b10]">
      {#if currentTrack?.coverArtUrl}
        <img src={currentTrack.coverArtUrl} class="absolute inset-0 h-full w-full scale-125 object-cover opacity-40 blur-3xl" aria-hidden="true" alt="" />
        <div class="absolute inset-0 bg-gradient-to-b from-black/50 via-black/30 to-black/80"></div>
      {/if}
    </div>

    <div class="relative flex h-full flex-col px-6" style="padding-top: max(var(--sat), 0.75rem); padding-bottom: max(var(--sab), 0.75rem)">

      <!-- Top: collapse -->
      <div class="flex items-center justify-between py-4">
        <button onclick={() => { fullPlayerOpen = false; }} class="flex size-10 items-center justify-center rounded-full text-white/70 active:bg-white/10" aria-label="Collapse player">
          <ChevronDown class="size-6" />
        </button>
        <p class="text-xs font-semibold uppercase tracking-widest text-white/50">Now Playing</p>
        <div class="size-10"></div>
      </div>

      <!-- Album art -->
      <div class="flex flex-1 items-center justify-center py-2">
        <div class="w-full max-w-[72vw]">
          {#if currentTrack?.coverArtUrl}
            <img src={currentTrack.coverArtUrl} alt={currentTrack?.title} class="aspect-square w-full rounded-[28px] object-cover shadow-[0_32px_80px_rgba(0,0,0,0.7)]" />
          {:else if currentTrack}
            <div class="flex aspect-square w-full items-center justify-center rounded-[28px] bg-white/8 text-5xl font-bold text-white/40">{initials(currentTrack.title)}</div>
          {/if}
        </div>
      </div>

      <!-- Track info + heart -->
      <div class="flex items-start justify-between gap-4 pb-2 pt-4">
        <div class="min-w-0">
          <p class="truncate text-2xl font-bold text-white">{currentTrack?.title ?? ''}</p>
          <p class="truncate text-base text-white/55">{currentTrack?.artist ?? ''}</p>
        </div>
        <button onclick={toggleFavorite} class="mt-1 flex size-10 shrink-0 items-center justify-center rounded-full active:bg-white/10 {isStarred ? 'text-rose-500' : 'text-white/45'}" aria-label={isStarred ? 'Remove from favorites' : 'Add to favorites'}>
          <Heart class="size-[22px] {isStarred ? 'fill-rose-500' : ''}" />
        </button>
      </div>

      <!-- Progress + times -->
      <div class="py-3">
        {#if isBuffering || $queueLoading}
          <div class="mb-2 h-1 w-full overflow-hidden rounded-full bg-white/15">
            <div class="h-full w-1/3 rounded-full bg-white/55" style="animation: player-buffer-slide 1.35s cubic-bezier(0.45,0,0.2,1) infinite"></div>
          </div>
        {:else}
          <Slider class="player-seek-slider" type="multiple" bind:value={seekVal} disabled={!currentTrack} min={0} max={isFinite($duration) && $duration > 0 ? $duration : 1} step={1} onpointerdown={() => { seekDragging = true; }} onValueChange={() => {}} onValueCommit={(v) => { seek(v); }} aria-label="Playback position" />
        {/if}
        <div class="mt-1 flex justify-between text-xs tabular-nums text-white/35">
          <span>{formatClockDuration($currentTime)}</span>
          <span>{formatClockDuration($duration)}</span>
        </div>
      </div>

      <!-- Main controls -->
      <div class="flex items-center justify-between py-2">
        <button onclick={() => $shuffleEnabled ? shuffleController.deactivateShuffle() : shuffleController.activateShuffle()} class="flex size-12 items-center justify-center rounded-full active:bg-white/10 {$shuffleEnabled ? 'text-primary' : 'text-white/40'}" aria-label="Toggle shuffle">
          <Shuffle class="size-6" />
        </button>
        <button onclick={prevTrack} class="flex size-14 items-center justify-center rounded-full text-white/90 active:bg-white/10" aria-label="Previous">
          <SkipBack class="size-7" />
        </button>
        <button onclick={togglePlay} disabled={!currentTrack || transportLocked} class="flex size-[68px] items-center justify-center rounded-full bg-white text-black shadow-xl active:scale-95 disabled:opacity-50" aria-label={showPauseButton ? 'Pause' : 'Play'}>
          {#if showPauseButton}<Pause class="size-7" fill="currentColor" />{:else}<Play class="size-7 translate-x-0.5" fill="currentColor" />{/if}
        </button>
        <button onclick={nextTrack} class="flex size-14 items-center justify-center rounded-full text-white/90 active:bg-white/10" aria-label="Next">
          <SkipForward class="size-7" />
        </button>
        <button onclick={cycleRepeatMode} class="flex size-12 items-center justify-center rounded-full active:bg-white/10 {$repeatMode !== 'off' ? 'text-primary' : 'text-white/40'}" aria-label="Cycle repeat mode">
          {#if $repeatMode === 'one'}<Repeat1 class="size-6" />{:else}<Repeat class="size-6" />{/if}
        </button>
      </div>

      <!-- Bottom actions: lyrics, queue -->
      <div class="flex items-center justify-around pb-4 pt-3">
        <button onclick={() => { showLyrics.update(v => !v); fullPlayerOpen = false; }} class="flex flex-col items-center gap-1.5 rounded-2xl px-5 py-2 active:bg-white/10 {$showLyrics ? 'text-primary' : 'text-white/45'}">
          <MicVocal class="size-5" />
          <span class="text-[11px] font-medium">Lyrics</span>
        </button>
        <button onclick={() => { mobileQueueOpen = true; fullPlayerOpen = false; }} class="flex flex-col items-center gap-1.5 rounded-2xl px-5 py-2 text-white/45 active:bg-white/10">
          <ListMusic class="size-5" />
          <span class="text-[11px] font-medium">Queue</span>
        </button>
      </div>
    </div>
  </div>
{/if}

<!-- Mobile queue overlay -->
{#if mobileQueueOpen}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="fixed inset-0 z-50 flex flex-col"
    transition:fly={{ y: 900, duration: 300, easing: cubicOut }}
  >
    <div class="absolute inset-0 bg-[#0b0b10]/95 backdrop-blur-2xl"></div>
    <div class="relative flex h-full flex-col" style="padding-top: max(var(--sat), 0.75rem); padding-bottom: max(var(--sab), 0.75rem)">

      <!-- Header -->
      <div class="flex shrink-0 items-center justify-between px-5 py-4">
        <button
          onclick={() => { mobileQueueOpen = false; }}
          class="flex size-10 items-center justify-center rounded-full text-white/70 active:bg-white/10"
          aria-label="Close queue"
        >
          <ChevronDown class="size-6" />
        </button>
        <p class="text-xs font-semibold uppercase tracking-widest text-white/50">Queue</p>
        <div class="size-10"></div>
      </div>

      <!-- Now playing -->
      {#if currentTrack}
        <div class="shrink-0 border-b border-white/[0.07] px-5 pb-4">
          <p class="mb-2 text-[10px] font-semibold uppercase tracking-widest text-white/35">Now Playing</p>
          <div class="flex items-center gap-3 rounded-2xl bg-white/[0.06] p-3">
            {#if currentTrack.coverArtUrl}
              <img class="size-12 shrink-0 rounded-xl object-cover shadow-md" src={currentTrack.coverArtUrl} alt={currentTrack.title} />
            {:else}
              <div class="grid size-12 shrink-0 place-items-center rounded-xl bg-white/10 text-xs font-bold text-white/50">{initials(currentTrack.title)}</div>
            {/if}
            <div class="min-w-0">
              <p class="truncate text-sm font-semibold text-white">{currentTrack.title}</p>
              <p class="truncate text-xs text-white/55">{currentTrack.artist}</p>
            </div>
            <div class="ml-auto flex size-8 shrink-0 items-center justify-center">
              <svg class="size-4 fill-primary text-primary" viewBox="0 0 24 24"><rect x="2" y="6" width="4" height="12" rx="1"/><rect x="9" y="3" width="4" height="18" rx="1"/><rect x="16" y="8" width="4" height="8" rx="1"/></svg>
            </div>
          </div>
        </div>
      {/if}

      <!-- Upcoming tracks -->
      <div class="min-h-0 flex-1 overflow-y-auto px-5 py-4">
        {#if $queue.slice($currentIndex + 1).length === 0}
          <p class="py-8 text-center text-sm text-white/30">No upcoming tracks</p>
        {:else}
          <p class="mb-3 text-[10px] font-semibold uppercase tracking-widest text-white/35">Up Next</p>
          {#each $queue.slice($currentIndex + 1) as song, i (song.id + i)}
            <button
              class="flex w-full items-center gap-3 rounded-xl px-2 py-2.5 text-left active:bg-white/10"
              onclick={() => {
                startQueue($queue, $currentIndex + 1 + i, { type: $playingFrom.type, name: $playingFrom.name, href: $playingFrom.href });
                mobileQueueOpen = false;
              }}
            >
              {#if song.coverArtUrl}
                <img class="size-10 shrink-0 rounded-lg object-cover opacity-80" src={song.coverArtUrl} alt={song.title} />
              {:else}
                <div class="grid size-10 shrink-0 place-items-center rounded-lg bg-white/10 text-xs font-bold text-white/40">{initials(song.title)}</div>
              {/if}
              <div class="min-w-0 flex-1">
                <p class="truncate text-sm font-medium text-white/85">{song.title}</p>
                <p class="truncate text-xs text-white/40">{song.artist}</p>
              </div>
            </button>
          {/each}
        {/if}
      </div>
    </div>
  </div>
{/if}

{:else}
<!-- ─── DESKTOP: existing player bar ──────────────────────────────────── -->
<footer class="player-bar liquid-glass shrink-0 border-t border-white/[0.08] px-4 py-3 {isBuffering || $queueLoading ? 'player-bar-loading' : ''}">
  <div class="grid w-full items-center gap-3 md:grid-cols-3" style="grid-template-columns: minmax(0,1fr) minmax(220px,1.4fr) minmax(0,1fr)">
    <div class="player-track-info flex min-w-0 items-center gap-3">
      {#if currentTrack}
        <SongContextMenu song={currentTrack} triggerClass="contents">
        <button
          class="player-track-art-button shrink-0 cursor-pointer"
          onclick={() => showQueue.update((open) => !open)}
          title="Toggle now playing"
          tabindex="-1"
        >
          {#if currentTrack.coverArtUrl}
            <img class="player-track-art size-11 rounded-md object-cover shadow-sm" src={currentTrack.coverArtUrl} alt={currentTrack.title} />
          {:else}
            <div class="player-track-art player-track-art-fallback grid size-11 shrink-0 place-items-center rounded-md bg-gradient-to-br from-muted to-muted/60 text-xs font-bold text-muted-foreground">
              {initials(currentTrack.title)}
            </div>
          {/if}
        </button>
        <div class="player-track-meta min-w-0">
          <div class="flex min-w-0 items-center gap-1.5 overflow-hidden">
            <button
              class="player-track-title min-w-0 flex-1 truncate text-left text-sm font-semibold leading-tight"
              onclick={() => currentTrack?.albumId && goto(`/album/${encodeURIComponent(currentTrack.albumId)}`)}
              title={currentTrack.album}
            >{currentTrack.title}</button>
            <SongTechBadge
              cached={desktopController.currentTrackCached}
              audioFormat={currentTrack.audioFormat}
              bitrateKbps={currentTrack.bitrateKbps}
              compact
            />
            {#if isSmartShuffleTrack}
              <Tooltip>
                <TooltipTrigger>
                  {#snippet child({ props })}
                    <span {...props} class="shrink-0 inline-flex cursor-default items-center justify-center rounded-full border border-primary/30 bg-primary/10 p-1 text-primary">
                      <Sparkles class="size-3" />
                    </span>
                  {/snippet}
                </TooltipTrigger>
                <TooltipContent side="top" sideOffset={6}>Smart Shuffle track</TooltipContent>
              </Tooltip>
            {/if}
            <Tooltip>
              <TooltipTrigger>
                {#snippet child({ props })}
                  <button
                    {...props}
                    onclick={toggleFavorite}
                    class={favoriteButtonClass}
                    aria-label={isStarred ? 'Remove from favorites' : 'Add to favorites'}
                  >
                    <Heart class="size-3 {isStarred ? 'fill-rose-500' : ''}" />
                  </button>
                {/snippet}
              </TooltipTrigger>
              <TooltipContent side="top" sideOffset={6}>
                {isStarred ? 'Remove from favorites' : 'Add to favorites'}
              </TooltipContent>
            </Tooltip>
          </div>
          {#if externalSourceLabel}
            <span class="block truncate text-xs text-sky-300/90 animate-pulse">
              Fetching from {externalSourceLabel}…
            </span>
          {:else}
            <SongArtistLinks
              artist={currentTrack.artist}
              class="block truncate text-xs text-muted-foreground max-w-full text-left"
              linkClass="hover:underline cursor-pointer"
            />
          {/if}
        </div>
        </SongContextMenu>
      {:else}
        <p class="text-sm text-muted-foreground">No track selected</p>
      {/if}
    </div>

    <div class="player-center-column flex flex-col items-center gap-1.5">
      <div class="player-transport flex items-center gap-1">
        <PlayerShuffleMenu
          {currentTrack}
          smartShuffleFetching={shuffleController.smartShuffleFetching}
          shuffleButtonClass={shuffleController.shuffleButtonClass}
          onActivateShuffle={shuffleController.activateShuffle}
          onActivateSmartShuffle={shuffleController.activateSmartShuffle}
          onDeactivateShuffle={shuffleController.deactivateShuffle}
          onShuffleArtist={shuffleController.shuffleArtist}
          onShuffleAlbum={shuffleController.shuffleAlbum}
        />

        <Tooltip>
          <TooltipTrigger>{#snippet child({ props })}<Button {...props} variant="ghost" size="icon-sm" class="player-transport-button text-muted-foreground hover:text-foreground" onclick={prevTrack} aria-label="Previous track"><SkipBack class="size-[18px]" /></Button>{/snippet}</TooltipTrigger>
          <TooltipContent side="top" sideOffset={6}>Previous track</TooltipContent>
        </Tooltip>

        <Button
          class="player-play-button rounded-full shadow-sm {showPauseButton ? 'is-playing' : ''} {isBuffering || $queueLoading ? 'is-buffering' : ''}"
          size="icon-lg"
          onclick={togglePlay}
          disabled={!currentTrack || transportLocked}
          aria-label={showPauseButton ? 'Pause' : 'Play'}
        >
          {#if showPauseButton}
            <Pause class="size-5" />
          {:else}
            <Play class="size-5" />
          {/if}
        </Button>

        <Tooltip>
          <TooltipTrigger>{#snippet child({ props })}<Button {...props} variant="ghost" size="icon-sm" class="player-transport-button text-muted-foreground hover:text-foreground" onclick={nextTrack} aria-label="Next track"><SkipForward class="size-[18px]" /></Button>{/snippet}</TooltipTrigger>
          <TooltipContent side="top" sideOffset={6}>Next track</TooltipContent>
        </Tooltip>

        <Tooltip>
          <TooltipTrigger>
            {#snippet child({ props })}
              <Button {...props} variant="ghost" size="icon-sm" class={repeatButtonClass} onclick={cycleRepeatMode} aria-label="Cycle repeat mode">
                {#if $repeatMode === 'one'}<Repeat1 class="size-3.5" />{:else}<Repeat class="size-3.5" />{/if}
              </Button>
            {/snippet}
          </TooltipTrigger>
          <TooltipContent side="top" sideOffset={6}>{repeatTooltip}</TooltipContent>
        </Tooltip>
      </div>

      <div class="player-progress-row flex w-full items-center gap-2">
        <span class="player-time-label w-10 text-right text-[11px] tabular-nums text-muted-foreground">{formatClockDuration($currentTime)}</span>
        <div class="player-progress-shell relative flex-1 {isBuffering || $queueLoading ? 'opacity-75 is-buffering' : ''}">
          {#if isBuffering || $queueLoading}
            <div class="pointer-events-none absolute inset-y-0 left-0 right-0 flex items-center">
              <div class="player-buffer-track h-1.5 w-full overflow-hidden rounded-full bg-muted/80">
                <div class="player-buffer-bar h-full w-1/3 rounded-full bg-primary/55"></div>
              </div>
            </div>
          {/if}
          <Slider
            class="player-seek-slider"
            type="multiple"
            bind:value={seekVal}
            disabled={!currentTrack}
            min={0}
            max={isFinite($duration) && $duration > 0 ? $duration : 1}
            step={1}
            onpointerdown={() => { seekDragging = true; }}
            onValueChange={() => {}}
            onValueCommit={(v) => { seek(v); }}
            aria-label="Playback position"
          />
        </div>
        <span class="player-time-label w-10 text-[11px] tabular-nums text-muted-foreground">{formatClockDuration($duration)}</span>
      </div>
    </div>

    <div class="player-actions flex min-w-0 items-center justify-end gap-2">
      <PlayerCastMenu
        castActive={castActive}
        discovering={castController.discovering}
        castDevice={castDevice}
        castDevices={castController.castDevices}
        disabled={!currentTrack}
        onDiscover={castController.discoverDevices}
        onStartCast={castController.startCast}
        onStopCast={castController.stopCast}
      />

      <Tooltip>
        <TooltipTrigger>{#snippet child({ props })}<Button {...props} variant="ghost" size="icon-sm" class={lyricsButtonClass} onclick={() => showLyrics.update((v) => !v)} aria-label="Lyrics"><MicVocal class="size-[18px]" /></Button>{/snippet}</TooltipTrigger>
        <TooltipContent side="top" sideOffset={6}>Lyrics</TooltipContent>
      </Tooltip>

      <PlayerVolume {castActive} {castVolume} />
    </div>
  </div>

</footer>
{/if}

<style>
  .player-bar {
    position: relative;
    overflow: hidden;
    transition: opacity 180ms ease;
  }

  .player-bar-loading {
    opacity: 0.88;
  }

  .player-track-info,
  .player-center-column,
  .player-actions {
    position: relative;
    z-index: 1;
  }

  .player-track-art-button {
    border-radius: 0.9rem;
    transition:
      transform 180ms ease,
      filter 180ms ease;
  }

  .player-track-art-button:hover {
    transform: translateY(-1px);
  }

  .player-track-art {
    transition:
      transform 220ms ease,
      filter 220ms ease,
      box-shadow 220ms ease;
    box-shadow:
      0 10px 26px hsl(var(--background) / 0.28),
      0 0 0 1px hsl(var(--foreground) / 0.06);
  }

  .player-track-art-button:hover .player-track-art {
    transform: scale(1.025);
    filter: saturate(1.03);
    box-shadow:
      0 16px 32px hsl(var(--background) / 0.34),
      0 0 0 1px hsl(var(--foreground) / 0.08);
  }

  .player-track-title {
    transition:
      color 180ms ease,
      transform 180ms ease,
      opacity 180ms ease;
  }

  .player-track-title:hover {
    transform: translateX(1px);
    text-decoration: underline;
    text-underline-offset: 0.18em;
  }

  .player-favorite-button {
    transition:
      transform 180ms ease,
      color 180ms ease,
      opacity 180ms ease;
  }

  .player-favorite-button:hover {
    transform: translateY(-1px);
  }

  .player-transport {
    gap: 0.3rem;
  }

  :global(.player-transport-button) {
    position: relative;
    transition:
      transform 180ms ease,
      color 180ms ease,
      background-color 180ms ease,
      opacity 180ms ease;
  }

  :global(.player-transport-button:hover:not(:disabled)) {
    transform: translateY(-1px);
  }

  :global(.player-transport-button.is-active::after) {
    content: '';
    position: absolute;
    bottom: 0.2rem;
    left: 50%;
    width: 0.28rem;
    height: 0.28rem;
    border-radius: 999px;
    background: hsl(var(--primary));
    transform: translateX(-50%);
    box-shadow: 0 0 12px hsl(var(--primary) / 0.42);
  }

  :global(.player-play-button) {
    position: relative;
    overflow: visible;
    transition:
      transform 180ms ease,
      box-shadow 220ms ease,
      filter 220ms ease;
    box-shadow:
      0 12px 28px hsl(var(--primary) / 0.2),
      0 0 0 1px hsl(var(--foreground) / 0.06);
  }

  :global(.player-play-button::before) {
    content: '';
    position: absolute;
    inset: -0.4rem;
    z-index: -1;
    border-radius: 999px;
    background: radial-gradient(circle, hsl(var(--primary) / 0.24), transparent 68%);
    opacity: 0;
    transform: scale(0.88);
    transition:
      opacity 220ms ease,
      transform 220ms ease;
  }

  :global(.player-play-button:hover:not(:disabled)) {
    transform: translateY(-1px) scale(1.015);
    box-shadow:
      0 16px 30px hsl(var(--primary) / 0.26),
      0 0 0 1px hsl(var(--foreground) / 0.08);
  }

  :global(.player-play-button:hover:not(:disabled)::before),
  :global(.player-play-button.is-playing::before) {
    opacity: 1;
    transform: scale(1);
  }

  :global(.player-play-button.is-playing) {
    animation: player-play-glow 2.6s ease-in-out infinite;
  }

  :global(.player-play-button.is-buffering) {
    animation: player-buffer-breathe 1.1s ease-in-out infinite;
  }

  :global(.player-play-button:disabled) {
    transform: none;
    box-shadow:
      0 8px 18px hsl(var(--background) / 0.18),
      0 0 0 1px hsl(var(--foreground) / 0.04);
  }

  .player-progress-row {
    gap: 0.65rem;
  }

  .player-time-label {
    line-height: 1;
    align-self: center;
    transform: translateY(-0.5px);
  }

  .player-progress-shell {
    min-height: 1rem;
    display: flex;
    align-items: center;
    transition:
      opacity 180ms ease,
      transform 180ms ease;
  }

  .player-progress-shell.is-buffering {
    transform: translateY(-0.5px);
  }

  .player-buffer-track {
    box-shadow: inset 0 0 0 1px hsl(var(--foreground) / 0.04);
  }

  .player-buffer-bar {
    animation: player-buffer-slide 1.35s cubic-bezier(0.45, 0, 0.2, 1) infinite;
  }

  :global(.player-seek-slider),
  :global(.player-volume-slider) {
    min-height: 1rem;
    position: relative;
    z-index: 2;
  }

  :global(.player-seek-slider [data-slot='slider-track']),
  :global(.player-volume-slider [data-slot='slider-track']) {
    width: 100%;
  }

  :global(.player-seek-slider[data-slot='slider']),
  :global(.player-volume-slider[data-slot='slider']) {
    min-height: 1rem;
  }

  .player-actions {
    gap: 0.35rem;
  }

  :global(.player-cast-icon) {
    animation: player-cast-pulse 2.1s ease-in-out infinite;
  }

  @keyframes player-buffer-slide {
    0% {
      transform: translateX(-140%) scaleX(0.85);
      opacity: 0.35;
    }

    55% {
      opacity: 0.95;
    }

    100% {
      transform: translateX(320%) scaleX(1.1);
      opacity: 0.3;
    }
  }

  @keyframes player-buffer-breathe {
    0%,
    100% {
      transform: scale(1);
      filter: saturate(1);
    }

    50% {
      transform: scale(0.97);
      filter: saturate(0.92);
    }
  }

  @keyframes player-play-glow {
    0%,
    100% {
      box-shadow:
        0 12px 28px hsl(var(--primary) / 0.2),
        0 0 0 1px hsl(var(--foreground) / 0.06);
    }

    50% {
      box-shadow:
        0 16px 34px hsl(var(--primary) / 0.28),
        0 0 0 1px hsl(var(--foreground) / 0.08);
    }
  }

  @keyframes player-cast-pulse {
    0%,
    100% {
      filter: drop-shadow(0 0 0 hsl(var(--primary) / 0));
    }

    50% {
      filter: drop-shadow(0 0 10px hsl(var(--primary) / 0.3));
    }
  }
</style>