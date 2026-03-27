import { desktopPlaybackSetEq } from '$lib/servers';

type PlayerEqControllerOptions = {
  getCastActive: () => boolean;
  getEqEnabled: () => boolean;
  getEqBands: () => number[];
};

export function createPlayerEqController(options: PlayerEqControllerOptions) {
  // Sync EQ settings to desktop backend
  $effect(() => {
    if (options.getCastActive()) return;
    desktopPlaybackSetEq(options.getEqEnabled(), options.getEqBands()).catch(() => undefined);
  });
}

export type PlayerEqController = ReturnType<typeof createPlayerEqController>;
