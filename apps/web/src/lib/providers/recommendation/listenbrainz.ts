import { invoke } from '@tauri-apps/api/core';
import type { UnifiedRecommendation as TrackRecommendation } from '$lib/data/types';

function normalize(value: string): string {
	return value.trim().toLowerCase();
}

/**
 * Fetch personalised track recommendations from ListenBrainz.
 * The username is read from app settings in Rust.
 */
export async function fetchListenBrainzRecommendations({
	likedArtists = [],
	limit = 12
}: {
	likedArtists?: string[];
	limit?: number;
} = {}): Promise<TrackRecommendation[]> {
	// Fetch more than needed to account for metadata lookup gaps
	const fetchCount = Math.max(limit * 4, 50);
	const recData = await invoke<any>('lbz_cf_recommendations', { count: fetchCount });

	const mbids: Array<{ recording_mbid: string; score: number }> =
		recData?.payload?.mbids ?? [];
	if (mbids.length === 0) return [];

	const mbidList = mbids.map((m) => m.recording_mbid);
	const meta: Record<string, any> = await invoke('lbz_recording_metadata', { mbids: mbidList });

	const liked = new Set(likedArtists.map(normalize));
	const maxScore = mbids[0]?.score || 1;
	const results: TrackRecommendation[] = [];

	for (const { recording_mbid, score } of mbids) {
		const item = meta[recording_mbid];
		if (!item) continue;
		const title = String(item.recording?.name ?? '').trim();
		const artist = String(item.artist_credit_name ?? '').trim();
		if (!title || !artist) continue;

		const normalizedScore = maxScore > 0 ? score / maxScore : 0;
		const likedBoost = liked.has(normalize(artist)) ? 0.2 : 0;
		const finalScore = Math.min(normalizedScore + likedBoost, 1);

		results.push({
			id: `lbz-${recording_mbid}`,
			title,
			artist,
			score: Number(finalScore.toFixed(3)),
			matchScore: Number(normalizedScore.toFixed(3)),
			artistLiked: likedBoost > 0,
			genreScore: 0,
			url: `https://musicbrainz.org/recording/${recording_mbid}`,
			source: 'listenbrainz' as const,
		});

		if (results.length >= limit) break;
	}

	return results;
}

// ── Scrobbling ────────────────────────────────────────────────────────────────

/**
 * Submit a "playing now" notification to ListenBrainz (fire-and-forget).
 * The token is read from app settings in Rust.
 */
export function lbzNowPlaying(
	artist: string,
	title: string,
	album?: string,
	duration?: number
): void {
	invoke('lbz_now_playing', {
		artist,
		title,
		album: album ?? null,
		durationMs: duration != null ? Math.round(duration * 1000) : null
	}).catch(() => undefined);
}

/**
 * Submit a completed listen to ListenBrainz (fire-and-forget).
 * The token is read from app settings in Rust.
 * @param listenedAt Unix timestamp (seconds) when the track started playing.
 */
export function lbzScrobble(
	artist: string,
	title: string,
	listenedAt: number,
	album?: string,
	duration?: number
): void {
	invoke('lbz_scrobble', {
		artist,
		title,
		listenedAt,
		album: album ?? null,
		durationMs: duration != null ? Math.round(duration * 1000) : null
	}).catch(() => undefined);
}
