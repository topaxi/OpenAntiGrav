//! Frame / block unpacking from the ATRAC9 bitstream.

use crate::band_extension::{BEX_DATA_LENGTHS, BEX_ENCODED_VALUE_COUNTS, BEX_GROUP_INFO};
use crate::bit_allocation::{calculate_mask, calculate_spectrum_codebook_index, create_gradient};
use crate::bit_reader::BitReader;
use crate::error::{Error, Result};
use crate::huffman::{ScaleFactorCodebooks, SpectrumCodebooks, decode_huffman_values};
use crate::scale_factors::read_scale_factors;
use crate::structures::{Block, BlockType, Channel, Frame};
use crate::tables::{
    BAND_TO_QUANT_UNIT_COUNT, MAX_BAND_COUNT, QUANT_UNIT_TO_CODEBOOK_INDEX,
    QUANT_UNIT_TO_COEFF_COUNT, QUANT_UNIT_TO_COEFF_INDEX,
};

pub struct UnpackContext {
    pub sf_codebooks: ScaleFactorCodebooks,
    pub spectrum_codebooks: SpectrumCodebooks,
}

impl UnpackContext {
    pub fn new() -> Self {
        Self {
            sf_codebooks: ScaleFactorCodebooks::new(),
            spectrum_codebooks: SpectrumCodebooks::new(),
        }
    }
}

impl Default for UnpackContext {
    fn default() -> Self {
        Self::new()
    }
}

pub fn unpack_frame(
    frame: &mut Frame,
    br: &mut BitReader,
    sample_rate_index: u8,
    ctx: &UnpackContext,
) -> Result<()> {
    crate::trace::stage("unpack_frame_start", br.position());
    for block in &mut frame.blocks {
        unpack_block(block, br, sample_rate_index, ctx)?;
        if block.first_in_superframe && frame.index_in_superframe > 0 {
            return Err(Error::UnpackSuperframeFlagInvalid);
        }
    }
    Ok(())
}

fn unpack_block(
    block: &mut Block,
    br: &mut BitReader,
    sample_rate_index: u8,
    ctx: &UnpackContext,
) -> Result<()> {
    read_block_header(block, br)?;
    crate::trace::stage("block_header", br.position());
    if block.block_type == BlockType::Lfe {
        unpack_lfe_block(block, br)?;
    } else {
        unpack_standard_block(block, br, sample_rate_index, ctx)?;
    }
    br.align(8);
    crate::trace::stage("align_8", br.position());
    Ok(())
}

fn read_block_header(block: &mut Block, br: &mut BitReader) -> Result<()> {
    block.first_in_superframe = br.read(1) == 0;
    block.reuse_band_params = br.read(1) != 0;
    if block.first_in_superframe && block.reuse_band_params && block.block_type != BlockType::Lfe {
        return Err(Error::UnpackReuseBandParamsInvalid);
    }
    Ok(())
}

fn unpack_standard_block(
    block: &mut Block,
    br: &mut BitReader,
    sample_rate_index: u8,
    ctx: &UnpackContext,
) -> Result<()> {
    if !block.reuse_band_params {
        read_band_params(block, br, sample_rate_index)?;
        crate::trace::stage("band_params", br.position());
    }
    read_gradient_params(block, br)?;
    crate::trace::stage("gradient_params", br.position());
    create_gradient(block)?;
    crate::trace::stage("create_gradient", br.position());
    read_stereo_params(block, br)?;
    crate::trace::stage("stereo_params", br.position());
    read_extension_params(block, br)?;
    crate::trace::stage("extension_params", br.position());

    let high_rate = sample_rate_index >= 8;

    for ch_idx in 0..block.channel_count {
        // Update coded units
        if block.primary_channel_index == ch_idx {
            block.channels[ch_idx].coded_quant_units = block.quantization_unit_count;
        } else {
            block.channels[ch_idx].coded_quant_units = block.stereo_quantization_unit;
        }

        let extension_unit = block.extension_unit;
        let first = block.first_in_superframe;
        let qprev = block.quantization_units_prev;
        // Baseline must be the *current frame* primary SF (after ch0 is decoded)
        let primary_baseline = if ch_idx == 0 {
            None
        } else {
            Some(block.channels[0].scale_factors)
        };

        read_scale_factors(
            &mut block.channels[ch_idx],
            br,
            &ctx.sf_codebooks,
            extension_unit,
            first,
            qprev,
            primary_baseline.as_ref(),
        )?;
        crate::trace::stage(&format!("scale_factors_ch{}", ch_idx), br.position());

        calculate_mask(&mut block.channels[ch_idx], block.quantization_unit_count);
        // Need block info for precisions - pass by extracting needed fields
        {
            let mode = block.gradient_mode;
            let boundary = block.gradient_boundary;
            let gradient = block.gradient;
            let qcount = block.quantization_unit_count;
            let ch = &mut block.channels[ch_idx];
            // inline precision calc to avoid borrow issues
            if mode != 0 {
                for i in 0..qcount {
                    ch.precisions[i] = ch.scale_factors[i] + ch.precision_mask[i] - gradient[i];
                    if ch.precisions[i] > 0 {
                        match mode {
                            1 => ch.precisions[i] /= 2,
                            2 => ch.precisions[i] = 3 * ch.precisions[i] / 8,
                            3 => ch.precisions[i] /= 4,
                            _ => {}
                        }
                    }
                }
            } else {
                for i in 0..qcount {
                    ch.precisions[i] = ch.scale_factors[i] - gradient[i];
                }
            }
            for i in 0..qcount {
                if ch.precisions[i] < 1 {
                    ch.precisions[i] = 1;
                }
            }
            for i in 0..boundary.max(0) as usize {
                if i < qcount {
                    ch.precisions[i] += 1;
                }
            }
            for i in 0..qcount {
                ch.precisions_fine[i] = 0;
                if ch.precisions[i] > 15 {
                    ch.precisions_fine[i] = ch.precisions[i] - 15;
                    ch.precisions[i] = 15;
                }
            }
        }

        let coded = block.channels[ch_idx].coded_quant_units;
        calculate_spectrum_codebook_index(&mut block.channels[ch_idx], coded, high_rate);

        read_spectra(
            &mut block.channels[ch_idx],
            br,
            &ctx.spectrum_codebooks,
            high_rate,
        )?;
        crate::trace::stage(&format!("spectra_ch{}", ch_idx), br.position());
        read_spectra_fine(&mut block.channels[ch_idx], br)?;
        crate::trace::stage(&format!("spectra_fine_ch{}", ch_idx), br.position());
    }

    block.quantization_units_prev = if block.band_extension_enabled {
        block.extension_unit
    } else {
        block.quantization_unit_count
    };
    Ok(())
}

fn read_band_params(block: &mut Block, br: &mut BitReader, sample_rate_index: u8) -> Result<()> {
    use crate::tables::{MAX_EXTENSION_BAND, MIN_BAND_COUNT};
    let high_rate = sample_rate_index >= 8;
    let hr = if high_rate { 1 } else { 0 };
    let min_bands = MIN_BAND_COUNT[hr] as i32;
    let max_ext = MAX_EXTENSION_BAND[hr] as i32;
    let max_bands = MAX_BAND_COUNT[sample_rate_index as usize] as i32;

    block.band_count = br.read(4) as i32 + min_bands;
    block.quantization_unit_count = BAND_TO_QUANT_UNIT_COUNT[block.band_count as usize] as usize;

    if block.band_count > max_bands {
        return Err(Error::UnpackBandParamsInvalid);
    }

    if block.block_type == BlockType::Stereo {
        block.stereo_band = br.read(4) as i32 + min_bands;
        block.stereo_quantization_unit =
            BAND_TO_QUANT_UNIT_COUNT[block.stereo_band as usize] as usize;
    } else {
        block.stereo_band = block.band_count;
        block.stereo_quantization_unit = block.quantization_unit_count;
    }

    if block.stereo_band > block.band_count {
        return Err(Error::UnpackBandParamsInvalid);
    }

    // Band extension flag is part of band params
    block.band_extension_enabled = br.read(1) != 0;
    if block.band_extension_enabled {
        block.extension_band = br.read(4) as i32 + min_bands;
        if block.extension_band < block.band_count || block.extension_band > max_ext {
            return Err(Error::UnpackBandParamsInvalid);
        }
        block.extension_unit = BAND_TO_QUANT_UNIT_COUNT[block.extension_band as usize] as usize;
    } else {
        block.extension_band = block.band_count;
        block.extension_unit = block.quantization_unit_count;
    }
    Ok(())
}

fn read_gradient_params(block: &mut Block, br: &mut BitReader) -> Result<()> {
    block.gradient_mode = br.read(2) as i32;
    if block.gradient_mode > 0 {
        block.gradient_end_unit = 31;
        block.gradient_end_value = 31;
        block.gradient_start_unit = br.read(5) as i32;
        block.gradient_start_value = br.read(5) as i32;
    } else {
        block.gradient_start_unit = br.read(6) as i32;
        block.gradient_end_unit = br.read(6) as i32 + 1;
        block.gradient_start_value = br.read(5) as i32;
        block.gradient_end_value = br.read(5) as i32;
    }
    block.gradient_boundary = br.read(4) as i32;

    if block.gradient_boundary as usize > block.quantization_unit_count {
        return Err(Error::UnpackGradBoundaryInvalid);
    }
    if block.gradient_start_unit < 0 || block.gradient_start_unit >= 48 {
        return Err(Error::UnpackGradStartUnitOob);
    }
    if block.gradient_end_unit < 0 || block.gradient_end_unit >= 48 {
        return Err(Error::UnpackGradEndUnitOob);
    }
    if block.gradient_start_unit > block.gradient_end_unit {
        return Err(Error::UnpackGradEndUnitInvalid);
    }
    if block.gradient_start_value < 0 || block.gradient_start_value >= 32 {
        return Err(Error::UnpackGradStartValueOob);
    }
    if block.gradient_end_value < 0 || block.gradient_end_value >= 32 {
        return Err(Error::UnpackGradEndValueOob);
    }
    Ok(())
}

fn read_stereo_params(block: &mut Block, br: &mut BitReader) -> Result<()> {
    if block.block_type != BlockType::Stereo {
        return Ok(());
    }
    block.primary_channel_index = br.read(1) as usize;
    block.has_joint_stereo_signs = br.read(1) != 0;
    // OpenAntiGrav patch: LibAtrac9 reads a sign only for the intensity units
    // (from the stereo unit up) and clears them when absent; 0.9.1 read one
    // per unit from zero, which desynchronised every stream that codes signs.
    if block.has_joint_stereo_signs {
        for i in block.stereo_quantization_unit..block.quantization_unit_count {
            block.joint_stereo_signs[i] = br.read(1) as i32;
        }
    } else {
        block.joint_stereo_signs.fill(0);
    }
    Ok(())
}

// OpenAntiGrav patch: a port of LibAtrac9's `ReadExtensionParams`,
// `BexReadHeader` and `BexReadData`. 0.9.1 read the stereo header as two
// unconditional bits and skipped the payload, so the values band extension
// needs were never read.
fn read_extension_params(block: &mut Block, br: &mut BitReader) -> Result<()> {
    let mut bex_band = 0;
    if block.band_extension_enabled {
        bex_band = BEX_GROUP_INFO[block.quantization_unit_count - 13].2;
        if block.block_type == BlockType::Stereo {
            bex_read_header(&mut block.channels[1], br, bex_band);
        } else {
            br.skip(1);
        }
    }
    block.has_extension_data = br.read(1) != 0;
    if !block.has_extension_data {
        return Ok(());
    }
    if !block.band_extension_enabled {
        block.bex_mode = br.read(2) as i32;
        block.bex_data_length = br.read(5) as i32;
        br.skip(block.bex_data_length as u32);
        return Ok(());
    }

    bex_read_header(&mut block.channels[0], br, bex_band);
    block.bex_data_length = br.read(5) as i32;
    if block.bex_data_length == 0 {
        return Ok(());
    }
    let bex_data_end = br.position() + block.bex_data_length as usize;
    bex_read_data(&mut block.channels[0], br, bex_band);
    if block.block_type == BlockType::Stereo {
        bex_read_data(&mut block.channels[1], br, bex_band);
    }
    if br.position() > bex_data_end {
        return Err(Error::UnpackExtensionDataInvalid);
    }
    Ok(())
}

fn bex_read_header(channel: &mut Channel, br: &mut BitReader, bex_band: usize) {
    let bex_mode = br.read(2) as i32;
    channel.bex_mode = if bex_band > 2 { bex_mode } else { 4 };
    channel.bex_value_count = BEX_ENCODED_VALUE_COUNTS[channel.bex_mode as usize][bex_band] as i32;
}

fn bex_read_data(channel: &mut Channel, br: &mut BitReader, bex_band: usize) {
    for i in 0..channel.bex_value_count as usize {
        let length = BEX_DATA_LENGTHS[channel.bex_mode as usize][bex_band][i];
        channel.bex_values[i] = br.read(length) as i32;
    }
}

fn read_spectra(
    channel: &mut Channel,
    br: &mut BitReader,
    spectrum_cbs: &SpectrumCodebooks,
    high_sample_rate: bool,
) -> Result<()> {
    channel.quantized_spectra.fill(0);
    let max_huff_precision = if high_sample_rate { 1 } else { 7 };

    for i in 0..channel.coded_quant_units {
        let _pos0 = br.position();
        let subband_count = QUANT_UNIT_TO_COEFF_COUNT[i] as usize;
        let precision = (channel.precisions[i] + 1) as usize;
        if precision <= max_huff_precision {
            let cb_set = channel.codebook_set[i] as usize;
            let quant_group = QUANT_UNIT_TO_CODEBOOK_INDEX[i] as usize;
            let huff = spectrum_cbs.get(cb_set, precision, quant_group);
            if huff.length == 0 {
                let start = QUANT_UNIT_TO_COEFF_INDEX[i];
                for j in start..QUANT_UNIT_TO_COEFF_INDEX[i + 1] {
                    channel.quantized_spectra[j] = br.read_signed(precision as u32);
                }

                continue;
            }
            let group_count = subband_count >> huff.value_count_power;
            let mut values = [0i32; 16];
            for j in 0..group_count {
                values[j] = huff.read(br, false);
            }
            decode_huffman_values(
                &mut channel.quantized_spectra,
                QUANT_UNIT_TO_COEFF_INDEX[i],
                subband_count,
                huff,
                &values,
            );
        } else {
            let start = QUANT_UNIT_TO_COEFF_INDEX[i];
            for j in start..QUANT_UNIT_TO_COEFF_INDEX[i + 1] {
                channel.quantized_spectra[j] = br.read_signed(precision as u32);
            }
        }
    }
    Ok(())
}

fn read_spectra_fine(channel: &mut Channel, br: &mut BitReader) -> Result<()> {
    channel.quantized_spectra_fine.fill(0);
    for i in 0..channel.coded_quant_units {
        if channel.precisions_fine[i] > 0 {
            let overflow_bits = (channel.precisions_fine[i] + 1) as u32;
            let start = QUANT_UNIT_TO_COEFF_INDEX[i];
            let end = QUANT_UNIT_TO_COEFF_INDEX[i + 1];
            for j in start..end {
                channel.quantized_spectra_fine[j] = br.read_signed(overflow_bits);
            }
        }
    }
    Ok(())
}

// OpenAntiGrav patch: a port of LibAtrac9's `UnpackLfeBlock` and its three
// helpers. 0.9.1 carried a placeholder that read the wrong number of bits, so
// every 5.1 and 7.1 stream desynchronised at its LFE block.
fn unpack_lfe_block(block: &mut Block, br: &mut BitReader) -> Result<()> {
    block.quantization_unit_count = 2;
    let reuse = block.reuse_band_params;
    let qcount = block.quantization_unit_count;
    let ch = &mut block.channels[0];
    ch.scale_factors.fill(0);
    for i in 0..qcount {
        ch.scale_factors[i] = br.read(5) as i32;
    }
    let precision = if reuse { 8 } else { 4 };
    for i in 0..qcount {
        ch.precisions[i] = precision;
        ch.precisions_fine[i] = 0;
    }
    ch.coded_quant_units = qcount;
    ch.quantized_spectra.fill(0);
    for i in 0..ch.coded_quant_units {
        if ch.precisions[i] <= 0 {
            continue;
        }
        let bits = (ch.precisions[i] + 1) as u32;
        for j in QUANT_UNIT_TO_COEFF_INDEX[i]..QUANT_UNIT_TO_COEFF_INDEX[i + 1] {
            ch.quantized_spectra[j] = br.read_signed(bits);
        }
    }
    Ok(())
}
