//! ATRAC9 configuration parsing (the 4-byte config data).

use crate::bit_reader::BitReader;
use crate::error::{Error, Result};
use crate::tables::{CHANNEL_CONFIGS, FRAME_SAMPLES_POWER, SAMPLE_RATES};

/// Parsed ATRAC9 configuration (from the 4-byte config blob).
#[derive(Debug, Clone)]
pub struct Config {
    pub config_data: [u8; 4],
    pub sample_rate_index: u8,
    pub channel_config_index: u8,
    pub frame_bytes: usize,
    pub superframe_index: u8,

    pub channel_count: usize,
    pub sample_rate: u32,
    pub frames_per_superframe: usize,
    pub frame_samples_power: u8,
    pub frame_samples: usize,
    pub superframe_bytes: usize,
    pub superframe_samples: usize,

    /// Channel layout for this config.
    pub block_count: usize,
    pub block_types: [u8; 5], // 0=Mono, 1=Stereo, 2=LFE
}

impl Config {
    /// Parse the standard 4-byte ATRAC9 config data.
    ///
    /// Layout (big-endian bit packing):
    /// ```text
    /// 8 bits  : header magic (must be 0xFE)
    /// 4 bits  : sample rate index
    /// 3 bits  : channel config index
    /// 1 bit   : validation (must be 0)
    /// 11 bits : frame bytes - 1
    /// 2 bits  : superframe index (frames = 1 << index)
    /// ```
    pub fn from_bytes(data: &[u8; 4]) -> Result<Self> {
        let mut br = BitReader::new(data);

        let header = br.read(8);
        let sample_rate_index = br.read(4) as u8;
        let channel_config_index = br.read(3) as u8;
        let validation_bit = br.read(1);
        let frame_bytes = br.read(11) as usize + 1;
        let superframe_index = br.read(2) as u8;

        if header != 0xFE || validation_bit != 0 {
            return Err(Error::BadConfigData);
        }
        if sample_rate_index > 15 || channel_config_index as usize >= CHANNEL_CONFIGS.len() {
            return Err(Error::BadConfigData);
        }

        let ch_cfg = &CHANNEL_CONFIGS[channel_config_index as usize];
        let frames_per_superframe = 1usize << superframe_index;
        let frame_samples_power = FRAME_SAMPLES_POWER[sample_rate_index as usize];
        let frame_samples = 1usize << frame_samples_power;
        let sample_rate = SAMPLE_RATES[sample_rate_index as usize];

        Ok(Config {
            config_data: *data,
            sample_rate_index,
            channel_config_index,
            frame_bytes,
            superframe_index,
            channel_count: ch_cfg.channel_count as usize,
            sample_rate,
            frames_per_superframe,
            frame_samples_power,
            frame_samples,
            superframe_bytes: frame_bytes * frames_per_superframe,
            superframe_samples: frame_samples * frames_per_superframe,
            block_count: ch_cfg.block_count as usize,
            block_types: ch_cfg.types,
        })
    }
}

/// Public codec information returned to the caller.
#[derive(Debug, Clone)]
pub struct CodecInfo {
    pub channels: usize,
    pub channel_config_index: u8,
    pub sampling_rate: u32,
    pub superframe_size: usize,
    pub frames_in_superframe: usize,
    pub frame_samples: usize,
    pub config_data: [u8; 4],
}

impl From<&Config> for CodecInfo {
    fn from(c: &Config) -> Self {
        CodecInfo {
            channels: c.channel_count,
            channel_config_index: c.channel_config_index,
            sampling_rate: c.sample_rate,
            superframe_size: c.superframe_bytes,
            frames_in_superframe: c.frames_per_superframe,
            frame_samples: c.frame_samples,
            config_data: c.config_data,
        }
    }
}
