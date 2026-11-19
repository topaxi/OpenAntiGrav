//! Bit allocation / gradient / precision calculation.
//! Port of LibAtrac9 bit_allocation.c

use crate::error::Result;
use crate::structures::{Block, Channel};

static BASE_CURVE: [u8; 48] = [
    1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 4, 4, 5, 5, 6, 7, 8, 9, 10, 11, 12, 13, 15, 16, 18, 19,
    20, 21, 22, 23, 24, 25, 26, 26, 27, 27, 28, 28, 28, 29, 29, 29, 29, 30, 30, 30, 30,
];

/// Precomputed gradient curves [length-1][i]
fn gradient_curve(unit_count: usize) -> Vec<u8> {
    let base_len = BASE_CURVE.len();
    let length = unit_count.clamp(1, base_len);
    (0..length)
        .map(|i| BASE_CURVE[i * base_len / length])
        .collect()
}

pub fn create_gradient(block: &mut Block) -> Result<()> {
    let value_count = block.gradient_end_value - block.gradient_start_value;
    let unit_count = block.gradient_end_unit - block.gradient_start_unit;

    for i in 0..block.gradient_end_unit.max(0) as usize {
        if i < block.gradient.len() {
            block.gradient[i] = block.gradient_start_value;
        }
    }
    let qcount = block.quantization_unit_count;
    for i in block.gradient_end_unit.max(0) as usize..=qcount {
        if i < block.gradient.len() {
            block.gradient[i] = block.gradient_end_value;
        }
    }

    if unit_count <= 0 || value_count == 0 {
        return Ok(());
    }

    let curve = gradient_curve(unit_count as usize);
    let start = block.gradient_start_unit.max(0) as usize;
    let end = block.gradient_end_unit.max(0) as usize;

    if value_count <= 0 {
        let scale = (-value_count - 1) as f64 / 31.0;
        let base_val = block.gradient_start_value - 1;
        for i in start..end {
            let c = *curve.get(i - start).unwrap_or(&0) as f64;
            if i < block.gradient.len() {
                block.gradient[i] = base_val - (c * scale) as i32;
            }
        }
    } else {
        let scale = (value_count - 1) as f64 / 31.0;
        let base_val = block.gradient_start_value + 1;
        for i in start..end {
            let c = *curve.get(i - start).unwrap_or(&0) as f64;
            if i < block.gradient.len() {
                block.gradient[i] = base_val + (c * scale) as i32;
            }
        }
    }
    Ok(())
}

pub fn calculate_mask(channel: &mut Channel, quant_unit_count: usize) {
    channel.precision_mask.fill(0);
    for i in 1..quant_unit_count {
        let delta = channel.scale_factors[i] - channel.scale_factors[i - 1];
        if delta > 1 {
            channel.precision_mask[i] += (delta - 1).min(5);
        } else if delta < -1 {
            channel.precision_mask[i - 1] += ((-delta) - 1).min(5);
        }
    }
}

pub fn calculate_precisions(channel: &mut Channel, block: &Block) {
    let qcount = block.quantization_unit_count;
    if block.gradient_mode != 0 {
        for i in 0..qcount {
            channel.precisions[i] =
                channel.scale_factors[i] + channel.precision_mask[i] - block.gradient[i];
            if channel.precisions[i] > 0 {
                match block.gradient_mode {
                    1 => channel.precisions[i] /= 2,
                    2 => channel.precisions[i] = 3 * channel.precisions[i] / 8,
                    3 => channel.precisions[i] /= 4,
                    _ => {}
                }
            }
        }
    } else {
        for i in 0..qcount {
            channel.precisions[i] = channel.scale_factors[i] - block.gradient[i];
        }
    }

    for i in 0..qcount {
        if channel.precisions[i] < 1 {
            channel.precisions[i] = 1;
        }
    }
    for i in 0..block.gradient_boundary.max(0) as usize {
        if i < qcount {
            channel.precisions[i] += 1;
        }
    }
    for i in 0..qcount {
        channel.precisions_fine[i] = 0;
        if channel.precisions[i] > 15 {
            channel.precisions_fine[i] = channel.precisions[i] - 15;
            channel.precisions[i] = 15;
        }
    }
}

pub fn calculate_spectrum_codebook_index(channel: &mut Channel, quant_units: usize, high_sample_rate: bool) {
    channel.codebook_set.fill(0);
    if quant_units <= 1 || high_sample_rate {
        return;
    }

    // Temporarily set sf[quant_units] = sf[quant_units-1] so last index is non-special
    let original = channel.scale_factors[quant_units];
    channel.scale_factors[quant_units] = channel.scale_factors[quant_units - 1];

    let mut avg = 0;
    if quant_units > 12 {
        for i in 0..12 {
            avg += channel.scale_factors[i];
        }
        avg = (avg + 6) / 12;
    }

    for i in 8..quant_units {
        let prev_sf = channel.scale_factors[i - 1];
        let next_sf = channel.scale_factors[i + 1];
        let min_sf = prev_sf.min(next_sf);
        if channel.scale_factors[i] - min_sf >= 3
            || channel.scale_factors[i] - prev_sf + channel.scale_factors[i] - next_sf >= 3
        {
            channel.codebook_set[i] = 1;
        }
    }

    for i in 12..quant_units {
        if channel.codebook_set[i] == 0 {
            let min_sf = channel.scale_factors[i - 1].min(channel.scale_factors[i + 1]);
            let extra = if crate::tables::QUANT_UNIT_TO_COEFF_COUNT[i] == 16 { 1 } else { 0 };
            if channel.scale_factors[i] - min_sf >= 2 && channel.scale_factors[i] >= avg - extra {
                channel.codebook_set[i] = 1;
            }
        }
    }

    channel.scale_factors[quant_units] = original;
}
