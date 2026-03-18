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
};

type PlayerCastControllerOptions = {
  getCurrentTrack: () => SongLike | null;
  getSeekDragging: () => boolean;

  setCurrentTime: (value: number) => void;
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

  let pollTimer: ReturnType<typeof setInterval> | null = null;

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
      options.setCurrentTime(status.currentTime);
      castPlaying = status.playerState === 'PLAYING';
      options.setCastPlaying(castPlaying);
    } else if (status.playerState === 'IDLE') {
      castPlaying = false;
      options.setCastPlaying(false);
    }

    if (lastPlayerState && lastPlayerState !== 'IDLE' && status.playerState === 'IDLE') {
      options.onAdvanceTrack?.();
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

      castDevice = device;
      castActive = true;
      castPlaying = false;
      syncStateToOwner();

      const track = options.getCurrentTrack();

      if (track) {
        await castPlayCmd({
          deviceName: device.name,
          deviceAddr: device.addr,
          devicePort: device.port,
          streamUrl: track.streamUrl,
          title: track.title,
          artist: track.artist,
          coverUrl: track.coverArtUrl ?? ''
        });

        castPlaying = true;
        syncStateToOwner();
        toast.success(`Casting to ${device.name}`, { id: toastId });
      } else {
        await castConnectCmd({
          deviceName: device.name,
          deviceAddr: device.addr,
          devicePort: device.port
        });

        const status = await castGetStatusCmd();
        applyStatus(status);
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
    syncStateToOwner();

    castPlayCmd({
      deviceName: castDevice.name,
      deviceAddr: castDevice.addr,
      devicePort: castDevice.port,
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
        name: session.deviceName,
        addr: session.deviceAddr,
        port: session.devicePort
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
      if (!castActive || options.getSeekDragging()) return;

      try {
        const status = await castGetStatusCmd();
        applyStatus(status);
      } catch {
        // ignore transient polling failures
      }
    }, 1000);
  }

  function stopPolling() {
    if (!pollTimer) return;
    clearInterval(pollTimer);
    pollTimer = null;
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