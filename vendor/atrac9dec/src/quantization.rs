//! Spectrum dequantization and scaling.
//! Port of LibAtrac9 quantization.c

use crate::tables::{
    QUANT_UNIT_TO_COEFF_COUNT, QUANT_UNIT_TO_COEFF_INDEX, QUANTIZER_FINE_STEP_SIZE,
    QUANTIZER_STEP_SIZE, SPECTRUM_SCALE,
};

const MAX_FRAME_SAMPLES: usize = 256;
const MAX_QUANT_UNITS: usize = 30;

/// Dequantize one channel's quantized spectra into floating-point coefficients.
pub fn dequantize_channel(
    quantized: &[i32; MAX_FRAME_SAMPLES],
    quantized_fine: &[i32; MAX_FRAME_SAMPLES],
    precisions: &[i32; MAX_QUANT_UNITS],
    precisions_fine: &[i32; MAX_QUANT_UNITS],
    coded_units: usize,
    spectra: &mut [f64; MAX_FRAME_SAMPLES],
) {
    spectra.fill(0.0);

    for band in 0..coded_units {
        let start = QUANT_UNIT_TO_COEFF_INDEX[band];
        let count = QUANT_UNIT_TO_COEFF_COUNT[band] as usize;
        let step = QUANTIZER_STEP_SIZE[precisions[band].clamp(0, 15) as usize];
        // Fine step: same table, using fine precision
        // OpenAntiGrav patch: LibAtrac9 scales the fine residual by its own
        // `QuantizerFineStepSize` table; 0.9.1 reused the coarse table.
        let step_fine = QUANTIZER_FINE_STEP_SIZE[precisions_fine[band].clamp(0, 15) as usize];

        for sb in 0..count {
            let idx = start + sb;
            let coarse = quantized[idx] as f64 * step;
            let fine = quantized_fine[idx] as f64 * step_fine;
            spectra[idx] = coarse + fine;
        }
    }
}

/// Apply scale factors to the spectrum of one channel.
pub fn scale_spectrum(
    spectra: &mut [f64; MAX_FRAME_SAMPLES],
    scale_factors: &[i32; 31],
    quant_unit_count: usize,
) {
    for i in 0..quant_unit_count {
        let sf = scale_factors[i].clamp(0, 31) as usize;
        let scale = SPECTRUM_SCALE[sf];
        let start = QUANT_UNIT_TO_COEFF_INDEX[i];
        let end = QUANT_UNIT_TO_COEFF_INDEX[i + 1];
        for sb in start..end {
            spectra[sb] *= scale;
        }
    }
}
