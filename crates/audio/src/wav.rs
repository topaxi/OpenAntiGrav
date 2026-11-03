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

use crate::mixer::{CHANNELS, Sound};

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

/// The longest sound [`decode`] takes, in bytes of PCM: about 46 s of 16-bit mono at
/// 22,050 Hz. A mod's sounds are short effects; a longer file is refused by name rather
/// than held in memory for the whole race.
pub const MAX_DECODED_BYTES: usize = 4 << 20;

/// Reads a RIFF/WAVE file of 16-bit integer PCM, mono or stereo, into a [`Sound`] at the
/// file's own rate (the mixer steps through it at the rate ratio, so nothing resamples
/// here).
///
/// # Errors
///
/// Names what the file is instead: not RIFF/WAVE, truncated, a `fmt ` that is not 16-bit
/// integer PCM of one or two channels, no `data`, or past [`MAX_DECODED_BYTES`].
pub fn decode(file: &[u8]) -> Result<Sound, String> {
    if file.len() < 12 || &file[..4] != b"RIFF" || &file[8..12] != b"WAVE" {
        return Err("not a RIFF/WAVE file".to_string());
    }
    let mut format = None;
    let mut at = 12;
    while at + 8 <= file.len() {
        let id = &file[at..at + 4];
        let size =
            u32::from_le_bytes([file[at + 4], file[at + 5], file[at + 6], file[at + 7]]) as usize;
        let body = at + 8;
        let end = body
            .checked_add(size)
            .filter(|end| *end <= file.len())
            .ok_or_else(|| {
                format!(
                    "the {:?} chunk runs past the end of the file",
                    chunk_name(id)
                )
            })?;
        match id {
            b"fmt " => {
                if size < 16 {
                    return Err("the fmt chunk is shorter than 16 bytes".to_string());
                }
                let word = |at: usize| u16::from_le_bytes([file[body + at], file[body + at + 1]]);
                let rate = u32::from_le_bytes([
                    file[body + 4],
                    file[body + 5],
                    file[body + 6],
                    file[body + 7],
                ]);
                format = Some((word(0), word(2), rate, word(14)));
            }
            b"data" => {
                let (tag, channels, rate, bits) =
                    format.ok_or_else(|| "the data chunk comes before fmt".to_string())?;
                if tag != 1 || bits != 16 {
                    return Err(format!(
                        "format tag {tag} at {bits} bits, only 16-bit integer PCM is read"
                    ));
                }
                if !(1..=2).contains(&channels) {
                    return Err(format!("{channels} channels, only mono or stereo are read"));
                }
                if size > MAX_DECODED_BYTES {
                    return Err(format!(
                        "{size} bytes of audio, at most {MAX_DECODED_BYTES}"
                    ));
                }
                let samples = file[body..end]
                    .chunks_exact(2)
                    .map(|pair| i16::from_le_bytes([pair[0], pair[1]]))
                    .collect();
                return Sound::new(samples, channels, rate).map_err(|e| e.to_string());
            }
            _ => {}
        }
        // Chunks are word aligned: an odd size is followed by a pad byte.
        at = end + (size & 1);
    }
    Err(if format.is_some() {
        "no data chunk".to_string()
    } else {
        "no fmt chunk".to_string()
    })
}

fn chunk_name(id: &[u8]) -> String {
    String::from_utf8_lossy(id).into_owned()
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

    #[test]
    fn a_pcm16_wav_decodes_at_its_own_rate_and_a_bad_one_is_named() {
        let mut pcm = Vec::new();
        for sample in [0i16, 1000, -1000, i16::MAX] {
            pcm.extend_from_slice(&sample.to_le_bytes());
        }
        let file = oag_formats::ps2_music::wav_with(&pcm, 22_050, 1);
        let sound = decode(&file).unwrap();
        assert_eq!(
            (sound.channels(), sound.sample_rate(), sound.frames()),
            (1, 22_050, 4)
        );
        let stereo = decode(&oag_formats::ps2_music::wav_with(&pcm, 44_100, 2)).unwrap();
        assert_eq!((stereo.channels(), stereo.frames()), (2, 2));

        assert!(decode(b"not a wav at all").unwrap_err().contains("RIFF"));
        let mut float = file.clone();
        float[20] = 3;
        assert!(decode(&float).unwrap_err().contains("16-bit integer PCM"));
        let mut cut = file.clone();
        cut.truncate(file.len() - 3);
        assert!(decode(&cut).unwrap_err().contains("past the end"));
        let mut surround = file.clone();
        surround[22] = 6;
        assert!(decode(&surround).unwrap_err().contains("6 channels"));
        assert!(decode(&file[..36]).unwrap_err().contains("no data"));
    }
}
