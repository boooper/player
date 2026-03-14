/**
 * ListenBrainz recommendation provider.
 *
 * Uses the collaborative-filter (CF) recommendation endpoint to fetch
 * personalised track recommendations for a ListenBrainz user, then
 * enriches the results with recording metadata via the metadata API.
 *
 * Also provides scrobbling (submit-listens) using the user's API token.
 *
 * Endpoints used:
 *   GET  https://api.listenbrainz.org/1/cf/recommendation/user/{username}/recording
 *   POST https://api.listenbrainz.org/1/metadata/recording/
 *   POST https://api.listenbrainz.org/1/submit-listens
 */

import type { TrackRecommendation } from '$lib/recommendation';

const LBZ_API = 'https://api.listenbrainz.org/1';

function normalize(value: string): string {
	return value.trim().toLowerCase();
}

/**
 * Fetch personalised track recommendations from ListenBrainz for the given user.
 * The seed artist/title are not used — LBz CF recommendations are based on the
 * user's full listening history, making them naturally personalised.
 */
export async function fetchListenBrainzRecommendations({
	username,
	likedArtists = [],
	limit = 12
}: {
	username: string;
	likedArtists?: string[];
	limit?: number;
}): Promise<TrackRecommendation[]> {
	if (!username.trim()) throw new Error('ListenBrainz username is required');

	// Fetch more than needed to account for metadata lookup gaps
	const fetchCount = Math.max(limit * 4, 50);
	const recResp = await fetch(
		`${LBZ_API}/cf/recommendation/user/${encodeURIComponent(username.trim())}/recording?count=${fetchCount}`
	);
	if (!recResp.ok) {
		if (recResp.status === 404)
			throw new Error(`ListenBrainz user "${username}" not found`);
		throw new Error(`ListenBrainz request failed with status ${recResp.status}`);
	}
	const recData = await recResp.json();
	const mbids: Array<{ recording_mbid: string; score: number }> =
		recData?.payload?.mbids ?? [];
	if (mbids.length === 0) return [];

	// Look up recording metadata for the returned MBIDs
	const mbidList = mbids.map((m) => m.recording_mbid);
	const metaResp = await fetch(`${LBZ_API}/metadata/recording/`, {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({ recording_mbids: mbidList })
	});
	if (!metaResp.ok)
		throw new Error(`ListenBrainz metadata request failed with status ${metaResp.status}`);
	const meta: Record<string, any> = await metaResp.json();

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
			url: `https://musicbrainz.org/recording/${recording_mbid}`
		});

		if (results.length >= limit) break;
	}

	return results;
}

// ── Scrobbling ────────────────────────────────────────────────────────────────

function submitListen(
	token: string,
	listenType: 'single' | 'playing_now',
	payload: {
		artist: string;
		title: string;
		album?: string;
		duration?: number;
		listenedAt?: number;
	}
): void {
	if (!token.trim()) return;

	const trackMeta: Record<string, unknown> = {
		artist_name: payload.artist,
		track_name: payload.title,
		...(payload.album ? { release_name: payload.album } : {}),
		...(payload.duration
			? { additional_info: { duration_ms: Math.round(payload.duration * 1000) } }
			: {})
	};

	const listen: Record<string, unknown> = { track_metadata: trackMeta };
	if (listenType === 'single' && payload.listenedAt != null) {
		listen.listened_at = payload.listenedAt;
	}

	fetch(`${LBZ_API}/submit-listens`, {
		method: 'POST',
		headers: {
			'Content-Type': 'application/json',
			Authorization: `Token ${token.trim()}`
		},
		body: JSON.stringify({ listen_type: listenType, payload: [listen] })
	}).catch(() => undefined);
}

/**
 * Submit a "playing now" notification to ListenBrainz (fire-and-forget).
 */
export function lbzNowPlaying(
	token: string,
	artist: string,
	title: string,
	album?: string,
	duration?: number
): void {
	submitListen(token, 'playing_now', { artist, title, album, duration });
}

/**
 * Submit a completed listen to ListenBrainz (fire-and-forget).
 * @param listenedAt Unix timestamp (seconds) when the track started playing.
 */
export function lbzScrobble(
	token: string,
	artist: string,
	title: string,
	listenedAt: number,
	album?: string,
	duration?: number
): void {
	submitListen(token, 'single', { artist, title, album, duration, listenedAt });
}
