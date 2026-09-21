//! `.mp4`: ISO Base Media File Format, which is what Wipeout 2048's movies are.
//!
//! Twenty-six files on the disc, all under `data/Videos/`: the boot intro, three
//! season recap movies (`2048Movie.mp4`/`2049Movie.mp4`/`2050Movie.mp4`), two
//! "best bits" reels (`bb2048.mp4`, `bb2048Zone8.mp4`) and twenty
//! `shipunlocks/<Team>2048_<class>.mp4` clips - five teams by four craft
//! classes. This module reads the **container header** and nothing else: what
//! a sample holds is H.264 video and, on `intro.mp4` alone, AAC audio, and
//! decoding either is out of process exactly as `.PMF`, `.IPF` and `.bik`
//! already are - see [ADR-0024] and this module's own
//! "Why hand-rolled, and why the container is not the codec" section below.
//!
//! ```text
//! ftyp  FileTypeBox        major/minor brand and compatible-brands list
//! moov  MovieBox             container: everything below is inside it
//!   trak  TrackBox            one per track (`intro.mp4` has two, one video
//!                              one audio; every other file has one, video only)
//!     mdia  MediaBox
//!       mdhd  MediaHeaderBox   this track's own timescale and duration
//!       hdlr  HandlerBox       `vide` or `soun` - which track this is
//!       minf  MediaInformationBox
//!         stbl  SampleTableBox
//!           stsd  SampleDescriptionBox   codec fourcc, width/height or
//!                                         sample rate/channel count
//!           stts  TimeToSampleBox        sample count and duration, run-length
//!           stsz  SampleSizeBox          sample count, independently
//! ```
//!
//! Every box not named above (`free`, `mdat`, `wide`, `edts`/`elst`, `ctts`,
//! `stsc`, `stco`, `stss`, `dinf`, `smhd`, `vmhd`, `udta`, `mvhd`...) is
//! skipped by size and never read. `edts`/`elst` in particular is present on
//! every track of every file measured (an edit list trimming one video frame
//! of decoder pre-roll on `intro.mp4`'s two tracks) and is left alone
//! deliberately: what [`Header`] reports is the *media* duration `mdhd`
//! declares, not the edited presentation duration, because nothing downstream
//! of [`parse`] trims playback yet. A future reader of `elst` should say so
//! rather than have this file's numbers silently start meaning something
//! narrower.
//!
//! # The box order is not what a reader would assume
//!
//! **`moov` is the last top-level box, after `mdat`, on all 26 files** - not
//! first, the way an authoring tool that writes a placeholder and seeks back
//! to fill it in would put it. `intro.mp4`'s top level is `ftyp`(24)
//! `free`(8) `mdat`(47,179,717) `moov`(61,352), the `mdat` payload sitting
//! between offset 32 and 47,179,749 with the sample table describing it
//! 47 MiB later in the file. [`parse`] cannot assume `moov` is anywhere in
//! particular; it walks every top-level box's declared size to find it,
//! which is also what proves the sizes sum to the file length - see
//! [`top_level_boxes`]. **`bb2048Zone8.mp4` is the one file whose middle two
//! boxes swap**: `ftyp` `mdat` `free` `moov`, `free` after `mdat` rather than
//! before it. A parser that assumed a fixed sequence past `ftyp` would be
//! wrong on one file in 26 and never notice on the other 25.
//!
//! # What is measured
//!
//! All 26 files parse, and three arithmetic invariants hold on every track of
//! every file - `crates/video/tests/mp4_ground_truth.rs` asserts them the way
//! `oag_video::bik`'s three are asserted against Wipeout HD's 37:
//!
//! 1. **Every top-level box's declared size sums to the file's own length** -
//!    [`top_level_boxes`] cannot return successfully otherwise, since it stops
//!    only when its running offset reaches the blob's end exactly.
//! 2. **`stsz`'s sample count equals the sum of `stts`'s `sample_count`
//!    entries.** Two different boxes counting the same samples: `intro.mp4`'s
//!    video track is 2,984 by both; its audio track is 4,666 by both.
//! 3. **`mdhd`'s duration equals `stts`'s `(sample_count, sample_delta)`
//!    entry multiplied out.** `intro.mp4`'s video track: 2,984 samples of
//!    1,001 ticks each is 2,986,984, which is exactly what `mdhd` declares at
//!    a 30,000 Hz timescale - 99.566 s, agreeing with `ffprobe`'s 99.57 s.
//!    Its audio track: 4,666 samples of 1,024 ticks each is 4,777,984 against
//!    a 48,000 Hz timescale, 99.541 s. Every one of the 26 files' `stts` boxes
//!    holds exactly one entry (constant frame rate) - [`parse`] treats more
//!    than one as [`Error::VariableFrameRate`] rather than averaging a rate
//!    nothing in the file states, since none of the 26 shipped files need it.
//!
//! `intro.mp4`'s numbers were independently measured by `ffprobe` before any
//! of this module was written (see `docs/formats/2048-frontend.md`'s "Movies
//! are MP4" section and `docs/formats/mp4.md`): H.264 960x544 at 30000/1001
//! fps, 2,984 video frames, 99.57 s, stereo 48 kHz AAC, 4,666 audio frames.
//! [`parse`] reproduces every one of those off the container alone, with no
//! H.264 or AAC decode - see [`Header`].
//!
//! # Why hand-rolled, and why the container is not the codec
//!
//! [ADR-0024]'s category order - a light pure-Rust library, then this
//! project's own reader for a format that is *Wipeout's own*, then `ffmpeg`
//! out of process - is a rule about **codecs**: it is why `crate::bik` reads
//! Bink's container by hand but hands the picture to `ffmpeg`, and it governs
//! H.264 and AAC here the same way. It says nothing about a **container**,
//! which is a different question from the codec inside it - `crate::pmf` and
//! `crate::ipf` already read PSMF's and IPU's wrappers by hand for exactly
//! this reason, and `crate::bik`'s own module doc makes the split explicit.
//! ISOBMFF is that same question asked of a third, and now fourth, format
//! this project did not invent.
//!
//! A pure-Rust ISOBMFF **demuxer** does exist to take instead of hand-rolling
//! one - `mp4parse` (MPL-2.0) and `mp4` (MIT) are both on crates.io, and
//! `symphonia`, already a dependency of `oag-game` for Wipeout HD's MP3 music
//! ([ADR-0024]), ships a `symphonia-format-isomp4` demuxer under the same
//! MPL-2.0 umbrella. None of the three was taken. The reason is not licence -
//! all three would clear [ADR-0024]'s bar - it is that [`oag-video`] is
//! written to depend on **nothing in the workspace**, so that the crates
//! reading the originals' containers carry no codec dependency in any feature
//! combination ([ADR-0050]), and every dependency taken by *anything* in this
//! crate is a dependency the whole crate now carries. What [`Header`] needs is
//! six numbers off eight box types - `ftyp`'s magic and seven fixed-offset
//! reads inside `moov` - which is a smaller and more auditable unit than a
//! general-purpose demuxer capable of fragmented MP4, `senc`, `sidx` and every
//! other box this project's 26 files do not use. `symphonia-format-isomp4`
//! also decodes nothing on its own; a general demuxer would have bought
//! coverage of boxes this project has no file to exercise, not a smaller
//! surface than the one below.
//!
//! [ADR-0024]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0024-in-process-codecs-and-ffmpeg-as-a-last-resort.md
//! [ADR-0050]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0050-format-crates-split-by-format-family.md
//! [`oag-video`]: https://github.com/topaxi/OpenAntiGrav/blob/main/crates/video/src/lib.rs

/// The four bytes every ISOBMFF file's first box declares as its type, at
/// offset 4 - `ftyp` (`FileTypeBox`, ISO/IEC 14496-12 §4.3). The four bytes
/// before it are that box's own size, which [`is_mp4`] does not check: a
/// truncated read still starts with a real `ftyp` and is worth reading as far
/// as it goes.
pub const FTYP: [u8; 4] = *b"ftyp";

/// Whether `blob` starts like an ISOBMFF / MP4 file.
///
/// The dispatch test: every file this project reads declares `ftyp` `mp42`
/// (`intro.mp4`: `66 74 79 70 6d 70 34 32`), but this checks only the box
/// type and not the brand - a `moov`-first or fragmented MP4 this project has
/// not seen is still worth naming as MP4 rather than refused outright.
#[must_use]
pub fn is_mp4(blob: &[u8]) -> bool {
    blob.len() >= 8 && blob[4..8] == FTYP
}

/// Something wrong with an MP4 blob, or a shape this module does not read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// Fewer bytes than any box header needs.
    TooShort {
        /// Bytes supplied.
        got: usize,
    },
    /// The blob does not start with `ftyp`.
    NotMp4,
    /// A box's declared size runs past the bytes available, or is too small
    /// to hold even its own header.
    BoxTruncated {
        /// Where the box that would not fit starts.
        at: usize,
    },
    /// A box this reader needed was not among its parent's children.
    MissingBox([u8; 4]),
    /// `moov` held no `trak` whose `hdlr` declared `vide`.
    NoVideoTrack,
    /// A track's `stts` held other than exactly one entry.
    ///
    /// Every track of all 26 shipped files holds one - constant frame rate -
    /// and [`parse`] refuses to average a rate spread over several entries
    /// into a single [`Header::frame_rate`] nothing in the file states.
    VariableFrameRate {
        /// The `trak`'s own handler - `vide` or `soun`.
        track: [u8; 4],
        /// How many `stts` entries it held.
        entries: usize,
    },
    /// The video track declared a zero width or height.
    ZeroDimension {
        /// Declared width.
        width: u32,
        /// Declared height.
        height: u32,
    },
    /// The video track's `mdhd` declared a zero timescale, which nothing can
    /// divide by.
    ZeroTimescale,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooShort { got } => write!(f, "{got} byte(s) is shorter than any box header"),
            Self::NotMp4 => write!(f, "does not start with ftyp"),
            Self::BoxTruncated { at } => write!(f, "a box at offset {at} does not fit the blob"),
            Self::MissingBox(kind) => {
                write!(f, "no {:?} box among its parent's children", FourCc(*kind))
            }
            Self::NoVideoTrack => write!(f, "moov holds no track whose handler is vide"),
            Self::VariableFrameRate { track, entries } => write!(
                f,
                "{:?} track's stts holds {entries} entries, not the one this reads a rate from",
                FourCc(*track)
            ),
            Self::ZeroDimension { width, height } => {
                write!(f, "the video track declares a {width}x{height} picture")
            }
            Self::ZeroTimescale => write!(f, "the video track's mdhd declares a zero timescale"),
        }
    }
}

impl std::error::Error for Error {}

/// Formats a box fourcc as its four ASCII bytes where they are printable, the
/// way every box name in this module's own docs is spelled.
struct FourCc([u8; 4]);

impl std::fmt::Debug for FourCc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match std::str::from_utf8(&self.0) {
            Ok(s) if s.chars().all(|c| c.is_ascii_graphic()) => write!(f, "{s}"),
            _ => write!(f, "{:02x?}", self.0),
        }
    }
}

/// One top-level box's kind and declared size, as [`top_level_boxes`] walks
/// them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoxEntry {
    /// The box's own four-byte type, e.g. `*b"moov"`.
    pub kind: [u8; 4],
    /// The box's whole declared size, header included - what ISO/IEC
    /// 14496-12 §4.2 calls its `size` (or `largesize`, for the rare box
    /// bigger than 4 GiB; none of this project's 26 files needs one).
    pub size: u64,
}

/// One box's header, already read: its kind and where its payload sits.
struct RawBox {
    kind: [u8; 4],
    /// Absolute offset of the box's own size field - what a [`Error`]
    /// reports when this box is the one that did not fit.
    start: usize,
    /// Absolute offset of the first payload byte, after the 8- or 16-byte
    /// header.
    body_start: usize,
    /// Absolute offset one past the box's last byte.
    body_end: usize,
}

/// Reads one box header at `at`, refusing anything that would run past
/// `limit`.
///
/// `limit` is `blob.len()` for a top-level walk and a parent's own
/// `body_end` for a walk over its children - either way, the byte one past
/// the last this box is allowed to claim.
fn read_box(blob: &[u8], at: usize, limit: usize) -> Result<RawBox, Error> {
    let head = blob.get(at..at + 8).ok_or(Error::BoxTruncated { at })?;
    let size32 = u32::from_be_bytes([head[0], head[1], head[2], head[3]]);
    let kind: [u8; 4] = [head[4], head[5], head[6], head[7]];

    let (size, header_len): (u64, usize) = match size32 {
        // A 64-bit `largesize` follows the type - ISO/IEC 14496-12 §4.2.2.
        // None of this project's 26 files uses one (the biggest single box,
        // `bb2048.mp4`'s 161 MB `mdat`, fits a 32-bit size with room to
        // spare), but the field exists for the day one does.
        1 => {
            let ext = blob
                .get(at + 8..at + 16)
                .ok_or(Error::BoxTruncated { at })?;
            (u64::from_be_bytes(ext.try_into().unwrap()), 16)
        }
        // The box runs to the end of its enclosing container - §4.2.2 again.
        // Valid only for the last child in whatever `limit` bounds this call
        // to; no box on any of the 26 files uses it either, since `moov` is
        // always the last top-level box with a real size of its own.
        0 => ((limit - at) as u64, 8),
        n => (u64::from(n), 8),
    };
    if size < header_len as u64 {
        return Err(Error::BoxTruncated { at });
    }
    let body_start = at + header_len;
    let box_end = usize::try_from(size)
        .ok()
        .and_then(|size| at.checked_add(size))
        .filter(|&end| end <= limit)
        .ok_or(Error::BoxTruncated { at })?;

    Ok(RawBox {
        kind,
        start: at,
        body_start,
        body_end: box_end,
    })
}

/// Every box directly inside `start..end`, in file order.
///
/// Succeeds only when the boxes' declared sizes run exactly from `start` to
/// `end` with nothing left over - a short leftover fails the next
/// [`read_box`] rather than being silently dropped, which is what makes a
/// successful top-level call the sum-of-sizes invariant this module's own
/// docs describe.
fn boxes_in(blob: &[u8], start: usize, end: usize) -> Result<Vec<RawBox>, Error> {
    let mut out = Vec::new();
    let mut pos = start;
    while pos < end {
        let b = read_box(blob, pos, end)?;
        pos = b.body_end;
        out.push(b);
    }
    Ok(out)
}

/// The first child of `parent` whose kind is `kind`.
fn find(blob: &[u8], parent: &RawBox, kind: &[u8; 4]) -> Result<RawBox, Error> {
    boxes_in(blob, parent.body_start, parent.body_end)?
        .into_iter()
        .find(|b| &b.kind == kind)
        .ok_or(Error::MissingBox(*kind))
}

/// Reads a big-endian field out of a box's payload, naming that box's own
/// start in the error when the field does not fit.
fn field<'a>(body: &'a [u8], at: usize, len: usize, owner: &RawBox) -> Result<&'a [u8], Error> {
    body.get(at..at + len)
        .ok_or(Error::BoxTruncated { at: owner.start })
}

/// Walks every top-level box in `blob`, refusing anything that does not
/// parse as a run of whole boxes reaching exactly to `blob.len()`.
///
/// This is the first of the three invariants this module's own docs name:
/// a successful return here **is** "every box's size sums to the file
/// length", since [`boxes_in`] cannot stop anywhere else. The ground-truth
/// test also asserts the box *order* this call reports - `moov` last on all
/// 26 files, and `bb2048Zone8.mp4`'s `free` after its `mdat` rather than
/// before - which a bare size sum would not catch.
///
/// # Errors
///
/// [`Error::NotMp4`] when the blob does not start `ftyp`, and
/// [`Error::BoxTruncated`] at whichever box's declared size runs past the
/// end of `blob` or leaves a slice too short to be a box at all.
pub fn top_level_boxes(blob: &[u8]) -> Result<Vec<BoxEntry>, Error> {
    if blob.len() < 8 {
        return Err(Error::TooShort { got: blob.len() });
    }
    if !is_mp4(blob) {
        return Err(Error::NotMp4);
    }
    Ok(boxes_in(blob, 0, blob.len())?
        .into_iter()
        .map(|b| BoxEntry {
            kind: b.kind,
            size: (b.body_end - b.start) as u64,
        })
        .collect())
}

/// What [`parse`] read off an `mp4a` `soun` track - present only on
/// `intro.mp4`, whose second `trak` is the one audio track any of the 26
/// files carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AudioTrack {
    /// The `stsd` sample entry's own fourcc - ISO/IEC 14496-14's `mp4a`
    /// (`ESDBox`-wrapped AAC) on `intro.mp4`, and the only one this project
    /// has a file for. Kept as the raw fourcc rather than an enum: this
    /// module reads the container, not the codec, and does not decode AAC to
    /// confirm what `mp4a` implies.
    pub codec: [u8; 4],
    /// `AudioSampleEntry.samplerate`'s integer half, ISO/IEC 14496-12
    /// §12.2.3 - 48,000 Hz on `intro.mp4`.
    pub sample_rate: u32,
    /// `AudioSampleEntry.channelcount`, same box - 2 (stereo) on
    /// `intro.mp4`.
    pub channel_count: u16,
    /// The audio track's own `stsz` sample count - 4,666 on `intro.mp4`,
    /// each sample 1,024 ticks of the track's 48,000 Hz `mdhd` timescale.
    pub frame_count: usize,
}

/// What [`parse`] read off the video track's own boxes.
///
/// Every field names the box (and, where it matters, the exact byte range)
/// it came from - see this module's own "What is measured" section for the
/// three invariants that hold across all 26 files.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Header {
    /// `VisualSampleEntry.width`, ISO/IEC 14496-12 §12.1.3, read off `stsd`'s
    /// first entry (`avc1` on every file measured) rather than `tkhd`'s: a
    /// `tkhd` track dimension is a *presentation* size and need not equal the
    /// coded picture, where `stsd` names what the decoder actually produces -
    /// 960 on `intro.mp4`.
    pub width: u32,
    /// `VisualSampleEntry.height`, same box - 544 on `intro.mp4`.
    pub height: u32,
    /// The video track's `stsz.sample_count` (`SampleSizeBox`, §8.7.3.2) -
    /// 2,984 on `intro.mp4`.
    pub frame_count: usize,
    /// The same count, independently: the sum of every `stts.sample_count`
    /// entry (`TimeToSampleBox`, §8.6.1.2) for the video track. Not folded
    /// into [`Self::frame_count`] - kept apart so a caller can assert the two
    /// agree, which `crates/video/tests/mp4_ground_truth.rs` does for all 26
    /// files.
    pub stts_sample_count: usize,
    /// The video track's own `mdhd.timescale` (`MediaHeaderBox`, §8.4.2) -
    /// units per second [`Self::duration`] and [`Self::frame_rate`] are
    /// counted in. 30,000 on 25 of the 26 files; `bb2048Zone8.mp4` alone
    /// declares 2,997.
    pub timescale: u32,
    /// The video track's `mdhd.duration`, in [`Self::timescale`] units -
    /// 2,986,984 on `intro.mp4`, which is 99.566 s at 30,000 Hz.
    pub duration: u64,
    /// The video track's own `stts.sample_delta`, unreduced - 1,001 on
    /// `intro.mp4`. Kept apart from [`Self::frame_rate`] (which divides this
    /// and [`Self::timescale`] by their GCD) so a caller can check the third
    /// invariant this module's own docs name - `duration == frame_count *
    /// frame_delta` - without undoing a reduction first.
    pub frame_delta: u32,
    /// Presentation rate as `(timescale, sample_delta)`, reduced by their
    /// GCD - `intro.mp4`'s single `stts` entry is `(30000, 1001)`, the
    /// container's own way of writing 29.97 Hz. **Not the same fraction on
    /// every file**: `bb2048Zone8.mp4`'s is `(2997, 100)`, a different
    /// rational that also happens to be close to 29.97 Hz - the two are not
    /// interchangeable and this module never rounds one into the other.
    pub frame_rate: (u32, u32),
    /// The `soun` track's own header, when `moov` carries one. `Some` only
    /// on `intro.mp4`; every other file is silent by construction.
    pub audio: Option<AudioTrack>,
}

impl Header {
    /// The video track's running time in seconds - [`Self::duration`] over
    /// [`Self::timescale`], both read off the same `mdhd`.
    #[must_use]
    pub fn seconds(&self) -> f64 {
        self.duration as f64 / f64::from(self.timescale)
    }
}

fn gcd(mut a: u32, mut b: u32) -> u32 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

/// `(a, b)` divided by their GCD - [`Header::frame_rate`]'s own reduction.
fn reduce(a: u32, b: u32) -> (u32, u32) {
    let g = gcd(a, b).max(1);
    (a / g, b / g)
}

/// The video track's own boxes, read - everything [`parse`] needs before it
/// knows whether `moov` even has an audio track to go with it.
struct VideoTrack {
    width: u32,
    height: u32,
    timescale: u32,
    duration: u64,
    stsz_count: usize,
    stts_sum: usize,
    sample_delta: u32,
}

/// One `trak`, resolved to what it turned out to be.
enum Track {
    Video(VideoTrack),
    Audio(AudioTrack),
    /// A handler this project's files never carry - `hint`, `meta`, and so
    /// on. [`parse`] skips it rather than refusing the file: nothing reads
    /// this track, and refusing the whole file over a track nothing needs
    /// would be a reader that is pickier than a player has to be.
    Other,
}

/// `mdhd`'s timescale and duration - ISO/IEC 14496-12 §8.4.2. The box is a
/// `FullBox` (4-byte version+flags) whose fields are 32-bit past version 0
/// and 64-bit at version 1, which is the one place in this module two
/// layouts share a box.
fn parse_mdhd(blob: &[u8], mdhd: &RawBox) -> Result<(u32, u64), Error> {
    let body = &blob[mdhd.body_start..mdhd.body_end];
    let version = *field(body, 0, 1, mdhd)?.first().unwrap();
    if version == 1 {
        let timescale = u32::from_be_bytes(field(body, 20, 4, mdhd)?.try_into().unwrap());
        let duration = u64::from_be_bytes(field(body, 24, 8, mdhd)?.try_into().unwrap());
        Ok((timescale, duration))
    } else {
        let timescale = u32::from_be_bytes(field(body, 12, 4, mdhd)?.try_into().unwrap());
        let duration = u32::from_be_bytes(field(body, 16, 4, mdhd)?.try_into().unwrap());
        Ok((timescale, u64::from(duration)))
    }
}

/// `hdlr.handler_type` - ISO/IEC 14496-12 §8.4.3, `HandlerBox`. `vide` for a
/// picture track, `soun` for sound; the four bytes at `+8`, after the
/// `FullBox` header and a `pre_defined` field this module never reads.
fn parse_hdlr(blob: &[u8], hdlr: &RawBox) -> Result<[u8; 4], Error> {
    let body = &blob[hdlr.body_start..hdlr.body_end];
    Ok(field(body, 8, 4, hdlr)?.try_into().unwrap())
}

/// `stsz.sample_count` - ISO/IEC 14496-12 §8.7.3.2, `SampleSizeBox`. The
/// `sample_size` field immediately before it is not read: every sample on
/// every track measured is `0` there (variable size, one entry per sample
/// following), and this module never seeks a sample by index.
fn parse_stsz(blob: &[u8], stsz: &RawBox) -> Result<usize, Error> {
    let body = &blob[stsz.body_start..stsz.body_end];
    Ok(u32::from_be_bytes(field(body, 8, 4, stsz)?.try_into().unwrap()) as usize)
}

/// `stts`'s entries summed, and the first entry's `sample_delta` - ISO/IEC
/// 14496-12 §8.6.1.2, `TimeToSampleBox`. `track` names the owning `trak`'s
/// handler, for [`Error::VariableFrameRate`].
///
/// # Errors
///
/// [`Error::VariableFrameRate`] when the box holds other than exactly one
/// entry - true of every track on all 26 shipped files, so this is refusing
/// a shape nothing here has been measured against rather than a shape that
/// occurs.
fn parse_stts(blob: &[u8], stts: &RawBox, track: [u8; 4]) -> Result<(usize, u32), Error> {
    let body = &blob[stts.body_start..stts.body_end];
    let entry_count = u32::from_be_bytes(field(body, 4, 4, stts)?.try_into().unwrap()) as usize;
    if entry_count != 1 {
        return Err(Error::VariableFrameRate {
            track,
            entries: entry_count,
        });
    }
    let entry = field(body, 8, 8, stts)?;
    let sample_count = u32::from_be_bytes(entry[0..4].try_into().unwrap()) as usize;
    let sample_delta = u32::from_be_bytes(entry[4..8].try_into().unwrap());
    Ok((sample_count, sample_delta))
}

/// The first entry inside an `stsd` (`SampleDescriptionBox`, §8.5.2) - the
/// box that names the codec and, for the one entry every track measured
/// carries, its picture or audio shape.
fn stsd_first_entry(blob: &[u8], stsd: &RawBox) -> Result<RawBox, Error> {
    // `FullBox` header (4 bytes) then `entry_count` (4 bytes) before the
    // entries themselves, each a box of its own.
    let entries_start = stsd.body_start + 8;
    if entries_start > stsd.body_end {
        return Err(Error::BoxTruncated { at: stsd.start });
    }
    boxes_in(blob, entries_start, stsd.body_end)?
        .into_iter()
        .next()
        .ok_or(Error::MissingBox(*b"stsd"))
}

/// `VisualSampleEntry.width`/`.height` - ISO/IEC 14496-12 §12.1.3. The entry
/// (`avc1` on every video track measured) opens with `SampleEntry`'s 8-byte
/// prefix (6 reserved + `data_reference_index`), then 16 more bytes of
/// `pre_defined`/`reserved` fields before width and height, each a
/// big-endian `u16`.
fn parse_stsd_video(blob: &[u8], stsd: &RawBox) -> Result<(u32, u32), Error> {
    let entry = stsd_first_entry(blob, stsd)?;
    let body = &blob[entry.body_start..entry.body_end];
    let dims = field(body, 24, 4, &entry)?;
    let width = u16::from_be_bytes(dims[0..2].try_into().unwrap());
    let height = u16::from_be_bytes(dims[2..4].try_into().unwrap());
    Ok((u32::from(width), u32::from(height)))
}

/// The `mp4a` entry's codec fourcc, sample rate and channel count -
/// `AudioSampleEntry`, ISO/IEC 14496-12 §12.2.3. After `SampleEntry`'s 8-byte
/// prefix: 8 bytes reserved, `channelcount` (`u16`), `samplesize` (`u16`,
/// unread), `pre_defined`/`reserved` (2 bytes each, unread), then
/// `samplerate` as a 16.16 fixed-point `u32` whose integer Hz is the high
/// half.
fn parse_stsd_audio(blob: &[u8], stsd: &RawBox) -> Result<([u8; 4], u32, u16), Error> {
    let entry = stsd_first_entry(blob, stsd)?;
    let body = &blob[entry.body_start..entry.body_end];
    let channel_count = u16::from_be_bytes(field(body, 16, 2, &entry)?.try_into().unwrap());
    let sample_rate_fixed = u32::from_be_bytes(field(body, 24, 4, &entry)?.try_into().unwrap());
    Ok((entry.kind, sample_rate_fixed >> 16, channel_count))
}

/// Reads one `trak`, all the way down to whichever leaf boxes it needs.
fn parse_trak(blob: &[u8], trak: &RawBox) -> Result<Track, Error> {
    let mdia = find(blob, trak, b"mdia")?;
    let (timescale, duration) = parse_mdhd(blob, &find(blob, &mdia, b"mdhd")?)?;
    let handler = parse_hdlr(blob, &find(blob, &mdia, b"hdlr")?)?;
    let minf = find(blob, &mdia, b"minf")?;
    let stbl = find(blob, &minf, b"stbl")?;
    let stsd = find(blob, &stbl, b"stsd")?;
    let stsz = find(blob, &stbl, b"stsz")?;
    let stts = find(blob, &stbl, b"stts")?;

    let stsz_count = parse_stsz(blob, &stsz)?;
    let (stts_sum, sample_delta) = parse_stts(blob, &stts, handler)?;

    match &handler {
        b"vide" => {
            let (width, height) = parse_stsd_video(blob, &stsd)?;
            Ok(Track::Video(VideoTrack {
                width,
                height,
                timescale,
                duration,
                stsz_count,
                stts_sum,
                sample_delta,
            }))
        }
        b"soun" => {
            let (codec, sample_rate, channel_count) = parse_stsd_audio(blob, &stsd)?;
            Ok(Track::Audio(AudioTrack {
                codec,
                sample_rate,
                channel_count,
                frame_count: stsz_count,
            }))
        }
        _ => Ok(Track::Other),
    }
}

/// Reads an MP4/ISOBMFF file's `moov` and returns what [`Header`] needs.
///
/// Walks every top-level box to find `moov` wherever it is - see this
/// module's own "The box order is not what a reader would assume" section -
/// then every `trak` inside it, keeping the one whose `hdlr` is `vide` and
/// the one (at most; `intro.mp4` is the only file with any) whose `hdlr` is
/// `soun`.
///
/// # Errors
///
/// [`Error::NotMp4`] or [`Error::TooShort`] for a blob that is not this
/// format at all; [`Error::BoxTruncated`] or [`Error::MissingBox`] for one
/// that is but whose `moov` this reader's fixed box path does not reach;
/// [`Error::NoVideoTrack`] when `moov` names no `vide` handler;
/// [`Error::VariableFrameRate`] for a track whose `stts` is not the
/// single-entry shape every one of the 26 shipped files has;
/// [`Error::ZeroDimension`] or [`Error::ZeroTimescale`] for a video track
/// that declares nothing playable.
pub fn parse(blob: &[u8]) -> Result<Header, Error> {
    if blob.len() < 8 {
        return Err(Error::TooShort { got: blob.len() });
    }
    if !is_mp4(blob) {
        return Err(Error::NotMp4);
    }

    let top = boxes_in(blob, 0, blob.len())?;
    let moov = top
        .into_iter()
        .find(|b| &b.kind == b"moov")
        .ok_or(Error::MissingBox(*b"moov"))?;

    let mut video = None;
    let mut audio = None;
    for trak in boxes_in(blob, moov.body_start, moov.body_end)?
        .into_iter()
        .filter(|b| &b.kind == b"trak")
    {
        match parse_trak(blob, &trak)? {
            Track::Video(v) => video = Some(v),
            Track::Audio(a) => audio = Some(a),
            Track::Other => {}
        }
    }

    let video = video.ok_or(Error::NoVideoTrack)?;
    if video.width == 0 || video.height == 0 {
        return Err(Error::ZeroDimension {
            width: video.width,
            height: video.height,
        });
    }
    if video.timescale == 0 {
        return Err(Error::ZeroTimescale);
    }

    Ok(Header {
        width: video.width,
        height: video.height,
        frame_count: video.stsz_count,
        stts_sample_count: video.stts_sum,
        timescale: video.timescale,
        duration: video.duration,
        frame_delta: video.sample_delta,
        frame_rate: reduce(video.timescale, video.sample_delta),
        audio,
    })
}

#[cfg(test)]
mod tests;
