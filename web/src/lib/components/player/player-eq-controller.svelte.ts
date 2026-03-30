import { desktopPlaybackSetEq } from '$lib/servers';
import type { EqFrequencyValues } from '$lib/audio/equalizer';

type PlayerEqControllerOptions = {
  getCastActive: () => boolean;
  getEqEnabled: () => boolean;
  getEqFrequencies: () => EqFrequencyValues;
  getEqBands: () => number[];
};

export function createPlayerEqController(options: PlayerEqControllerOptions) {
  // Sync EQ settings to desktop backend
  $effect(() => {
    if (options.getCastActive()) return;
    desktopPlaybackSetEq(options.getEqEnabled(), options.getEqFrequencies(), options.getEqBands() as EqFrequencyValues).catch(() => undefined);
  });
}

export type PlayerEqController = ReturnType<typeof createPlayerEqController>;
