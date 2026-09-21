//! A movie's own audio track, in either shape this project has found one:
//! undecoded ATRAC3+ blocks (`.PMF`, the PSP) or already-decoded PCM (`.PSS`,
//! the PS2 - see `oag_video::pss`).
//!
//! Split out of `movie.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change to the PSP
//! path, made at the same time the PS2 path stopped reporting `None`
//! unconditionally.

use std::path::{Path, PathBuf};

use anyhow::Result;
use log::warn;
use oag_video::{bik, pmf, pss};

use super::container_audio::{self, ContainerTrack};
use crate::at3;

/// Bytes of sub-header on every audio PES payload in a `.PMF`.
///
/// `pmf::demux` strips the PES header itself and hands over what follows, and
/// what follows is **not** an ATRAC3+ frame: it is four bytes and then a slice
/// of the frame stream. The first two are zero on every packet of every movie
/// on the disc; the third and fourth are a big-endian offset from the end of
/// this header to the first frame that *starts* inside the packet, the bytes
/// before it being the tail of the frame the previous packet began.
///
/// **Measured, on all 323 packets of `Intro.PMF` and every other movie with a
/// track.** Reading that offset lands on the ATRAC3+ sync word 323 times out of
/// 323, and it is what confirms the field is a pointer rather than a counter:
/// packet 0 carries two whole 752-byte frames and 509 bytes of a third, and
/// packet 1's offset is the 243 bytes that complete it.
///
/// Nothing here needs the pointer to *reassemble* the stream - concatenating
/// every packet's payload in order gives the frames back contiguously - but the
/// first packet's is used, because a movie whose first frame does not begin at
/// offset zero would otherwise be decoded half a frame out.
const AUDIO_PES_HEADER_LEN: usize = 4;

/// The sync word every ATRAC3+ frame inside a `.PMF` begins with.
///
/// **A RIFF-wrapped `.at3` has no such word**: `PSP_GAME/SND0.AT3`'s data chunk
/// starts straight in on the codec payload, and a scan of all 25,760 bytes of
/// it finds `0f d0` nowhere. So this is the `.PMF`'s framing rather than the
/// codec's, and it has to come off before `ffmpeg` will read a block.
const ATRAC3PLUS_SYNC: [u8; 2] = [0x0f, 0xd0];

/// Bytes of header on every ATRAC3+ frame inside a `.PMF`, sync word included.
///
/// `0f d0` then the codec config word - see `at3::codec_config`, which the
/// disc's own `.at3` entries carry in the same shape - then four zero bytes.
/// Constant across all 865 frames of `Intro.PMF` and every frame of every other
/// movie with a track.
///
/// The evidence that it is exactly eight is that stripping eight leaves a block
/// that decodes: `Intro.PMF`'s frames are 752 bytes apart, 752 - 8 is 744, and
/// 744-byte blocks decode to 865 whole blocks of 2,048 samples with no
/// remainder. It also lines the payload up with what a `.at3` stores - both
/// begin `3a` - where stripping only the two-byte sync word does not, and
/// `ffmpeg` rejects that with "frame data doesn't match channel configuration"
/// rather than decoding noise.
const ATRAC3PLUS_FRAME_HEADER_LEN: usize = 8;

/// What [`MovieAudio`] actually holds - the three shapes a track has come in
/// so far, and the "how to get to PCM" that differs between them.
enum MovieAudioKind {
    /// ATRAC3+ blocks, headers off, back to back - exactly what
    /// [`crate::at3::riff`] wants for its `data` chunk. Decoded through
    /// `ffmpeg` and cached, because nothing in this workspace decodes
    /// ATRAC3+ itself.
    Atrac3Plus { blocks: Vec<u8>, block_align: u16 },
    /// Already-decoded interleaved 16-bit PCM: the PS2's `.PSS` track, which
    /// carries no compression at all (see `oag_video::pss`'s "format 1 is
    /// 16-bit PCM, not PS-ADPCM"), so there is nothing to decode - only to
    /// hold until playback wants it, the same as the ATRAC3+ blocks are held
    /// undecoded until then.
    Pcm(Vec<i16>),
    /// An audio stream that lives inside its own video container - Bink's or
    /// MP4's - rather than beside it. See [`ContainerTrack`] and
    /// [`container_audio::decode`].
    Container(ContainerTrack),
}

/// A movie's audio track, held until playback wants it decoded.
///
/// Held rather than decoded on the spot because the ATRAC3+ path shells out to
/// `ffmpeg` and lands in a **different** cache from the one the pictures use -
/// see [`crate::boot::default_audio_cache_dir`] - and [`super::open`] is given
/// only the movie cache. Keeping the two apart is also what lets `--prefetch`
/// and the viewer open a movie without ever paying for its sound. The PS2 path
/// has no such cost to defer - see [`MovieAudioKind::Pcm`] - but is held the
/// same way regardless, so [`crate::boot::load_movie_sound`] has one
/// interface for both.
pub struct MovieAudio {
    channels: u16,
    sample_rate: u32,
    kind: MovieAudioKind,
}

// Written out rather than derived for the reason `crate::audio::Dump` gives:
// the ATRAC3+ case is most of a megabyte of codec payload, and a `{:?}` of a
// `Movie` should say how much there is rather than print it.
impl std::fmt::Debug for MovieAudio {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut out = f.debug_struct("MovieAudio");
        out.field("channels", &self.channels);
        out.field("sample_rate", &self.sample_rate);
        match &self.kind {
            MovieAudioKind::Atrac3Plus {
                blocks,
                block_align,
            } => out
                .field("codec", &"atrac3plus")
                .field("blocks", &(blocks.len() / usize::from(*block_align).max(1)))
                .field("block_align", block_align),
            MovieAudioKind::Pcm(samples) => out
                .field("codec", &"pcm16")
                .field("samples", &samples.len()),
            MovieAudioKind::Container(track) => out
                .field("codec", &track.descriptor)
                .field("source", &track.source),
        }
        .finish()
    }
}

impl MovieAudio {
    /// Channels per frame.
    #[must_use]
    pub fn channels(&self) -> u16 {
        self.channels
    }

    /// Frames per second.
    #[must_use]
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Bytes per ATRAC3+ block. `None` for anything that is not ATRAC3+ -
    /// a PS2 PCM track or a container-hosted one - which have no such block
    /// structure to report.
    #[must_use]
    pub fn block_align(&self) -> Option<u16> {
        match &self.kind {
            MovieAudioKind::Atrac3Plus { block_align, .. } => Some(*block_align),
            MovieAudioKind::Pcm(_) | MovieAudioKind::Container(_) => None,
        }
    }

    /// How many whole ATRAC3+ blocks the track holds. Zero for a PCM or
    /// container-hosted track, neither of which is blocked that way.
    #[must_use]
    pub fn block_count(&self) -> usize {
        match &self.kind {
            MovieAudioKind::Atrac3Plus {
                blocks,
                block_align,
            } => blocks.len() / usize::from(*block_align).max(1),
            MovieAudioKind::Pcm(_) | MovieAudioKind::Container(_) => 0,
        }
    }

    /// How long the track is, in seconds.
    ///
    /// **For the ATRAC3+ case, this is not the movie's own duration and
    /// should not be expected to match it.** A block is 2,048 samples
    /// whatever the encoder had left to put in it, so a track always runs to
    /// the end of a whole number of them: `Intro.PMF` declares 40.04 s and its
    /// 865 blocks are 40.17 s. The difference is padding, not a demux that ran
    /// long. A PCM track has no such block granularity - its sample count is
    /// exactly what `oag_video::pss` demuxed. See [`ContainerTrack::seconds`]
    /// for what a container-hosted track reports and how exact it is.
    #[must_use]
    pub fn seconds(&self) -> f64 {
        match &self.kind {
            MovieAudioKind::Atrac3Plus { .. } => {
                self.block_count() as f64 * f64::from(at3::SAMPLES_PER_BLOCK)
                    / f64::from(self.sample_rate.max(1))
            }
            MovieAudioKind::Pcm(samples) => {
                samples.len() as f64
                    / f64::from(self.channels.max(1))
                    / f64::from(self.sample_rate.max(1))
            }
            MovieAudioKind::Container(track) => track.seconds,
        }
    }

    /// The clause [`crate::boot::load_movie_sound`]'s boot report inserts to
    /// say what codec or framing this track decoded from.
    pub(crate) fn codec_clause(&self) -> String {
        match &self.kind {
            MovieAudioKind::Atrac3Plus { block_align, .. } => {
                format!(
                    "{} ATRAC3+ block(s) of {block_align} bytes",
                    self.block_count()
                )
            }
            MovieAudioKind::Pcm(_) => "16-bit PCM, no compression to decode".to_string(),
            MovieAudioKind::Container(track) => track.descriptor.clone(),
        }
    }

    /// Decodes the track to interleaved PCM.
    ///
    /// The ATRAC3+ case shells out to `ffmpeg` and the cache; the PCM case is
    /// already decoded (there is no codec) and this just clones it; the
    /// container case shells out to `ffmpeg` the same way ATRAC3+ does, but
    /// pointed at the container's own cached file instead of a wrapper this
    /// project builds - see [`container_audio::decode`].
    ///
    /// # Errors
    ///
    /// As [`crate::at3::decode_frames`]: an `ffmpeg` that is absent or fails,
    /// or a cache file that will not write or read back. Never fails for a
    /// PCM track.
    pub fn decode(&self, cache_dir: &Path) -> Result<at3::Pcm> {
        match &self.kind {
            MovieAudioKind::Atrac3Plus {
                blocks,
                block_align,
            } => at3::decode_frames(
                blocks,
                at3::Format {
                    channels: self.channels,
                    sample_rate: self.sample_rate,
                    block_align: *block_align,
                },
                cache_dir,
            ),
            MovieAudioKind::Container(track) => {
                container_audio::decode(track, self.channels, self.sample_rate, cache_dir)
            }
            MovieAudioKind::Pcm(samples) => Ok(at3::Pcm {
                samples: samples.clone(),
                channels: self.channels,
                sample_rate: self.sample_rate,
            }),
        }
    }
}

/// Unwraps a demuxed `.PMF`'s audio packets into ATRAC3+ blocks.
///
/// Two layers come off, and both are the container's rather than the codec's:
/// [`AUDIO_PES_HEADER_LEN`] bytes on each packet, then
/// [`ATRAC3PLUS_FRAME_HEADER_LEN`] on each frame.
///
/// `block_align` is **measured rather than assumed**, because the disc uses two
/// values: the two 40-second reels are 744 bytes a block and the other seven
/// tracks are 560. It is the distance between the first two sync words, which
/// is then checked against every remaining frame - a stream whose sync words are
/// not evenly spaced is one this has read wrong, and saying so is better than
/// handing `ffmpeg` a block size that decodes into noise.
///
/// `None` means there is no track to play: a header that declares audio but a
/// demux that found no packets, a sample-rate code this build does not know, or
/// a stream whose framing does not check out. Never an error, because none of
/// those should cost a movie its picture.
pub(super) fn movie_audio(
    demuxed: &pmf::Demuxed,
    stream: pmf::AudioStream,
    key: &str,
) -> Option<MovieAudio> {
    let first = demuxed.audio.first()?;
    let Some(sample_rate) = stream.frequency_hz() else {
        warn!(
            "{key}'s audio is sample-rate code {}, which this build does not know, so \
             it stays silent",
            stream.frequency_code
        );
        return None;
    };

    // Where the first whole frame starts, past whatever tail of a previous one
    // the packet opens with. Zero on every movie on the disc, and read anyway
    // rather than assumed: it is the one field that tells us.
    //
    // Read through `get`, because `pmf::demux` hands over whatever followed the
    // PES header and that can be fewer than four bytes on a truncated stream -
    // and a movie that loses its sound must not also lose its picture to a
    // panic.
    let pointer: [u8; 2] = first.get(2..4).and_then(|b| b.try_into().ok())?;
    let start = usize::from(u16::from_be_bytes(pointer));

    let mut stream_bytes = Vec::new();
    for packet in &demuxed.audio {
        stream_bytes.extend_from_slice(&packet[AUDIO_PES_HEADER_LEN.min(packet.len())..]);
    }
    let body = stream_bytes.get(start..).unwrap_or_default();

    if !body.starts_with(&ATRAC3PLUS_SYNC) {
        warn!("{key}'s first audio frame carries no sync word, so it stays silent");
        return None;
    }

    // Stepped by eight because a frame is always a whole number of eight-byte
    // groups - `block_align / 8 - 1` is what the config word stores - which
    // keeps a `0f d0` that happens to fall inside codec payload from being
    // mistaken for the next frame.
    let stride = (ATRAC3PLUS_FRAME_HEADER_LEN..body.len().saturating_sub(1))
        .step_by(8)
        .find(|&at| body[at..at + 2] == ATRAC3PLUS_SYNC)?;
    let Ok(block_align) = u16::try_from(stride - ATRAC3PLUS_FRAME_HEADER_LEN) else {
        return None;
    };

    // A trailing partial frame is dropped rather than padded: the movie's own
    // duration is carried by its PTS range, so a fragment of a block would add
    // noise at the end and nothing else.
    let frames = body.len() / stride;
    let mut blocks = Vec::with_capacity(frames * usize::from(block_align));
    for index in 0..frames {
        let at = index * stride;
        if body[at..at + 2] != ATRAC3PLUS_SYNC {
            warn!("{key}'s audio loses framing at frame {index} of {frames}, so it stays silent");
            return None;
        }
        blocks.extend_from_slice(&body[at + ATRAC3PLUS_FRAME_HEADER_LEN..at + stride]);
    }

    Some(MovieAudio {
        channels: u16::from(stream.channels),
        sample_rate,
        kind: MovieAudioKind::Atrac3Plus {
            blocks,
            block_align,
        },
    })
}

/// Turns a demuxed `.PSS` `private_stream_1` track into PCM, if it is one this
/// build knows how to play.
///
/// **`None` for anything but [`pss::Format::is_pcm16`].** A `format` code this
/// build has not measured is not decoded on a guess - see the module docs on
/// `oag_video::pss` for how "PS-ADPCM" was ruled out for the one value this
/// disc's two files actually carry. Also `None` for a channel count other than
/// the one measured (2, interleaved every `interleave` bytes): de-interleaving
/// a different count would need this to also parse a scheme it has never seen,
/// which is worse than silence.
pub(super) fn pss_audio(demuxed: &pss::Demuxed, key: &str) -> Option<MovieAudio> {
    if !demuxed.format.is_pcm16() {
        warn!(
            "{key}'s audio is format code {}, which this build only knows how to play as \
             PCM (format 1), so it stays silent",
            demuxed.format.format
        );
        return None;
    }
    if demuxed.format.channels != 2 {
        warn!(
            "{key}'s audio declares {} channel(s), and this build only knows the measured \
             2-channel interleave, so it stays silent",
            demuxed.format.channels
        );
        return None;
    }
    let interleave = demuxed.format.interleave;
    if interleave == 0 || !interleave.is_multiple_of(2) {
        warn!(
            "{key}'s audio declares a {interleave}-byte interleave, which is not a whole \
             number of 16-bit samples, so it stays silent"
        );
        return None;
    }

    // Two channels, `interleave` bytes (`interleave / 2` samples) of one, then
    // `interleave` bytes of the other, repeating - see the module docs on
    // `oag_video::pss` for the evidence this is the layout rather than plain
    // sample-interleaved stereo.
    let pair = interleave * 2;
    let mut left = Vec::with_capacity(demuxed.body.len() / 4);
    let mut right = Vec::with_capacity(demuxed.body.len() / 4);
    for chunk in demuxed.body.chunks(pair) {
        let (l, r) = chunk.split_at(interleave.min(chunk.len()));
        for sample in l.as_chunks::<2>().0 {
            left.push(i16::from_le_bytes(*sample));
        }
        for sample in r.as_chunks::<2>().0 {
            right.push(i16::from_le_bytes(*sample));
        }
    }

    let frames = left.len().min(right.len());
    let mut samples = Vec::with_capacity(frames * 2);
    for i in 0..frames {
        samples.push(left[i]);
        samples.push(right[i]);
    }

    Some(MovieAudio {
        channels: 2,
        sample_rate: demuxed.format.sample_rate,
        kind: MovieAudioKind::Pcm(samples),
    })
}

/// Picks and describes a Bink file's own audio track, `None` if it declares
/// none.
///
/// `source` is the cached copy of the container's own bytes -
/// [`super::bink::open`] ensures it exists (writing it if a video-cache hit
/// skipped the write `super::bink::transcode` would otherwise have done)
/// before calling this.
///
/// **Track 0, and said so whenever there is a choice.**
/// [`oag_video::bik::Header::audio`] carries the file's own track order, and
/// this build has found no HD language/region rule that picks among several -
/// the two logo reels are the only files with more than one (four each), and
/// nothing in this project has measured which of the four a PS3 actually
/// plays. Track 0 is a choice, not a measurement, and [`MovieAudio::codec_clause`]
/// says so on every file that has more than one; a file with exactly one
/// track has no such choice to report.
pub(super) fn bink_audio(
    tracks: &[bik::AudioTrack],
    source: PathBuf,
    key: &str,
    video_seconds: f64,
) -> Option<MovieAudio> {
    let chosen = tracks.first()?;
    let codec = if chosen.is_dct() {
        "binkaudio_dct"
    } else {
        "binkaudio_rdft"
    };
    let descriptor = if tracks.len() > 1 {
        format!(
            "{codec}, track 0 of {} (chosen, not measured)",
            tracks.len()
        )
    } else {
        format!("{codec}, track 0 of 1")
    };
    Some(MovieAudio {
        channels: chosen.channels(),
        sample_rate: chosen.sample_rate,
        kind: MovieAudioKind::Container(ContainerTrack {
            key: key.to_string(),
            source,
            stream_index: 0,
            descriptor,
            // No per-block sample count is in a Bink header at all - see
            // `ContainerTrack::seconds`'s own doc for why the video's own
            // duration stands in for the audio's until it is decoded.
            seconds: video_seconds,
        }),
    })
}

/// Describes an MP4 file's own audio track.
///
/// `source` is the cached copy of the container's own bytes -
/// [`super::mp4::open`] ensures it exists the same way [`bink_audio`]'s
/// caller does. Unlike a Bink file, only one of the 26 shipped MP4s carries a
/// track at all, so there is no track to choose among and no note to add.
pub(super) fn mp4_audio(
    track: &oag_video::mp4::AudioTrack,
    source: PathBuf,
    key: &str,
) -> MovieAudio {
    let codec = if &track.codec == b"mp4a" {
        "aac".to_string()
    } else {
        String::from_utf8_lossy(&track.codec).into_owned()
    };
    // Exact, unlike Bink's approximation: `AudioTrack::frame_delta`'s own doc
    // is the evidence that a tick of this track's own `mdhd` timescale is a
    // PCM sample here.
    let seconds = track.frame_count as f64 * f64::from(track.frame_delta)
        / f64::from(track.sample_rate.max(1));
    MovieAudio {
        channels: track.channel_count,
        sample_rate: track.sample_rate,
        kind: MovieAudioKind::Container(ContainerTrack {
            key: key.to_string(),
            source,
            stream_index: 0,
            descriptor: format!("{codec}, track 0 of 1"),
            seconds,
        }),
    }
}

#[cfg(test)]
mod tests;
