const FEATURE_PATTERNS = [
  /\s+feat\.?\s+/gi,
  /\s+featuring\s+/gi,
  /\s+ft\.?\s+/gi
];

const COLLAB_SEPARATORS = [
  /\s+•\s+/g,
  /\s+\|\s+/g,
  /\s+\/\s+/g,
  /\s+;\s+/g
];

function normalizeWhitespace(value: string): string {
  return value.trim().replace(/\s+/g, ' ');
}

export function splitSongArtists(value: string): string[] {
  if (!value.trim()) return [];

  let normalized = value;
  for (const pattern of FEATURE_PATTERNS) {
    normalized = normalized.replace(pattern, ' • ');
  }
  for (const pattern of COLLAB_SEPARATORS) {
    normalized = normalized.replace(pattern, ' • ');
  }

  return Array.from(
    new Set(
      normalized
        .split('•')
        .map((part) => normalizeWhitespace(part))
        .filter(Boolean)
    )
  );
}

export function primarySongArtist(value: string): string {
  return splitSongArtists(value)[0] ?? normalizeWhitespace(value);
}

export function formatSongArtists(value: string): string {
  const artists = splitSongArtists(value);
  return artists.length ? artists.join(' · ') : normalizeWhitespace(value);
}
