import {
  castConnect as castConnectCmd,
  castDiscover,
  castPlay as castPlayCmd,
  castPause as castPauseCmd,
  castResume as castResumeCmd,
  castStop as castStopCmd,
  castSetVolume as castSetVolumeCmd,
  castSeek as castSeekCmd,
  castGetSession as castGetSessionCmd,
  castGetStatus as castGetStatusCmd,
  type CastDeviceInfo
} from '$lib/servers';
import { toast } from 'svelte-sonner';

type SongLike = {
  id: string;
  title: string;
  artist: string;
  streamUrl: string;
  coverArtUrl?: string | null;
  duration?: number;
};

type PlayerCastControllerOptions = {
  getCurrentTrack: () => SongLike | null;
  getSeekDragging: () => boolean;

  setCurrentTime: (value: number) => void;
  setDuration: (value: number) => void;
  setCastVolume: (value: number | null) => void;
  setCastPlaying: (value: boolean) => void;
  setCastActive: (value: boolean) => void;
  setCastDevice: (value: CastDeviceInfo | null) => void;

  onPauseLocalPlayback?: () => void | Promise<void>;
  onAdvanceTrack?: () => void;
};

export function createPlayerCastController(options: PlayerCastControllerOptions) {
  let castDevices = $state<CastDeviceInfo[]>([]);
  let discovering = $state(false);
  let castActive = $state(false);
  let castPlaying = $state(false);
  let castDevice = $state<CastDeviceInfo | null>(null);
  let castVolume = $state<number | null>(null);
  let lastPlayerState = $state<string | null>(null);
  // Last state that was not IDLE — used to confirm a song actually played
  // before advancing. lastPlayerState itself becomes 'IDLE' on the first idle
  // poll, so checking it on the second poll would always be false.
  let lastNonIdleState: string | null = null;
  // Counts consecutive IDLE polls. We require 2 in a row before advancing the
  // track, so that transient IDLE responses from network hiccups or external
  // cast events don't prematurely skip to the next song.
  let consecutiveIdleCount = 0;

  let pollTimer: ReturnType<typeof setInterval> | null = null;
  let pollInFlight = false;

  function syncStateToOwner() {
    options.setCastActive(castActive);
    options.setCastPlaying(castPlaying);
    options.setCastDevice(castDevice);
    options.setCastVolume(castVolume);
  }

  function applyStatus(status: { currentTime: number; playerState: string; volumeLevel: number | null }) {
    castVolume = status.volumeLevel;
    options.setCastVolume(status.volumeLevel);

    if (status.playerState === 'PLAYING' || status.playerState === 'PAUSED') {
      consecutiveIdleCount = 0;
      lastNonIdleState = status.playerState;
      const t = status.currentTime;
      if (Number.isFinite(t) && t >= 0) options.setCurrentTime(t);
      castPlaying = status.playerState === 'PLAYING';
      options.setCastPlaying(castPlaying);
    } else if (status.playerState === 'BUFFERING') {
      // Buffering is an active state — reset idle counter and keep castPlaying as-is
      // so a brief BUFFERING → IDLE blip doesn't trigger a track advance.
      consecutiveIdleCount = 0;
      lastNonIdleState = status.playerState;
    } else if (status.playerState === 'IDLE') {
      consecutiveIdleCount++;
      castPlaying = false;
      options.setCastPlaying(false);

      // Only advance after 2 consecutive IDLE polls so transient network blips
      // don't cause spurious skips. Use lastNonIdleState (not lastPlayerState)
      // because lastPlayerState is already 'IDLE' by the second poll.
      if (consecutiveIdleCount === 2 && lastNonIdleState !== null) {
        options.onAdvanceTrack?.();
      }
    }

    lastPlayerState = status.playerState;
  }

  async function discoverDevices() {
    if (castActive || discovering) return;
    discovering = true;
    castDevices = [];

    try {
      castDevices = await castDiscover();
      if (castDevices.length === 0) {
        toast.info('No Cast devices found on your network');
      }
    } catch (error) {
      toast.error(`Cast discovery failed: ${error}`);
    } finally {
      discovering = false;
    }
  }

  async function startCast(device: CastDeviceInfo) {
    const toastId = toast.loading(`Connecting to ${device.name}...`);

    try {
      await options.onPauseLocalPlayback?.();

      // Set device early so the UI shows "connecting", but do NOT set
      // castActive yet — that triggers the polling loop, which would open
      // concurrent TLS connections while the play/connect command is in
      // flight and cause WSAECONNABORTED (OS error 10053).
      castDevice = device;
      castPlaying = false;
      consecutiveIdleCount = 0;
      lastPlayerState = null;
      lastNonIdleState = null;
      syncStateToOwner();

      const track = options.getCurrentTrack();

      if (track) {
        options.setCurrentTime(0);
        options.setDuration((track.duration ?? 0) > 0 ? track.duration! : 0);

        await castPlayCmd({
          deviceName: device.name,
          deviceAddr: device.addr,
          devicePort: device.port,
          songId: track.id,
          streamUrl: track.streamUrl,
          title: track.title,
          artist: track.artist,
          coverUrl: track.coverArtUrl ?? ''
        });

        // Only mark active (starts polling) after the command succeeds.
        castActive = true;
        castPlaying = true;
        syncStateToOwner();
        toast.success(`Casting to ${device.name}`, { id: toastId });
      } else {
        await castConnectCmd({
          deviceName: device.name,
          deviceAddr: device.addr,
          devicePort: device.port
        });

        castActive = true;
        const status = await castGetStatusCmd();
        applyStatus(status);
        syncStateToOwner();
        toast.success(`Connected to ${device.name}`, { id: toastId });
      }
    } catch (error) {
      castActive = false;
      castPlaying = false;
      castVolume = null;
      castDevice = null;
      syncStateToOwner();
      toast.error(`Cast failed: ${error}`, { id: toastId });
    }
  }

  async function stopCast() {
    const name = castDevice?.name ?? 'device';

    castActive = false;
    castPlaying = false;
    castVolume = null;
    castDevice = null;
    lastPlayerState = null;
    lastNonIdleState = null;
    consecutiveIdleCount = 0;
    syncStateToOwner();

    try {
      await castStopCmd();
    } catch {}

    toast.success(`Stopped casting to ${name}`);
  }

  async function togglePlayPause() {
    if (!castActive) return;

    const track = options.getCurrentTrack();

    if (castPlaying) {
      castPlaying = false;
      syncStateToOwner();

      castPauseCmd().catch((error) => {
        castPlaying = true;
        syncStateToOwner();
        toast.error(`Cast pause failed: ${error}`);
      });
      return;
    }

    if (track && (!lastPlayerState || lastPlayerState === 'IDLE')) {
      castPlaying = true;
      syncStateToOwner();

      castPlayCmd({
        deviceName: castDevice?.name ?? 'Chromecast',
        deviceAddr: castDevice?.addr ?? '',
        devicePort: castDevice?.port ?? 0,
        songId: track.id,
        streamUrl: track.streamUrl,
        title: track.title,
        artist: track.artist,
        coverUrl: track.coverArtUrl ?? ''
      }).catch((error) => {
        castPlaying = false;
        syncStateToOwner();
        toast.error(`Cast play failed: ${error}`);
      });

      return;
    }

    castPlaying = true;
    syncStateToOwner();

    castResumeCmd().catch((error) => {
      castPlaying = false;
      syncStateToOwner();
      toast.error(`Cast resume failed: ${error}`);
    });
  }

  function playTrackOnCast(track: SongLike) {
    if (!castActive || !castDevice) return;

    castPlaying = true;
    consecutiveIdleCount = 0;
    lastPlayerState = null;
    lastNonIdleState = null;
    options.setCurrentTime(0);
    options.setDuration((track.duration ?? 0) > 0 ? track.duration! : 0);
    syncStateToOwner();

    castPlayCmd({
      deviceName: castDevice.name,
      deviceAddr: castDevice.addr,
      devicePort: castDevice.port,
      songId: track.id,
      streamUrl: track.streamUrl,
      title: track.title,
      artist: track.artist,
      coverUrl: track.coverArtUrl ?? ''
    }).catch((error) => {
      castPlaying = false;
      syncStateToOwner();
      toast.error(`Cast update failed: ${error}`);
    });
  }

  function seek(seconds: number) {
    if (!castActive) return;
    castSeekCmd(seconds).catch((error) => toast.error(`Cast seek failed: ${error}`));
  }

  function setVolume(value: number) {
    if (!castActive) return;
    castSetVolumeCmd(value).catch(() => undefined);
  }

  async function restoreSession() {
    try {
      const session = await castGetSessionCmd();
      if (!session) return;

      castDevice = {
        name: session.name,
        addr: session.addr,
        port: session.port
      };
      castActive = true;
      syncStateToOwner();

      try {
        const status = await castGetStatusCmd();
        applyStatus(status);
      } catch {
        castActive = false;
        castPlaying = false;
        castVolume = null;
        castDevice = null;
        syncStateToOwner();
      }
    } catch {
      // ignore restore failure
    }
  }

  function startPolling() {
    stopPolling();

    pollTimer = setInterval(async () => {
      if (!castActive || options.getSeekDragging() || pollInFlight) return;
      pollInFlight = true;
      try {
        const status = await castGetStatusCmd();
        applyStatus(status);
      } catch {
        // ignore transient polling failures
      } finally {
        pollInFlight = false;
      }
    }, 1500);
  }

  function stopPolling() {
    if (!pollTimer) return;
    clearInterval(pollTimer);
    pollTimer = null;
    pollInFlight = false;
  }

  $effect(() => {
    if (!castActive) {
      stopPolling();
      return;
    }

    startPolling();

    return () => {
      stopPolling();
    };
  });

  return {
    get castDevices() {
      return castDevices;
    },
    get discovering() {
      return discovering;
    },
    get castActive() {
      return castActive;
    },
    get castPlaying() {
      return castPlaying;
    },
    get castDevice() {
      return castDevice;
    },
    get castVolume() {
      return castVolume;
    },

    discoverDevices,
    startCast,
    stopCast,
    togglePlayPause,
    playTrackOnCast,
    seek,
    setVolume,
    restoreSession,
    startPolling,
    stopPolling
  };
}

export type PlayerCastController = ReturnType<typeof createPlayerCastController>;