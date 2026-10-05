//! The MPEG streams Wipeout HD keeps its music in, read and decoded in process.
//!
//! The counterpart of [`crate::at3`] for the other container this project
//! plays, and deliberately a much smaller module: `symphonia` does the work,
//! where ATRAC3+ has no Rust decoder at all and has to go out to `ffmpeg`.
//!
//! # Why this one does not go through `ffmpeg`
//!
//! `ffmpeg` is what we reach for when there is nothing to implement against and
//! no light dependency to take: ATRAC3+, which [ADR-0019] puts out of process
//! for exactly that reason, and the video path. MP3 is neither. Decoding it
//! here means **Wipeout HD's music plays on a machine with no `ffmpeg`
//! installed**, which is a real difference to a player rather than a tidiness
//! argument - the PSP titles degrade to silence there and say so, and HD does
//! not have to.
//!
//! # And no cache, for the same reason
//!
//! [`crate::at3`] writes decoded PCM to `data/cache/audio/` because an
//! out-of-process `ffmpeg` run costs seconds. This decodes far faster than real
//! time in process, so a cache would be a second copy of every track on disk to
//! save a fraction of a second. [`describe`] is the part that had to stay
//! cheap, because `crate::music` lists fifteen tracks at boot, and it reads the
//! header alone and decodes nothing.
//!
//! [ADR-0019]: ../../../docs/architecture/adr/0019-atrac3plus-out-of-process.md

use anyhow::{Context, Result, bail};
use symphonia::core::codecs::audio::AudioDecoderOptions;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, FormatReader, Track, TrackType};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;

use crate::at3::Pcm;

/// What an MPEG stream declares about itself, without decoding it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stream {
    /// How many channels it carries.
    pub channels: u16,
    /// Its sample rate.
    pub sample_rate: u32,
    /// How long it is, when the stream says.
    ///
    /// `None` for one that declares no frame count - which nothing on the HD
    /// disc is, every track there carrying the `Info` tag that states it. A
    /// caller treats it as "not a track this can list" rather than deriving a
    /// length from the byte count, which would only be right at a constant
    /// bitrate and nothing says the file is one.
    pub seconds: Option<f64>,
}

/// What `blob` declares, or `None` when it is not an MPEG stream at all.
///
/// `None` rather than an error for the same reason [`crate::at3::describe`]'s
/// callers treat a parse failure as a skip: this is asked of archive entries
/// that are mostly something else, and "not one of these" is the ordinary
/// answer rather than a fault.
///
/// **Cheap on purpose.** `crate::music` asks this of every declared track at
/// boot; it reads the stream's header and its declared frame count and decodes
/// no audio.
#[must_use]
pub fn describe(blob: &[u8]) -> Option<Stream> {
    let (_, track) = probe(blob).ok()?;
    let audio = track.codec_params.as_ref()?.audio()?;
    let sample_rate = audio.sample_rate?;
    Some(Stream {
        channels: u16::try_from(audio.channels.as_ref()?.count()).ok()?,
        sample_rate,
        // Frames over the rate rather than `TimeBase::calc_time`, because that
        // is all the arithmetic is and it keeps the units visible.
        seconds: track
            .num_frames
            .filter(|_| sample_rate > 0)
            .map(|frames| frames as f64 / f64::from(sample_rate)),
    })
}

/// Decodes an MPEG stream to interleaved 16-bit PCM.
///
/// # Errors
///
/// A blob that is not an MPEG stream, one whose codec `symphonia` will not
/// open, or a stream that decodes to nothing at all.
pub fn decode(blob: &[u8]) -> Result<Pcm> {
    let (mut format, track) = probe(blob)?;
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

    let mut decoder = symphonia::default::get_codecs()
        .make_audio_decoder(audio, &AudioDecoderOptions::default())
        .context("opening the MP3 decoder")?;

    let mut samples: Vec<i16> = Vec::new();
    let mut frame: Vec<i16> = Vec::new();
    loop {
        let packet = match format.next_packet() {
            Ok(Some(packet)) => packet,
            Ok(None) => break,
            Err(error) => bail!("reading an MP3 packet: {error}"),
        };
        if packet.track_id != track_id {
            continue;
        }
        // **A packet that will not decode is skipped, not fatal.** The disc's
        // own files decode whole; this is the shape a truncated entry takes,
        // and stopping at the first bad frame gives a short track rather than
        // no music at all.
        let Ok(decoded) = decoder.decode(&packet) else {
            continue;
        };
        frame.clear();
        decoded.copy_to_vec_interleaved(&mut frame);
        samples.extend_from_slice(&frame);
    }

    if samples.is_empty() {
        bail!("the MP3 stream decoded to no samples");
    }
    Ok(Pcm {
        samples,
        channels,
        sample_rate,
    })
}

/// Opens `blob` as MPEG and returns its reader and its one audio track.
///
/// Hinted as `mp3` rather than sniffed, because every caller already knows what
/// it has: the name came out of a title package's own declaration. The probe
/// still checks the content, so a hint that turns out wrong is an error rather
/// than a misreading.
///
/// The blob is copied into the cursor. The alternative is threading a lifetime
/// through both public functions to save one copy of a file that is about to be
/// decoded into twenty times its own size.
fn probe(blob: &[u8]) -> Result<(Box<dyn FormatReader>, Track)> {
    let source = MediaSourceStream::new(
        Box::new(std::io::Cursor::new(blob.to_vec())),
        <_>::default(),
    );
    let mut hint = Hint::new();
    hint.with_extension("mp3");

    let format = symphonia::default::get_probe()
        .probe(
            &hint,
            source,
            FormatOptions::default(),
            MetadataOptions::default(),
        )
        .context("this is not an MPEG stream")?;
    let track = format
        .default_track(TrackType::Audio)
        .context("the stream carries no audio track")?
        .clone();
    Ok((format, track))
}

#[cfg(test)]
mod tests;
