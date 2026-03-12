<script lang="ts">
  import {
    DropdownMenu,
    DropdownMenuTrigger,
    DropdownMenuContent,
    DropdownMenuItem,
    DropdownMenuSeparator,
  } from '$lib/components/ui/dropdown-menu';
  import {
    Pause,
    Play,
    Repeat,
    Repeat1,
    Shuffle,
    SkipBack,
    SkipForward,
    Sparkles,
    ChevronDown,
    Volume2,
    VolumeX,
    Mic2,
    Heart,
    Disc3,
    User2
  } from '@lucide/svelte';
  import {
    currentIndex,
    currentTime,
    duration,
    isPlaying,
    nextTrack,
    prevTrack,
    queue,
    repeatMode,
    shuffleEnabled,
    shouldAutoplay,
    toggleShuffle,
    cycleRepeatMode,
    volume,
    upNextEnabled,
    smartShuffleMode,
    appendToQueue,
    pruneQueueHistory,
    showLyrics,
    seekRequest,
    starredSongIds,
    playQueue
  } from '$lib/stores/player';
  import { fetchUpNextSongs, fetchSubsonicSimilar, starSubsonicSong, unstarSubsonicSong, lfmNowPlaying, lfmScrobble, lfmUserTaste, fetchSubsonicArtistAlbums, fetchSubsonicAlbumSongs } from '$lib/api';
  import { fetchLikedArtists } from '$lib/api';
  import { toast } from 'svelte-sonner';
  import { Button, Slider } from '$lib/components/ui';
  import { appSettings } from '$lib/stores/settings';
  import SongContextMenu from '$lib/components/SongContextMenu.svelte';
  import { goto } from '$app/navigation';
  import { showQueue } from '$lib/stores/player';

  const lastFmApiKey = $derived($appSettings.lastFmApiKey);
  const lastFmConnected = $derived($appSettings.lastFmConnected);
  const shuffleBtnClass = $derived($smartShuffleMode || $shuffleEnabled ? 'text-primary' : 'text-muted-foreground hover:text-foreground');

  // ── Scrobbling ─────────────────────────────────────────────────────────────
  // Send "now playing" when the track changes, then scrobble once the user
  // has listened for ≥50% of the track (or 240 seconds, per Last.fm spec).
  // Both are fire-and-forget — errors are absorbed so playback is never affected.
  let scrobbledTrackId = '';
  let scrobbleStartTime = 0;

  $effect(() => {
    const track = currentTrack;
    if (!track || !lastFmConnected) return;
    // New track started
    scrobbledTrackId = '';
    scrobbleStartTime = Math.floor(Date.now() / 1000);
    lfmNowPlaying(track.artist, track.title, track.album || undefined, track.duration || undefined);
  });

  $effect(() => {
    const t = $currentTime;
    const dur = $duration;
    const track = currentTrack;
    if (!track || !lastFmConnected || scrobbledTrackId === track.id) return;
    // Scrobble threshold: 50% or 240 seconds, whichever is less
    const threshold = dur > 0 ? Math.min(dur * 0.5, 240) : 240;
    if (t >= threshold) {
      scrobbledTrackId = track.id;
      lfmScrobble(track.artist, track.title, scrobbleStartTime, track.album || undefined, track.duration || undefined);
    }
  });

  async function toggleFavorite() {
    if (!currentTrack) return;
    const id = currentTrack.id;
    if (isStarred) {
      starredSongIds.update((ids) => { const s = new Set(ids); s.delete(id); return s; });
      try {
        await unstarSubsonicSong(id, currentTrack.artist, currentTrack.title);
      } catch {
        starredSongIds.update((ids) => new Set([...ids, id]));
        toast.error('Failed to remove from favorites');
      }
    } else {
      starredSongIds.update((ids) => new Set([...ids, id]));
      try {
        await starSubsonicSong(id, currentTrack.artist, currentTrack.title);
      } catch {
        starredSongIds.update((ids) => { const s = new Set(ids); s.delete(id); return s; });
        toast.error('Failed to add to favorites');
      }
    }
  }

  let audioEl = $state<HTMLAudioElement | null>(null);

  const currentTrack = $derived($queue[$currentIndex] ?? null);
  const isStarred = $derived(currentTrack ? $starredSongIds.has(currentTrack.id) : false);

  // Local state for the seek slider — synced from $currentTime when not dragging
  let seekVal = $state<number[]>([0]);
  let seekDragging = $state(false);
  let isBuffering = $state(false);
  $effect(() => {
    if (!seekDragging) seekVal = [$currentTime];
  });

  // Local state for the volume slider
  let volVal = $state<number[]>([$volume * 100]);
  let volDragging = $state(false);
  $effect(() => {
    if (!volDragging) volVal = [$volume * 100];
  });

  // Effect 1: load new track src
  $effect(() => {
    const track = currentTrack;
    if (!audioEl || !track?.streamUrl) return;
    if (audioEl.src === track.streamUrl) return;
    audioEl.src = track.streamUrl;
    currentTime.set(0);
    // Use the duration reported by the Subsonic API immediately — don't wait
    // for loadedmetadata, which often never fires a finite value for streams.
    duration.set(track.duration > 0 ? track.duration : 0);
  });

  // Effect 2: sync volume to audio element
  $effect(() => {
    if (audioEl) audioEl.volume = $volume;
  });

  // Effect 3: trigger autoplay
  $effect(() => {
    if (!audioEl || !$shouldAutoplay) return;
    shouldAutoplay.set(false);
    audioEl
      .play()
      .then(() => isPlaying.set(true))
      .catch(() => isPlaying.set(false));
  });
  function activateShuffle() {
    smartShuffleMode.set(false);
    shuffleEnabled.set(true);
  }

  function activateSmartShuffle() {
    shuffleEnabled.set(true);
    smartShuffleMode.set(true);
  }

  function deactivateShuffle() {
    shuffleEnabled.set(false);
    smartShuffleMode.set(false);
  }

  async function shuffleArtist() {
    if (!currentTrack) return;
    const artist = currentTrack.artist;
    const toastId = toast.loading(`Loading ${artist}…`);
    try {
      const albums = await fetchSubsonicArtistAlbums(artist, 50);
      const allSongs = (await Promise.all(albums.map(a => fetchSubsonicAlbumSongs(a.id)))).flat();
      if (!allSongs.length) throw new Error('No songs found');
      // Fisher-Yates shuffle
      for (let i = allSongs.length - 1; i > 0; i--) {
        const j = Math.floor(Math.random() * (i + 1));
        [allSongs[i], allSongs[j]] = [allSongs[j], allSongs[i]];
      }
      smartShuffleMode.set(false);
      shuffleEnabled.set(true);
      playQueue(allSongs, 0);
      toast.success(`Shuffling ${artist}`, { id: toastId });
    } catch {
      toast.error('Failed to load artist songs', { id: toastId });
    }
  }

  async function shuffleAlbum() {
    if (!currentTrack?.albumId) return;
    const albumName = currentTrack.album;
    const toastId = toast.loading(`Loading ${albumName}…`);
    try {
      const songs = await fetchSubsonicAlbumSongs(currentTrack.albumId);
      if (!songs.length) throw new Error('No songs found');
      for (let i = songs.length - 1; i > 0; i--) {
        const j = Math.floor(Math.random() * (i + 1));
        [songs[i], songs[j]] = [songs[j], songs[i]];
      }
      smartShuffleMode.set(false);
      shuffleEnabled.set(true);
      playQueue(songs, 0);
      toast.success(`Shuffling ${albumName}`, { id: toastId });
    } catch {
      toast.error('Failed to load album songs', { id: toastId });
    }
  }

  // Smart Shuffle: every SMART_SHUFFLE_INJECT_EVERY tracks, weave Last.fm-powered
  // recommendations into the upcoming queue. Falls back to Subsonic similar if
  // no Last.fm key is configured. Songs are inserted a few positions ahead so
  // they feel naturally mixed in rather than dumped at the end.
  const SMART_SHUFFLE_INJECT_EVERY = 3;
  let smartShufflePlayCount = 0;
  let smartShuffleLastIdx = -1;
  let smartShuffleInflight = false;
  let smartShuffleFetching = $state(false);

  $effect(() => {
    const track = $queue[$currentIndex];
    const idx = $currentIndex;

    if (!$smartShuffleMode) {
      // Reset counters when Smart Shuffle is toggled off
      smartShufflePlayCount = 0;
      smartShuffleLastIdx = -1;
      return;
    }
    if (!track) return;

    // Only count actual track advances, not queue mutations
    if (idx === smartShuffleLastIdx) return;
    smartShuffleLastIdx = idx;
    smartShufflePlayCount++;

    // Prune played history so old songs don't pile up at the bottom
    pruneQueueHistory(1);

    if (smartShufflePlayCount < SMART_SHUFFLE_INJECT_EVERY || smartShuffleInflight) return;

    smartShufflePlayCount = 0;
    smartShuffleInflight = true;
    smartShuffleFetching = true;

    // Capture current state before the async gap
    const existingIds = new Set($queue.map(s => s.id));
    const capturedIdx = idx;
    const { artist, title, id } = track;

    const doFetch: Promise<import('$lib/api').SubsonicSong[]> = lastFmApiKey
      ? Promise.all([
          fetchLikedArtists().then(stored => stored.map(a => a.name)).catch((): string[] => []),
          lfmUserTaste().catch((): string[] => [])
        ]).then(([liked, taste]) => {
          // Merge liked artists + Last.fm top artists, deduplicated
          const merged = [...new Set([...liked, ...taste])];
          return fetchUpNextSongs({ apiKey: lastFmApiKey, artist, title, likedArtists: merged, limit: 3 });
        })
      : fetchSubsonicSimilar(id, 3);

    doFetch
      .then(songs => {
        const fresh = songs.filter(s => !existingIds.has(s.id));
        if (!fresh.length) return;
        queue.update(items => {
          const next = [...items];
          // Weave songs in starting 2 positions ahead, spacing them 2 apart
          const base = Math.min(capturedIdx + 2, next.length);
          fresh.forEach((song, i) => {
            next.splice(Math.min(base + i * 2, next.length), 0, song);
          });
          return next;
        });
      })
      .catch(() => undefined)
      .finally(() => {
        smartShuffleInflight = false;
        smartShuffleFetching = false;
      });
  });

  let upNextFetching = false;
  let lastUpNextSeed = '';

  $effect(() => {
    const items = $queue;
    const index = $currentIndex;
    const repeat = $repeatMode;

    // In radio mode (no smart shuffle), also prune played history
    if ($upNextEnabled && !$smartShuffleMode && index > 1) {
      pruneQueueHistory(1);
    }

    const nearEnd = items.length > 0 && index >= items.length - 1;
    if (!nearEnd || !$upNextEnabled || repeat !== 'off' || !lastFmApiKey) return;

    const track = items[index];
    if (!track) return;
    const seed = `${track.artist}::${track.title}`;
    if (seed === lastUpNextSeed || upNextFetching) return;
    lastUpNextSeed = seed;
    upNextFetching = true;

    fetchLikedArtists()
      .then((stored) => stored.map((a) => a.name))
      .catch(() => [] as string[])
      .then((liked) =>
        fetchUpNextSongs({
          apiKey: lastFmApiKey,
          artist: track.artist,
          title: track.title,
          likedArtists: liked,
          limit: 5
        })
      )
      .then((songs) => {
        if (songs.length) appendToQueue(songs);
      })
      .catch(() => undefined)
      .finally(() => {
        upNextFetching = false;
      });
  });

  function initials(name: string): string {
    return name
      .split(' ')
      .filter(Boolean)
      .slice(0, 2)
      .map((part) => part[0]?.toUpperCase() ?? '')
      .join('');
  }

  function togglePlay() {
    if (!audioEl || !currentTrack) return;
    if ($isPlaying) {
      audioEl.pause();
    } else {
      audioEl.play().catch(() => {
        isPlaying.set(false);
      });
    }
  }

  function onVolumeWheel(e: WheelEvent) {
    e.preventDefault();
    // deltaY < 0 = scroll up = louder; deltaY > 0 = scroll down = quieter
    const delta = e.deltaY < 0 ? 0.05 : -0.05;
    const next = Math.max(0, Math.min(1, $volume + delta));
    volume.set(next);
    if (audioEl) audioEl.volume = next;
  }

  function seek(values: number[]) {
    const value = Math.max(0, Number(values[0] ?? 0));
    // Update store + local val BEFORE releasing the drag lock so the sync
    // $effect sees the correct value and doesn't snap back to the old position.
    currentTime.set(value);
    seekVal = [value];
    seekDragging = false;
    if (audioEl) audioEl.currentTime = value;
  }

  function changeVolume(values: number[]) {
    const value = Math.max(0, Math.min(1, Number(values[0] ?? 0) / 100));
    volume.set(value);
    if (audioEl) audioEl.volume = value;
  }

  function commitVolume(values: number[]) {
    changeVolume(values);
    volDragging = false;
  }

  let premuteVolume = $state(0.8);
  function toggleMute() {
    if ($volume <= 0.01) {
      const restore = premuteVolume > 0.01 ? premuteVolume : 0.8;
      volume.set(restore);
      if (audioEl) audioEl.volume = restore;
    } else {
      premuteVolume = $volume;
      volume.set(0);
      if (audioEl) audioEl.volume = 0;
    }
  }

  // Handle seek requests from the lyrics panel
  $effect(() => {
    const t = $seekRequest;
    if (t === null || !audioEl) return;
    audioEl.currentTime = t;
    currentTime.set(t);
    seekRequest.set(null);
  });

  function fmt(seconds: number): string {
    if (!isFinite(seconds) || isNaN(seconds)) return '0:00';
    const safe = Math.max(0, Math.floor(seconds));
    const mm = Math.floor(safe / 60);
    const ss = safe % 60;
    return `${mm}:${ss.toString().padStart(2, '0')}`;
  }
</script>

<footer class="shrink-0 border-t border-border/80 bg-background/95 px-4 py-3 backdrop-blur">
  <div class="grid w-full items-center gap-3 md:grid-cols-3" style="grid-template-columns: 1fr minmax(420px, 2fr) 1fr">
    <!-- Track info -->
    <div class="flex min-w-0 items-center gap-3">
      {#if currentTrack}
        <SongContextMenu song={currentTrack} triggerClass="contents">
          {#snippet children()}
            <button
              class="shrink-0 cursor-pointer"
              onclick={() => showQueue.update(v => !v)}
              title="Toggle queue"
              tabindex="-1"
            >
              {#if currentTrack.coverArtUrl}
                <img class="size-11 rounded-md object-cover shadow-sm hover:opacity-80 transition-opacity" src={currentTrack.coverArtUrl} alt={currentTrack.title} />
              {:else}
                <div class="grid size-11 shrink-0 place-items-center rounded-md bg-gradient-to-br from-muted to-muted/60 text-xs font-bold text-muted-foreground hover:opacity-80 transition-opacity">
                  {initials(currentTrack.title)}
                </div>
              {/if}
            </button>
          {/snippet}
        </SongContextMenu>
        <div class="min-w-0">
          <div class="flex items-center gap-1.5">
            <button
              class="block truncate text-sm font-semibold leading-tight hover:underline cursor-pointer max-w-full text-left"
              onclick={() => currentTrack?.albumId && goto(`/album/${encodeURIComponent(currentTrack.albumId)}`)}
              title={currentTrack.album}
            >{currentTrack.title}</button>
            <button
              onclick={toggleFavorite}
              class="shrink-0 text-muted-foreground transition-colors duration-150 hover:text-foreground {isStarred ? '!text-rose-500' : ''}"
              aria-label={isStarred ? 'Remove from favorites' : 'Add to favorites'}
              title={isStarred ? 'Remove from favorites' : 'Add to favorites'}
            >
              <Heart class="size-3.5 {isStarred ? 'fill-rose-500' : ''}" />
            </button>
          </div>
          <button
            class="block truncate text-xs text-muted-foreground hover:underline cursor-pointer max-w-full text-left"
            onclick={() => goto(`/artist/${encodeURIComponent(currentTrack.artist)}`)}
            title="Go to artist"
          >{currentTrack.artist}</button>
        </div>
      {:else}
        <p class="text-sm text-muted-foreground">No track selected</p>
      {/if}
    </div>

    <!-- Playback controls -->
    <div class="flex flex-col items-center gap-1.5">
      <div class="flex items-center gap-1">
        <!-- Shuffle button -->
        <DropdownMenu>
          <DropdownMenuTrigger>
            {#snippet child({ props })}
              <Button
                {...props}
                variant="ghost"
                size="icon-sm"
                class={shuffleBtnClass}
                aria-label="Shuffle options"
                title={$smartShuffleMode ? 'Smart Shuffle on' : $shuffleEnabled ? 'Shuffle on' : 'Shuffle off'}
              >
                {#if $smartShuffleMode}
                  <Sparkles class="size-3.5 transition-opacity {smartShuffleFetching ? 'animate-pulse' : ''}" />
                {:else}
                  <Shuffle class="size-3.5" />
                {/if}
              </Button>
            {/snippet}
          </DropdownMenuTrigger>
          <DropdownMenuContent side="top" align="start" class="min-w-56">
            <DropdownMenuItem onclick={activateShuffle} class="gap-3 {$shuffleEnabled && !$smartShuffleMode ? 'text-primary' : ''}">
              <Shuffle class="size-4 shrink-0" />
              <div>
                <p class="font-medium">Shuffle</p>
                <p class="text-xs text-muted-foreground">Play queue in random order</p>
              </div>
              {#if $shuffleEnabled && !$smartShuffleMode}
                <span class="ml-auto size-1.5 rounded-full bg-primary"></span>
              {/if}
            </DropdownMenuItem>
            <DropdownMenuItem onclick={activateSmartShuffle} class="gap-3 {$smartShuffleMode ? 'text-primary' : ''}">
              <Sparkles class="size-4 shrink-0" />
              <div>
                <p class="font-medium">Smart Shuffle</p>
                <p class="text-xs text-muted-foreground">Weaves in Last.fm recommendations</p>
              </div>
              {#if $smartShuffleMode}
                <span class="ml-auto size-1.5 rounded-full bg-primary"></span>
              {/if}
            </DropdownMenuItem>
            {#if currentTrack}
              <DropdownMenuSeparator />
              <DropdownMenuItem onclick={shuffleArtist} class="gap-3">
                <Mic2 class="size-4 shrink-0" />
                <div>
                  <p class="font-medium">Shuffle Artist</p>
                  <p class="truncate max-w-36 text-xs text-muted-foreground">{currentTrack.artist}</p>
                </div>
              </DropdownMenuItem>
              <DropdownMenuItem onclick={shuffleAlbum} disabled={!currentTrack.albumId} class="gap-3">
                <Disc3 class="size-4 shrink-0" />
                <div>
                  <p class="font-medium">Shuffle Album</p>
                  <p class="truncate max-w-36 text-xs text-muted-foreground">{currentTrack.album}</p>
                </div>
              </DropdownMenuItem>
            {/if}
            <DropdownMenuSeparator />
            <DropdownMenuItem onclick={deactivateShuffle} class="gap-3 {!$shuffleEnabled && !$smartShuffleMode ? 'text-primary' : 'text-muted-foreground'}">
              <span class="size-4 shrink-0 flex items-center justify-center text-xs font-bold">—</span>
              <div>
                <p class="font-medium">Off</p>
                <p class="text-xs text-muted-foreground">Play in order</p>
              </div>
              {#if !$shuffleEnabled && !$smartShuffleMode}
                <span class="ml-auto size-1.5 rounded-full bg-primary"></span>
              {/if}
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
        <Button
          variant="ghost"
          size="icon-sm"
          class="text-muted-foreground hover:text-foreground"
          onclick={prevTrack}
          aria-label="Previous track"
        >
          <SkipBack class="size-[18px]" />
        </Button>
        <Button
          class="rounded-full shadow-sm"
          size="icon-lg"
          onclick={togglePlay}
          disabled={!currentTrack}
          aria-label={$isPlaying ? 'Pause' : 'Play'}
        >
          {#if $isPlaying}
            <Pause class="size-5" />
          {:else}
            <Play class="size-5" />
          {/if}
        </Button>
        <Button
          variant="ghost"
          size="icon-sm"
          class="text-muted-foreground hover:text-foreground"
          onclick={nextTrack}
          aria-label="Next track"
        >
          <SkipForward class="size-[18px]" />
        </Button>
        <Button
          variant="ghost"
          size="icon-sm"
          class={$repeatMode !== 'off' ? 'text-primary' : 'text-muted-foreground hover:text-foreground'}
          onclick={cycleRepeatMode}
          aria-label="Cycle repeat mode"
        >
          {#if $repeatMode === 'one'}
            <Repeat1 class="size-3.5" />
          {:else}
            <Repeat class="size-3.5" />
          {/if}
        </Button>
      </div>
      <div class="flex w-full items-center gap-2">
        <span class="w-10 text-right text-[11px] tabular-nums text-muted-foreground">{fmt($currentTime)}</span>
        <div class="relative flex-1 {isBuffering ? 'opacity-60' : ''}">
          {#if isBuffering}
            <div class="pointer-events-none absolute inset-y-0 left-0 right-0 flex items-center">
              <div class="h-1.5 w-full overflow-hidden rounded-full bg-muted">
                <div class="h-full w-1/3 animate-[buffering_1.2s_ease-in-out_infinite] rounded-full bg-primary/50"></div>
              </div>
            </div>
          {/if}
          <Slider
            type="multiple"
            value={seekVal}
            min={0}
            max={isFinite($duration) && $duration > 0 ? $duration : 1}
            step={1}
            onpointerdown={() => { seekDragging = true; }}
            onValueChange={(v) => { seekVal = v; }}
            onValueCommit={(v) => { seek(v); }}
            aria-label="Playback position"
          />
        </div>
        <span class="w-10 text-[11px] tabular-nums text-muted-foreground">{fmt($duration)}</span>
      </div>
    </div>

    <!-- Volume -->
    <div class="flex items-center justify-end gap-2">
      <Button
        variant="ghost"
        size="icon-sm"
        class={$showLyrics ? 'text-primary' : 'text-muted-foreground hover:text-foreground'}
        onclick={() => showLyrics.update((v) => !v)}
        aria-label="Lyrics"
      >
        <Mic2 class="size-[18px]" />
      </Button>
      <div
        class="flex w-full items-center gap-2 md:max-w-44"
        onwheel={onVolumeWheel}
        role="group"
        aria-label="Volume"
      >
        {#if $volume <= 0.01}
          <button onclick={toggleMute} aria-label="Unmute" class="shrink-0 text-muted-foreground hover:text-foreground transition-colors">
            <VolumeX class="size-[18px]" />
          </button>
        {:else}
          <button onclick={toggleMute} aria-label="Mute" class="shrink-0 text-muted-foreground hover:text-foreground transition-colors">
            <Volume2 class="size-[18px]" />
          </button>
        {/if}
        <Slider
          type="multiple"
          value={volVal}
          min={0}
          max={100}
          step={1}
          onpointerdown={() => { volDragging = true; }}
          onValueChange={(v) => { volVal = v; changeVolume(v); }}
          onValueCommit={commitVolume}
          aria-label="Volume"
        />
      </div>
    </div>
  </div>

  <audio
    bind:this={audioEl}
    onplay={() => isPlaying.set(true)}
    onpause={() => isPlaying.set(false)}
    onwaiting={() => { isBuffering = true; }}
    onseeking={() => { isBuffering = true; }}
    onseeked={() => { isBuffering = false; }}
    oncanplay={() => { isBuffering = false; }}
    oncanplaythrough={() => { isBuffering = false; }}
    ontimeupdate={() => {
      currentTime.set(audioEl?.currentTime ?? 0);
    }}
    onloadedmetadata={() => {
      const d = audioEl?.duration ?? NaN;
      // Only override the API-provided duration if the element gives a better finite value
      if (isFinite(d) && d > 0) duration.set(d);
    }}
    ondurationchange={() => {
      const d = audioEl?.duration ?? NaN;
      if (isFinite(d) && d > 0) duration.set(d);
    }}
    onended={nextTrack}
  ></audio>
</footer>
