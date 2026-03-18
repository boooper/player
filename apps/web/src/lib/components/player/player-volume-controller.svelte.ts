import { volume } from '$lib/stores/player';
import { desktopPlaybackSetVolume, castSetVolume as castSetVolumeCmd } from '$lib/servers';
import { isTauri } from '$lib/tauri';

type PlayerVolumeControllerOptions = {
  getCastActive: () => boolean;
  getCastVolume: () => number | null;
  getLocalVolume: () => number;
  getAudioEl?: () => HTMLAudioElement | null;
};

const EPS = 0.0001;
const DEFAULT_RESTORE_VOLUME = 0.8;

export function createPlayerVolumeController(options: PlayerVolumeControllerOptions) {
  const desktopPlayback = isTauri();

  let volVal = $state<number[]>([options.getLocalVolume() * 100]);
  let volDragging = $state(false);
  let premuteVolume = $state(DEFAULT_RESTORE_VOLUME);

  function clampVolume(value: number) {
    return Math.max(0, Math.min(1, value));
  }

  function sliderToVolume(values: number[]) {
    return clampVolume(Number(values[0] ?? 0) / 100);
  }

  function syncFromExternalVolume(value: number) {
    if (!volDragging) {
      volVal = [clampVolume(value) * 100];
    }
  }

  function setLocalVolume(value: number) {
    volume.set(value);

    if (desktopPlayback) {
      desktopPlaybackSetVolume(value).catch(() => undefined);
      return;
    }

    const audio = options.getAudioEl?.() ?? document.querySelector('audio');
    if (audio instanceof HTMLMediaElement) {
      audio.volume = value;
    }
  }

  function beginDrag() {
    volDragging = true;
  }

  function endDrag() {
    volDragging = false;
  }

  function changeVolume(values: number[]) {
    const value = sliderToVolume(values);
    const castActive = options.getCastActive();
    const castVolume = options.getCastVolume();
    const localVolume = options.getLocalVolume();

    volVal = values;

    if (castActive) {
      if (Math.abs((castVolume ?? 0) - value) < EPS) return;
      castSetVolumeCmd(value).catch(() => undefined);
      return;
    }

    if (Math.abs(localVolume - value) < EPS) return;
    setLocalVolume(value);
  }

  function commitVolume(values: number[]) {
    volDragging = false;
    changeVolume(values);
  }

  function setAbsoluteVolume(value: number) {
    const next = clampVolume(value);
    volVal = [next * 100];

    if (options.getCastActive()) {
      castSetVolumeCmd(next).catch(() => undefined);
      return;
    }

    setLocalVolume(next);
  }

  function toggleMute(currentEffectiveVolume: number) {
    const current = clampVolume(currentEffectiveVolume);

    if (current <= 0.01) {
      const restore = premuteVolume > 0.01 ? premuteVolume : DEFAULT_RESTORE_VOLUME;
      volVal = [restore * 100];

      if (options.getCastActive()) {
        castSetVolumeCmd(restore).catch(() => undefined);
        return;
      }

      setLocalVolume(restore);
      return;
    }

    premuteVolume = current;
    volVal = [0];

    if (options.getCastActive()) {
      castSetVolumeCmd(0).catch(() => undefined);
      return;
    }

    setLocalVolume(0);
  }

  function onWheel(event: WheelEvent, currentEffectiveVolume: number) {
    event.preventDefault();
    const delta = event.deltaY < 0 ? 0.05 : -0.05;
    setAbsoluteVolume(currentEffectiveVolume + delta);
  }

  return {
    get volVal() {
      return volVal;
    },
    set volVal(value: number[]) {
      volVal = value;
    },

    get volDragging() {
      return volDragging;
    },

    beginDrag,
    endDrag,
    changeVolume,
    commitVolume,
    toggleMute,
    onWheel,
    syncFromExternalVolume,
    setAbsoluteVolume
  };
}

export type PlayerVolumeController = ReturnType<typeof createPlayerVolumeController>;