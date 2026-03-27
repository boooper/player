/** Extracts the source slug from an `ext-{source}-{id}` style ID. */
export function getExternalSource(id: string | null | undefined): string | null {
  if (!id?.startsWith('ext-')) return null;
  const slug = id.slice(4, id.indexOf('-', 4));
  return slug || null;
}

/** Returns a display label for the source, capitalising the slug by default. */
export function getExternalSourceLabel(id: string | null | undefined): string | null {
  const source = getExternalSource(id);
  if (!source) return null;
  // Known overrides for non-obvious capitalisation.
  const overrides: Record<string, string> = {
    squidwtf: 'SquidWTF',
    qobuz: 'Qobuz',
    deezer: 'Deezer',
  };
  return overrides[source] ?? source.charAt(0).toUpperCase() + source.slice(1);
}
