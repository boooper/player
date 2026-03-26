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
      options.setIsBuffering(false);
      isPlaying.set(false);
      currentTime.set(0);
      duration.set(0);
      return;
    }
    if (loadedTrackId === track.id) return;
    loadedTrackId = track.id;
    const isCrossfade = crossfadePending;
    const crossfadeMs = isCrossfade ? Math.round(options.getCrossfadeSeconds() * 1000) : undefined;
    crossfadePending = false;
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

  // Preload next track
  $effect(() => {
    if (options.getCastActive()) return;
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

  // Crossfade trigger — fires when position enters the crossfade window and the next
  // track is already preloaded in memory. Only then do we advance the queue early.
  $effect(() => {
    if (options.getCastActive()) return;
    const crossfadeSeconds = options.getCrossfadeSeconds();
    if (!crossfadeSeconds || crossfadeSeconds <= 0) return;
    const items = queueRef.current;
    const index = currentIndexRef.current;
    const next = items[index + 1];
    if (!next?.streamUrl) return;
    if (preloadedTrackId !== next.id) return; // only crossfade when already buffered
    if (crossfadePending) return;
    const track = options.getCurrentTrack();
    const dur = (track?.duration ?? 0) > 0 ? track!.duration! : durationRef.current;
    if (dur <= 0) return;
    const pos = currentTimeRef.current;
    if (pos < dur - crossfadeSeconds || pos >= dur) return;
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
