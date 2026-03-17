use crate::lyrics::{clean_plain_lyrics, LyricsResult};
use genius_lyrics::get_lyrics_from_url;

fn slugify_genius_part(value: &str) -> String {
    let normalized = value
        .to_lowercase()
        .replace('\'', "")
        .replace('.', "")
        .replace('/', " ")
        .replace(':', " ")
        .replace('&', " and ");
    let mut out = String::new();
    let mut last_dash = false;
    for ch in normalized.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    out.trim_matches('-').to_string()
}

fn build_genius_url_candidates(artist: &str, title: &str) -> Vec<String> {
    let mut candidates = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let artist_slug = slugify_genius_part(artist);
    let title_slug = slugify_genius_part(title);

    for candidate_title in [title_slug.clone(), slugify_genius_part(&title.replace(" and ", " "))] {
        if artist_slug.is_empty() || candidate_title.is_empty() {
            continue;
        }
        let url = format!("https://genius.com/{}-{}-lyrics", artist_slug, candidate_title);
        if seen.insert(url.clone()) {
            candidates.push(url);
        }
    }

    candidates
}

pub async fn fetch_from_genius(artist: &str, title: &str, _album: &str, duration: f64) -> Option<LyricsResult> {
    for url in build_genius_url_candidates(artist, title) {
        let Ok(lyrics) = get_lyrics_from_url(&url).await else {
            continue;
        };
        let Some(plain_lyrics) = clean_plain_lyrics(&lyrics) else {
            continue;
        };

        // If we have a valid duration, synthesize an LRC by estimating a
        // words-per-second rate and placing timestamps proportional to
        // each line's word count. Also skip obvious section headers like
        // "Intro"/"Chorus"/"Verse" so timing stays aligned to actual
        // lyric lines.
        let synced_lyrics = if duration > 0.0 {
            let raw_lines: Vec<String> = plain_lyrics
                .lines()
                .map(str::trim)
                .map(|s| s.to_string())
                .collect();

            let mut lines: Vec<String> = Vec::new();
            let mut word_counts: Vec<usize> = Vec::new();

            let section_re = regex::Regex::new(r"(?i)^(intro|verse|chorus|bridge|outro|pre-chorus|interlude|hook|refrain)\b").unwrap();
            let bracket_re = regex::Regex::new(r"^\[.*\]$").unwrap();

            for l in raw_lines.into_iter() {
                let trimmed = l.trim();
                if trimmed.is_empty() {
                    continue;
                }
                if section_re.is_match(trimmed) || bracket_re.is_match(trimmed) {
                    continue;
                }
                let wc = trimmed.split_whitespace().count();
                if wc == 0 {
                    continue;
                }
                lines.push(trimmed.to_string());
                word_counts.push(wc);
            }

            if lines.is_empty() {
                None
            } else {
                let total_words: usize = word_counts.iter().sum();
                let wps = if total_words > 0 {
                    (total_words as f64) / duration
                } else {
                    // fallback: spread evenly by line count
                    1.0
                };

                let mut out = String::new();
                let mut elapsed = 0.0_f64;
                for (i, line) in lines.into_iter().enumerate() {
                    let wc = word_counts[i];
                    let dt = if total_words > 0 { (wc as f64) / wps } else { duration / (word_counts.len() as f64) };
                    // place timestamp at current elapsed (start of the line)
                    let total_ms = (elapsed * 1000.0).round() as u64;
                    let mm = total_ms / 60000;
                    let ss = (total_ms % 60000) / 1000;
                    let ms = total_ms % 1000;
                    out.push_str(&format!("[{mm:02}:{ss:02}.{:03}]{}\n", ms, line));
                    elapsed += dt;
                }
                Some(out.trim_end().to_string())
            }
        } else {
            None
        };

        return Some(LyricsResult {
            plain_lyrics: Some(plain_lyrics),
            synced_lyrics,
            instrumental: false,
            provider: Some("Genius".to_string()),
            cached: false,
        });
    }

    None
}
