<script lang="ts">
  import { onMount, untrack } from 'svelte';
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
    User2,
    Cast
  } from '@lucide/svelte';
  import {
    focusTrack,
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
    enableShuffle,
    enableSmartShuffle,
    disableShuffle,
    cycleRepeatMode,
    volume,
    upNextEnabled,
    smartShuffleMode,
    appendToQueue,
    pruneQueueHistory,
    showLyrics,
    seekRequest,
    togglePlayRequest,
    starredSongIds,
    playQueue,
    addRecentlyPlayedSong,
    markSmartShuffleTracks,
    smartShuffleTrackIds,
    restorePlaybackRequest
  } from '$lib/stores/player';
  import { showQueue } from '$lib/stores/player';
  import { DESKTOP_PLAYBACK_CACHE_UPDATED_EVENT, fetchSimilarSongs, starSong, unstarSong, lfmNowPlaying, lfmScrobble, lfmUserTaste, fetchArtistAlbums, fetchAlbumSongs, castConnect as castConnectCmd, castDiscover, castPlay as castPlayCmd, castPause as castPauseCmd, castResume as castResumeCmd, castStop as castStopCmd, castSetVolume as castSetVolumeCmd, castSeek as castSeekCmd, castGetSession as castGetSessionCmd, castGetStatus as castGetStatusCmd, desktopPlaybackLoad, desktopPlaybackPause, desktopPlaybackPlay, desktopPlaybackPreload, desktopPlaybackSeek, desktopPlaybackSetEq, desktopPlaybackSetVolume, desktopPlaybackStatus, desktopPlaybackStop, desktopPlaybackIsCached, type CastDeviceInfo } from '$lib/servers';
  import { getUpNextSongs } from '$lib/discovery';
  import { fetchLikedArtists, saveVolume } from '$lib/servers';
  import { EQ_FREQUENCIES, normalizeEqBands } from '$lib/audio/equalizer';
  import { formatClockDuration } from '$lib/utils';
  import { lbzNowPlaying, lbzScrobble } from '$lib/providers/recommendation/listenbrainz';
  import { toast } from 'svelte-sonner';
  import { formatSongArtists, primarySongArtist } from '$lib/song-artists';
  import SongArtistLinks from '$lib/components/SongArtistLinks.svelte';
  import { Button, Slider } from '$lib/components/ui';
  import { backendSettings } from '$lib/stores/backend-settings';
  import SongContextMenu from '$lib/components/SongContextMenu.svelte';
  import SongTechBadge from '$lib/components/SongTechBadge.svelte';
  import { goto } from '$app/navigation';
  import { isTauri } from '$lib/tauri';

  // ── Cast state ──────────────────────────────────────────────────────────────
  let castDevices = $state<CastDeviceInfo[]>([]);
  let discovering = $state(false);
  let castActive = $state(false);
  let castPlaying = $state(false);
  let castDevice = $state<CastDeviceInfo | null>(null);
  let castVolume = $state<number | null>(null);
  const desktopPlayback = isTauri();
  let desktopLoadedTrackId = $state<string | null>(null);
  let desktopEndedTrackId = $state<string | null>(null);
  let desktopPreloadedTrackId = $state<string | null>(null);
  let desktopLoadPending = $state(false);
  let currentTrackCached = $state(false);
  let seekVal = $state<number[]>([0]);
  let seekDragging = $state(false);
  let isBuffering = $state(false);
  let volVal = $state<number[]>([$volume * 100]);
  let volDragging = $state(false);

  async function discoverDevices() {
    if (castActive || discovering) return;
    discovering = true;
    castDevices = [];
    try {
      castDevices = await castDiscover();
      if (castDevices.length === 0) toast.info('No Cast devices found on your network');
    } catch (e) {
      toast.error(`Cast discovery failed: ${e}`);
    } finally {
      discovering = false;
    }
  }

  async function startCast(device: CastDeviceInfo) {
    const toastId = toast.loading(`Connecting to ${device.name}...`);
    try {
      if ($isPlaying) {
        pauseLocalAudio();
        if (desktopPlayback) await desktopPlaybackPause().catch(() => undefined);
      }
      castDevice = device;
      castActive = true;
      castPlaying = false;
      if (currentTrack) {
        await castPlayCmd({
          deviceName: device.name,
          deviceAddr: device.addr,
          devicePort: device.port,
          streamUrl: currentTrack.streamUrl,
          title: currentTrack.title,
          artist: currentTrack.artist,
          coverUrl: currentTrack.coverArtUrl ?? '',
        });
        castPlaying = true;
        toast.success(`Casting to ${device.name}`, { id: toastId });
      } else {
        await castConnectCmd({
          deviceName: device.name,
          deviceAddr: device.addr,
          devicePort: device.port,
        });
        const status = await castGetStatusCmd();
        castVolume = status.volumeLevel;
        volVal = [status.volumeLevel * 100];
        _lastPlayerState = status.playerState;
        toast.success(`Connected to ${device.name}`, { id: toastId });
      }
    } catch (e) {
      castActive = false;
      castPlaying = false;
      castVolume = null;
      castDevice = null;
      toast.error(`Cast failed: ${e}`, { id: toastId });
    }
  }

  async function stopCast() {
    const name = castDevice?.name ?? 'device';
    castActive = false;
    castPlaying = false;
    castVolume = null;
    castDevice = null;
    try { await castStopCmd(); } catch {}
    toast.success(`Stopped casting to ${name}`);
  }
  // Poll the Chromecast every second while casting to keep the seek bar in sync
  let _lastPlayerState: string | null = null;
  $effect(() => {
    if (!castActive) return;
    const id = setInterval(async () => {
      if (!castActive || seekDragging) return;
      try {
        const status = await castGetStatusCmd();
        castVolume = status.volumeLevel;
        if (!volDragging) volVal = [status.volumeLevel * 100];
        // Update UI time + playing flag when available
        if (status.playerState === 'PLAYING' || status.playerState === 'PAUSED') {
          currentTime.set(status.currentTime);
          castPlaying = status.playerState === 'PLAYING';
        } else if (status.playerState === 'IDLE') {
          castPlaying = false;
        }

        // If device transitioned to IDLE, advance the queue
        if (_lastPlayerState && _lastPlayerState !== 'IDLE' && status.playerState === 'IDLE') {
          // Use store action to advance; this will set shouldAutoplay and
          // Effect 3 will route playback through Cast when active.
          nextTrack();
        }
        _lastPlayerState = status.playerState;
      } catch {
        // ignore transient errors — next tick will retry
      }
    }, 1000);
    return () => clearInterval(id);
  });

  onMount(() => {
    function handleTogglePlay() { togglePlay(); }
    function handleDesktopCacheUpdated(event: Event) {
      const songId = (event as CustomEvent<{ songId?: string }>).detail?.songId;
      if (!songId || !currentTrack || songId !== currentTrack.id) return;
      currentTrackCached = true;
    }

    window.addEventListener('player:toggle-play', handleTogglePlay);
    window.addEventListener(DESKTOP_PLAYBACK_CACHE_UPDATED_EVENT, handleDesktopCacheUpdated);
    let desktopPoll = 0;

    if (desktopPlayback) {
      castGetSessionCmd()
        .then(async (session) => {
          if (!session) return;
          castDevice = {
            name: session.deviceName,
            addr: session.deviceAddr,
            port: session.devicePort,
          };
          castActive = true;

          try {
            const status = await castGetStatusCmd();
            currentTime.set(status.currentTime);
            castPlaying = status.playerState === 'PLAYING';
            castVolume = status.volumeLevel;
            volVal = [status.volumeLevel * 100];
            _lastPlayerState = status.playerState;
          } catch {
            castActive = false;
            castPlaying = false;
            castVolume = null;
            castDevice = null;
          }
        })
        .catch(() => undefined);
    }

    if (desktopPlayback) {
      desktopPoll = window.setInterval(() => {
        if (castActive) return;
        desktopPlaybackStatus()
          .then((status) => {
            const activeTrackId = currentTrack?.id ?? null;
            if (!activeTrackId) {
              if (status.loaded || status.playing) {
                desktopPlaybackStop().catch(() => undefined);
              }
              desktopLoadedTrackId = null;
              desktopEndedTrackId = null;
              desktopLoadPending = false;
              currentTime.set(0);
              duration.set(0);
              isPlaying.set(false);
              isBuffering = false;
              return;
            }

            if (status.trackId && status.trackId !== activeTrackId) {
              desktopPlaybackStop().catch(() => undefined);
              desktopLoadedTrackId = activeTrackId;
              desktopEndedTrackId = null;
              desktopLoadPending = false;
              currentTime.set(0);
              duration.set(currentTrack?.duration > 0 ? currentTrack.duration : 0);
              isPlaying.set(false);
              isBuffering = false;
              return;
            }

            if (status.trackId) desktopLoadedTrackId = status.trackId;
            if (!seekDragging) currentTime.set(status.position ?? 0);
            if (status.duration > 0) duration.set(status.duration);
            if (desktopLoadPending) {
              if (status.playing) {
                desktopLoadPending = false;
                isPlaying.set(true);
                isBuffering = false;
              } else {
                isPlaying.set(true);
                isBuffering = true;
              }
            } else {
              isPlaying.set(status.playing);
              isBuffering = false;
            }

            if (status.ended && status.trackId && desktopEndedTrackId !== status.trackId) {
              desktopEndedTrackId = status.trackId;
              nextTrack();
            } else if (!status.ended) {
              desktopEndedTrackId = null;
            }
          })
          .catch(() => undefined);
      }, 250);
    }
    return () => {
      window.removeEventListener('player:toggle-play', handleTogglePlay);
      window.removeEventListener(DESKTOP_PLAYBACK_CACHE_UPDATED_EVENT, handleDesktopCacheUpdated);
      if (desktopPoll) clearInterval(desktopPoll);
    };
  });

  const lastFmApiKey = $derived($backendSettings.lastFmApiKey);
  const lastFmConnected = $derived($backendSettings.lastFmConnected);
  const lbzToken = $derived($backendSettings.listenBrainzToken);
  const crossfadeMs = $derived(Math.max(0, Math.round(($backendSettings.crossfadeSeconds ?? 4) * 1000)));
  const eqEnabled = $derived($backendSettings.eqEnabled ?? false);
  const eqBands = $derived(normalizeEqBands($backendSettings.eqBands));
  const zeroEqBands = normalizeEqBands([]);
  const shuffleBtnClass = $derived($smartShuffleMode || $shuffleEnabled ? 'text-primary' : 'text-muted-foreground hover:text-foreground');
  const transportLocked = $derived(desktopPlayback && (desktopLoadPending || isBuffering));
  const showPauseButton = $derived(castActive ? castPlaying : ($isPlaying && !isBuffering && !desktopLoadPending));

  function randomInt(min: number, max: number): number {
    return Math.floor(Math.random() * (max - min + 1)) + min;
  }

  $effect(() => {
    const track = currentTrack;
    if (!desktopPlayback || !track) {
      currentTrackCached = false;
      return;
    }

    let cancelled = false;
    desktopPlaybackIsCached(track)
      .then((cached) => {
        if (!cancelled) currentTrackCached = cached;
      })
      .catch(() => {
        if (!cancelled) currentTrackCached = false;
      });

    return () => {
      cancelled = true;
    };
  });

  function shuffleItems<T>(items: T[]): T[] {
    const next = [...items];
    for (let i = next.length - 1; i > 0; i--) {
      const j = Math.floor(Math.random() * (i + 1));
      [next[i], next[j]] = [next[j], next[i]];
    }
    return next;
  }

  // ── Scrobbling ─────────────────────────────────────────────────────────────
  // Send "now playing" when the track changes, then scrobble once the user
  // has listened for ≥50% of the track (or 240 seconds, per Last.fm spec).
  // Both are fire-and-forget — errors are absorbed so playback is never affected.
  let scrobbledTrackId = '';
  let scrobbleStartTime = 0;

  $effect(() => {
    const track = currentTrack;
    if (!track) return;
    // New track started
    scrobbledTrackId = '';
    scrobbleStartTime = Math.floor(Date.now() / 1000);
    addRecentlyPlayedSong(track);
    if (lastFmConnected) lfmNowPlaying(track.artist, track.title, track.album || undefined, track.duration || undefined);
    if (lbzToken) lbzNowPlaying(lbzToken, track.artist, track.title, track.album || undefined, track.duration || undefined);
  });

  $effect(() => {
    const t = $currentTime;
    const dur = $duration;
    const track = currentTrack;
    if (!track || scrobbledTrackId === track.id) return;
    // Scrobble threshold: 50% or 240 seconds, whichever is less
    const threshold = dur > 0 ? Math.min(dur * 0.5, 240) : 240;
    if (t >= threshold) {
      scrobbledTrackId = track.id;
      if (lastFmConnected) lfmScrobble(track.artist, track.title, scrobbleStartTime, track.album || undefined, track.duration || undefined);
      if (lbzToken) lbzScrobble(lbzToken, track.artist, track.title, scrobbleStartTime, track.album || undefined, track.duration || undefined);
    }
  });

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

  let deckAEl = $state<HTMLAudioElement | null>(null);
  let deckBEl = $state<HTMLAudioElement | null>(null);
  let activeDeck = $state<'a' | 'b'>('a');
  let crossfadeInProgress = false;
  let crossfadeTimer = 0;
  let preloadedIndex = -1;
  let audioContext: AudioContext | null = null;
  let deckASource: MediaElementAudioSourceNode | null = null;
  let deckBSource: MediaElementAudioSourceNode | null = null;
  let eqFilterChains: BiquadFilterNode[][] = [];
  let audioGraphUnavailable = false;

  const currentTrack = $derived($queue[$currentIndex] ?? null);
  const isStarred = $derived(currentTrack ? $starredSongIds.has(currentTrack.id) : false);
  const isSmartShuffleTrack = $derived(currentTrack ? $smartShuffleTrackIds.has(currentTrack.id) : false);

  function getActiveAudio(): HTMLAudioElement | null {
    return activeDeck === 'a' ? deckAEl : deckBEl;
  }

  function getInactiveAudio(): HTMLAudioElement | null {
    return activeDeck === 'a' ? deckBEl : deckAEl;
  }

  function syncLibraryFocus(track: typeof currentTrack): void {
    if (!track) return;
    focusTrack.set({
      title: track.title,
      artist: track.artist,
      imageUrl: track.coverArtUrl,
      source: 'library',
      album: track.album
    });
  }

  function stopCrossfadeTimer(): void {
    if (!crossfadeTimer) return;
    clearInterval(crossfadeTimer);
    crossfadeTimer = 0;
  }

  function pauseLocalAudio(): void {
    deckAEl?.pause();
    deckBEl?.pause();
  }

  function shouldPreloadUpcomingTrack(positionSeconds: number, durationSeconds: number): boolean {
    if (durationSeconds <= 0) return false;

    // Preload no earlier than 80% played unless the configured crossfade window
    // would otherwise leave too little time to fetch and prepare the next track.
    const preloadLeadSeconds = Math.max(($backendSettings.crossfadeSeconds ?? 4) + 2, 4);
    const percentThreshold = durationSeconds * 0.8;
    const timeThreshold = Math.max(0, durationSeconds - preloadLeadSeconds);
    const triggerAt = Math.min(percentThreshold, timeThreshold);

    return positionSeconds >= triggerAt;
  }

  $effect(() => {
    if (desktopPlayback) return;
    if (!eqFilterChains.length) return;
    const gains = eqEnabled ? eqBands : zeroEqBands;
    eqFilterChains.forEach((filters) => {
      filters.forEach((filter, index) => {
        filter.gain.value = gains[index];
      });
    });
  });

  $effect(() => {
    if (desktopPlayback) return;
    if (!eqEnabled || audioGraphUnavailable) return;
    if (!deckAEl || !deckBEl) return;
    resumeAudioContext().catch(() => undefined);
  });

  $effect(() => {
    if (!desktopPlayback || castActive) return;
    desktopPlaybackSetEq(eqEnabled, eqBands).catch(() => undefined);
  });

  function createEqFilterChain(): BiquadFilterNode[] {
    return EQ_FREQUENCIES.map((frequency, index) => {
      const filter = audioContext!.createBiquadFilter();
      filter.type = index === 0 ? 'lowshelf' : index === EQ_FREQUENCIES.length - 1 ? 'highshelf' : 'peaking';
      filter.frequency.value = frequency;
      filter.Q.value = 1;
      filter.gain.value = 0;
      return filter;
    });
  }

  function ensureAudioGraph(): boolean {
    if (typeof window === 'undefined' || audioGraphUnavailable || deckASource || !audioContext) return Boolean(deckASource);
    if (!deckAEl || !deckBEl) return false;

    try {
      const sourceA = audioContext.createMediaElementSource(deckAEl);
      const sourceB = audioContext.createMediaElementSource(deckBEl);
      const filtersA = createEqFilterChain();
      const filtersB = createEqFilterChain();

      for (const filters of [filtersA, filtersB]) {
        for (let i = 0; i < filters.length - 1; i++) {
          filters[i].connect(filters[i + 1]);
        }
        filters[filters.length - 1].connect(audioContext.destination);
      }

      sourceA.connect(filtersA[0]);
      sourceB.connect(filtersB[0]);

      deckASource = sourceA;
      deckBSource = sourceB;
      eqFilterChains = [filtersA, filtersB];
      return true;
    } catch (error) {
      audioGraphUnavailable = true;
      console.error('Failed to initialize EQ audio graph', error);
      toast.error('Equalizer could not be initialized. Playback will continue without EQ.');
      return false;
    }
  }

  async function resumeAudioContext(): Promise<void> {
    if (!eqEnabled && !audioContext) return;
    if (audioGraphUnavailable) return;

    if (!audioContext) {
      try {
        audioContext = new AudioContext();
      } catch (error) {
        audioGraphUnavailable = true;
        console.error('Failed to create EQ audio context', error);
        toast.error('Equalizer could not be initialized. Playback will continue without EQ.');
        return;
      }
    }

    if (audioContext.state !== 'running') {
      await audioContext.resume().catch(() => undefined);
    }

    if (audioContext.state !== 'running') {
      if (!deckASource) {
        await audioContext.close().catch(() => undefined);
        audioContext = null;
      }
      return;
    }

    ensureAudioGraph();
  }

  // Local state for the seek slider — synced from $currentTime when not dragging
  $effect(() => {
    if (!seekDragging) seekVal = [$currentTime];
  });

  // Local state for the volume slider
  $effect(() => {
    if (!volDragging) volVal = [$volume * 100];
  });

  // Effect 1: load the active track into the active deck
  $effect(() => {
    const track = currentTrack;
    if (desktopPlayback) {
      if (castActive) return;
      if (!track?.streamUrl) {
        desktopPlaybackStop().catch(() => undefined);
        desktopLoadedTrackId = null;
        desktopEndedTrackId = null;
        desktopPreloadedTrackId = null;
        desktopLoadPending = false;
        isBuffering = false;
        isPlaying.set(false);
        currentTime.set(0);
        duration.set(0);
        return;
      }
      if (desktopLoadedTrackId === track.id) return;
      desktopLoadedTrackId = track.id;
      currentTime.set(0);
      duration.set(track.duration > 0 ? track.duration : 0);
      const autoplay = $shouldAutoplay;
      isBuffering = true;
      desktopLoadPending = autoplay;
      isPlaying.set(autoplay);
      if (autoplay) shouldAutoplay.set(false);
      desktopPlaybackLoad(track, autoplay)
        .then(() => {
          if (!autoplay) {
            desktopLoadPending = false;
            isBuffering = false;
            isPlaying.set(false);
          }
        })
        .catch(() => {
          desktopLoadedTrackId = null;
          desktopLoadPending = false;
          isBuffering = false;
          isPlaying.set(false);
          toast.error('Desktop playback failed to load the track');
        });
      return;
    }
    const activeAudio = getActiveAudio();
    if (!activeAudio || !track?.streamUrl || crossfadeInProgress) return;
    if (activeAudio.src === track.streamUrl) return;
    activeAudio.src = track.streamUrl;
    activeAudio.preload = 'auto';
    currentTime.set(0);
    duration.set(track.duration > 0 ? track.duration : 0);
  });

  // Effect 2: sync volume to the local decks when not crossfading
  $effect(() => {
    if (desktopPlayback) {
      if (!castActive) desktopPlaybackSetVolume($volume).catch(() => undefined);
      return;
    }
    if (crossfadeInProgress) return;
    const activeAudio = getActiveAudio();
    const inactiveAudio = getInactiveAudio();
    if (activeAudio) activeAudio.volume = $volume;
    if (inactiveAudio && inactiveAudio.paused) inactiveAudio.volume = 0;
  });

  $effect(() => {
    const restore = $restorePlaybackRequest;
    const track = currentTrack;
    if (!restore || !track || restore.songId !== track.id) return;

    if (desktopPlayback) {
      if (castActive || desktopLoadedTrackId !== track.id || desktopLoadPending) return;
      const position = restore.position;
      restorePlaybackRequest.set(null);
      applyRestoredPlaybackPosition(position);
      return;
    }

    const activeAudio = getActiveAudio();
    if (!activeAudio || !track.streamUrl || activeAudio.src !== track.streamUrl) return;

    const position = restore.position;
    const finishRestore = () => {
      restorePlaybackRequest.set(null);
      applyRestoredPlaybackPosition(position);
    };

    if (activeAudio.readyState > 0) {
      finishRestore();
      return;
    }

    const handleLoadedMetadata = () => {
      activeAudio.removeEventListener('loadedmetadata', handleLoadedMetadata);
      finishRestore();
    };

    activeAudio.addEventListener('loadedmetadata', handleLoadedMetadata, { once: true });
    return () => {
      activeAudio.removeEventListener('loadedmetadata', handleLoadedMetadata);
    };
  });

  // Effect 3: preload the upcoming song into the inactive deck
  $effect(() => {
    const next = $queue[$currentIndex + 1];
    const trackDuration = currentTrack?.duration && currentTrack.duration > 0 ? currentTrack.duration : $duration;
    const readyToPreload = shouldPreloadUpcomingTrack($currentTime, trackDuration);

    if (desktopPlayback) {
      if (castActive) return;
      if (!next?.streamUrl) {
        desktopPreloadedTrackId = null;
        return;
      }
      if (!readyToPreload) return;
      if (desktopPreloadedTrackId === next.id) return;
      desktopPreloadedTrackId = next.id;
      desktopPlaybackPreload(next).catch(() => {
        if (desktopPreloadedTrackId === next.id) desktopPreloadedTrackId = null;
      });
      return;
    }
    const inactiveAudio = getInactiveAudio();
    if (!inactiveAudio || crossfadeInProgress || castActive) return;
    if (!next?.streamUrl) {
      preloadedIndex = -1;
      return;
    }
    if (!readyToPreload) return;
    if (preloadedIndex === $currentIndex + 1 && inactiveAudio.src === next.streamUrl) return;
    inactiveAudio.src = next.streamUrl;
    inactiveAudio.preload = 'auto';
    inactiveAudio.load();
    inactiveAudio.volume = 0;
    preloadedIndex = $currentIndex + 1;
  });

  async function startCrossfade(nextIndex: number): Promise<void> {
    if (crossfadeInProgress || castActive) return;
    const next = $queue[nextIndex];
    const fromAudio = getActiveAudio();
    const toAudio = getInactiveAudio();
    if (!next || !fromAudio || !toAudio) return;

    if (toAudio.src !== next.streamUrl) {
      toAudio.src = next.streamUrl;
      toAudio.preload = 'auto';
      toAudio.load();
    }

    try {
      await resumeAudioContext();
      toAudio.currentTime = 0;
      toAudio.volume = 0;
      await toAudio.play();
    } catch {
      return;
    }

    crossfadeInProgress = true;
    preloadedIndex = -1;
    const oldAudio = fromAudio;
    const oldDeck = activeDeck;
    activeDeck = activeDeck === 'a' ? 'b' : 'a';
    currentIndex.set(nextIndex);
    syncLibraryFocus(next);
    currentTime.set(0);
    duration.set(next.duration > 0 ? next.duration : 0);

    const fadeDuration = Math.max(0, crossfadeMs);
    if (fadeDuration === 0) {
      stopCrossfadeTimer();
      oldAudio.pause();
      oldAudio.currentTime = 0;
      oldAudio.volume = 0;
      toAudio.volume = $volume;
      if (oldDeck === 'a') deckAEl?.removeAttribute('src');
      else deckBEl?.removeAttribute('src');
      crossfadeInProgress = false;
      return;
    }

    const startedAt = performance.now();
    stopCrossfadeTimer();
    crossfadeTimer = window.setInterval(() => {
      const progress = Math.min(1, (performance.now() - startedAt) / fadeDuration);
      oldAudio.volume = Math.max(0, (1 - progress) * $volume);
      toAudio.volume = Math.min($volume, progress * $volume);

      if (progress >= 1) {
        stopCrossfadeTimer();
        oldAudio.pause();
        oldAudio.currentTime = 0;
        oldAudio.volume = 0;
        if (oldDeck === 'a') deckAEl?.removeAttribute('src');
        else deckBEl?.removeAttribute('src');
        crossfadeInProgress = false;
      }
    }, 50);
  }

  // Effect 4: start a real overlap fade near the end of the current track
  $effect(() => {
    if (desktopPlayback) return;
    const next = $queue[$currentIndex + 1];
    const repeat = $repeatMode;
    if (castActive || crossfadeInProgress || !next || repeat === 'one' || !$isPlaying) return;

    const remaining = $duration - $currentTime;
    if (crossfadeMs <= 0 || remaining <= 0 || remaining > crossfadeMs / 1000) return;
    untrack(() => { startCrossfade($currentIndex + 1); });
  });

  // Effect 5: trigger autoplay (routes through Cast when active)
  $effect(() => {
    if (!$shouldAutoplay) return;

    if (castActive && castDevice) {
      shouldAutoplay.set(false);
      const track = currentTrack;
      if (track) {
        castPlaying = true;
        castPlayCmd({
          deviceName: castDevice.name,
          deviceAddr: castDevice.addr,
          devicePort: castDevice.port,
          streamUrl: track.streamUrl,
          title: track.title,
          artist: track.artist,
          coverUrl: track.coverArtUrl ?? '',
        }).catch((e) => {
          toast.error(`Cast update failed: ${e}`);
          castPlaying = false;
        });
      }
      return;
    }

    if (desktopPlayback) {
      const track = currentTrack;
      if (!track) return;
      if (desktopLoadPending || isBuffering) return;
      if (desktopLoadedTrackId === track.id) {
        shouldAutoplay.set(false);
        desktopLoadPending = true;
        isBuffering = true;
        desktopPlaybackSeek(0)
          .then(() => desktopPlaybackPlay())
          .then(() => {
            isPlaying.set(true);
            isBuffering = false;
            desktopLoadPending = false;
          })
          .catch(() => {
            isPlaying.set(false);
            isBuffering = false;
            desktopLoadPending = false;
          });
      }
      return;
    }

    const activeAudio = getActiveAudio();
    if (!activeAudio) return;
    shouldAutoplay.set(false);
    resumeAudioContext()
      .then(() => activeAudio.play())
      .then(() => isPlaying.set(true))
      .catch(() => isPlaying.set(false));
  });

  // Effect 6: external toggle-play requests
  $effect(() => {
    if ($togglePlayRequest === 0) return;
    untrack(() => togglePlay());
  });
  function activateShuffle() {
    smartShuffleMode.set(false);
    enableShuffle();
  }

  function activateSmartShuffle() {
    enableSmartShuffle();
  }

  function deactivateShuffle() {
    disableShuffle();
  }

  async function shuffleArtist() {
    if (!currentTrack) return;
    const artist = primarySongArtist(currentTrack.artist);
    const toastId = toast.loading(`Loading ${artist}…`);
    try {
      const albums = await fetchArtistAlbums(artist, 50);
      const allSongs = (await Promise.all(albums.map(a => fetchAlbumSongs(a.id)))).flat();
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
      const songs = await fetchAlbumSongs(currentTrack.albumId);
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
  const SMART_SHUFFLE_MIN_GAP = 2;
  const SMART_SHUFFLE_MAX_GAP = 5;
  const SMART_SHUFFLE_MIN_FETCH = 3;
  const SMART_SHUFFLE_MAX_FETCH = 6;
  const SMART_SHUFFLE_MAX_INSERT = 4;
  const SMART_SHUFFLE_MIN_OFFSET = 1;
  const SMART_SHUFFLE_MAX_OFFSET = 3;
  const SMART_SHUFFLE_MIN_SPACING = 1;
  const SMART_SHUFFLE_MAX_SPACING = 3;
  let smartShufflePlayCount = 0;
  let smartShuffleLastIdx = -1;
  let smartShuffleInflight = false;
  let smartShuffleFetching = $state(false);
  let smartShuffleNextInjectAfter = randomInt(SMART_SHUFFLE_MIN_GAP, SMART_SHUFFLE_MAX_GAP);

  $effect(() => {
    const track = $queue[$currentIndex];
    const idx = $currentIndex;

    if (!$smartShuffleMode) {
      // Reset counters when Smart Shuffle is toggled off
      smartShufflePlayCount = 0;
      smartShuffleLastIdx = -1;
      smartShuffleNextInjectAfter = randomInt(SMART_SHUFFLE_MIN_GAP, SMART_SHUFFLE_MAX_GAP);
      return;
    }
    if (!track) return;

    // Only count actual track advances, not queue mutations
    if (idx === smartShuffleLastIdx) return;
    smartShuffleLastIdx = idx;
    smartShufflePlayCount++;

    // Prune played history so old songs don't pile up at the bottom
    pruneQueueHistory(1);

    if (smartShufflePlayCount < smartShuffleNextInjectAfter || smartShuffleInflight) return;

    smartShufflePlayCount = 0;
    smartShuffleNextInjectAfter = randomInt(SMART_SHUFFLE_MIN_GAP, SMART_SHUFFLE_MAX_GAP);
    smartShuffleInflight = true;
    smartShuffleFetching = true;

    // Capture current state before the async gap
    const existingIds = new Set($queue.map(s => s.id));
    const capturedIdx = idx;
    const { artist, title, id } = track;

    const fetchLimit = randomInt(SMART_SHUFFLE_MIN_FETCH, SMART_SHUFFLE_MAX_FETCH);
    const doFetch: Promise<import('$lib/servers').Song[]> = lastFmApiKey
      ? Promise.all([
          fetchLikedArtists().then(stored => stored.map(a => a.name)).catch((): string[] => []),
          lfmUserTaste().catch((): string[] => [])
        ]).then(([liked, taste]) => {
          // Merge liked artists + Last.fm top artists, deduplicated
          const merged = [...new Set([...liked, ...taste])];
          return getUpNextSongs({ artist, title, likedArtists: merged, limit: fetchLimit });
        })
      : fetchSimilarSongs(id, fetchLimit);

    doFetch
      .then(songs => {
        const freshPool = shuffleItems(songs.filter(s => !existingIds.has(s.id)));
        const fresh = freshPool.slice(0, randomInt(1, Math.min(SMART_SHUFFLE_MAX_INSERT, freshPool.length)));
        if (!fresh.length) return;
        markSmartShuffleTracks(fresh);
        queue.update(items => {
          const next = [...items];
          let insertAt = Math.min(capturedIdx + randomInt(SMART_SHUFFLE_MIN_OFFSET, SMART_SHUFFLE_MAX_OFFSET), next.length);
          fresh.forEach((song) => {
            next.splice(insertAt, 0, song);
            insertAt = Math.min(insertAt + randomInt(SMART_SHUFFLE_MIN_SPACING, SMART_SHUFFLE_MAX_SPACING), next.length);
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
        getUpNextSongs({
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
    if (transportLocked) return;
    if (castActive) {
      if (castPlaying) {
        castPlaying = false;
        castPauseCmd().catch((e) => { castPlaying = true; toast.error(`Cast pause failed: ${e}`); });
      } else if (currentTrack && (!_lastPlayerState || _lastPlayerState === 'IDLE')) {
        castPlaying = true;
        castPlayCmd({
          deviceName: castDevice?.name ?? 'Chromecast',
          deviceAddr: castDevice?.addr ?? '',
          devicePort: castDevice?.port ?? 0,
          streamUrl: currentTrack.streamUrl,
          title: currentTrack.title,
          artist: currentTrack.artist,
          coverUrl: currentTrack.coverArtUrl ?? '',
        }).catch((e) => {
          castPlaying = false;
          toast.error(`Cast play failed: ${e}`);
        });
      } else {
        castPlaying = true;
        castResumeCmd().catch((e) => { castPlaying = false; toast.error(`Cast resume failed: ${e}`); });
      }
      return;
    }
    if (!currentTrack) return;
    if (desktopPlayback) {
      if ($isPlaying) {
        desktopPlaybackPause().then(() => isPlaying.set(false)).catch(() => undefined);
      } else {
        desktopPlaybackPlay().then(() => isPlaying.set(true)).catch(() => {
          isPlaying.set(false);
        });
      }
      return;
    }
    const activeAudio = getActiveAudio();
    if (!activeAudio) return;
    if ($isPlaying) {
      pauseLocalAudio();
    } else {
      resumeAudioContext().then(() => activeAudio.play()).catch(() => {
        isPlaying.set(false);
      });
    }
  }

  let volSaveTimer = 0;
  function debounceSaveVolume(value: number) {
    clearTimeout(volSaveTimer);
    volSaveTimer = window.setTimeout(() => saveVolume(value), 500);
  }

  function onVolumeWheel(e: WheelEvent) {
    e.preventDefault();
    const delta = e.deltaY < 0 ? 0.05 : -0.05;
    const base = castActive ? (castVolume ?? $volume) : $volume;
    const next = Math.max(0, Math.min(1, base + delta));
    volVal = [next * 100];

    if (castActive) {
      castVolume = next;
      castSetVolumeCmd(next).catch(() => {});
      return;
    }

    volume.set(next);
    if (desktopPlayback) {
      desktopPlaybackSetVolume(next).catch(() => undefined);
    } else if (!crossfadeInProgress) {
      const activeAudio = getActiveAudio();
      if (activeAudio) activeAudio.volume = next;
    }
    debounceSaveVolume(next);
  }

  function seek(values: number[]) {
    const value = Math.max(0, Number(values[0] ?? 0));
    currentTime.set(value);
    seekVal = [value];
    seekDragging = false;
    if (castActive) {
      castSeekCmd(value).catch((e) => toast.error(`Cast seek failed: ${e}`));
    } else if (desktopPlayback) {
      desktopPlaybackSeek(value).catch(() => undefined);
    } else {
      const activeAudio = getActiveAudio();
      if (activeAudio) activeAudio.currentTime = value;
    }
  }

  function changeVolume(values: number[]) {
    const value = Math.max(0, Math.min(1, Number(values[0] ?? 0) / 100));

    // Avoid redundant writes that can cause effect update loops
    const EPS = 0.0001;
    if (castActive) {
      if (Math.abs((castVolume ?? 0) - value) < EPS) return;
      castVolume = value;
      castSetVolumeCmd(value).catch(() => {});
      return;
    }

    if (Math.abs($volume - value) < EPS) return;
    volume.set(value);

    if (desktopPlayback) {
      desktopPlaybackSetVolume(value).catch(() => undefined);
    } else if (!crossfadeInProgress) {
      const activeAudio = getActiveAudio();
      if (activeAudio) activeAudio.volume = value;
    }
  }

  function commitVolume(values: number[]) {
    changeVolume(values);
    volDragging = false;
    if (castActive) return;
    saveVolume(Math.max(0, Math.min(1, Number(values[0] ?? 0) / 100)));
  }

  let premuteVolume = $state(0.8);
  function toggleMute() {
    const activeVolume = castActive ? (castVolume ?? $volume) : $volume;
    if (activeVolume <= 0.01) {
      const restore = premuteVolume > 0.01 ? premuteVolume : 0.8;
      volVal = [restore * 100];
      if (castActive) {
        castVolume = restore;
        castSetVolumeCmd(restore).catch(() => {});
        return;
      }

      volume.set(restore);
      if (desktopPlayback) {
        desktopPlaybackSetVolume(restore).catch(() => undefined);
      } else if (!crossfadeInProgress) {
        const activeAudio = getActiveAudio();
        if (activeAudio) activeAudio.volume = restore;
      }
      saveVolume(restore);
    } else {
      premuteVolume = activeVolume;
      volVal = [0];
      if (castActive) {
        castVolume = 0;
        castSetVolumeCmd(0).catch(() => {});
        return;
      }

      volume.set(0);
      if (desktopPlayback) {
        desktopPlaybackSetVolume(0).catch(() => undefined);
      } else if (!crossfadeInProgress) {
        const activeAudio = getActiveAudio();
        if (activeAudio) activeAudio.volume = 0;
      }
    }
  }

  // Handle seek requests from the lyrics panel
  $effect(() => {
    const t = $seekRequest;
    if (t === null) return;
    if (desktopPlayback) {
      desktopPlaybackSeek(t).catch(() => undefined);
    } else {
      const activeAudio = getActiveAudio();
      if (!activeAudio) return;
      activeAudio.currentTime = t;
    }
    currentTime.set(t);
    seekRequest.set(null);
  });

  function handleAudioPlay(deck: 'a' | 'b') {
    if (deck === activeDeck) isPlaying.set(true);
  }

  function handleAudioPause() {
    if (deckAEl?.paused !== false && deckBEl?.paused !== false) isPlaying.set(false);
  }

  function handleAudioWaiting(deck: 'a' | 'b') {
    if (deck === activeDeck) isBuffering = true;
  }

  function handleAudioCanPlay(deck: 'a' | 'b') {
    if (deck === activeDeck) isBuffering = false;
  }

  function handleAudioTimeUpdate(deck: 'a' | 'b') {
    if (deck !== activeDeck) return;
    const audio = deck === 'a' ? deckAEl : deckBEl;
    currentTime.set(audio?.currentTime ?? 0);
  }

  function handleAudioDuration(deck: 'a' | 'b') {
    if (deck !== activeDeck) return;
    const audio = deck === 'a' ? deckAEl : deckBEl;
    const d = audio?.duration ?? NaN;
    if (isFinite(d) && d > 0) duration.set(d);
  }

  function handleAudioEnded(deck: 'a' | 'b') {
    if (crossfadeInProgress || deck !== activeDeck) return;
    nextTrack();
  }

  function fmt(seconds: number): string {
    return formatClockDuration(seconds);
  }

  function applyRestoredPlaybackPosition(target: number): void {
    const value = Math.max(0, target);
    if (desktopPlayback) {
      desktopPlaybackSeek(value).catch(() => undefined);
    } else {
      const activeAudio = getActiveAudio();
      if (activeAudio) activeAudio.currentTime = value;
    }
    currentTime.set(value);
  }
</script>

<footer class="player-bar app-shell-footer shrink-0 border-t border-border/40 px-4 py-3 {isBuffering ? 'player-bar-loading' : ''}">
  <div class="grid w-full items-center gap-3 md:grid-cols-3" style="grid-template-columns: 1fr minmax(420px, 2fr) 1fr">
    <!-- Track info -->
    <div class="player-track-info flex min-w-0 items-center gap-3">
      {#if currentTrack}
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
          <div class="flex items-center gap-1.5">
            <div class="min-w-0 flex items-center gap-1.5">
              <button
                class="player-track-title block max-w-full whitespace-normal break-words text-left text-sm font-semibold leading-tight"
                onclick={() => currentTrack?.albumId && goto(`/album/${encodeURIComponent(currentTrack.albumId)}`)}
                title={currentTrack.album}
              >{currentTrack.title}</button>
              <SongTechBadge
                id={currentTrack.id}
                cached={desktopPlayback ? currentTrackCached : null}
                audioFormat={currentTrack.audioFormat}
                bitrateKbps={currentTrack.bitrateKbps}
                compact
              />
              {#if isSmartShuffleTrack}
                <span class="shrink-0 rounded-full border border-primary/30 bg-primary/10 px-1.5 py-0.5 text-[10px] font-semibold uppercase tracking-wide text-primary">
                  Smart Shuffle
                </span>
              {/if}
            </div>
            <button
              onclick={toggleFavorite}
              class="player-favorite-button shrink-0 text-muted-foreground {isStarred ? '!text-rose-500' : ''}"
              aria-label={isStarred ? 'Remove from favorites' : 'Add to favorites'}
              title={isStarred ? 'Remove from favorites' : 'Add to favorites'}
            >
              <Heart class="size-3.5 {isStarred ? 'fill-rose-500' : ''}" />
            </button>
          </div>
            <SongArtistLinks
              artist={currentTrack.artist}
              class="block truncate text-xs text-muted-foreground max-w-full text-left"
              linkClass="hover:underline cursor-pointer"
            />
        </div>
      {:else}
        <p class="text-sm text-muted-foreground">No track selected</p>
      {/if}
    </div>

    <!-- Playback controls -->
    <div class="player-center-column flex flex-col items-center gap-1.5">
      <div class="player-transport flex items-center gap-1">
        <!-- Shuffle button -->
        <DropdownMenu>
          <DropdownMenuTrigger>
            {#snippet child({ props })}
              <Button
                {...props}
                variant="ghost"
                size="icon-sm"
                class={`player-transport-button ${shuffleBtnClass}`}
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
                  <p class="truncate max-w-36 text-xs text-muted-foreground">{formatSongArtists(currentTrack.artist)}</p>
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
          class="player-transport-button text-muted-foreground hover:text-foreground"
          onclick={prevTrack}
          aria-label="Previous track"
        >
          <SkipBack class="size-[18px]" />
        </Button>
        <Button
          class="player-play-button rounded-full shadow-sm {showPauseButton ? 'is-playing' : ''} {isBuffering ? 'is-buffering' : ''}"
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
        <Button
          variant="ghost"
          size="icon-sm"
          class="player-transport-button text-muted-foreground hover:text-foreground"
          onclick={nextTrack}
          aria-label="Next track"
        >
          <SkipForward class="size-[18px]" />
        </Button>
        <Button
          variant="ghost"
          size="icon-sm"
          class={`player-transport-button ${$repeatMode !== 'off' ? 'text-primary' : 'text-muted-foreground hover:text-foreground'}`}
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
      <div class="player-progress-row flex w-full items-center gap-2">
        <span class="player-time-label w-10 text-right text-[11px] tabular-nums text-muted-foreground">{fmt($currentTime)}</span>
        <div class="player-progress-shell relative flex-1 {isBuffering ? 'opacity-75 is-buffering' : ''}">
          {#if isBuffering}
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
            onValueChange={(v) => { /* value is bound via bind:value; no-op to avoid feedback loop */ }}
            onValueCommit={(v) => { seek(v); }}
            aria-label="Playback position"
          />
        </div>
        <span class="player-time-label w-10 text-[11px] tabular-nums text-muted-foreground">{fmt($duration)}</span>
      </div>
    </div>

    <!-- Volume -->
    <div class="player-actions flex items-center justify-end gap-2">
      <!-- Cast -->
      <DropdownMenu onOpenChange={(open) => { if (open && !castActive && !discovering) discoverDevices(); }}>
        <DropdownMenuTrigger>
          {#snippet child({ props })}
            <Button
              {...props}
              variant="ghost"
              size="icon-sm"
              class={`player-transport-button ${castActive ? 'text-primary is-active' : 'text-muted-foreground hover:text-foreground'}`}
              aria-label="Cast"
            >
              <Cast class="size-[18px] {castActive ? 'player-cast-icon' : ''}" />
            </Button>
          {/snippet}
        </DropdownMenuTrigger>
        <DropdownMenuContent side="top" align="end" class="min-w-48">
          {#if castActive && castDevice}
            <div class="px-2 py-1.5">
              <p class="text-[11px] uppercase tracking-wider text-muted-foreground">Casting to</p>
              <p class="text-sm font-semibold">{castDevice.name}</p>
            </div>
            <DropdownMenuSeparator />
            <DropdownMenuItem onclick={stopCast} class="text-destructive focus:text-destructive">
              Stop casting
            </DropdownMenuItem>
          {:else if discovering}
            <div class="flex items-center gap-2 px-3 py-2 text-sm text-muted-foreground">
              <span class="block size-3 animate-spin rounded-full border-2 border-current border-t-transparent"></span>
              Discovering devices…
            </div>
          {:else if castDevices.length === 0}
            <div class="px-3 py-2 text-sm text-muted-foreground">No Cast devices found</div>
            <DropdownMenuSeparator />
            <DropdownMenuItem onclick={discoverDevices}>Scan again</DropdownMenuItem>
          {:else}
            <div class="px-2 py-1 text-[11px] uppercase tracking-wider text-muted-foreground">Cast to device</div>
            {#each castDevices as device (device.addr)}
              <DropdownMenuItem onclick={() => startCast(device)} disabled={!currentTrack} class="gap-2">
                <Cast class="size-4 shrink-0" />
                {device.name}
              </DropdownMenuItem>
            {/each}
            <DropdownMenuSeparator />
            <DropdownMenuItem onclick={discoverDevices}>Scan again</DropdownMenuItem>
          {/if}
        </DropdownMenuContent>
      </DropdownMenu>
      <Button
        variant="ghost"
        size="icon-sm"
        class={`player-transport-button ${$showLyrics ? 'text-primary is-active' : 'text-muted-foreground hover:text-foreground'}`}
        onclick={() => showLyrics.update((v) => !v)}
        aria-label="Lyrics"
      >
        <Mic2 class="size-[18px]" />
      </Button>
      <div
        class="player-volume-group flex w-full items-center gap-2 md:max-w-44"
        onwheel={onVolumeWheel}
        role="group"
        aria-label="Volume"
      >
        {#if (castActive ? (castVolume ?? $volume) : $volume) <= 0.01}
          <button onclick={toggleMute} aria-label="Unmute" class="player-volume-button shrink-0 text-muted-foreground hover:text-foreground">
            <VolumeX class="size-[18px]" />
          </button>
        {:else}
          <button onclick={toggleMute} aria-label="Mute" class="player-volume-button shrink-0 text-muted-foreground hover:text-foreground">
            <Volume2 class="size-[18px]" />
          </button>
        {/if}
        <div class="player-volume-shell">
         <Slider
          class="player-volume-slider"
          type="multiple"
          bind:value={volVal}
          min={0}
          max={100}
          step={1}
          onpointerdown={() => { volDragging = true; }}
          onValueChange={(v) => { changeVolume(v); }}
          onValueCommit={commitVolume}
          aria-label="Volume"
        />
        </div>
      </div>
    </div>
  </div>

  <audio
    bind:this={deckAEl}
    crossorigin="anonymous"
    preload="auto"
    onplay={() => handleAudioPlay('a')}
    onpause={handleAudioPause}
    onwaiting={() => { handleAudioWaiting('a'); }}
    onseeking={() => { handleAudioWaiting('a'); }}
    onseeked={() => { handleAudioCanPlay('a'); }}
    oncanplay={() => { handleAudioCanPlay('a'); }}
    oncanplaythrough={() => { handleAudioCanPlay('a'); }}
    ontimeupdate={() => { handleAudioTimeUpdate('a'); }}
    onloadedmetadata={() => { handleAudioDuration('a'); }}
    ondurationchange={() => { handleAudioDuration('a'); }}
    onended={() => { handleAudioEnded('a'); }}
  ></audio>
  <audio
    bind:this={deckBEl}
    crossorigin="anonymous"
    preload="auto"
    onplay={() => handleAudioPlay('b')}
    onpause={handleAudioPause}
    onwaiting={() => { handleAudioWaiting('b'); }}
    onseeking={() => { handleAudioWaiting('b'); }}
    onseeked={() => { handleAudioCanPlay('b'); }}
    oncanplay={() => { handleAudioCanPlay('b'); }}
    oncanplaythrough={() => { handleAudioCanPlay('b'); }}
    ontimeupdate={() => { handleAudioTimeUpdate('b'); }}
    onloadedmetadata={() => { handleAudioDuration('b'); }}
    ondurationchange={() => { handleAudioDuration('b'); }}
    onended={() => { handleAudioEnded('b'); }}
  ></audio>
</footer>

<style>
  .player-bar {
    position: relative;
    overflow: hidden;
    transition:
      opacity 180ms ease,
      background-color 220ms ease,
      border-color 220ms ease;
  }

  .player-bar::before {
    content: '';
    position: absolute;
    inset: 0;
    pointer-events: none;
    background:
      linear-gradient(180deg, hsl(var(--foreground) / 0.035), transparent 42%),
      radial-gradient(circle at 50% 0%, hsl(var(--primary) / 0.12), transparent 48%);
    opacity: 0.7;
    transition: opacity 220ms ease;
  }

  .player-bar-loading {
    opacity: 0.9;
  }

  .player-bar-loading::before {
    opacity: 1;
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

  .player-favorite-button,
  .player-volume-button {
    transition:
      transform 180ms ease,
      color 180ms ease,
      opacity 180ms ease;
  }

  .player-favorite-button:hover,
  .player-volume-button:hover {
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

  .player-volume-group {
    padding-left: 0.25rem;
  }

  .player-volume-shell {
    position: relative;
    flex: 1 1 auto;
    min-height: 1rem;
    display: flex;
    align-items: center;
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
