//! Scale-factor decoding.
//! Port of LibAtrac9 scale_factors.c

use crate::bit_reader::BitReader;
use crate::error::{Error, Result};
use crate::huffman::ScaleFactorCodebooks;
use crate::structures::Channel;
use crate::tables::SCALE_FACTOR_WEIGHTS;

/// Read scale factors for one channel.
pub fn read_scale_factors(
    channel: &mut Channel,
    br: &mut BitReader,
    codebooks: &ScaleFactorCodebooks,
    extension_unit: usize,
    first_in_superframe: bool,
    quant_units_prev: usize,
    primary_sf: Option<&[i32; 31]>,
) -> Result<()> {
    channel.scale_factors.fill(0);
    channel.scale_factor_coding_mode = br.read(2) as i32;

    let mode = channel.scale_factor_coding_mode;
    let is_primary = channel.channel_index == 0;

    if is_primary {
        match mode {
            0 => read_vlc_delta_offset(channel, br, codebooks, extension_unit),
            1 => read_clc_offset(channel, br, extension_unit),
            2 => {
                if first_in_superframe {
                    return Err(Error::UnpackScaleFactorModeInvalid);
                }
                let prev = channel.scale_factors_prev;
                read_vlc_distance_to_baseline(
                    channel,
                    br,
                    codebooks,
                    &prev,
                    quant_units_prev,
                    extension_unit,
                )
            }
            3 => {
                if first_in_superframe {
                    return Err(Error::UnpackScaleFactorModeInvalid);
                }
                let prev = channel.scale_factors_prev;
                read_vlc_delta_offset_with_baseline(
                    channel,
                    br,
                    codebooks,
                    &prev,
                    quant_units_prev,
                    extension_unit,
                )
            }
            _ => return Err(Error::UnpackScaleFactorModeInvalid),
        }
    } else {
        let baseline = primary_sf.copied().unwrap_or(channel.scale_factors_prev);
        let prev = channel.scale_factors_prev;
        match mode {
            0 => read_vlc_delta_offset(channel, br, codebooks, extension_unit),
            1 => read_vlc_distance_to_baseline(
                channel,
                br,
                codebooks,
                &baseline,
                extension_unit,
                extension_unit,
            ),
            2 => read_vlc_delta_offset_with_baseline(
                channel,
                br,
                codebooks,
                &baseline,
                extension_unit,
                extension_unit,
            ),
            3 => {
                if first_in_superframe {
                    return Err(Error::UnpackScaleFactorModeInvalid);
                }
                read_vlc_distance_to_baseline(
                    channel,
                    br,
                    codebooks,
                    &prev,
                    quant_units_prev,
                    extension_unit,
                )
            }
            _ => return Err(Error::UnpackScaleFactorModeInvalid),
        }
    }

    for i in 0..extension_unit {
        if channel.scale_factors[i] < 0 || channel.scale_factors[i] > 31 {
            return Err(Error::UnpackScaleFactorOob);
        }
    }

    channel.scale_factors_prev = channel.scale_factors;
    Ok(())
}

fn read_clc_offset(channel: &mut Channel, br: &mut BitReader, extension_unit: usize) {
    let bit_length = br.read(2) as u32 + 2;
    let base_value = if bit_length < 5 { br.read(5) as i32 } else { 0 };
    for i in 0..extension_unit {
        channel.scale_factors[i] = br.read(bit_length) as i32 + base_value;
    }
}

fn read_vlc_delta_offset(
    channel: &mut Channel,
    br: &mut BitReader,
    codebooks: &ScaleFactorCodebooks,
    extension_unit: usize,
) {
    let weight_index = br.read(3) as usize;
    let weights = &SCALE_FACTOR_WEIGHTS[weight_index.min(7)];
    let base_value = br.read(5) as i32;
    let bit_length = br.read(2) as usize + 3; // 3..6 → index into unsigned tables
    let cb_idx = bit_length.min(6);
    let codebook = &codebooks.unsigned[cb_idx];

    channel.scale_factors[0] = br.read(bit_length as u32) as i32;
    for i in 1..extension_unit {
        let delta = codebook.read(br, false);
        channel.scale_factors[i] =
            (channel.scale_factors[i - 1] + delta) & (codebook.value_max - 1);
    }
    for i in 0..extension_unit {
        channel.scale_factors[i] += base_value - weights[i] as i32;
    }
}

fn read_vlc_distance_to_baseline(
    channel: &mut Channel,
    br: &mut BitReader,
    codebooks: &ScaleFactorCodebooks,
    baseline: &[i32; 31],
    baseline_length: usize,
    extension_unit: usize,
) {
    let bit_length = br.read(2) as usize + 2; // 2..5
    let cb_idx = bit_length.min(5);
    let codebook = &codebooks.signed[cb_idx];
    let unit_count = extension_unit.min(baseline_length);

    for i in 0..unit_count {
        let distance = codebook.read(br, true);
        channel.scale_factors[i] = (baseline[i] + distance) & 31;
    }
    for i in unit_count..extension_unit {
        channel.scale_factors[i] = br.read(5) as i32;
    }
}

fn read_vlc_delta_offset_with_baseline(
    channel: &mut Channel,
    br: &mut BitReader,
    codebooks: &ScaleFactorCodebooks,
    baseline: &[i32; 31],
    baseline_length: usize,
    extension_unit: usize,
) {
    let base_value = br.read_offset_binary(5);
    let bit_length = br.read(2) as usize + 1; // 1..4
    let cb_idx = bit_length.min(6);
    let codebook = &codebooks.unsigned[cb_idx];
    let unit_count = extension_unit.min(baseline_length);

    channel.scale_factors[0] = br.read(bit_length as u32) as i32;
    for i in 1..unit_count {
        let delta = codebook.read(br, false);
        channel.scale_factors[i] =
            (channel.scale_factors[i - 1] + delta) & (codebook.value_max - 1);
    }
    for i in 0..unit_count {
        channel.scale_factors[i] += base_value + baseline[i];
    }
    for i in unit_count..extension_unit {
        channel.scale_factors[i] = br.read(5) as i32;
    }
}
