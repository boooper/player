use biquad::{Coefficients, DirectForm1, ToHertz, Type};

pub const EQ_FREQUENCIES: [f32; 9] = [60.0, 170.0, 310.0, 600.0, 1000.0, 3000.0, 6000.0, 12000.0, 16000.0];
pub const EQ_BANDS: usize = 9;
const EQ_Q: f32 = 1.0;

pub fn normalize_eq_bands(bands: &[f32]) -> [f32; EQ_BANDS] {
    let mut normalized = [0.0; EQ_BANDS];
    for (index, slot) in normalized.iter_mut().enumerate() {
        *slot = bands.get(index).copied().unwrap_or(0.0).clamp(-12.0, 12.0);
    }
    normalized
}

pub fn normalize_eq_frequencies(frequencies: &[f32]) -> [f32; EQ_BANDS] {
    let mut normalized = EQ_FREQUENCIES;
    for (index, slot) in normalized.iter_mut().enumerate() {
        *slot = frequencies.get(index).copied().unwrap_or(EQ_FREQUENCIES[index]).clamp(20.0, 20000.0);
    }
    normalized
}

/// Computes per-band loudness compensation gains (dB) based on the current volume.
/// At lower volumes the human ear perceives less bass and treble (equal-loudness /
/// Fletcher-Munson effect), so we boost those frequencies to compensate.
pub fn compute_loudness_compensation(volume: f32) -> [f32; EQ_BANDS] {
    // Maximum per-band boosts (dB) applied at the lowest perceptible volume.
    // Mapped to EQ_FREQUENCIES: [60, 170, 310, 600, 1000, 3000, 6000, 12000, 16000]
    const MAX_COMP: [f32; EQ_BANDS] = [9.0, 5.0, 2.0, 1.0, 0.0, 0.0, 1.0, 3.0, 5.0];
    // Strength: 0.0 at vol=1.0 (reference), 1.0 at vol≈0.01 (−40 dB)
    let db_down = -20.0 * volume.max(0.001_f32).log10();
    let strength = (db_down / 40.0).clamp(0.0, 1.0);
    let mut comp = [0.0_f32; EQ_BANDS];
    for (c, max) in comp.iter_mut().zip(MAX_COMP.iter()) {
        *c = max * strength;
    }
    comp
}

pub fn build_filter_bank(
    sample_rate: u32,
    channels: u16,
    frequencies: [f32; EQ_BANDS],
    bands: [f32; EQ_BANDS],
) -> Result<Vec<Vec<DirectForm1<f32>>>, String> {
    let mut bank = Vec::with_capacity(channels as usize);
    for _ in 0..channels as usize {
        bank.push(build_filter_chain(sample_rate, frequencies, bands)?);
    }
    Ok(bank)
}

fn build_filter_chain(
    sample_rate: u32,
    frequencies: [f32; EQ_BANDS],
    bands: [f32; EQ_BANDS],
) -> Result<Vec<DirectForm1<f32>>, String> {
    let rate = (sample_rate as f32).hz();
    let mut filters = Vec::with_capacity(EQ_BANDS);

    for (index, gain) in bands.into_iter().enumerate() {
        let frequency = frequencies[index].hz();
        let filter_type = if index == 0 {
            Type::LowShelf(gain)
        } else if index == EQ_BANDS - 1 {
            Type::HighShelf(gain)
        } else {
            Type::PeakingEQ(gain)
        };
        let coeffs = Coefficients::<f32>::from_params(filter_type, rate, frequency, EQ_Q)
            .map_err(|error| format!("EQ coefficient error: {error:?}"))?;
        filters.push(DirectForm1::new(coeffs));
    }

    Ok(filters)
}
