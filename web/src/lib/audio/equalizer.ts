export const EQ_FREQUENCIES = [60, 170, 310, 600, 1000, 3000, 6000, 12000, 16000] as const;
export const EQ_MIN_GAIN = -12;
export const EQ_MAX_GAIN = 12;

export type EqPresetId =
  | 'flat'
  | 'bass_boost'
  | 'bass_reducer'
  | 'treble_boost'
  | 'treble_reducer'
  | 'vocal_boost'
  | 'electronic'
  | 'small_speakers'
  | 'rock'
  | 'pop'
  | 'jazz'
  | 'classical'
  | 'hip_hop'
  | 'acoustic'
  | 'lounge'
  | 'loudness'
  | 'custom';

export type EqBandValues = [number, number, number, number, number, number, number, number, number];
export type EqFrequencyValues = [number, number, number, number, number, number, number, number, number];

export const DEFAULT_EQ_BANDS: EqBandValues = [0, 0, 0, 0, 0, 0, 0, 0, 0];
export const DEFAULT_EQ_FREQUENCIES: EqFrequencyValues = [...EQ_FREQUENCIES] as EqFrequencyValues;

export function normalizeEqFrequencies(values: number[] | null | undefined): EqFrequencyValues {
  return EQ_FREQUENCIES.map((def, index) => {
    const v = values?.[index];
    return typeof v === 'number' && Number.isFinite(v) ? Math.max(20, Math.min(20000, v)) : def;
  }) as EqFrequencyValues;
}

export const EQ_PRESETS: Array<{ id: EqPresetId; label: string; frequencies: EqFrequencyValues; bands: EqBandValues }> = [
  {
    id: 'flat', label: 'Flat',
    frequencies: [60,  170, 310, 600,  1000, 3000, 6000, 12000, 16000],
    bands:       [ 0,   0,   0,   0,    0,    0,    0,    0,    0],
  },
  {
    id: 'bass_boost', label: 'Bass Boost',
    frequencies: [50,  100, 200, 400,  800,  2500, 5000, 10000, 14000],
    bands:       [ 6,   5,   3,   1,    0,   -1,   -1,   -2,   -2],
  },
  {
    id: 'bass_reducer', label: 'Bass Reducer',
    frequencies: [50,  100, 200, 400,  800,  2500, 5000, 10000, 14000],
    bands:       [-6,  -5,  -3,  -1,    0,    1,    1,    2,    2],
  },
  {
    id: 'treble_boost', label: 'Treble Boost',
    frequencies: [60,  170, 310, 600,  1200, 3500, 8000, 14000, 18000],
    bands:       [-2,  -1,   0,   0,    1,    2,    4,    5,    6],
  },
  {
    id: 'treble_reducer', label: 'Treble Reducer',
    frequencies: [60,  170, 310, 600,  1200, 3500, 8000, 14000, 18000],
    bands:       [ 2,   1,   0,   0,   -1,   -2,   -4,   -5,   -6],
  },
  {
    id: 'vocal_boost', label: 'Vocal Boost',
    frequencies: [60,  170, 310, 800,  2000, 4000, 7000, 12000, 16000],
    bands:       [-3,  -2,   0,   3,    5,    4,    2,    0,   -1],
  },
  {
    id: 'electronic', label: 'Electronic',
    frequencies: [40,  120, 310, 600,  1000, 2500, 7000, 14000, 18000],
    bands:       [ 5,   4,   1,  -1,    0,    1,    2,    4,    5],
  },
  {
    id: 'small_speakers', label: 'Small Speakers',
    frequencies: [100, 200, 400, 800,  1600, 3000, 6000, 10000, 14000],
    bands:       [ 4,   3,   2,   1,    0,    1,    2,    2,    1],
  },
  {
    id: 'rock', label: 'Rock',
    frequencies: [60,  150, 310, 600,  1200, 3500, 7000, 12000, 16000],
    bands:       [ 5,   3,   2,   0,   -1,    1,    3,    4,    4],
  },
  {
    id: 'pop', label: 'Pop',
    frequencies: [60,  200, 400, 1000, 2000, 5000, 8000, 14000, 18000],
    bands:       [-1,   2,   4,   4,    2,    0,   -1,   -1,   -2],
  },
  {
    id: 'jazz', label: 'Jazz',
    frequencies: [60,  150, 300, 600,  1200, 3000, 6000, 12000, 16000],
    bands:       [ 3,   2,   1,   2,    0,    0,    1,    2,    3],
  },
  {
    id: 'classical', label: 'Classical',
    frequencies: [60,  170, 310, 600,  1000, 3000, 6000, 12000, 16000],
    bands:       [ 4,   3,   2,   0,    0,    0,    1,    3,    4],
  },
  {
    id: 'hip_hop', label: 'Hip-Hop',
    frequencies: [50,  100, 250, 500,  1000, 3000, 6000, 12000, 16000],
    bands:       [ 6,   5,   2,  -1,    1,    0,   -1,    2,    3],
  },
  {
    id: 'acoustic', label: 'Acoustic',
    frequencies: [60,  170, 310, 800,  1600, 3200, 6400, 12000, 16000],
    bands:       [ 3,   2,   3,   2,    3,    2,    2,    2,    3],
  },
  {
    id: 'lounge', label: 'Lounge',
    frequencies: [60,  170, 400, 800,  2000, 4000, 7000, 12000, 16000],
    bands:       [-2,  -1,   0,   2,    3,    2,    1,   -1,   -2],
  },
  {
    id: 'loudness', label: 'Loudness',
    frequencies: [50,  100, 310, 600,  1000, 3000, 6000, 14000, 18000],
    bands:       [ 6,   4,   0,   0,    1,    0,    0,    3,    5],
  },
  {
    id: 'custom', label: 'Custom',
    frequencies: [60,  170, 310, 600,  1000, 3000, 6000, 12000, 16000],
    bands:       [ 0,   0,   0,   0,    0,    0,    0,    0,    0],
  },
];

export function clampEqBand(value: number): number {
  return Math.max(EQ_MIN_GAIN, Math.min(EQ_MAX_GAIN, Number.isFinite(value) ? value : 0));
}

export function normalizeEqBands(values: number[] | null | undefined): EqBandValues {
  return EQ_FREQUENCIES.map((_, index) => clampEqBand(values?.[index] ?? 0)) as EqBandValues;
}

export function findEqPresetId(bands: number[], freqs: number[]): EqPresetId {
  const normBands = normalizeEqBands(bands);
  const normFreqs = normalizeEqFrequencies(freqs);
  const preset = EQ_PRESETS.find(
    (item) =>
      item.id !== 'custom' &&
      item.bands.every((g, i) => g === normBands[i]) &&
      item.frequencies.every((f, i) => Math.round(f) === Math.round(normFreqs[i]))
  );
  return preset?.id ?? 'custom';
}
