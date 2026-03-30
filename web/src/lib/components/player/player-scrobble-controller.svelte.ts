import { fromStore } from 'svelte/store';
import { currentTime, duration, addRecentlyPlayedSong } from '$lib/stores/player';
import { lfmNowPlaying, lfmScrobble, type Song } from '$lib/servers';
import { lbzNowPlaying, lbzScrobble } from '$lib/providers/recommendation/listenbrainz';
import { recordPlay, cacheSongGenre } from '$lib/servers/play-history';
import { getArtistInfo } from '$lib/data/metadata';

type PlayerScrobbleControllerOptions = {
  getCurrentTrack: () => Song | null;
  getLastFmConnected: () => boolean;
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
    lbzNowPlaying(track.artist, track.title, track.album || undefined, track.duration || undefined);

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
    if (track.genre) {
      cacheSongGenre(track.id, track.artist, track.genre);
    } else {
      getArtistInfo(track.artist)
        .then((info) => {
          const genre = info?.genre || info?.tags?.[0];
          if (genre) cacheSongGenre(track.id, track.artist, genre);
        })
        .catch(() => undefined);
    }

    if (options.getLastFmConnected()) {
      lfmScrobble(track.artist, track.title, scrobbleStartTime, track.album || undefined, track.duration || undefined);
    }
    lbzScrobble(track.artist, track.title, scrobbleStartTime, track.album || undefined, track.duration || undefined);
  });
}

export type PlayerScrobbleController = ReturnType<typeof createPlayerScrobbleController>;
