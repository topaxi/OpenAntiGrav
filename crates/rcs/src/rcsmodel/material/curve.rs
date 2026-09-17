//! A material's own animated curve: which parameter-table entries it drives,
//! and the [`edgeanim::Clip`] it samples them from.
//!
//! See `docs/formats/edge-animation.md` and
//! `docs/ghidra/functions/ps3-hdfury-eu/billboards.md`'s 2026-09-17 sections
//! for the full recovery trail. In short: `Billboard_UpdateInstanceUvs` walks
//! a `.rcsmodel`'s own material table (its `+0x2c`/`+0x30` header fields,
//! which [`crate::rcsmodel::Model::parse`]'s own `materials` field already
//! is) and calls `AnimCurve_EvaluateChannels` for every material whose
//! `+0x20`-then-`+0xc` pointer is non-null. That pointer is what
//! [`Curve::parse`] reads.

use oag_formats::ByteOrder;

use crate::edgeanim;

const BE: ByteOrder = ByteOrder::Big;

/// One channel a [`Curve`] drives: a material parameter-table entry and
/// which of its four components.
///
/// Read off `AnimCurve_EvaluateChannels`'s own decompile: a channel
/// descriptor is one `u16`, `target_index` in bits 2-15 and `component` in
/// bits 0-1 (`uVar2 & 0xfffc` and `uVar2 & 3` respectively).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Channel {
    /// Index into the material's own **raw** parameter table -
    /// `target_index * 0x20` bytes from the table [`super::Material`]'s own
    /// `+0x34` field names.
    ///
    /// **Not an index into [`super::Material::parameters`].** That list
    /// drops every sampler entry ([`super::parameters::KIND_SAMPLER`]), so it
    /// is shorter than the raw table and its positions do not line up with
    /// this field once a material has any sampler before the entry a channel
    /// targets - `321go_startfinish.rcsmodel`'s materials all do, two of
    /// them. A caller that needs the entry back reads the raw table directly
    /// (`table = Material's own +0x34`, entry at `table + target_index *
    /// 0x20`) rather than indexing `parameters`.
    pub target_index: u32,
    /// Which of the parameter's four `f32` components this channel writes,
    /// `0..=3`.
    pub component: u8,
}

/// A material's own animated curve.
///
/// **The clip's own period is read here, not by [`edgeanim::Clip`] itself** -
/// `AnimCurve_EvaluateChannels` wraps time with `fmodf(time, curve->period)`
/// before ever reaching the clip, and `period` lives in the curve struct this
/// module reads (`inner+0x04` relative to the clip - the same field
/// [`edgeanim::Clip::duration`] already is, read twice for two different
/// reasons: the clip needs it to compute `sampleFrequency * time`, the curve
/// needs it to wrap `time` in the first place).
#[derive(Debug, Clone, PartialEq)]
pub struct Curve {
    channels: Vec<Channel>,
    clip: edgeanim::Clip,
}

impl Curve {
    /// Reads a material record's own curve, or `None` if the record has none
    /// (most don't - see [`super::Material::curve`]), the pointer chain runs
    /// outside `data`, or the clip at the end of it needs a decode path
    /// [`edgeanim::Clip`] does not implement.
    pub(crate) fn parse(data: &[u8], material_at: usize) -> Option<Self> {
        if material_at + 0x24 > data.len() {
            return None;
        }
        let plus20 = BE.u32(data, material_at + 0x20) as usize;
        if plus20 == 0 || plus20 + 0x10 > data.len() {
            return None;
        }
        let curve_at = BE.u32(data, plus20 + 0xc) as usize;
        if curve_at == 0 || curve_at + 0x28 > data.len() {
            return None;
        }
        let clip_at = BE.u32(data, curve_at) as usize;
        let clip = edgeanim::Clip::parse(data, clip_at)?;

        // The channel count is not in the curve struct itself -
        // `AnimCurve_EvaluateChannels` reads it from the clip's own header
        // (confidence 90 on `docs/ghidra/functions/ps3-hdfury-eu/billboards.md`'s
        // 2026-09-17 section: every live curve checked has this equal the
        // channel-descriptor count it actually holds).
        let count = usize::from(clip.num_anim_user_channels());
        let mut channels = Vec::with_capacity(count);
        for i in 0..count {
            let at = curve_at + 4 + i * 2;
            if at + 2 > data.len() {
                break;
            }
            let desc = BE.u16(data, at);
            channels.push(Channel {
                target_index: u32::from(desc >> 2),
                component: (desc & 3) as u8,
            });
        }
        Some(Self { channels, clip })
    }

    /// Every channel this curve drives.
    #[must_use]
    pub fn channels(&self) -> &[Channel] {
        &self.channels
    }

    /// The clip this curve samples.
    #[must_use]
    pub fn clip(&self) -> &edgeanim::Clip {
        &self.clip
    }

    /// Every channel's own value at `time` seconds, wrapped against the
    /// clip's own [`edgeanim::Clip::duration`] the way
    /// `AnimCurve_EvaluateChannels`'s own `fmodf(time, curve->period)` does.
    ///
    /// **The channel-to-`sample_user_channel` mapping is inferred, not read
    /// off a decompile of `AnimCurve_EvaluateChannels` itself** - this module
    /// passes [`Channel::component`] as the clip's own `channel_index`, which
    /// produces a sane, smooth, monotonic result on all eight channels
    /// (2 components x 4 materials) of `321go_startfinish.rcsmodel`: one
    /// component of each curve stays flat at exactly the value the clip's own
    /// bytes hold at frame 0, the other ramps smoothly with no discontinuity
    /// at every intra-frameset boundary sampled. A wrong mapping would not
    /// produce that shape by accident, but this has not been checked against
    /// `AnimCurve_EvaluateChannels`'s own decompile line by line.
    #[must_use]
    pub fn sample(&self, data: &[u8], time: f32) -> Vec<(Channel, f32)> {
        let wrapped = if self.clip.duration > 0.0 {
            time.rem_euclid(self.clip.duration)
        } else {
            time
        };
        self.channels
            .iter()
            .map(|&channel| {
                let value =
                    self.clip
                        .sample_user_channel(data, u16::from(channel.component), wrapped);
                (channel, value)
            })
            .collect()
    }
}
