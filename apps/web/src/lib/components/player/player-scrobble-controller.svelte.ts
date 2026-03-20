import { fromStore } from 'svelte/store';
import { currentTime, duration, addRecentlyPlayedSong } from '$lib/stores/player';
import { lfmNowPlaying, lfmScrobble } from '$lib/servers';
import { lbzNowPlaying, lbzScrobble } from '$lib/providers/recommendation/listenbrainz';
import { recordPlay } from '$lib/servers/play-history';

type SongLike = {
  id: string;
  title: string;
  artist: string;
  album?: string | null;
  coverArtUrl?: string | null;
  duration?: number | null;
};

type PlayerScrobbleControllerOptions = {
  getCurrentTrack: () => SongLike | null;
  getLastFmConnected: () => boolean;
  getLbzToken: () => string | null | undefined;
};

const currentTimeRef = fromStore(currentTime);
const durationRef = fromStore(duration);

export function createPlayerScrobbleController(options: PlayerScrobbleControllerOptions) {
  let scrobbledTrackId = '';
  let playRecordedTrackId = '';
  let scrobbleStartTime = 0;

  // On track change: send nowPlaying signals.
  // Cleanup records the play for skipped tracks (played < 50% so the threshold
  // effect never fired), capturing the true completion fraction at skip time.
  $effect(() => {
    const track = options.getCurrentTrack();
    if (!track) return;

    scrobbledTrackId = '';
    playRecordedTrackId = '';
    scrobbleStartTime = Math.floor(Date.now() / 1000);
    addRecentlyPlayedSong(track);

    if (options.getLastFmConnected()) {
      lfmNowPlaying(track.artist, track.title, track.album || undefined, track.duration || undefined);
    }
    const lbzToken = options.getLbzToken();
    if (lbzToken) {
      lbzNowPlaying(lbzToken, track.artist, track.title, track.album || undefined, track.duration || undefined);
    }

    // Cleanup: fires when the user skips to another track.
    // Only records if the 50% threshold effect hasn't already written a record
    // (avoids duplicates). Captures skips (< 50% played) as a negative signal.
    return () => {
      if (playRecordedTrackId === track.id) return; // already recorded via threshold
      const t = currentTimeRef.current;
      if (t < 5) return; // ignore accidental plays < 5 s
      const dur = durationRef.current > 0 ? durationRef.current : (track.duration ?? 0);
      const completedFraction = dur > 0 ? Math.min(t / dur, 1) : 0;
      recordPlay({
        songId: track.id,
        artist: track.artist,
        title: track.title,
        album: track.album,
        coverArtUrl: track.coverArtUrl,
        durationSecs: track.duration ?? (dur > 0 ? dur : undefined),
        completedFraction,
      });
    };
  });

  // Fires when the 50% threshold is reached — the primary play record.
  // Using this as the main recording point means stats update while the song
  // is still playing, not only after the user moves to the next track.
  $effect(() => {
    const t = currentTimeRef.current;
    const dur = durationRef.current;
    const track = options.getCurrentTrack();
    if (!track || scrobbledTrackId === track.id) return;
    const threshold = dur > 0 ? Math.min(dur * 0.5, 240) : 240;
    if (t < threshold) return;

    scrobbledTrackId = track.id;
    playRecordedTrackId = track.id;

    // Record the play with the current completion fraction at scrobble time
    recordPlay({
      songId: track.id,
      artist: track.artist,
      title: track.title,
      album: track.album,
      coverArtUrl: track.coverArtUrl,
      durationSecs: track.duration ?? (dur > 0 ? dur : undefined),
      completedFraction: dur > 0 ? Math.min(t / dur, 1) : 0.5,
    });

    if (options.getLastFmConnected()) {
      lfmScrobble(track.artist, track.title, scrobbleStartTime, track.album || undefined, track.duration || undefined);
    }
    const lbzToken = options.getLbzToken();
    if (lbzToken) {
      lbzScrobble(lbzToken, track.artist, track.title, scrobbleStartTime, track.album || undefined, track.duration || undefined);
    }
  });
}

export type PlayerScrobbleController = ReturnType<typeof createPlayerScrobbleController>;
