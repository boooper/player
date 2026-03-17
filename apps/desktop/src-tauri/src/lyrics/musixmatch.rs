use crate::lyrics::{clean_plain_lyrics, normalize_album, normalize_artist, normalize_title, LyricsResult};
use musixmatch_inofficial::{
    models::{SortOrder, SubtitleFormat, TrackId},
    Error as MusixmatchError, Musixmatch,
};

fn musixmatch_error_is_miss(error: &MusixmatchError) -> bool {
    matches!(error, MusixmatchError::NotAvailable | MusixmatchError::NotFound)
}

pub async fn fetch_from_musixmatch(
    artist: &str,
    title: &str,
    album: &str,
    duration: f64,
) -> Result<Option<LyricsResult>, MusixmatchError> {
    let mxm = Musixmatch::builder().no_storage().build()?;

    let matcher_track = mxm
        .matcher_track(title, artist, album, false, false, false)
        .await
        .ok();
    let search_tracks = mxm
        .track_search()
        .q_track(title)
        .q_artist(artist)
        .f_has_lyrics()
        .s_track_rating(SortOrder::Desc)
        .send(5, 1)
        .await
        .unwrap_or_default();

    let track = matcher_track.or_else(|| {
        search_tracks.into_iter().find(|track| {
            let artist_ok = normalize_artist(&track.artist_name) == normalize_artist(artist);
            let title_ok = normalize_title(&track.track_name) == normalize_title(title);
            let album_ok = album.is_empty() || normalize_album(&track.album_name) == normalize_album(album);
            let duration_ok = duration <= 0.0
                || (track.track_length as f64 - duration).abs() <= 3.0
                || (track.track_length as f64 - duration).abs() <= 8.0 && title_ok;
            artist_ok && title_ok && album_ok && duration_ok
        })
    });

    let Some(track) = track else {
        return Ok(None);
    };

    let plain_lyrics = match mxm.track_lyrics(TrackId::TrackId(track.track_id)).await {
        Ok(lyrics) => clean_plain_lyrics(&lyrics.lyrics_body),
        Err(error) if musixmatch_error_is_miss(&error) => None,
        Err(error) => return Err(error),
    };

    let synced_lyrics = match mxm
        .track_subtitle(
            TrackId::TrackId(track.track_id),
            SubtitleFormat::Lrc,
            if duration > 0.0 { Some(duration as f32) } else { None },
            if duration > 0.0 { Some(2.0) } else { None },
        )
        .await
    {
        Ok(subtitle) => clean_plain_lyrics(&subtitle.subtitle_body),
        Err(error) if musixmatch_error_is_miss(&error) => None,
        Err(error) => return Err(error),
    };

    if plain_lyrics.is_none() && synced_lyrics.is_none() && !track.instrumental {
        return Ok(None);
    }

    Ok(Some(LyricsResult {
        plain_lyrics,
        synced_lyrics,
        instrumental: track.instrumental,
        provider: Some("Musixmatch".to_string()),
        cached: false,
    }))
}
