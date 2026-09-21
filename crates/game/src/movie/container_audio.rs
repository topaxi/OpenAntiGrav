//! An audio track that lives inside its own movie container - Bink's
//! `binkaudio_dct`/`_rdft` and MP4's AAC - rather than beside the picture the
//! way a `.PMF`'s ATRAC3+ frames or a `.PSS`'s PCM samples do.
//!
//! Mirrors `crate::at3`'s shell-out-and-cache idiom, and the difference from
//! it is the whole reason this is a separate module rather than a third case
//! folded into that one: `at3::decode`/`decode_frames` are handed a codec
//! payload with no picture in it at all, where a container track's own bytes
//! *are* the file [`super::bink::transcode`]/[`super::mp4::transcode`] have
//! already written to disk for `ffmpeg` to read the video out of. `ffmpeg` is
//! told which stream to pull with `-map` instead of being handed a bespoke
//! wrapper the way [`crate::at3::riff`] builds one for bare ATRAC3+ frames.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use log::info;

use crate::at3::Pcm;

/// One audio stream inside a movie's own video container.
///
/// `source` is the cached copy of the container's own bytes that
/// [`super::bink::open`] and [`super::mp4::open`] ensure exists before
/// building one of these - the same file their own `transcode` hands to
/// `ffmpeg` for the picture. Kept as a path rather than the container's bytes
/// themselves so a [`super::Movie`] that never plays its sound
/// (`--prefetch`, the asset viewer) never carries them twice.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct ContainerTrack {
    /// The cache key this movie was opened with - see `crate::movie::open`'s
    /// own doc for why it changes when the bytes do. Reused here rather than
    /// hashing `source` afresh, which would mean reading a multi-megabyte
    /// file again just to name the PCM cache that decodes it.
    pub key: String,
    /// Where the container's own bytes sit on disk, under the **video**
    /// cache directory - not `cache_dir` in [`decode`], which is the audio
    /// cache and holds only the PCM this produces. See
    /// [`super::track::MovieAudio`]'s own doc for why the two are kept apart.
    pub source: PathBuf,
    /// `ffmpeg -map 0:a:<n>` - counts only audio streams, in the container's
    /// own order, which is [`Self::key`]'s file read back through whichever
    /// demuxer `ffmpeg` picks for the extension on [`Self::source`].
    pub stream_index: usize,
    /// What [`super::track::MovieAudio::codec_clause`] reports for this
    /// track - the codec name and, when the container declared more than one
    /// candidate track, which one was picked and on what basis. Composed once
    /// at construction (`super::bink::bink_audio`/`super::mp4::mp4_audio`)
    /// rather than out of raw fields here, because composing it is exactly as
    /// much container-specific knowledge as reading the header was.
    pub descriptor: String,
    /// This track's own duration in seconds, for
    /// [`super::track::MovieAudio::seconds`].
    ///
    /// **Exact for MP4**: `frame_count * frame_delta / sample_rate` off the
    /// audio `trak`'s own `stsz` and `stts` boxes - see
    /// [`oag_video::mp4::AudioTrack::frame_delta`]'s own doc for why a tick of
    /// that box's timescale is a PCM sample on this file.
    ///
    /// **An approximation for Bink**: [`oag_video::bik::Header`] carries no
    /// audio sample count at all, only a per-track sample rate and the
    /// largest single decoded frame's byte size, so this is the *video*
    /// track's own measured duration standing in until the audio is actually
    /// decoded - see `super::bink::bink_audio`.
    pub seconds: f64,
}

/// Decodes one audio stream out of a container, through the cache.
///
/// `channels` and `sample_rate` come from the caller
/// ([`super::track::MovieAudio::decode`]) rather than from `track` itself,
/// the same split [`crate::at3::decode_frames`] makes: they are properties of
/// the *stream*, already read off the container header when [`ContainerTrack`]
/// was built, and handing them in here rather than storing a third copy on
/// this struct keeps one place answering "what does this track sound like".
///
/// # Errors
///
/// An `ffmpeg` that is absent or fails, or a cache file that will not write or
/// read back. **Never re-parses the container**: if `track.source` is
/// missing, that is a bug in whichever `open` built this [`ContainerTrack`],
/// not a malformed movie, and is reported as the plain I/O error it is rather
/// than folded into ADR-0019's "no ffmpeg" degradation path.
pub(super) fn decode(
    track: &ContainerTrack,
    channels: u16,
    sample_rate: u32,
    cache_dir: &Path,
) -> Result<Pcm> {
    let out = cache_path(track, channels, sample_rate, cache_dir);

    if let Some(pcm) = read_cached(&out, channels, sample_rate) {
        return Ok(pcm);
    }

    std::fs::create_dir_all(cache_dir)
        .with_context(|| format!("creating {}", cache_dir.display()))?;
    run_ffmpeg(track, channels, sample_rate, &out)?;

    read_cached(&out, channels, sample_rate)
        .with_context(|| format!("{} decoded to nothing usable", out.display()))
}

/// Where this stream's decoded samples land - keyed by the movie's own
/// content-addressed [`ContainerTrack::key`] plus which stream and geometry,
/// the same "geometry is in the name" rule [`crate::at3::cache_path`] states.
fn cache_path(
    track: &ContainerTrack,
    channels: u16,
    sample_rate: u32,
    cache_dir: &Path,
) -> PathBuf {
    cache_dir.join(format!(
        "{}-a{}-{channels}ch-{sample_rate}hz.s16le",
        track.key, track.stream_index
    ))
}

/// Reads a cache file back, or `None` if it is missing or the wrong shape.
///
/// The same not-empty-and-a-whole-number-of-frames check
/// [`crate::at3::read_cached`] applies, and for the same reason: an `ffmpeg`
/// killed partway through leaves a truncated file behind, and that must be
/// re-decoded rather than played as a shorter track.
fn read_cached(path: &Path, channels: u16, sample_rate: u32) -> Option<Pcm> {
    let bytes = std::fs::read(path).ok()?;
    let per_frame = usize::from(channels) * 2;
    if bytes.is_empty() || per_frame == 0 || !bytes.len().is_multiple_of(per_frame) {
        return None;
    }
    let samples = bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| i16::from_le_bytes(*pair))
        .collect();
    Some(Pcm {
        samples,
        channels,
        sample_rate,
    })
}

/// Runs `ffmpeg` to pull one audio stream out of a container into raw
/// interleaved `s16le`.
///
/// `-map 0:a:<n>` rather than `-map 0:<track id>`: `ffmpeg`'s own stream
/// indices are per **kind** (video streams and audio streams numbered
/// separately from zero), which is why [`ContainerTrack::stream_index`] is
/// documented as counting only audio streams - a Bink track's own `id` field
/// happens to agree with this because every shipped file numbers its audio
/// tracks `0..n` with no other stream kind between them, but this is not
/// assumed here, only relied on for track 0.
fn run_ffmpeg(
    track: &ContainerTrack,
    channels: u16,
    sample_rate: u32,
    output: &Path,
) -> Result<()> {
    info!(
        "decoding {}'s audio stream {} into {} (once; cached after this)",
        track.source.display(),
        track.stream_index,
        output.display()
    );

    let map = format!("0:a:{}", track.stream_index);
    let result = std::process::Command::new("ffmpeg")
        .arg("-hide_banner")
        .args(["-loglevel", "error"])
        .arg("-y")
        .arg("-i")
        .arg(&track.source)
        .args(["-vn"])
        .args(["-map", &map])
        .args(["-acodec", "pcm_s16le"])
        .args(["-ar", &sample_rate.to_string()])
        .args(["-ac", &channels.to_string()])
        .args(["-f", "s16le"])
        .arg(output)
        .output();

    match result {
        Ok(done) if done.status.success() => Ok(()),
        Ok(done) => bail!(
            "ffmpeg exited with {}: {}",
            done.status,
            String::from_utf8_lossy(&done.stderr).trim()
        ),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => bail!(
            "ffmpeg is not on PATH. Install it to hear {}; without it the movie \
             still plays, in silence",
            track.source.display()
        ),
        Err(e) => Err(e).context("running ffmpeg"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn track(cache_dir: &Path) -> ContainerTrack {
        ContainerTrack {
            key: "test".to_string(),
            source: cache_dir.join("test.bik"),
            stream_index: 0,
            descriptor: "binkaudio_dct, track 0 of 1".to_string(),
            seconds: 1.0,
        }
    }

    /// The cache path is stable for the same track and geometry, and changes
    /// when either does - the same rule the video cache's `cache_name` is
    /// asserted against.
    #[test]
    fn the_cache_path_is_keyed_by_track_and_geometry() {
        let dir = Path::new("/cache");
        let a = cache_path(&track(dir), 2, 48_000, dir);
        let b = cache_path(&track(dir), 2, 48_000, dir);
        assert_eq!(a, b, "the same track and geometry name the same file");

        let mut other = track(dir);
        other.key = "different".to_string();
        assert_ne!(
            cache_path(&other, 2, 48_000, dir),
            a,
            "key changes the name"
        );

        assert_ne!(
            cache_path(&track(dir), 1, 48_000, dir),
            a,
            "channel count changes the name"
        );
        assert_ne!(
            cache_path(&track(dir), 2, 44_100, dir),
            a,
            "sample rate changes the name"
        );
    }

    /// A cache file that is not a whole number of frames - the shape an
    /// `ffmpeg` killed partway through leaves behind - is not returned as a
    /// track, the same rule `crate::at3::read_cached` applies.
    #[test]
    fn a_truncated_cache_file_is_rejected() {
        let dir = std::env::temp_dir().join("oag-container-audio-test");
        std::fs::create_dir_all(&dir).expect("creating a scratch dir");
        let path = dir.join("truncated.s16le");
        std::fs::write(&path, [0u8; 3]).expect("writing 3 stray bytes");
        assert!(read_cached(&path, 2, 48_000).is_none());
        let _ = std::fs::remove_file(&path);
    }

    /// A whole number of stereo frames reads back interleaved, left first -
    /// the same round trip [`crate::at3::from_s16le`] is asserted against.
    #[test]
    fn a_whole_cache_file_reads_back_as_pcm() {
        let dir = std::env::temp_dir().join("oag-container-audio-test");
        std::fs::create_dir_all(&dir).expect("creating a scratch dir");
        let path = dir.join("whole.s16le");
        let mut bytes = Vec::new();
        for sample in [1i16, -1, 2, -2] {
            bytes.extend_from_slice(&sample.to_le_bytes());
        }
        std::fs::write(&path, &bytes).expect("writing whole frames");
        let pcm = read_cached(&path, 2, 48_000).expect("a whole number of frames");
        assert_eq!(pcm.samples, vec![1, -1, 2, -2]);
        assert_eq!(pcm.channels, 2);
        assert_eq!(pcm.sample_rate, 48_000);
        let _ = std::fs::remove_file(&path);
    }
}
