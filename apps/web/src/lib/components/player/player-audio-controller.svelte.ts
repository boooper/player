import { untrack } from 'svelte';
import { fromStore } from 'svelte/store';
import {
  currentTime,
  duration,
  isPlaying,
  shouldAutoplay,
  repeatMode,
  restorePlaybackRequest,
  volume,
  queue,
  currentIndex,
  focusTrack,
  nextTrack
} from '$lib/stores/player';

type SongLike = {
  id: string;
  title: string;
  artist: string;
  coverArtUrl?: string | null;
  album?: string | null;
  streamUrl?: string | null;
  duration?: number;
};

type PlayerAudioControllerOptions = {
  getDeckAEl: () => HTMLAudioElement | null;
  getDeckBEl: () => HTMLAudioElement | null;
  getCastActive: () => boolean;
  getCrossfadeMs: () => number;
  getCrossfadeSeconds: () => number;
  getCurrentTrack: () => SongLike | null;
  setIsBuffering: (v: boolean) => void;
  resumeAudioContext: () => Promise<void>;
};

const queueRef = fromStore(queue);
const currentIndexRef = fromStore(currentIndex);
const currentTimeRef = fromStore(currentTime);
const durationRef = fromStore(duration);
const isPlayingRef = fromStore(isPlaying);
const volumeRef = fromStore(volume);
const repeatModeRef = fromStore(repeatMode);
const restoreRef = fromStore(restorePlaybackRequest);

export function createPlayerAudioController(options: PlayerAudioControllerOptions) {
  let activeDeck = $state<'a' | 'b'>('a');
  let crossfadeInProgress = false;
  let crossfadeTimer = 0;
  let preloadedIndex = -1;

  function getActiveAudio(): HTMLAudioElement | null {
    return activeDeck === 'a' ? options.getDeckAEl() : options.getDeckBEl();
  }

  function getInactiveAudio(): HTMLAudioElement | null {
    return activeDeck === 'a' ? options.getDeckBEl() : options.getDeckAEl();
  }

  function pauseLocalAudio(): void {
    options.getDeckAEl()?.pause();
    options.getDeckBEl()?.pause();
  }

  function stopCrossfadeTimer(): void {
    if (!crossfadeTimer) return;
    clearInterval(crossfadeTimer);
    crossfadeTimer = 0;
  }

  function shouldPreload(positionSeconds: number, durationSeconds: number): boolean {
    if (durationSeconds <= 0) return false;
    const leadSeconds = Math.max((options.getCrossfadeSeconds() ?? 4) + 2, 4);
    const triggerAt = Math.min(durationSeconds * 0.8, Math.max(0, durationSeconds - leadSeconds));
    return positionSeconds >= triggerAt;
  }

  // Load track when current track changes
  $effect(() => {
    if (options.getCastActive()) return;
    const track = options.getCurrentTrack();
    const activeAudio = getActiveAudio();
    if (!activeAudio || !track?.streamUrl || crossfadeInProgress) return;
    if (activeAudio.src === track.streamUrl) return;
    activeAudio.src = track.streamUrl;
    activeAudio.preload = 'auto';
    currentTime.set(0);
    duration.set((track.duration ?? 0) > 0 ? track.duration! : 0);
  });

  // Volume sync
  $effect(() => {
    if (crossfadeInProgress) return;
    const vol = volumeRef.current;
    const activeAudio = getActiveAudio();
    const inactiveAudio = getInactiveAudio();
    if (activeAudio) activeAudio.volume = vol;
    if (inactiveAudio && inactiveAudio.paused) inactiveAudio.volume = 0;
  });

  // Restore playback position
  $effect(() => {
    const restore = restoreRef.current;
    const track = options.getCurrentTrack();
    if (!restore || !track || restore.songId !== track.id) return;

    const activeAudio = getActiveAudio();
    if (!activeAudio || !track.streamUrl || activeAudio.src !== track.streamUrl) return;

    const position = restore.position;
    const finishRestore = () => {
      restorePlaybackRequest.set(null);
      applyPosition(position);
    };

    if (activeAudio.readyState > 0) { finishRestore(); return; }

    const handleLoadedMetadata = () => { finishRestore(); };
    activeAudio.addEventListener('loadedmetadata', handleLoadedMetadata, { once: true });
    return () => { activeAudio.removeEventListener('loadedmetadata', handleLoadedMetadata); };
  });

  // Preload next track
  $effect(() => {
    if (options.getCastActive() || crossfadeInProgress) return;
    const items = queueRef.current;
    const index = currentIndexRef.current;
    const next = items[index + 1];
    const inactiveAudio = getInactiveAudio();
    if (!inactiveAudio) return;
    if (!next?.streamUrl) { preloadedIndex = -1; return; }
    const track = options.getCurrentTrack();
    const trackDuration = (track?.duration ?? 0) > 0 ? track!.duration! : durationRef.current;
    if (!shouldPreload(currentTimeRef.current, trackDuration)) return;
    if (preloadedIndex === index + 1 && inactiveAudio.src === next.streamUrl) return;
    inactiveAudio.src = next.streamUrl;
    inactiveAudio.preload = 'auto';
    inactiveAudio.load();
    inactiveAudio.volume = 0;
    preloadedIndex = index + 1;
  });

  // Crossfade trigger
  $effect(() => {
    const next = queueRef.current[currentIndexRef.current + 1];
    if (options.getCastActive() || crossfadeInProgress || !next || repeatModeRef.current === 'one' || !isPlayingRef.current) return;
    const remaining = durationRef.current - currentTimeRef.current;
    const fadeMs = options.getCrossfadeMs();
    if (fadeMs <= 0 || remaining <= 0 || remaining > fadeMs / 1000) return;
    untrack(() => { startCrossfade(currentIndexRef.current + 1); });
  });

  async function startCrossfade(nextIndex: number): Promise<void> {
    if (crossfadeInProgress || options.getCastActive()) return;
    const items = queueRef.current;
    const next = items[nextIndex];
    const fromAudio = getActiveAudio();
    const toAudio = getInactiveAudio();
    if (!next || !fromAudio || !toAudio) return;

    if (toAudio.src !== next.streamUrl) {
      toAudio.src = next.streamUrl!;
      toAudio.preload = 'auto';
      toAudio.load();
    }

    try {
      await options.resumeAudioContext();
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
    focusTrack.set({
      title: next.title,
      artist: next.artist,
      imageUrl: next.coverArtUrl ?? null,
      source: 'library',
      album: next.album ?? null
    });
    currentTime.set(0);
    duration.set((next.duration ?? 0) > 0 ? next.duration! : 0);

    const fadeDuration = Math.max(0, options.getCrossfadeMs());
    if (fadeDuration === 0) {
      stopCrossfadeTimer();
      oldAudio.pause();
      oldAudio.currentTime = 0;
      oldAudio.volume = 0;
      toAudio.volume = volumeRef.current;
      if (oldDeck === 'a') options.getDeckAEl()?.removeAttribute('src');
      else options.getDeckBEl()?.removeAttribute('src');
      crossfadeInProgress = false;
      return;
    }

    const startedAt = performance.now();
    stopCrossfadeTimer();
    crossfadeTimer = window.setInterval(() => {
      const vol = volumeRef.current;
      const progress = Math.min(1, (performance.now() - startedAt) / fadeDuration);
      oldAudio.volume = Math.max(0, (1 - progress) * vol);
      toAudio.volume = Math.min(vol, progress * vol);
      if (progress >= 1) {
        stopCrossfadeTimer();
        oldAudio.pause();
        oldAudio.currentTime = 0;
        oldAudio.volume = 0;
        if (oldDeck === 'a') options.getDeckAEl()?.removeAttribute('src');
        else options.getDeckBEl()?.removeAttribute('src');
        crossfadeInProgress = false;
      }
    }, 50);
  }

  function handleAudioPlay(deck: 'a' | 'b') {
    if (deck === activeDeck) isPlaying.set(true);
  }

  function handleAudioPause() {
    const a = options.getDeckAEl();
    const b = options.getDeckBEl();
    if (a?.paused !== false && b?.paused !== false) isPlaying.set(false);
  }

  function handleAudioWaiting(deck: 'a' | 'b') {
    if (deck === activeDeck) options.setIsBuffering(true);
  }

  function handleAudioCanPlay(deck: 'a' | 'b') {
    if (deck === activeDeck) options.setIsBuffering(false);
  }

  function handleAudioTimeUpdate(deck: 'a' | 'b') {
    if (deck !== activeDeck) return;
    const audio = deck === 'a' ? options.getDeckAEl() : options.getDeckBEl();
    currentTime.set(audio?.currentTime ?? 0);
  }

  function handleAudioDuration(deck: 'a' | 'b') {
    if (deck !== activeDeck) return;
    const audio = deck === 'a' ? options.getDeckAEl() : options.getDeckBEl();
    const d = audio?.duration ?? NaN;
    if (isFinite(d) && d > 0) duration.set(d);
  }

  function handleAudioEnded(deck: 'a' | 'b') {
    if (crossfadeInProgress || deck !== activeDeck) return;
    nextTrack();
  }

  function togglePlay() {
    const activeAudio = getActiveAudio();
    if (!activeAudio) return;
    if (isPlayingRef.current) {
      pauseLocalAudio();
    } else {
      options.resumeAudioContext().then(() => activeAudio.play()).catch(() => isPlaying.set(false));
    }
  }

  function seek(value: number) {
    const activeAudio = getActiveAudio();
    if (activeAudio) activeAudio.currentTime = value;
  }

  function applyPosition(value: number) {
    const activeAudio = getActiveAudio();
    if (activeAudio) activeAudio.currentTime = Math.max(0, value);
    currentTime.set(Math.max(0, value));
  }

  function handleAutoplay() {
    const activeAudio = getActiveAudio();
    if (!activeAudio) return;
    shouldAutoplay.set(false);
    options.resumeAudioContext()
      .then(() => activeAudio.play())
      .then(() => isPlaying.set(true))
      .catch(() => isPlaying.set(false));
  }

  return {
    handleAudioPlay,
    handleAudioPause,
    handleAudioWaiting,
    handleAudioCanPlay,
    handleAudioTimeUpdate,
    handleAudioDuration,
    handleAudioEnded,
    pauseLocalAudio,
    togglePlay,
    seek,
    applyPosition,
    handleAutoplay
  };
}

export type PlayerAudioController = ReturnType<typeof createPlayerAudioController>;
