//! Internal decoder data structures.
//! Port of LibAtrac9 structures.h

use crate::imdct::Mdct;

pub const MAX_CHANNEL_COUNT: usize = 8;
pub const MAX_BLOCK_COUNT: usize = 5;
pub const MAX_BLOCK_CHANNEL_COUNT: usize = 2;
pub const MAX_FRAME_SAMPLES: usize = 256;
pub const MAX_QUANT_UNITS: usize = 30;
pub const MAX_BEX_VALUES: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockType {
    Mono = 0,
    Stereo = 1,
    Lfe = 2,
}

impl From<u8> for BlockType {
    fn from(v: u8) -> Self {
        match v {
            1 => BlockType::Stereo,
            2 => BlockType::Lfe,
            _ => BlockType::Mono,
        }
    }
}

#[derive(Debug)]
pub struct Channel {
    pub channel_index: usize,
    pub mdct: Mdct,

    pub pcm: [f64; MAX_FRAME_SAMPLES],
    pub spectra: [f64; MAX_FRAME_SAMPLES],

    pub coded_quant_units: usize,
    pub scale_factor_coding_mode: i32,

    pub scale_factors: [i32; 31],
    pub scale_factors_prev: [i32; 31],

    pub precisions: [i32; MAX_QUANT_UNITS],
    pub precisions_fine: [i32; MAX_QUANT_UNITS],
    pub precision_mask: [i32; MAX_QUANT_UNITS],

    pub codebook_set: [i32; MAX_QUANT_UNITS],

    pub quantized_spectra: [i32; MAX_FRAME_SAMPLES],
    pub quantized_spectra_fine: [i32; MAX_FRAME_SAMPLES],

    pub bex_mode: i32,
    pub bex_value_count: i32,
    pub bex_values: [i32; MAX_BEX_VALUES],
    /// OpenAntiGrav patch: LibAtrac9's per-channel noise generator for band extension.
    pub rng: crate::band_extension::Rng,
}

impl Channel {
    pub fn new(channel_index: usize, frame_samples_power: u8) -> Self {
        Self {
            channel_index,
            mdct: Mdct::new(frame_samples_power as u32),
            pcm: [0.0; MAX_FRAME_SAMPLES],
            spectra: [0.0; MAX_FRAME_SAMPLES],
            coded_quant_units: 0,
            scale_factor_coding_mode: 0,
            scale_factors: [0; 31],
            scale_factors_prev: [0; 31],
            precisions: [0; MAX_QUANT_UNITS],
            precisions_fine: [0; MAX_QUANT_UNITS],
            precision_mask: [0; MAX_QUANT_UNITS],
            codebook_set: [0; MAX_QUANT_UNITS],
            quantized_spectra: [0; MAX_FRAME_SAMPLES],
            quantized_spectra_fine: [0; MAX_FRAME_SAMPLES],
            bex_mode: 0,
            bex_value_count: 0,
            bex_values: [0; MAX_BEX_VALUES],
            rng: crate::band_extension::Rng::default(),
        }
    }
}

#[derive(Debug)]
pub struct Block {
    pub block_type: BlockType,
    pub block_index: usize,
    pub channels: Vec<Channel>,
    pub channel_count: usize,

    pub first_in_superframe: bool,
    pub reuse_band_params: bool,

    pub band_count: i32,
    pub stereo_band: i32,
    pub extension_band: i32,
    pub quantization_unit_count: usize,
    pub stereo_quantization_unit: usize,
    pub extension_unit: usize,
    pub quantization_units_prev: usize,

    pub gradient: [i32; 31],
    pub gradient_mode: i32,
    pub gradient_start_unit: i32,
    pub gradient_start_value: i32,
    pub gradient_end_unit: i32,
    pub gradient_end_value: i32,
    pub gradient_boundary: i32,

    pub primary_channel_index: usize,
    pub has_joint_stereo_signs: bool,
    pub joint_stereo_signs: [i32; MAX_QUANT_UNITS],

    pub band_extension_enabled: bool,
    pub has_extension_data: bool,
    pub bex_data_length: i32,
    pub bex_mode: i32,
}

impl Block {
    pub fn new(block_index: usize, block_type: BlockType, frame_samples_power: u8) -> Self {
        let channel_count = match block_type {
            BlockType::Stereo => 2,
            _ => 1,
        };
        let channels = (0..channel_count)
            .map(|i| Channel::new(i, frame_samples_power))
            .collect();

        Self {
            block_type,
            block_index,
            channels,
            channel_count,
            first_in_superframe: false,
            reuse_band_params: false,
            band_count: 0,
            stereo_band: 0,
            extension_band: 0,
            quantization_unit_count: 0,
            stereo_quantization_unit: 0,
            extension_unit: 0,
            quantization_units_prev: 0,
            gradient: [0; 31],
            gradient_mode: 0,
            gradient_start_unit: 0,
            gradient_start_value: 0,
            gradient_end_unit: 0,
            gradient_end_value: 0,
            gradient_boundary: 0,
            primary_channel_index: 0,
            has_joint_stereo_signs: false,
            joint_stereo_signs: [0; MAX_QUANT_UNITS],
            band_extension_enabled: false,
            has_extension_data: false,
            bex_data_length: 0,
            bex_mode: 0,
        }
    }
}

#[derive(Debug)]
pub struct Frame {
    pub index_in_superframe: usize,
    pub blocks: Vec<Block>,
}

impl Frame {
    pub fn new(block_types: &[u8], block_count: usize, frame_samples_power: u8) -> Self {
        let blocks = (0..block_count)
            .map(|i| Block::new(i, BlockType::from(block_types[i]), frame_samples_power))
            .collect();
        Self {
            index_in_superframe: 0,
            blocks,
        }
    }
}
