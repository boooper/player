import { fromStore } from 'svelte/store';
import {
  currentTime,
  duration,
  isPlaying,
  shouldAutoplay,
  nextTrack,
  volume,
  queue,
  currentIndex,
  restorePlaybackRequest,
} from '$lib/stores/player';
import {
  DESKTOP_PLAYBACK_CACHE_UPDATED_EVENT,
  desktopPlaybackLoad,
  desktopPlaybackPause,
  desktopPlaybackPlay,
  desktopPlaybackPreload,
  desktopPlaybackSeek,
  desktopPlaybackSetLoudnessCompensation,
  desktopPlaybackSetNormalization,
  desktopPlaybackSetNormalizationMode,
  desktopPlaybackSetVolume,
  desktopPlaybackStatus,
  desktopPlaybackStop,
  desktopPlaybackIsCached
} from '$lib/servers';
import { toast } from 'svelte-sonner';

type SongLike = {
  id: string;
  streamUrl?: string | null;
  duration?: number;
  album?: string | null;
};

type PlayerDesktopControllerOptions = {
  getCurrentTrack: () => SongLike | null;
  getCastActive: () => boolean;
  getSeekDragging: () => boolean;
  getIsBuffering: () => boolean;
  getCrossfadeSeconds: () => number;
  getGaplessEnabled: () => boolean;
  getNormalizationEnabled: () => boolean;
  getNormalizationMode: () => 'lufs' | 'rms';
  getLoudnessCompensationEnabled: () => boolean;
  getSmartCrossfadeEnabled: () => boolean;
  setIsBuffering: (v: boolean) => void;
  onRestoreCastSession: () => Promise<void>;
};

const shouldAutoplayRef = fromStore(shouldAutoplay);
const isPlayingRef = fromStore(isPlaying);
const volumeRef = fromStore(volume);
const queueRef = fromStore(queue);
const currentIndexRef = fromStore(currentIndex);
const currentTimeRef = fromStore(currentTime);
const durationRef = fromStore(duration);
const restoreRef = fromStore(restorePlaybackRequest);

export function createPlayerDesktopController(options: PlayerDesktopControllerOptions) {
  let loadedTrackId = $state<string | null>(null);
  let endedTrackId = $state<string | null>(null);
  let preloadedTrackId = $state<string | null>(null);
  let loadPending = $state(false);
  let currentTrackCached = $state(false);
  let crossfadePending = $state(false);
  let smartCrossfadePoint = $state<number | null>(null);
  let currentBpm = $state<number | null>(null);
  let preloadedBpm = $state<number | null>(null);
  /** Crossfade duration (ms) computed by smart crossfade; overrides getCrossfadeSeconds() when set. */
  let pendingCrossfadeMs = $state<number | null>(null);
  // True while the crossfade IPC call is in-flight — suppress the "wrong track" stop
  let crossfadeInFlight = false;

  function shouldPreload(positionSeconds: number, durationSeconds: number): boolean {
    if (durationSeconds <= 0) return false;
    const leadSeconds = Math.max((options.getCrossfadeSeconds() ?? 4) + 2, 4);
    const triggerAt = Math.min(durationSeconds * 0.8, Math.max(0, durationSeconds - leadSeconds));
    return positionSeconds >= triggerAt;
  }

  // Status poll + cache-updated event
  $effect(() => {
    options.onRestoreCastSession().catch(() => undefined);

    function handleCacheUpdated(event: Event) {
      const songId = (event as CustomEvent<{ songId?: string }>).detail?.songId;
      const track = options.getCurrentTrack();
      if (!songId || !track || songId !== track.id) return;
      currentTrackCached = true;
    }
    window.addEventListener(DESKTOP_PLAYBACK_CACHE_UPDATED_EVENT, handleCacheUpdated);

    const poll = window.setInterval(() => {
      if (options.getCastActive()) return;
      desktopPlaybackStatus()
        .then((status) => {
          const activeTrackId = options.getCurrentTrack()?.id ?? null;
          if (!activeTrackId) {
            if (status.loaded || status.playing) desktopPlaybackStop().catch(() => undefined);
            loadedTrackId = null;
            endedTrackId = null;
            loadPending = false;
            currentTime.set(0);
            duration.set(0);
            isPlaying.set(false);
            options.setIsBuffering(false);
            return;
          }

          // Gapless advance: the engine switched to the preloaded track on its
          // own. Sync the queue and UI without issuing a new load command.
          if (
            status.gaplessAdvancedTo &&
            status.gaplessAdvancedTo !== activeTrackId &&
            status.gaplessAdvancedTo === preloadedTrackId
          ) {
            loadedTrackId = status.gaplessAdvancedTo;
            preloadedTrackId = null;
            endedTrackId = null;
            loadPending = false;
            options.setIsBuffering(false);
            currentTime.set(status.position ?? 0);
            if (status.duration > 0) duration.set(status.duration);
            isPlaying.set(true);
            shouldAutoplay.set(false);
            nextTrack(); // advance queue to match what the engine is already playing
            return;
          }

          if (status.trackId && status.trackId !== activeTrackId && !crossfadeInFlight) {
            // The engine still has the previous track while the new one loads.
            // Only send stop when no load is already in-flight: if loadPending is
            // true the stop could race with the load and clear the just-loaded
            // track before it starts playing.
            if (!loadPending) {
              desktopPlaybackStop().catch(() => undefined);
            }
            loadedTrackId = activeTrackId;
            endedTrackId = null;
            currentTime.set(0);
            const track = options.getCurrentTrack();
            duration.set(track && (track.duration ?? 0) > 0 ? track.duration! : 0);
            isPlaying.set(false);
            if (!loadPending) options.setIsBuffering(false);
            return;
          }

          if (status.trackId) loadedTrackId = status.trackId;
          if (!options.getSeekDragging()) currentTime.set(status.position ?? 0);
          if (status.duration > 0) duration.set(status.duration);
          smartCrossfadePoint = status.smartCrossfadePoint ?? null;
          currentBpm = status.currentTrackBpm ?? null;
          preloadedBpm = status.preloadedTrackBpm ?? null;

          if (loadPending) {
            isPlaying.set(true);
            if (status.playing) {
              loadPending = false;
              options.setIsBuffering(false);
            } else {
              options.setIsBuffering(true);
            }
          } else {
            isPlaying.set(status.playing);
            options.setIsBuffering(false);
          }

          if (status.ended && status.trackId && endedTrackId !== status.trackId) {
            endedTrackId = status.trackId;
            nextTrack();
          } else if (!status.ended) {
            endedTrackId = null;
          }
        })
        .catch(() => undefined);
    }, 100);

    return () => {
      window.removeEventListener(DESKTOP_PLAYBACK_CACHE_UPDATED_EVENT, handleCacheUpdated);
      clearInterval(poll);
    };
  });

  // Cache check
  $effect(() => {
    const track = options.getCurrentTrack();
    if (!track) { currentTrackCached = false; return; }
    let cancelled = false;
    desktopPlaybackIsCached(track as Parameters<typeof desktopPlaybackIsCached>[0])
      .then((cached) => { if (!cancelled) currentTrackCached = cached; })
      .catch(() => { if (!cancelled) currentTrackCached = false; });
    return () => { cancelled = true; };
  });

  // Load when current track changes
  $effect(() => {
    if (options.getCastActive()) return;
    const track = options.getCurrentTrack();
    if (!track?.streamUrl) {
      desktopPlaybackStop().catch(() => undefined);
      loadedTrackId = null;
      endedTrackId = null;
      preloadedTrackId = null;
      loadPending = false;
      smartCrossfadePoint = null;
      currentBpm = null;
      preloadedBpm = null;
      pendingCrossfadeMs = null;
      options.setIsBuffering(false);
      isPlaying.set(false);
      currentTime.set(0);
      duration.set(0);
      return;
    }
    if (loadedTrackId === track.id) return;
    loadedTrackId = track.id;
    smartCrossfadePoint = null;
    currentBpm = null;
    preloadedBpm = null;
    const isCrossfade = crossfadePending;
    const crossfadeMs = isCrossfade
      ? (pendingCrossfadeMs ?? Math.round(options.getCrossfadeSeconds() * 1000))
      : undefined;
    crossfadePending = false;
    pendingCrossfadeMs = null;
    currentTime.set(0);
    duration.set((track.duration ?? 0) > 0 ? track.duration! : 0);
    const autoplay = isCrossfade || shouldAutoplayRef.current;
    if (!isCrossfade) options.setIsBuffering(true);
    loadPending = autoplay && !isCrossfade;
    isPlaying.set(autoplay);
    if (!isCrossfade && autoplay) shouldAutoplay.set(false);
    crossfadeInFlight = isCrossfade;
    desktopPlaybackLoad(track as Parameters<typeof desktopPlaybackLoad>[0], autoplay, crossfadeMs)
      .then(() => {
        crossfadeInFlight = false;
        // Rust returns only after the track is fully loaded and playing is set —
        // clear the buffering spinner immediately instead of waiting for the poll.
        loadPending = false;
        options.setIsBuffering(false);
        // Explicitly sync isPlaying: the poll may have set it to false while the
        // load was in-flight (mismatched-trackId branch), so restore the correct
        // state now that the load has confirmed success.
        isPlaying.set(autoplay);
      })
      .catch((error) => {
        crossfadeInFlight = false;
        loadedTrackId = null;
        loadPending = false;
        options.setIsBuffering(false);
        isPlaying.set(false);
        const msg = typeof error === 'string' ? error : (error instanceof Error ? error.message : null);
        toast.error(msg ?? 'Desktop playback failed to load the track');
      });
  });

  // Volume sync
  $effect(() => {
    if (options.getCastActive()) return;
    desktopPlaybackSetVolume(volumeRef.current).catch(() => undefined);
  });

  // Normalization sync
  $effect(() => {
    if (options.getCastActive()) return;
    desktopPlaybackSetNormalization(options.getNormalizationEnabled()).catch(() => undefined);
  });

  // Normalization mode sync
  $effect(() => {
    if (options.getCastActive()) return;
    desktopPlaybackSetNormalizationMode(options.getNormalizationMode()).catch(() => undefined);
  });

  // Loudness compensation sync
  $effect(() => {
    if (options.getCastActive()) return;
    desktopPlaybackSetLoudnessCompensation(options.getLoudnessCompensationEnabled()).catch(() => undefined);
  });

  // Preload next track — skipped when both gapless and crossfade are off, since
  // neither feature needs the track in memory before the current one ends.
  $effect(() => {
    if (options.getCastActive()) return;
    if (!options.getGaplessEnabled() && (options.getCrossfadeSeconds() ?? 0) <= 0) return;
    const items = queueRef.current;
    const index = currentIndexRef.current;
    const next = items[index + 1];
    if (!next?.streamUrl) { preloadedTrackId = null; return; }
    const track = options.getCurrentTrack();
    const trackDuration = (track?.duration ?? 0) > 0 ? track!.duration! : durationRef.current;
    if (!shouldPreload(currentTimeRef.current, trackDuration)) return;
    if (preloadedTrackId === next.id) return;
    preloadedTrackId = next.id;
    desktopPlaybackPreload(next as Parameters<typeof desktopPlaybackPreload>[0]).catch(() => {
      if (preloadedTrackId === next.id) preloadedTrackId = null;
    });
  });

  // Crossfade trigger — fires when position enters the crossfade window.
  // Smart crossfade mode adds three layers on top of a plain time-based trigger:
  //   1. Beat detection  – uses the track's analysed BPM so durations snap to beats.
  //   2. Phrase alignment – snaps the trigger point to the nearest 16-beat (4-bar)
  //                         phrase boundary, producing musically natural transitions.
  //   3. Dynamic duration – crossfade length = 4 beats at the average BPM of the
  //                         outgoing and incoming tracks (clamped 1.5–8 s).
  $effect(() => {
    if (options.getCastActive()) return;
    const crossfadeSeconds = options.getCrossfadeSeconds();
    if (!crossfadeSeconds || crossfadeSeconds <= 0) return;
    const items = queueRef.current;
    const index = currentIndexRef.current;
    const next = items[index + 1];
    if (!next?.streamUrl) return;
    if (preloadedTrackId !== next.id) return;
    if (crossfadePending) return;
    const track = options.getCurrentTrack();
    const nominalDur = (track?.duration ?? 0) > 0 ? track!.duration! : durationRef.current;
    if (nominalDur <= 0) return;
    const pos = currentTimeRef.current;

    const smart = options.getSmartCrossfadeEnabled();

    if (smart && currentBpm != null && currentBpm > 0) {
      // ── Smart crossfade: beat detection + phrase alignment + dynamic duration ──

      // 1. Dynamic fade duration = 4 beats at the average BPM of both tracks.
      const avgBpm = preloadedBpm != null && preloadedBpm > 0
        ? (currentBpm + preloadedBpm) / 2
        : currentBpm;
      const avgBeatSecs = 60.0 / avgBpm;
      const dynamicFadeSecs = Math.max(1.5, Math.min(8, 4 * avgBeatSecs));

      // 2. Natural end (silence detection), falling back to nominal duration.
      const naturalEnd = smartCrossfadePoint != null
        ? Math.min(smartCrossfadePoint, nominalDur)
        : nominalDur;

      // 3. Phrase alignment: snap the ideal trigger to the nearest 16-beat boundary.
      //    We assume the track starts at beat 1 of bar 1 (phrase offset = 0).
      const beatSecs = 60.0 / currentBpm;
      const phraseSecs = 16 * beatSecs; // 4 bars × 4 beats
      const idealTrigger = naturalEnd - dynamicFadeSecs;

      let triggerAt: number;
      if (idealTrigger > 0 && phraseSecs > 0) {
        // Round to the nearest phrase boundary.
        const phraseIdx = Math.round(idealTrigger / phraseSecs);
        const phraseAligned = phraseIdx * phraseSecs;
        // Clamp: must be positive and leave room for the fade.
        triggerAt = Math.max(0, Math.min(phraseAligned, nominalDur - dynamicFadeSecs));
      } else {
        triggerAt = Math.max(0, naturalEnd - dynamicFadeSecs);
      }

      if (pos < triggerAt || pos >= naturalEnd) return;
      pendingCrossfadeMs = Math.round(dynamicFadeSecs * 1000);
    } else if (smart && smartCrossfadePoint != null) {
      // ── Silence-detection only (BPM not yet available) ──
      const naturalEnd = Math.min(smartCrossfadePoint, nominalDur);
      if (pos < naturalEnd - crossfadeSeconds || pos >= naturalEnd) return;
      pendingCrossfadeMs = null; // use user crossfadeSeconds
    } else {
      // ── Plain time-based crossfade ──
      if (pos < nominalDur - crossfadeSeconds || pos >= nominalDur) return;
      pendingCrossfadeMs = null;
    }

    crossfadePending = true;
    shouldAutoplay.set(true);
    nextTrack();
  });

  function togglePlay() {
    if (isPlayingRef.current) {
      desktopPlaybackPause().then(() => isPlaying.set(false)).catch(() => undefined);
    } else {
      desktopPlaybackPlay().then(() => isPlaying.set(true)).catch(() => isPlaying.set(false));
    }
  }

  function seek(value: number) {
    desktopPlaybackSeek(value).catch(() => undefined);
  }

  // Restore playback position (desktop path)
  $effect(() => {
    if (options.getCastActive()) return;
    const restore = restoreRef.current;
    const track = options.getCurrentTrack();
    if (!restore || !track || restore.songId !== track.id) return;
    if (loadedTrackId !== track.id || loadPending) return;
    const position = restore.position;
    restorePlaybackRequest.set(null);
    applyPosition(position);
  });

  function applyPosition(value: number) {
    const clamped = Math.max(0, value);
    desktopPlaybackSeek(clamped).catch(() => undefined);
    currentTime.set(clamped);
  }

  function handleAutoplay() {
    const track = options.getCurrentTrack();
    if (!track) return;
    if (loadPending || options.getIsBuffering()) return;
    if (loadedTrackId !== track.id) return;
    shouldAutoplay.set(false);
    loadPending = true;
    options.setIsBuffering(true);
    desktopPlaybackSeek(0)
      .then(() => desktopPlaybackPlay())
      .then(() => {
        isPlaying.set(true);
        options.setIsBuffering(false);
        loadPending = false;
      })
      .catch(() => {
        isPlaying.set(false);
        options.setIsBuffering(false);
        loadPending = false;
      });
  }

  return {
    get loadedTrackId() { return loadedTrackId; },
    get loadPending() { return loadPending; },
    get currentTrackCached() { return currentTrackCached; },
    togglePlay,
    seek,
    applyPosition,
    handleAutoplay
  };
}

export type PlayerDesktopController = ReturnType<typeof createPlayerDesktopController>;
