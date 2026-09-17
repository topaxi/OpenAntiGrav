//! Sony's Edge Animation Tools clip format, as HD's per-material
//! `uvOffset`/`uvScale` curves author it.
//!
//! See `docs/formats/edge-animation.md` for the full recovery trail - the
//! evaluator's own two embedded assert strings
//! (`edgeanim_evaluate_ppu.cpp:469`/`:283`), the `"EA02"` tag, the
//! self-relative offset convention, and the four-clip cross-validation this
//! module's tests reproduce. `docs/ghidra/functions/ps3-hdfury-eu/billboards.md`'s
//! 2026-09-17 sections are the RE trail that located the clip in the first
//! place: `AnimCurve_EvaluateChannels` (`0x005f9bd0`) calling
//! `AnimCurve_SampleChannel` (`0x0066c3c8`) calling `EdgeAnim_EvaluateClip`
//! (`0x0066b840`), whose body carries the assert strings this module is named
//! after.
//!
//! # What this decodes, and what it deliberately does not
//!
//! **Only the non-bit-packed scalar ("user channel") path.** HD's PPU
//! evaluator itself asserts `anim->offsetPackingSpecs == 0` before doing
//! anything else - bit-packed clips are SPU-only in this SDK version, and
//! every clip on `321go_startfinish.rcsmodel` (the only file this has been
//! checked against) has that field zero. A clip with rotation, translation or
//! scale channels, or a nonzero `offsetPackingSpecs`, is out of scope: parsing
//! it fails closed via [`Clip::parse`] returning `None`, rather than guessing
//! at a codec this project has not seen exercised.
//!
//! # Self-relative offsets
//!
//! Every "offset" field in this format is relative to **the field's own
//! address**, not the clip's start: a zero value means "absent", and a
//! nonzero value `v` at file offset `f` resolves to `f + v`. This is read
//! directly off the evaluator's own decompile (`iVar30 = iVar39 + 0x38 +
//! iVar32;` - the field's own address plus its own value) and confirmed
//! empirically: resolving the frame-set info offset this way produces a
//! monotonically increasing `baseFrame` sequence across all four clips this
//! module was checked against; resolving it any other way does not.

use oag_formats::ByteOrder;

const BE: ByteOrder = ByteOrder::Big;

/// `"EA02"` - Edge Animation's own version tag, the field this format uses in
/// place of a container magic. Confirmed present, byte-identical, on all four
/// curves of `321go_startfinish.rcsmodel`.
const TAG: u32 = 0x4541_3032;

/// The self-relative offset convention every pointer field in this format
/// uses. `None` if the field's own value is zero ("absent"), matching the
/// evaluator's own `if (value != 0) target = field_address + value;`.
fn resolve(data: &[u8], field_at: usize) -> Option<usize> {
    if field_at + 4 > data.len() {
        return None;
    }
    let value = BE.u32(data, field_at);
    (value != 0).then(|| field_at + value as usize)
}

/// One entry of the frame-set index table: the first absolute frame this
/// frame-set covers, and how many intra-frames (frames strictly inside it,
/// short of the next frame-set's own boundary) it carries.
#[derive(Debug, Clone, Copy)]
struct FrameSet {
    base_frame: u16,
    num_intra_frames: u16,
}

/// One Edge Animation Tools clip: the header this module reads plus enough of
/// its own tables to evaluate a scalar ("user") channel at a point in time.
///
/// Joint (rotation/translation/scale) channels are not read - see the module
/// docs - so this carries no skeleton and no joint-related field at all.
#[derive(Debug, Clone, PartialEq)]
pub struct Clip {
    /// File offset of the clip header itself - every self-relative offset in
    /// this struct is already resolved against it, and [`Clip::frame_set_data`]
    /// needs it again for the frame-set DMA array's own targets, which are
    /// relative to this same address rather than to the field that names them.
    at: usize,
    /// The clip's own duration in seconds - one authored loop.
    pub duration: f32,
    /// Frames per second this clip's `numFrames`/`numFrameSets` are counted
    /// in. `time * sample_frequency` is the frame position
    /// [`Clip::sample_user_channel`] looks up.
    pub sample_frequency: f32,
    num_frame_sets: u16,
    frame_set_info: usize,
    frame_set_dma: usize,
    /// The animated-user-channel index table: `num_anim_user_channels`
    /// `u16`s, each the caller-facing channel id a [`Clip::sample_user_channel`]
    /// request is matched against.
    ///
    /// **Located empirically, not off a named header field.** It sits in the
    /// 16-byte-aligned block immediately before [`Self::frame_set_dma`] on
    /// every clip checked - see `docs/formats/edge-animation.md`'s "channel
    /// tables" section for why a header-field reading landed on the wrong
    /// four bytes here (a version difference from the public struct this was
    /// cross-checked against, not a guess).
    anim_user_table: usize,
    num_anim_user_channels: u16,
}

impl Clip {
    /// Reads a clip header at `at`, or `None` if the tag does not match, a
    /// table falls outside `data`, or the clip needs a decode path this
    /// module does not implement (bit-packed, or any joint channel).
    #[must_use]
    pub fn parse(data: &[u8], at: usize) -> Option<Self> {
        if at + 0x60 > data.len() || BE.u32(data, at) != TAG {
            return None;
        }
        let duration = BE.f32(data, at + 0x04);
        let sample_frequency = BE.f32(data, at + 0x08);
        let num_frame_sets = BE.u16(data, at + 0x12);
        // Every non-user channel count (rotation, translation, scale, and
        // their constant counterparts) must be zero: this module has no
        // decode path for any of them. See the module docs.
        for field in [0x16, 0x18, 0x1a, 0x1c, 0x1e, 0x20, 0x22] {
            if at + field + 2 > data.len() || BE.u16(data, at + field) != 0 {
                return None;
            }
        }
        let num_anim_user_channels = BE.u16(data, at + 0x24);
        // `offsetPackingSpecs` must be absent - HD's own evaluator asserts
        // this before reading anything else (`edgeanim_evaluate_ppu.cpp:469`,
        // `anim->offsetPackingSpecs == 0`). See the module docs for why this
        // field sits at `+0x4c` here rather than the public struct's `+0x50`.
        if resolve(data, at + 0x4c).is_some() {
            return None;
        }
        let frame_set_dma = resolve(data, at + 0x34)?;
        let frame_set_info = resolve(data, at + 0x38)?;
        if num_frame_sets == 0
            || frame_set_info + usize::from(num_frame_sets) * 4 > data.len()
            || frame_set_dma + usize::from(num_frame_sets) * 8 > data.len()
        {
            return None;
        }
        let anim_user_table = frame_set_dma.checked_sub(0x10)?;
        if anim_user_table + usize::from(num_anim_user_channels) * 2 > data.len() {
            return None;
        }
        Some(Self {
            at,
            duration,
            sample_frequency,
            num_frame_sets,
            frame_set_info,
            frame_set_dma,
            anim_user_table,
            num_anim_user_channels,
        })
    }

    /// How many animated scalar ("user") channels this clip carries - the
    /// same count `AnimCurve_EvaluateChannels` reads out of the curve's own
    /// inner clip header (not out of the curve struct that points at it) to
    /// know how many channel descriptors to read.
    #[must_use]
    pub fn num_anim_user_channels(&self) -> u16 {
        self.num_anim_user_channels
    }

    fn frame_set(&self, data: &[u8], i: u16) -> FrameSet {
        let at = self.frame_set_info + usize::from(i) * 4;
        FrameSet {
            base_frame: BE.u16(data, at),
            num_intra_frames: BE.u16(data, at + 2),
        }
    }

    /// The frame-set whose own span covers `frame_integer`, by the same
    /// binary search `_edgeAnimGetFrameSetIndex` performs (bisecting on
    /// `baseFrame`, ending on the entry left of the crossing rather than an
    /// exact hit - the last entry's own `numIntraFrames` is 0 and is never
    /// selected by this search, only reached as a fall-through of the
    /// second-to-last).
    fn frame_set_index(&self, data: &[u8], frame_integer: u32) -> u16 {
        let mut left = 0u16;
        let mut right = self.num_frame_sets - 1;
        while left + 1 != right {
            let mid = left + (right - left) / 2;
            if frame_integer < u32::from(self.frame_set(data, mid).base_frame) {
                right = mid;
            } else {
                left = mid;
            }
        }
        left
    }

    /// The frame-set's own data block: `frame_set_dma[i]`'s second word,
    /// resolved against the *clip's* own address rather than the field's -
    /// the one place in this format the self-relative convention is anchored
    /// somewhere other than the offset field itself, matching
    /// `dmaArray[frameSetIndex*2+1] + anim` in the evaluator's own decompile.
    fn frame_set_data(&self, data: &[u8], i: u16) -> usize {
        let w1 = BE.u32(data, self.frame_set_dma + usize::from(i) * 8 + 4);
        self.at + w1 as usize
    }

    fn find_channel(&self, data: &[u8], channel_index: u16) -> Option<u32> {
        (0..u32::from(self.num_anim_user_channels))
            .find(|&i| BE.u16(data, self.anim_user_table + i as usize * 2) == channel_index)
    }

    /// Evaluates one user (scalar) channel at `time` seconds since the clip's
    /// own start, or `0.0` if `channel_index` names no channel this clip
    /// carries.
    ///
    /// **`time` is not wrapped against [`Self::duration`] here.** The
    /// evaluator's own `fmodf(time, curve->period)` happens one level up, in
    /// the caller that owns the curve's period - a `Clip` alone does not know
    /// it, only the `Curve` wrapper that reads a material's own curve struct
    /// does (see `oag_rcs::rcsmodel::material::curve`).
    ///
    /// Time below zero clamps to the first frame, matching the evaluator's
    /// own `if (frame < 0) frame = 0;`.
    #[must_use]
    pub fn sample_user_channel(&self, data: &[u8], channel_index: u16, time: f32) -> f32 {
        let Some(anim_id) = self.find_channel(data, channel_index) else {
            return 0.0;
        };

        let frame = (time * self.sample_frequency).max(0.0);
        let frame_integer_whole = frame as u32;
        let fs_index = self.frame_set_index(data, frame_integer_whole);
        let fs = self.frame_set(data, fs_index);
        let fs_data = self.frame_set_data(data, fs_index);

        let frame_set_frame = frame - f32::from(fs.base_frame);
        let mut frame_integer = frame_set_frame as u32;
        let mut frame_fraction = frame_set_frame - frame_integer as f32;
        if frame_integer > u32::from(fs.num_intra_frames) {
            frame_integer = u32::from(fs.num_intra_frames);
            frame_fraction = 1.0;
        }

        // Offsets to the various data blocks within this frame-set, per
        // `_edgeAnimEvaluate`'s own arithmetic: an 8-entry `u16` size header,
        // then boundary ("initial") values for every constant/animated
        // channel in R/T/S/U order, then the intra keyframe presence bitmap,
        // then the intra keyframe values themselves, packed R/T/S/U.
        //
        // R, T and S are always empty here (`Clip::parse` refuses any file
        // with a rotation/translation/scale channel), so only the user-data
        // offsets do real work; the rest collapse to `fs_data + 16`.
        let sizes: [usize; 8] = std::array::from_fn(|k| usize::from(BE.u16(data, fs_data + k * 2)));
        let initial_r = fs_data + 16;
        let initial_t = initial_r + sizes[0];
        let initial_s = initial_t + sizes[1];
        let initial_u = initial_s + sizes[2];
        let intra_bits = initial_u + sizes[3];
        let intra_frame_count = u32::from(fs.num_intra_frames);
        let intra_r = intra_bits
            + (usize::from(self.num_anim_user_channels) * intra_frame_count as usize).div_ceil(8);
        let intra_t = intra_r + sizes[4];
        let intra_s = intra_t + sizes[5];
        let intra_u = (intra_s + sizes[6] + 3) & !3;
        let next_fs = (intra_u + sizes[7] + 15) & !15;
        let final_r = next_fs + 16;
        let final_t = final_r + usize::from(BE.u16(data, next_fs));
        let final_s = final_t + usize::from(BE.u16(data, next_fs + 2));
        let final_u = final_s + usize::from(BE.u16(data, next_fs + 4));

        let (key_a, key_b, alpha) = bracketing_keyframes(
            data,
            frame_integer,
            intra_frame_count,
            anim_id,
            4,
            intra_bits,
            0,
            initial_u,
            intra_u,
            final_u,
            frame_fraction,
        );
        let a = BE.f32(data, key_a);
        let b = BE.f32(data, key_b);
        a * (1.0 - alpha) + b * alpha
    }
}

/// A single bit of the intra-frame presence bitmap, MSB-first within its
/// byte - matching `spu_slqwbytebc`'s own left-justified masking in the
/// evaluator's SIMD bit-counting helpers.
fn get_bit(data: &[u8], base_bit: usize, i: u32) -> bool {
    let bit = base_bit + i as usize;
    (data[bit / 8] >> (7 - (bit % 8))) & 1 != 0
}

/// How many of the `num_bits` bits starting at `(byte_addr, bit_offset)` are
/// set - `_edgeAnimCountEnabledBits`, done a bit at a time rather than with
/// its SIMD popcount, since this runs at authoring-inspection and test speed
/// only, never per frame in a shipped render loop.
fn count_bits(data: &[u8], byte_addr: usize, bit_offset: u32, num_bits: u32) -> u32 {
    let base = byte_addr * 8 + bit_offset as usize;
    (0..num_bits).filter(|&i| get_bit(data, base, i)).count() as u32
}

/// The two keyframes that bracket `frame_integer` for one channel, and the
/// lerp weight between them - `_edgeAnimGetBracketingKeyframes`, restricted to
/// the case this module supports: a single float per key (`stride == 4`,
/// always, since only [`Clip::sample_user_channel`] calls this).
#[allow(clippy::too_many_arguments)]
fn bracketing_keyframes(
    data: &[u8],
    frame_integer: u32,
    intra_frame_count: u32,
    channel_id: u32,
    stride: usize,
    intra_bits_adr: usize,
    intra_bits_ofs: u32,
    initial_adr: usize,
    intra_adr: usize,
    final_adr: usize,
    frame_fraction: f32,
) -> (usize, usize, f32) {
    // This channel's own intra keys are stored after every earlier channel's
    // - count how many of those precede it to find where they start.
    let num_enabled_prev = count_bits(
        data,
        intra_bits_adr,
        intra_bits_ofs,
        channel_id * intra_frame_count,
    );
    let intra_adr = intra_adr + num_enabled_prev as usize * stride;
    let intra_bits_ofs = intra_bits_ofs + channel_id * intra_frame_count;

    let num_bits = count_bits(data, intra_bits_adr, intra_bits_ofs, intra_frame_count);
    let num_prev_bits = count_bits(data, intra_bits_adr, intra_bits_ofs, frame_integer);

    let base_bit = intra_bits_adr * 8 + intra_bits_ofs as usize;
    let last_prev_set = (0..frame_integer)
        .rev()
        .find(|&i| get_bit(data, base_bit, i));
    let b_bits = frame_integer as i64 - last_prev_set.map_or(-1, i64::from) - 1;

    let first_after_set = (frame_integer..intra_frame_count).find(|&i| get_bit(data, base_bit, i));
    let a_bits = first_after_set.unwrap_or(intra_frame_count) as i64 - frame_integer as i64 + 1;

    let key_a = if num_prev_bits == 0 {
        initial_adr + channel_id as usize * stride
    } else {
        intra_adr + (num_prev_bits - 1) as usize * stride
    };
    let key_b = if num_prev_bits == num_bits {
        final_adr + channel_id as usize * stride
    } else {
        intra_adr + num_prev_bits as usize * stride
    };

    let denom = a_bits + b_bits;
    let alpha = if denom != 0 {
        (b_bits as f32 + frame_fraction) / denom as f32
    } else {
        0.0
    };
    (key_a, key_b, alpha)
}
