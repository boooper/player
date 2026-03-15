export const EQ_FREQUENCIES = [60, 230, 910, 3600, 14000] as const;
export const EQ_MIN_GAIN = -12;
export const EQ_MAX_GAIN = 12;

export type EqPresetId =
  | 'flat'
  | 'bass_boost'
  | 'treble_boost'
  | 'vocal_boost'
  | 'electronic'
  | 'small_speakers'
  | 'custom';

export type EqBandValues = [number, number, number, number, number];

export const DEFAULT_EQ_BANDS: EqBandValues = [0, 0, 0, 0, 0];

export const EQ_PRESETS: Array<{ id: EqPresetId; label: string; bands: EqBandValues }> = [
  { id: 'flat', label: 'Flat', bands: [0, 0, 0, 0, 0] },
  { id: 'bass_boost', label: 'Bass Boost', bands: [6, 4, 1, -1, -2] },
  { id: 'treble_boost', label: 'Treble Boost', bands: [-2, -1, 1, 4, 6] },
  { id: 'vocal_boost', label: 'Vocal Boost', bands: [-3, -1, 4, 5, 2] },
  { id: 'electronic', label: 'Electronic', bands: [5, 2, -1, 3, 5] },
  { id: 'small_speakers', label: 'Small Speakers', bands: [4, 3, 1, 2, 0] },
  { id: 'custom', label: 'Custom', bands: [0, 0, 0, 0, 0] },
];

export function clampEqBand(value: number): number {
  return Math.max(EQ_MIN_GAIN, Math.min(EQ_MAX_GAIN, Number.isFinite(value) ? value : 0));
}

export function normalizeEqBands(values: number[] | null | undefined): EqBandValues {
  return EQ_FREQUENCIES.map((_, index) => clampEqBand(values?.[index] ?? 0)) as EqBandValues;
}

export function findEqPresetId(values: number[]): EqPresetId {
  const normalized = normalizeEqBands(values);
  const preset = EQ_PRESETS.find((item) => item.id !== 'custom' && item.bands.every((gain, index) => gain === normalized[index]));
  return preset?.id ?? 'custom';
}
