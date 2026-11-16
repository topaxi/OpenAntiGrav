//! A player's own audio file: MP3, FLAC, Ogg Vorbis or Ogg Opus, decoded in process.
//!
//! [`crate::mp3`] is the disc-music face of the same reader; this is the part that
//! takes the container from the file's extension. `symphonia` reads MP3, FLAC and
//! Ogg and decodes MP3, FLAC and Vorbis. It has no Opus decoder, so an Opus stream
//! is demuxed by `symphonia`'s Ogg reader and decoded by `opus-rs`, a pure
//! Rust crate, which keeps every desktop, mobile and web target free of a C
//! library.

use anyhow::{Context, Result, bail};
use opus_rs::OpusDecoder;
use symphonia::core::codecs::audio::well_known::CODEC_ID_OPUS;
use symphonia::core::codecs::audio::{AudioDecoder, AudioDecoderOptions};
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, FormatReader, Track, TrackType};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;

use crate::at3::Pcm;

/// The extensions [`decode`] reads, lower case.
pub const EXTENSIONS: [&str; 5] = ["mp3", "flac", "ogg", "oga", "opus"];

/// The most samples per channel one Opus packet holds: 120 ms at 48 kHz.
const OPUS_MAX_PACKET_FRAMES: usize = 5760;

/// Opens `blob` as the container `extension` names.
///
/// The extension is a hint, not a verdict: the probe still checks the content,
/// so a wrong name is an error rather than a misreading. The blob is copied into
/// the cursor to avoid a lifetime through every public function, for a file about
/// to be decoded into many times its own size.
pub(crate) fn probe(blob: &[u8], extension: &str) -> Result<(Box<dyn FormatReader>, Track)> {
    let source = MediaSourceStream::new(
        Box::new(std::io::Cursor::new(blob.to_vec())),
        <_>::default(),
    );
    let mut hint = Hint::new();
    hint.with_extension(extension);

    let format = symphonia::default::get_probe()
        .probe(
            &hint,
            source,
            FormatOptions::default(),
            MetadataOptions::default(),
        )
        .with_context(|| format!("this is not a readable .{extension} stream"))?;
    let track = format
        .default_track(TrackType::Audio)
        .context("the stream carries no audio track")?
        .clone();
    Ok((format, track))
}

enum Backend {
    Symphonia(Box<dyn AudioDecoder>),
    Opus(Box<OpusDecoder>),
}

/// Decodes `blob`, a file whose name ended in `extension`, to interleaved 16-bit PCM.
///
/// # Errors
///
/// A blob that is not the stream its extension says, a codec nothing here
/// decodes (Opus with more than two channels), or a stream that decodes to
/// nothing at all.
pub fn decode(blob: &[u8], extension: &str) -> Result<Pcm> {
    let (mut format, track) = probe(blob, extension)?;
    let track_id = track.id;
    let params = track
        .codec_params
        .as_ref()
        .context("the track declares no codec")?;
    let audio = params.audio().context("the track is not audio")?;
    let channels = u16::try_from(
        audio
            .channels
            .as_ref()
            .context("the track declares no channel layout")?
            .count(),
    )
    .context("an implausible channel count")?;
    let sample_rate = audio.sample_rate.context("the track declares no rate")?;

    let mut backend = if audio.codec == CODEC_ID_OPUS {
        if channels > 2 {
            bail!("Opus with {channels} channels is not read, only mono and stereo");
        }
        let decoder = OpusDecoder::new(
            i32::try_from(sample_rate).context("an implausible sample rate")?,
            usize::from(channels),
        )
        .map_err(|error| anyhow::anyhow!("opening the Opus decoder: {error}"))?;
        Backend::Opus(Box::new(decoder))
    } else {
        Backend::Symphonia(
            symphonia::default::get_codecs()
                .make_audio_decoder(audio, &AudioDecoderOptions::default())
                .context("opening the decoder")?,
        )
    };

    let mut samples: Vec<i16> = Vec::new();
    let mut frame: Vec<i16> = Vec::new();
    let mut opus_out: Vec<f32> = Vec::new();
    loop {
        let packet = match format.next_packet() {
            Ok(Some(packet)) => packet,
            Ok(None) => break,
            Err(error) => bail!("reading a packet: {error}"),
        };
        if packet.track_id != track_id {
            continue;
        }
        // A packet that will not decode is skipped, not fatal: a truncated file
        // gives a short song rather than none.
        match &mut backend {
            Backend::Symphonia(decoder) => {
                let Ok(decoded) = decoder.decode(&packet) else {
                    continue;
                };
                frame.clear();
                decoded.copy_to_vec_interleaved(&mut frame);
                samples.extend_from_slice(&frame);
            }
            Backend::Opus(decoder) => {
                opus_out.resize(OPUS_MAX_PACKET_FRAMES * usize::from(channels), 0.0);
                let Ok(per_channel) =
                    decoder.decode(&packet.data, OPUS_MAX_PACKET_FRAMES, &mut opus_out)
                else {
                    continue;
                };
                // The Ogg reader trims the end of the stream to its last granule.
                let ch = usize::from(channels);
                let start = usize::try_from(packet.trim_start.get()).unwrap_or(0) * ch;
                let end = usize::try_from(packet.trim_end.get()).unwrap_or(0) * ch;
                let written = per_channel * ch;
                let kept = start.min(written)..written.saturating_sub(end).max(start.min(written));
                samples.extend(opus_out[kept].iter().map(|&sample| {
                    // Clamped, so the cast is the rounding it is meant to be.
                    #[allow(clippy::cast_possible_truncation)]
                    let pcm = (sample * 32768.0).round().clamp(-32768.0, 32767.0) as i16;
                    pcm
                }));
            }
        }
    }

    // Ogg Opus declares a pre-skip: decoder warm-up that is not music.
    if matches!(backend, Backend::Opus(_)) {
        let skip = track.delay.unwrap_or(0) as usize * usize::from(channels);
        samples.drain(..skip.min(samples.len()));
    }

    if samples.is_empty() {
        bail!("the stream decoded to no samples");
    }
    Ok(Pcm {
        samples,
        channels,
        sample_rate,
    })
}

#[cfg(test)]
mod tests;
