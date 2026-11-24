//! Pure Rust ATRAC9 audio decoder.
//!
//! Port of [LibAtrac9](https://github.com/Thealexbarney/LibAtrac9) (MIT).

// OpenAntiGrav: helpers the published crate kept but no longer calls.
#![allow(dead_code)]

mod band_extension;
mod bit_allocation;
mod bit_reader;
mod config;
mod error;
mod huffman;
mod huffman_tables;
mod imdct;
mod quantization;
mod scale_factors;
mod structures;
mod tables;
mod trace;
mod unpack;

pub use config::{CodecInfo, Config};
pub use error::{Error, Result};

use bit_reader::{BitReader, clamp16, round_half_up};
use config::Config as At9Config;
use quantization::{dequantize_channel, scale_spectrum};
use structures::{BlockType, Frame};
use tables::QUANT_UNIT_TO_COEFF_INDEX;
use unpack::UnpackContext;

/// ATRAC9 decoder instance.
pub struct Atrac9Decoder {
    config: At9Config,
    frame: Frame,
    unpack_ctx: UnpackContext,
}

impl std::fmt::Debug for Atrac9Decoder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Atrac9Decoder")
            .field("config", &self.config)
            .finish()
    }
}

impl Atrac9Decoder {
    /// Create a new decoder from the 4-byte ATRAC9 config data.
    pub fn new(config_data: &[u8; 4]) -> Result<Self> {
        let config = At9Config::from_bytes(config_data)?;
        let frame = Frame::new(
            &config.block_types,
            config.block_count,
            config.frame_samples_power,
        );
        Ok(Self {
            config,
            frame,
            unpack_ctx: UnpackContext::new(),
        })
    }

    pub fn codec_info(&self) -> CodecInfo {
        CodecInfo::from(&self.config)
    }

    pub fn config(&self) -> &At9Config {
        &self.config
    }

    /// Decode one superframe of ATRAC9 data into interleaved `i16` PCM.
    ///
    /// Returns the number of input bytes consumed.
    pub fn decode(&mut self, input: &[u8], output: &mut [i16]) -> Result<usize> {
        let info = self.codec_info();
        let samples_per_frame = info.channels * info.frame_samples;
        let needed = samples_per_frame * info.frames_in_superframe;
        if output.len() < needed {
            return Err(Error::BufferTooSmall);
        }
        if input.len() < info.superframe_size {
            return Err(Error::UnexpectedEof);
        }

        let mut br = BitReader::new(input);
        let frames = info.frames_in_superframe;
        let channels = info.channels;
        let frame_samples = info.frame_samples;

        for frame_idx in 0..frames {
            self.frame.index_in_superframe = frame_idx;
            unpack::unpack_frame(
                &mut self.frame,
                &mut br,
                self.config.sample_rate_index,
                &self.unpack_ctx,
            )?;

            // OpenAntiGrav patch: LibAtrac9's `DecodeFrame` order - dequantize,
            // intensity stereo, scale, band extension, IMDCT. 0.9.1 scaled
            // before copying the intensity units, so the secondary channel
            // carried the primary's scale factors there instead of its own.
            for block in &mut self.frame.blocks {
                for ch in &mut block.channels {
                    dequantize_channel(
                        &ch.quantized_spectra,
                        &ch.quantized_spectra_fine,
                        &ch.precisions,
                        &ch.precisions_fine,
                        ch.coded_quant_units,
                        &mut ch.spectra,
                    );
                }

                if block.block_type == BlockType::Stereo
                    && block.stereo_quantization_unit < block.quantization_unit_count
                {
                    let primary = block.primary_channel_index;
                    let secondary = 1 - primary;
                    for i in block.stereo_quantization_unit..block.quantization_unit_count {
                        let sign = block.joint_stereo_signs[i];
                        let start = QUANT_UNIT_TO_COEFF_INDEX[i];
                        let end = QUANT_UNIT_TO_COEFF_INDEX[i + 1];
                        for sb in start..end {
                            let src = block.channels[primary].spectra[sb];
                            block.channels[secondary].spectra[sb] =
                                if sign > 0 { -src } else { src };
                        }
                    }
                }

                for ch in &mut block.channels {
                    scale_spectrum(
                        &mut ch.spectra,
                        &ch.scale_factors,
                        block.quantization_unit_count,
                    );
                }

                band_extension::apply_band_extension(block);

                // IMDCT
                for ch in &mut block.channels {
                    let mut pcm = [0.0f64; 256];
                    let specs = ch.spectra;
                    ch.mdct
                        .run(&specs[..ch.mdct.size], &mut pcm[..ch.mdct.size]);
                    ch.pcm = pcm;
                }
            }

            // Interleave PCM into output
            let out_offset = frame_idx * samples_per_frame;
            // Collect channel pointers in order
            let mut ch_pcm: Vec<&[f64; 256]> = Vec::new();
            for block in &self.frame.blocks {
                for ch in &block.channels {
                    ch_pcm.push(&ch.pcm);
                }
            }
            for smpl in 0..frame_samples {
                for (ch, pcm) in ch_pcm.iter().enumerate() {
                    if ch < channels {
                        let v = pcm[smpl];
                        // Round and clamp
                        output[out_offset + smpl * channels + ch] = clamp16(round_half_up(v));
                    }
                }
            }
        }

        let bytes_used = (br.position() + 7) / 8;
        Ok(bytes_used.max(info.superframe_size))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_bad_magic() {
        let bad = [0x00, 0x00, 0x00, 0x00];
        assert!(matches!(
            Atrac9Decoder::new(&bad),
            Err(Error::BadConfigData)
        ));
    }

    #[test]
    fn parses_sample_config() {
        let config = [0xFE, 0x74, 0x17, 0xF0];
        let dec = Atrac9Decoder::new(&config).expect("valid config");
        let info = dec.codec_info();
        assert_eq!(info.sampling_rate, 48000);
        assert_eq!(info.channels, 2);
        assert_eq!(info.frame_samples, 256);
        assert_eq!(info.frames_in_superframe, 4);
        assert_eq!(info.superframe_size, 768);
    }
}
