//! Writing rendered audio out, which is how a headless run gets checked.
//!
//! This environment has no window and cannot listen, so `--dump-audio` is the
//! audio counterpart of `--screenshot`: the only end-to-end evidence available
//! that the mixer produced what it was supposed to. Building it in the same
//! change as the mixer is deliberate - without it, every later phase would ship
//! with an exit criterion nobody here can run.
//!
//! The header comes from [`oag_formats::ps2_music::wav_with`] so the workspace
//! keeps one WAV writer. Only the sample conversion lives here, because the
//! mixer works in `f32` and a `data` chunk wants 16-bit little-endian.

use crate::mixer::CHANNELS;

/// Converts interleaved `f32` samples to a complete WAV file.
///
/// Samples are clamped to `-1.0..=1.0` before scaling. Clamping rather than
/// wrapping matters: a sum of voices can exceed unity, and a wrapped sample is
/// a loud click in the middle of an otherwise correct render, which reads as a
/// mixer bug that is not there.
#[must_use]
pub fn from_samples(samples: &[f32], sample_rate: u32) -> Vec<u8> {
    let mut pcm = Vec::with_capacity(samples.len() * 2);
    for sample in samples {
        pcm.extend_from_slice(&to_i16(*sample).to_le_bytes());
    }
    oag_formats::ps2_music::wav_with(&pcm, sample_rate, CHANNELS as u16)
}

/// Scales one sample into 16-bit range.
///
/// `i16::MIN` is used as the scale on both sides so that `-1.0` reaches the
/// full negative rail and `1.0` stops one step short of clipping, which is the
/// usual convention and keeps a full-scale sine symmetric.
fn to_i16(sample: f32) -> i16 {
    let scaled = sample.clamp(-1.0, 1.0) * -f32::from(i16::MIN);
    scaled.clamp(f32::from(i16::MIN), f32::from(i16::MAX)) as i16
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_header_says_what_the_payload_is() {
        let file = from_samples(&[0.0; 8], 44_100);
        assert_eq!(&file[..4], b"RIFF");
        assert_eq!(&file[8..12], b"WAVE");
        // Channels at 22, sample rate at 24, per the canonical layout.
        assert_eq!(u16::from_le_bytes([file[22], file[23]]), CHANNELS as u16);
        assert_eq!(
            u32::from_le_bytes([file[24], file[25], file[26], file[27]]),
            44_100
        );
        assert_eq!(file.len(), 44 + 8 * 2, "eight samples is sixteen bytes");
    }

    #[test]
    fn full_scale_reaches_the_rails_without_wrapping() {
        assert_eq!(to_i16(-1.0), i16::MIN);
        assert_eq!(to_i16(1.0), i16::MAX);
        assert_eq!(to_i16(0.0), 0);
    }

    #[test]
    fn an_overloaded_sample_clamps_instead_of_wrapping_into_a_click() {
        assert_eq!(to_i16(7.5), i16::MAX);
        assert_eq!(to_i16(-7.5), i16::MIN);
        assert_eq!(to_i16(f32::INFINITY), i16::MAX);
        assert_eq!(to_i16(f32::NEG_INFINITY), i16::MIN);
    }

    #[test]
    fn a_round_trip_preserves_the_samples_it_can() {
        let samples = [0.0, 0.5, -0.5, 0.25];
        let file = from_samples(&samples, 48_000);
        let pcm = &file[44..];
        let decoded: Vec<i16> = pcm
            .as_chunks::<2>()
            .0
            .iter()
            .map(|c| i16::from_le_bytes([c[0], c[1]]))
            .collect();
        for (sample, got) in samples.iter().zip(&decoded) {
            let back = f32::from(*got) / -f32::from(i16::MIN);
            assert!((back - sample).abs() < 1e-4, "{sample} came back as {back}");
        }
    }
}
