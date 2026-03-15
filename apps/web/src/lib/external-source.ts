export type ExternalSource = 'deezer' | 'qobuz' | 'squidwtf';

export function getExternalSource(id: string | null | undefined): ExternalSource | null {
  if (!id?.startsWith('ext-')) return null;
  if (id.startsWith('ext-deezer-')) return 'deezer';
  if (id.startsWith('ext-qobuz-')) return 'qobuz';
  if (id.startsWith('ext-squidwtf-')) return 'squidwtf';
  return null;
}

export function getExternalSourceLabel(id: string | null | undefined): string | null {
  const source = getExternalSource(id);
  if (!source) return null;
  if (source === 'squidwtf') return 'SquidWTF';
  if (source === 'qobuz') return 'Qobuz';
  return 'Deezer';
}
