use biquad::{Coefficients, DirectForm1, ToHertz, Type};

pub const EQ_FREQUENCIES: [f32; 5] = [60.0, 230.0, 910.0, 3600.0, 14000.0];
pub const EQ_BANDS: usize = 5;
const EQ_Q: f32 = 1.0;

pub fn normalize_eq_bands(bands: &[f32]) -> [f32; EQ_BANDS] {
    let mut normalized = [0.0; EQ_BANDS];
    for (index, slot) in normalized.iter_mut().enumerate() {
        *slot = bands.get(index).copied().unwrap_or(0.0).clamp(-12.0, 12.0);
    }
    normalized
}

pub fn build_filter_bank(
    sample_rate: u32,
    channels: u16,
    bands: [f32; EQ_BANDS],
) -> Result<Vec<Vec<DirectForm1<f32>>>, String> {
    let mut bank = Vec::with_capacity(channels as usize);
    for _ in 0..channels as usize {
        bank.push(build_filter_chain(sample_rate, bands)?);
    }
    Ok(bank)
}

fn build_filter_chain(
    sample_rate: u32,
    bands: [f32; EQ_BANDS],
) -> Result<Vec<DirectForm1<f32>>, String> {
    let rate = (sample_rate as f32).hz();
    let mut filters = Vec::with_capacity(EQ_BANDS);

    for (index, gain) in bands.into_iter().enumerate() {
        let frequency = EQ_FREQUENCIES[index].hz();
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
