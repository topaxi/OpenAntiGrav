//! ATRAC3+, decoded out of process and cached.
//!
//! Almost every sound on the PSP disc is ATRAC3+ and nothing in this workspace
//! can decode one. [ADR-0019](../../../docs/architecture/adr/0019-atrac3plus-out-of-process.md)
//! settled how that is answered, and it is the answer H.264 already has in
//! `oag_game::movie`: shell out to `ffmpeg` once, keep the result under
//! `data/cache/`, read the cache every time after. GStreamer was measured first
//! and cannot do it at all - not one of 1,401 installed elements advertises
//! `atrac3plus` caps, because gst-libav's codec map has no entry for it.
//!
//! **A missing `ffmpeg` is never fatal.** It comes back as an error naming the
//! tool, and the caller turns that into silence plus a line on stdout, exactly
//! as a missing `ffmpeg` turns the intro into a black picture rather than a
//! failed boot.
//!
//! # Two shapes go in, one comes out
//!
//! | Input | Where it comes from | What happens |
//! | --- | --- | --- |
//! | RIFF-wrapped `.at3` | A `Data.wad` entry | Handed to `ffmpeg` unaltered |
//! | Bare ATRAC3+ frames | `oag_video::pmf::Demuxed::audio` | Wrapped by [`riff`] first |
//!
//! The second shape exists because `ffmpeg`'s `mpegps` demuxer cannot see a
//! `.PMF`'s audio track at all - probed to 50 MB on `Intro.PMF`, only the H.264
//! video ever appears - so the frames have to come out of our own demuxer and
//! be given a container `ffmpeg` will open.
//!
//! # What the cache is keyed by
//!
//! The **content** of the bytes handed to `ffmpeg`, not a name or an archive
//! offset. `oag_game::movie` keys on `{name hash}-{size}` because a movie is
//! always addressed by a WAD name; a sound may arrive wrapped, from a demux, or
//! straight off the disc, and only the bytes themselves distinguish those. It
//! also means changing [`riff`] invalidates every wrapped entry by itself,
//! since the wrapper is part of what is hashed.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
#[cfg(not(target_arch = "wasm32"))]
use log::info;
use oag_core::hash::StateHasher;

/// Decoded interleaved samples and what they are.
///
/// Shaped for [`oag_audio::Sound::new`], which takes exactly these three
/// things: nothing here converts, resamples or mixes down.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pcm {
    /// Signed 16-bit, interleaved, left first.
    pub samples: Vec<i16>,
    /// Channels per frame: 2 for the soundtrack, 1 for the voice clips.
    pub channels: u16,
    /// Frames per second. Every ATRAC3+ stream on the disc is 44,100.
    pub sample_rate: u32,
}

/// The stream parameters the RIFF wrapper carries and the cache name records.
///
/// `block_align` is **per file** and must come from the stream. `SND0.AT3` and
/// the soundtrack entries are 560 bytes a block; the 32 short mono clips at
/// `Data.wad` indices 899-930 are 280. Copying one file's value onto another
/// produces a decode that is wrong rather than one that fails.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Format {
    /// Channels per frame.
    pub channels: u16,
    /// Frames per second.
    pub sample_rate: u32,
    /// Bytes per ATRAC3+ block.
    pub block_align: u16,
}

/// ATRAC3+ samples per block, the `wValidBitsPerSample` field's meaning in a
/// `WAVE_FORMAT_EXTENSIBLE` header for this codec.
///
/// Not a bit depth despite the field name: read off `SND0.AT3`, which stores
/// 2048 there beside a `wBitsPerSample` of zero.
pub const SAMPLES_PER_BLOCK: u16 = 2048;

/// The `WAVE_FORMAT_EXTENSIBLE` subformat GUID that means ATRAC3+.
///
/// `E923AABF-CB58-4471-A119-FFFA01E4CE62`, in the mixed-endian byte order a
/// `GUID` is stored in - which is why this is a byte array rather than the
/// dashed form: the bytes are what goes in the file.
const ATRAC3PLUS_GUID: [u8; 16] = [
    0xbf, 0xaa, 0x23, 0xe9, 0x58, 0xcb, 0x71, 0x44, 0xa1, 0x19, 0xff, 0xfa, 0x01, 0xe4, 0xce, 0x62,
];

/// The first two bytes of the codec extra data, before the config word.
///
/// `01 00` on every RIFF-wrapped stream on the disc, at all three block sizes
/// and both channel counts, so it is a constant rather than something derived.
const CODEC_EXTRA_PREFIX: [u8; 2] = [0x01, 0x00];

/// The codec config word the extra data carries, as [`Format`] states it.
///
/// **Derived rather than copied, and the disc is what settled it.** The four
/// non-zero bytes used to be reproduced verbatim off `SND0.AT3` - `01 00 28
/// 45` - with what they meant left open. Reading the `fmt ` chunk of every
/// RIFF-wrapped ATRAC3+ entry in `Data.wad` gives three combinations, and one
/// formula accounts for all of them:
///
/// | `block_align` | channels | entries | config word |
/// | --- | --- | --- | --- |
/// | 280 | 1 | 32 | `0x2422` |
/// | 560 | 1 | 32 | `0x2445` |
/// | 560 | 2 | 28 | `0x2845` |
///
/// The low 10 bits are `block_align / 8 - 1` (34, 69, 69) and the top 6 are
/// `8 + channels` (9, 9, 10). A fourth combination confirms it from a
/// completely separate direction: the **8-byte header on every ATRAC3+ frame
/// inside a `.PMF`** carries this same word in its third and fourth bytes -
/// `28 45` on the disc's 560-byte movie tracks, and `28 5c` on `Intro.PMF`,
/// whose blocks are 744 bytes. `(8 + 2) << 10 | (744 / 8 - 1)` is `0x285c`.
/// See `oag_game::movie` for that header.
///
/// This matters because the constant it replaces was a **stereo 560** word.
/// Writing it into `Intro.PMF`'s stereo 744 stream, or into any of the mono
/// clips, states a geometry the `fmt ` chunk beside it contradicts. `ffmpeg`
/// happens not to read it - it decodes `Intro.PMF` correctly either way,
/// which is exactly why this would never have been noticed by listening.
fn codec_config(format: Format) -> u16 {
    // Saturating rather than wrapping: a caller cannot produce a block size
    // this cannot describe, and a silent wrap would write a plausible word for
    // the wrong geometry - the failure this whole function exists to remove.
    let blocks = (format.block_align / 8).saturating_sub(1) & 0x3ff;
    let channels = (8u16.saturating_add(format.channels)) & 0x3f;
    (channels << 10) | blocks
}

/// Bytes in the `fmt ` chunk body this wrapper writes.
///
/// 52: the 16 of a `WAVEFORMATEX` core, 2 for `cbSize`, then the 34 `cbSize`
/// declares - 2 for `wValidBitsPerSample`, 4 for `dwChannelMask`, 16 for the
/// subformat GUID and 12 of codec extra data.
const FMT_LEN: u32 = 52;

/// Bytes [`riff`] writes before the first frame.
///
/// 80: the 12-byte RIFF/WAVE preamble, the 8-byte `fmt ` chunk header, its
/// [`FMT_LEN`] body, and the 8-byte `data` chunk header.
const HEADER_LEN: usize = 12 + 8 + FMT_LEN as usize + 8;

/// Decodes a RIFF-wrapped ATRAC3+ file, through the cache.
///
/// This is a `Data.wad` entry exactly as it is stored - the disc's own
/// container, handed to `ffmpeg` unaltered.
///
/// # Errors
///
/// A blob that is not a RIFF/WAVE with a readable `fmt ` chunk, an `ffmpeg`
/// that is absent or fails, or a cache file that will not write or read back.
pub fn decode(at3: &[u8], cache_dir: &Path) -> Result<Pcm> {
    let format = read_format(at3)?;
    decode_riff(at3, format, cache_dir)
}

/// Fills the cache for a RIFF-wrapped ATRAC3+ file without reading it back.
///
/// [`decode`] minus the samples, and the difference is the whole reason it
/// exists: `oag_game::prefetch` converts all 93 of the disc's streams and wants
/// none of them, and reading each one back would be 0.70 GB of PCM through
/// memory to be dropped a line later. Returns the cache file's path.
///
/// The already-cached test is the same rule [`from_s16le`] applies - not empty,
/// a whole number of frames - taken off the file's metadata rather than its
/// contents, so a resumed run costs one `stat` per stream instead of one read.
///
/// # Errors
///
/// As [`decode`]: a blob that is not a readable RIFF/WAVE, an `ffmpeg` that is
/// absent or fails, or a cache file that will not write.
pub fn ensure_cached(at3: &[u8], cache_dir: &Path) -> Result<PathBuf> {
    let format = read_format(at3)?;
    let out = cache_path(at3, format, cache_dir);
    if !is_usable(&out, format) {
        transcode(at3, &out, format)?;
    }
    Ok(out)
}

/// Where this stream's samples are (or would be) in the cache, or `None` for a
/// blob that is not a readable RIFF.
///
/// What a manifest of which cache files belong to which disc is built from:
/// the name is the content key, so it is the same file [`ensure_cached`] writes.
#[must_use]
pub fn cache_file(at3: &[u8], cache_dir: &Path) -> Option<PathBuf> {
    read_format(at3)
        .ok()
        .map(|format| cache_path(at3, format, cache_dir))
}

/// Whether this stream's samples are already in the cache.
///
/// The question `oag_game::prefetch` asks while planning, so that the total it
/// reports is what is left to do rather than what exists. `false` for a blob
/// that is not a readable RIFF at all: it has no cache file by construction,
/// and [`ensure_cached`] is the one that reports why.
#[must_use]
pub fn is_cached(at3: &[u8], cache_dir: &Path) -> bool {
    read_format(at3).is_ok_and(|format| is_usable(&cache_path(at3, format, cache_dir), format))
}

/// The same shape rule [`from_s16le`] applies - not empty, a whole number of
/// frames - read off the file's metadata rather than its contents.
///
/// Metadata rather than a read because the answer is wanted for all 93 streams
/// at once and the contents come to 0.70 GB. It catches the case that actually
/// happens for the same reason [`read_cached`] does: an `ffmpeg` killed partway
/// through leaves a truncated file behind.
fn is_usable(path: &Path, format: Format) -> bool {
    let per_frame = u64::from(format.channels) * 2;
    std::fs::metadata(path)
        .ok()
        .filter(|meta| meta.is_file())
        .is_some_and(|meta| {
            per_frame != 0 && meta.len() != 0 && meta.len().is_multiple_of(per_frame)
        })
}

/// Decodes bare ATRAC3+ frames, wrapping them first.
///
/// `format` has to be supplied because bare frames carry none of it: a `.PMF`'s
/// PSMF header states the channel count and a frequency code, and the block
/// size comes from the audio stream descriptor rather than from the frames.
///
/// `oag_game::movie::MovieAudio::decode` is the caller, and the only one: it
/// unwraps a `.PMF`'s own two layers of framing first, which is where the
/// `block_align` this cannot infer comes from.
///
/// # Errors
///
/// As [`decode`], minus the parse: the wrapper this builds is always readable.
pub fn decode_frames(frames: &[u8], format: Format, cache_dir: &Path) -> Result<Pcm> {
    let wrapped = riff(frames, format);
    decode_riff(&wrapped, format, cache_dir)
}

/// Wraps bare ATRAC3+ frames in the RIFF header `ffmpeg` opens.
///
/// The layout is read off `PSP_GAME/SND0.AT3`, whose own 52-byte `fmt ` chunk
/// is the reference. Every field below is that file's, with `channels`,
/// `sample_rate` and `block_align` taken from the caller instead:
///
/// ```text
/// +0x00  "RIFF"  u32 size-8  "WAVE"
/// +0x0c  "fmt "  u32 52
/// +0x14  u16     wFormatTag          0xFFFE (WAVE_FORMAT_EXTENSIBLE)
/// +0x16  u16     nChannels
/// +0x18  u32     nSamplesPerSec
/// +0x1c  u32     nAvgBytesPerSec
/// +0x20  u16     nBlockAlign         per file - 560 or 280 on this disc
/// +0x22  u16     wBitsPerSample      0
/// +0x24  u16     cbSize              34
/// +0x26  u16     wValidBitsPerSample 2048 (samples per block, not a depth)
/// +0x28  u32     dwChannelMask       3
/// +0x2c  u8[16]  SubFormat GUID
/// +0x3c  u8[12]  codec extra data - see `codec_config`
/// +0x48  "data"  u32 length
/// ```
///
/// This is `ffmpeg`'s container rather than Wipeout's, which is why it is built
/// here and not in `oag-formats` - the same argument `oag_game::movie::ipum`
/// makes for the `ipum` header it writes for the PS2's IPU bitstreams.
#[must_use]
pub fn riff(frames: &[u8], format: Format) -> Vec<u8> {
    // Derived rather than carried, because it is derivable: one block is
    // `block_align` bytes and holds `SAMPLES_PER_BLOCK` samples per channel.
    let avg_bytes_per_sec =
        u32::from(format.block_align) * format.sample_rate / u32::from(SAMPLES_PER_BLOCK).max(1);

    let mut out = Vec::with_capacity(HEADER_LEN + frames.len());
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&((HEADER_LEN - 8 + frames.len()) as u32).to_le_bytes());
    out.extend_from_slice(b"WAVE");

    out.extend_from_slice(b"fmt ");
    out.extend_from_slice(&FMT_LEN.to_le_bytes());
    out.extend_from_slice(&0xfffe_u16.to_le_bytes());
    out.extend_from_slice(&format.channels.to_le_bytes());
    out.extend_from_slice(&format.sample_rate.to_le_bytes());
    out.extend_from_slice(&avg_bytes_per_sec.to_le_bytes());
    out.extend_from_slice(&format.block_align.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&34u16.to_le_bytes());
    out.extend_from_slice(&SAMPLES_PER_BLOCK.to_le_bytes());
    out.extend_from_slice(&3u32.to_le_bytes());
    out.extend_from_slice(&ATRAC3PLUS_GUID);
    out.extend_from_slice(&CODEC_EXTRA_PREFIX);
    out.extend_from_slice(&codec_config(format).to_be_bytes());
    // The remaining eight bytes are zero on every entry on the disc.
    out.extend_from_slice(&[0u8; 8]);

    out.extend_from_slice(b"data");
    out.extend_from_slice(&(frames.len() as u32).to_le_bytes());
    out.extend_from_slice(frames);
    out
}

/// What a RIFF-wrapped stream declares about itself, before anything decodes
/// it.
///
/// Enough to decide whether a `Data.wad` entry is a soundtrack track without
/// handing 2 MiB of ATRAC3+ to `ffmpeg` to find out - see
/// `oag_sound::MusicSource`, which pairs the two discs' soundtracks by
/// length.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stream {
    /// The `fmt ` chunk's geometry.
    pub format: Format,
    /// The `fact` chunk's sample count: decoded samples **per channel**.
    ///
    /// `None` for a stream that carries no `fact` chunk, which is every one
    /// [`riff`] writes and none of the disc's own. It is not the same number as
    /// the block count times [`SAMPLES_PER_BLOCK`] - the encoder pads the last
    /// block, and on the disc's soundtrack entries the difference is a few
    /// hundred samples. That is why a duration comes from here rather than from
    /// the stored size.
    pub samples: Option<u32>,
}

impl Stream {
    /// How long the stream is, in seconds, when it says.
    #[must_use]
    pub fn seconds(&self) -> Option<f64> {
        let samples = self.samples?;
        (self.format.sample_rate > 0)
            .then(|| f64::from(samples) / f64::from(self.format.sample_rate))
    }
}

/// Reads what the `fmt ` and `fact` chunks declare.
///
/// Chunk-walked rather than read at a fixed offset: `Data.wad`'s entries carry
/// `fact` and `smpl` chunks as well, in an order nothing guarantees. The walk
/// stops at `data`, whose body is the whole stream and holds no chunks -
/// nothing on the disc puts anything after it.
///
/// Parsed here rather than in `oag-formats` because these fields are all the
/// transcoder needs and RIFF is not one of the disc's own formats - it is
/// Microsoft's, and the only reason it appears is that Sony's encoder wrote it.
///
/// # Errors
///
/// A blob that is not a RIFF/WAVE, or one with no readable `fmt ` chunk.
pub fn describe(blob: &[u8]) -> Result<Stream> {
    if blob.len() < 12 || !blob.starts_with(b"RIFF") || &blob[8..12] != b"WAVE" {
        bail!("not a RIFF/WAVE file");
    }

    let mut at = 12usize;
    let mut format = None;
    let mut samples = None;
    while at + 8 <= blob.len() {
        let id = &blob[at..at + 4];
        let len = u32::from_le_bytes(blob[at + 4..at + 8].try_into().expect("four bytes")) as usize;
        // A `data` body is the entire stream, so a caller that peeked only the
        // header has it truncated. That is not a malformed file, and the walk
        // is over either way.
        if id == b"data" {
            break;
        }
        let body = blob
            .get(at + 8..at + 8 + len)
            .context("a RIFF chunk runs past the end of the file")?;

        if id == b"fmt " {
            // 16 is a bare `WAVEFORMATEX`; everything read below is inside it.
            if body.len() < 16 {
                bail!("the fmt chunk is {} bytes, too short to read", body.len());
            }
            format = Some(Format {
                channels: u16::from_le_bytes(body[2..4].try_into().expect("two bytes")),
                sample_rate: u32::from_le_bytes(body[4..8].try_into().expect("four bytes")),
                block_align: u16::from_le_bytes(body[12..14].try_into().expect("two bytes")),
            });
        } else if id == b"fact" && body.len() >= 4 {
            samples = Some(u32::from_le_bytes(
                body[..4].try_into().expect("four bytes"),
            ));
        }

        // RIFF pads every odd-length chunk to an even boundary, and the pad
        // byte is not counted in the length.
        at += 8 + len + (len & 1);
    }

    let format = format.context("no fmt chunk")?;
    Ok(Stream { format, samples })
}

/// Reads `channels`, `sample_rate` and `block_align` out of a RIFF `fmt `
/// chunk.
fn read_format(blob: &[u8]) -> Result<Format> {
    Ok(describe(blob)?.format)
}

/// Decodes a complete RIFF file, reading the cache when it is already there.
///
/// `pub(crate)` rather than private: [`crate::at9`] shares it. Once a blob is
/// confirmed RIFF-wrapped ATRAC9 by its own subformat GUID, decoding it is
/// exactly this - `ffmpeg` reads the real GUID out of the file itself and
/// picks its own decoder, so nothing downstream of that check is
/// ATRAC3+-specific at all. See `at9`'s own module doc.
pub(crate) fn decode_riff(riff: &[u8], format: Format, cache_dir: &Path) -> Result<Pcm> {
    let out = cache_path(riff, format, cache_dir);

    if let Some(pcm) = read_cached(&out, format) {
        return Ok(pcm);
    }

    transcode(riff, &out, format)?;

    read_cached(&out, format)
        .with_context(|| format!("{} decoded to nothing usable", out.display()))
}

/// Where a stream's decoded samples land.
///
/// Geometry is in the **name**, so a cache file is self-describing and reading
/// one back needs no sidecar and no header parse - the same reason
/// `oag_game::movie` puts its codec in the filename rather than trusting the
/// contents.
fn cache_path(riff: &[u8], format: Format, cache_dir: &Path) -> PathBuf {
    cache_dir.join(format!(
        "{}-{}ch-{}hz.s16le",
        content_key(riff),
        format.channels,
        format.sample_rate
    ))
}

/// Runs the decode unconditionally, writing `out` and the input beside it.
///
/// Split from [`decode_riff`] so that [`ensure_cached`] can reach the same
/// conversion without the read-back, rather than growing a second copy of it.
fn transcode(riff: &[u8], out: &Path, format: Format) -> Result<()> {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (riff, out, format);
        return Err(NoDecoderHere.into());
    }
    #[cfg(not(target_arch = "wasm32"))]
    transcode_with_ffmpeg(riff, out, format)
}

/// The music this module decodes cannot be decoded where the game runs: in a
/// browser there is no `ffmpeg` to run, and no Rust ATRAC3+ decoder exists
/// (the decision on one is deferred). The music stays absent, never faked; a
/// caller tells this apart with `downcast_ref` to say so once rather than per
/// track. See docs/tools/web.md, "Sound".
#[derive(Debug, Clone, Copy)]
pub struct NoDecoderHere;

impl std::fmt::Display for NoDecoderHere {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(
            "ATRAC3+ and RIFF-wrapped ATRAC9 are decoded by ffmpeg, which a browser cannot run",
        )
    }
}

impl std::error::Error for NoDecoderHere {}

#[cfg(not(target_arch = "wasm32"))]
fn transcode_with_ffmpeg(riff: &[u8], out: &Path, format: Format) -> Result<()> {
    let cache_dir = out.parent().unwrap_or(Path::new("."));
    std::fs::create_dir_all(cache_dir)
        .with_context(|| format!("creating {}", cache_dir.display()))?;

    // Written beside the cache because it is the exact input ffmpeg saw, so a
    // decode that comes out wrong can be reproduced by hand against the same
    // bytes. Same reasoning as `oag_game::movie::transcode`'s `.h264` file.
    let source = cache_dir.join(format!("{}.at3", content_key(riff)));
    std::fs::write(&source, riff).with_context(|| format!("writing {}", source.display()))?;

    run_ffmpeg(&source, out, format)
}

/// Reads a cache file back, or `None` if it is missing or the wrong shape.
///
/// The shape check is a whole number of frames and not empty. It is cheap and
/// it catches the case that actually happens: an `ffmpeg` killed partway
/// through leaves a truncated file behind, and a truncated file must be
/// re-decoded rather than played as a shorter track.
fn read_cached(path: &Path, format: Format) -> Option<Pcm> {
    from_s16le(&std::fs::read(path).ok()?, format)
}

/// Turns raw little-endian `s16le` into a [`Pcm`], or `None` if it is not a
/// whole number of frames.
///
/// Split from [`read_cached`] so the shape rule is testable without a
/// filesystem - the same split `oag_game::movie::frame_at` gets from its own
/// loop.
fn from_s16le(bytes: &[u8], format: Format) -> Option<Pcm> {
    let per_frame = usize::from(format.channels) * 2;
    if bytes.is_empty() || per_frame == 0 || !bytes.len().is_multiple_of(per_frame) {
        return None;
    }
    let samples = bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| i16::from_le_bytes([pair[0], pair[1]]))
        .collect();
    Some(Pcm {
        samples,
        channels: format.channels,
        sample_rate: format.sample_rate,
    })
}

/// FNV-1a over the bytes, as sixteen hex digits.
///
/// [`oag_core::hash::StateHasher`] rather than the standard library's default
/// hasher, and the distinction is the whole point: `RandomState` is seeded per
/// process, so a key built from it would name a different file every run and
/// the cache would never be hit. This one is fixed for all time by
/// construction - it is the determinism tripwire's own hasher.
fn content_key(bytes: &[u8]) -> String {
    let mut hasher = StateHasher::new();
    hasher.write(bytes);
    format!("{:016x}", hasher.finish())
}

/// Runs `ffmpeg` to decode `input` into raw interleaved `s16le` at `output`.
///
/// Raw rather than WAV on purpose: `oag_audio::wav` writes and does not read,
/// so a WAV cache would mean carrying a RIFF parser just to get back to the
/// samples that were already there.
///
/// `-ar` and `-ac` restate what the `fmt ` chunk declared, so the file always
/// matches the geometry its own name records. Verified to be a no-op on
/// `frontend1.at3`: forcing 2 channels at 44,100 Hz and letting `ffmpeg`
/// choose produce byte-identical output.
#[cfg(not(target_arch = "wasm32"))]
fn run_ffmpeg(input: &Path, output: &Path, format: Format) -> Result<()> {
    info!(
        "decoding {} into {} (once; cached after this)",
        input.display(),
        output.display()
    );

    // Captured rather than inherited, unlike `oag_game::movie::run_ffmpeg`, and
    // for a reason particular to this codec: the ATRAC3+ decoder hands the
    // raw muxer one packet per block with a repeated dts, and ffmpeg logs
    // "non monotonically increasing dts" at **error** level for every one of
    // them - 638 lines for a 30-second track, none of which mean anything.
    // `-loglevel error` cannot filter them out because that is the level they
    // are at, so they are held and printed only if the run actually fails.
    let result = std::process::Command::new("ffmpeg")
        .arg("-hide_banner")
        .args(["-loglevel", "error"])
        .arg("-y")
        .arg("-i")
        .arg(input)
        .args(["-vn"])
        .args(["-acodec", "pcm_s16le"])
        .args(["-ar", &format.sample_rate.to_string()])
        .args(["-ac", &format.channels.to_string()])
        .args(["-f", "s16le"])
        .arg(output)
        .output();

    match result {
        Ok(done) if done.status.success() => Ok(()),
        Ok(done) => bail!(
            "ffmpeg exited with {}: {}. If it reports an unknown decoder, this \
             build of ffmpeg lacks atrac3plus",
            done.status,
            String::from_utf8_lossy(&done.stderr).trim()
        ),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => bail!(
            "ffmpeg is not on PATH. Install it to hear the PSP soundtrack; \
             without it the game still runs, in silence"
        ),
        Err(e) => Err(e).context("running ffmpeg"),
    }
}

#[cfg(test)]
mod tests;
