//! The `Anim Transform` node (class `0x3c0`): a keyframed local matrix.
//!
//! A scene-graph transform whose translation, rotation and scale are each an
//! authored keyframe track. It is what makes trackside scenery *move*, as
//! distinct from the per-material texture transform in
//! [`super::mesh_tex_transforms`] that makes a surface *scroll*.
//!
//! **Every Pulse circuit authors it** (393 nodes over the twelve, 474 meshes
//! below them), so treating the class as the identity (as
//! [`super::world_transforms`] once did) drops the node's placement too: 245 of
//! those meshes composed to the world origin. See
//! `docs/rendering/scenery-animation.md`.
//!
//! # Where the layout comes from
//!
//! Read at instruction level from the class's binder and evaluators in
//! `psp-pulse-usa/BOOT.BIN`, via the registration site
//! `AnimTransform_Register` (`0x0890009c`), which is the call
//! passing class id `0x3c0` to `Vex_RegisterClass`. Evidence, including how each
//! field was pinned, is in
//! `docs/ghidra/functions/psp-pulse-usa/anim-transform.md`.
//!
//! ```text
//! +0x00 u16    unused; 0 on all 393 nodes
//! +0x02 u16    translation key count
//! +0x04 u16    rotation key count
//! +0x06 u16    scale key count
//! +0x08 u32    rotation key times      -> u16 frames
//! +0x0c u32    translation key times   -> u16 frames
//! +0x10 f32[3] translation base
//! +0x1c u32    rotation key values     -> (s16, s16, s16) per key
//! +0x20 f32[3] translation quantum, per axis
//! +0x2c u32    translation key values  -> (s16, s16, s16) per key
//! +0x30 f32    denominator of `AnimTransform_Update`'s rate multiplier
//! +0x34 u32    flags: bit0 widens a translation key to an f32 triple, bit2 a
//!              rotation key to an f32 quaternion. Zero on all 393 Pulse nodes
//! +0x38 u32    scale key times         -> u16 frames
//! +0x3c f32    seconds per key unit; 1/60 on all 393 nodes
//! +0x40 u32    scale key values        -> (s16, s16, s16) per key, 1/256
//! ```
//!
//! The four pointers are byte offsets **relative to the payload**, which the
//! binder (`0x088fe5a4`) turns into absolute pointers in place. A key count of
//! `0` still stores one key, and the six arrays tile `[0x50, payload_len)`
//! exactly on all 393, which is what pins the field map.
//!
//! # Wipeout HD writes the same layout big-endian
//!
//! Field for field, with only the byte order changed: 5,518 nodes across all
//! seven archives decode, their six arrays tile from `0x50` with at most 15
//! bytes of alignment padding (Pulse leaves none), and `seconds_per_key` is
//! `1/60` and `flags` `0` on every one. So [`anim_transform`] takes the order
//! rather than sniffing it: a payload has no magic. See
//! `docs/formats/hd-status.md`.

use super::Node;

/// One channel's authored keys: `u16` times in frames, and a value per key in the
/// channel's own units.
///
/// **Values are widened to `f32` and otherwise unscaled** (an `s16` key of `256`
/// is `256.0`, not `1.0`): the scaling is the channel's (`1/32767` rotation,
/// `1/256` scale, the node's per-axis quantum on translation) and belongs in
/// [`AnimTransform::sample`]. Widening lets [`wide`](Self::wide) keys share every
/// line of the evaluator.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AnimChannel {
    /// Key times, in key units (frames at 1/60 s).
    pub times: Vec<u16>,
    /// `(x, y, z)` at each key, raw.
    pub values: Vec<[f32; 3]>,
    /// The `w` of each key on a [`wide`](Self::wide) rotation channel storing a
    /// whole quaternion; empty elsewhere, where `w` is reconstructed.
    pub w: Vec<f32>,
    /// Whether the file stored `f32` keys rather than `s16` ones (the node's
    /// `+0x34` flag).
    pub wide: bool,
}

impl AnimChannel {
    /// Whether this channel has anything to evaluate.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.times.is_empty() || self.values.is_empty()
    }

    /// The pair of key indices and the blend factor at `t` frames, as all three
    /// evaluators pick one: hold the first key below the first time, the last at
    /// or past the last, else blend from the key before the first one above `t`.
    /// `step` is the node's `FixedFrames` attribute, which snaps to the preceding
    /// key (as the texture-transform block's step flag does).
    fn at(&self, t: f32, step: bool) -> (usize, usize, f32) {
        let last = self.values.len() - 1;
        let hold = |i: usize| (i, i, 0.0);
        let Some(&first_time) = self.times.first() else {
            return hold(0);
        };
        if t < f32::from(first_time) {
            return hold(0);
        }
        // The engine compares the *integer* frame against the last key time and
        // falls through to the hold when it is not below it.
        if t >= f32::from(self.times[self.times.len() - 1]) {
            return hold(last);
        }
        let i = self
            .times
            .iter()
            .position(|&key| t < f32::from(key))
            .unwrap_or(last)
            .max(1);
        let (t0, t1) = (f32::from(self.times[i - 1]), f32::from(self.times[i]));
        if step || t1 <= t0 {
            return hold(i - 1);
        }
        (i - 1, i, (t - t0) / (t1 - t0))
    }

    /// The blended raw value at `t` frames.
    fn sample(&self, t: f32, step: bool) -> [f32; 3] {
        let (a, b, f) = self.at(t, step);
        let (a, b) = (self.values[a], self.values[b]);
        std::array::from_fn(|k| a[k] + (b[k] - a[k]) * f)
    }

    /// The blended `w` at `t` frames, or `None` on a channel that stores none.
    fn sample_w(&self, t: f32, step: bool) -> Option<f32> {
        let (a, b, f) = self.at(t, step);
        let (a, b) = (*self.w.get(a)?, *self.w.get(b)?);
        Some(a + (b - a) * f)
    }
}

/// An `Anim Transform` node's three authored channels.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AnimTransform {
    /// Translation keys. World units are `value * quantum + base`.
    pub translation: AnimChannel,
    /// Per-axis quantum the translation keys are multiplied by, from `+0x20`.
    pub translation_quantum: [f32; 3],
    /// Translation origin the scaled keys are added to, from `+0x10`.
    pub translation_base: [f32; 3],
    /// Rotation keys: the `(x, y, z)` of a **unit quaternion** in 1/32767
    /// units, `w` reconstructed as `sqrt(1 - x^2 - y^2 - z^2)`.
    pub rotation: AnimChannel,
    /// Scale keys, in 1/256 units (`256` is `1.0`, as in the texture-transform
    /// block).
    pub scale: AnimChannel,
    /// Seconds per key-time unit, from `+0x3c`; `1/60` on every node read.
    pub seconds_per_key: f32,
    /// The word at `+0x34`: which key *width* each channel uses; see
    /// [`TRANSLATION_IS_FLOAT`] and [`ROTATION_IS_QUATERNION`].
    ///
    /// **Zero on all 393 of Pulse's nodes**, so the widened forms were unknown
    /// until HD was read: 97 of its 5,518 carry `0x5` (the grid-camera paths and
    /// one billboard's fish). Any other bit is unaccounted for and appears on
    /// neither disc.
    pub flags: u32,
    /// The float at `+0x30`: the denominator of the rate multiplier
    /// `AnimTransform_Update` (`0x088fe0a8`) applies to its delta time,
    /// `max(1.0, node+0x5c / this)`.
    ///
    /// **Inert on this disc**: the multiplier is guarded on both terms being
    /// non-zero and nothing read writes `node+0x5c`. Its *units* are unknown, so
    /// do not compute with it.
    pub rate_denominator: f32,
    /// The node's `LoopEnd` attribute in seconds, or [`DEFAULT_LOOP_SECONDS`]
    /// where none is authored.
    ///
    /// The wrap is the engine's own (`AnimTransform_Update`, `0x088fe0a8`:
    /// `fmodf(t, LoopEnd)`). [`AnimTransform::sample`] evaluates `seconds % loop`
    /// instead of integrating a clock; they agree while a node starts at zero and
    /// is never paused, every node on the disc.
    ///
    /// **Not in the payload**: a node attribute, so [`anim_transform`] cannot fill
    /// it and [`anim_transforms`] does.
    pub loop_seconds: f32,
    /// The node's `FixedFrames` attribute: snap to the preceding key instead of
    /// blending. 16 nodes over the twelve circuits set it, the ones authored as
    /// key *pairs* one frame apart (a teleport, which blending would smear).
    pub step: bool,
}

/// What `LoopEnd` defaults to when a node authors none: the binder's
/// `6000.0 / seconds_per_key` key units, 6,000 seconds at 1/60. Long enough that
/// nothing on the disc reaches it: a node with no `LoopEnd` is not meant to loop.
pub const DEFAULT_LOOP_SECONDS: f32 = 6000.0;

impl AnimTransform {
    /// The local matrix at `seconds`, in [`super::transform`]'s row-major
    /// convention (rows 0 to 2 the basis, row 3 the translation).
    ///
    /// Reproduces `AnimTransform_Evaluate` (`0x088fe400`): rotation writes the
    /// basis, translation row 3, then scale multiplies rows 0 to 2, so scale,
    /// then rotate, then translate. `seconds` wraps at
    /// [`loop_seconds`](Self::loop_seconds) and [`step`](Self::step) picks blend
    /// or snap, both node attributes, so a hand-built transform gets the engine's
    /// defaults.
    #[must_use]
    pub fn sample(&self, seconds: f32) -> [f32; 16] {
        let unit = if self.seconds_per_key > 0.0 {
            self.seconds_per_key
        } else {
            1.0 / 60.0
        };
        let period = if self.loop_seconds > 0.0 {
            self.loop_seconds
        } else {
            DEFAULT_LOOP_SECONDS
        };
        let t = (seconds % period) / unit;
        let step = self.step;

        let mut m = super::IDENTITY;
        if !self.rotation.is_empty() {
            let q = self.rotation.sample(t, step);
            // The widened form stores the quaternion outright; the `s16` form
            // stores three components in 1/32767 and reconstructs `w`.
            let [x, y, z] = if self.rotation.wide {
                q
            } else {
                [q[0] / 32767.0, q[1] / 32767.0, q[2] / 32767.0]
            };
            let w = self.rotation.sample_w(t, step).unwrap_or_else(|| {
                // The evaluator's own reconstruction (a blended pair can leave the
                // sum a hair over 1, which would root a negative).
                (1.0 - x * x - y * y - z * z).max(0.0).sqrt()
            });
            m = quaternion_matrix([x, y, z, w]);
        }
        if !self.translation.is_empty() {
            let v = self.translation.sample(t, step);
            for k in 0..3 {
                m[12 + k] = v[k] * self.translation_quantum[k] + self.translation_base[k];
            }
            m[15] = 1.0;
        }
        if !self.scale.is_empty() {
            let s = self.scale.sample(t, step);
            for row in 0..3 {
                let factor = s[row] / 256.0;
                for col in 0..4 {
                    m[row * 4 + col] *= factor;
                }
            }
        }
        m
    }
}

/// A unit quaternion as a row-major matrix, in the convention
/// [`super::transform`] documents.
fn quaternion_matrix([x, y, z, w]: [f32; 4]) -> [f32; 16] {
    let (xx, yy, zz) = (x * x, y * y, z * z);
    let (xy, xz, yz) = (x * y, x * z, y * z);
    let (wx, wy, wz) = (w * x, w * y, w * z);
    [
        1.0 - 2.0 * (yy + zz),
        2.0 * (xy + wz),
        2.0 * (xz - wy),
        0.0,
        2.0 * (xy - wz),
        1.0 - 2.0 * (xx + zz),
        2.0 * (yz + wx),
        0.0,
        2.0 * (xz + wy),
        2.0 * (yz - wx),
        1.0 - 2.0 * (xx + yy),
        0.0,
        0.0,
        0.0,
        0.0,
        1.0,
    ]
}

/// Decodes one `Anim Transform` payload.
///
/// `None` for a payload too short for the header or whose key arrays run past
/// its end, never a partly-filled transform: a channel read off the end would
/// place a mesh somewhere arbitrary instead of failing visibly.
///
/// `order` is the containing file's ([`super::byte_order`]): a payload carries no
/// magic. **HD writes this class big-endian (5,518 nodes)**, and a little-endian
/// read does not fail loudly: counts come out in the tens of thousands, the key
/// arrays run past the payload, and every node decodes to `None` and falls back
/// to the identity, the Pulse defect exactly.
#[must_use]
pub fn anim_transform(payload: &[u8], order: oag_formats::ByteOrder) -> Option<AnimTransform> {
    if payload.len() < 0x50 {
        return None;
    }
    let u16_at = |at: usize| order.u16(payload, at);
    let u32_at = |at: usize| order.u32(payload, at);
    let f32_at = |at: usize| order.f32(payload, at);
    let f32x3_at = |at: usize| [f32_at(at), f32_at(at + 4), f32_at(at + 8)];

    let flags = u32_at(0x34);

    // A count of zero still stores one key (the evaluators read `values[0]`
    // through the "before the first key" branch), so a channel is empty only
    // when the *evaluator* skips it; the stored key is still parsed and the
    // arrays tile whatever the counts.
    //
    // `width` is the key's stride in bytes, selected by the flag word: 6 for an
    // `s16` triple, 12 for an `f32` triple, 16 for an `f32` quaternion. See [`Key`].
    let channel = |count: usize, times_at: usize, values_at: usize, key: Key| {
        let stored = count.max(1);
        let times = u32_at(times_at) as usize;
        let values = u32_at(values_at) as usize;
        if times + stored * 2 > payload.len() || values + stored * key.width() > payload.len() {
            return None;
        }
        if count == 0 {
            return Some(AnimChannel::default());
        }
        let read = |at: usize| -> [f32; 3] {
            match key {
                Key::Short => std::array::from_fn(|k| f32::from(u16_at(at + k * 2) as i16)),
                Key::Float3 | Key::Float4 => std::array::from_fn(|k| f32_at(at + k * 4)),
            }
        };
        Some(AnimChannel {
            times: (0..count).map(|i| u16_at(times + i * 2)).collect(),
            values: (0..count).map(|i| read(values + i * key.width())).collect(),
            w: match key {
                Key::Float4 => (0..count)
                    .map(|i| f32_at(values + i * key.width() + 12))
                    .collect(),
                Key::Short | Key::Float3 => Vec::new(),
            },
            wide: key != Key::Short,
        })
    };

    let translation_key = if flags & TRANSLATION_IS_FLOAT != 0 {
        Key::Float3
    } else {
        Key::Short
    };
    let rotation_key = if flags & ROTATION_IS_QUATERNION != 0 {
        Key::Float4
    } else {
        Key::Short
    };

    Some(AnimTransform {
        translation: channel(usize::from(u16_at(0x02)), 0x0c, 0x2c, translation_key)?,
        translation_quantum: f32x3_at(0x20),
        translation_base: f32x3_at(0x10),
        rotation: channel(usize::from(u16_at(0x04)), 0x08, 0x1c, rotation_key)?,
        scale: channel(usize::from(u16_at(0x06)), 0x38, 0x40, Key::Short)?,
        seconds_per_key: f32_at(0x3c),
        flags,
        rate_denominator: f32_at(0x30),
        loop_seconds: DEFAULT_LOOP_SECONDS,
        step: false,
    })
}

/// How wide one key of a channel is, which the node's `+0x34` word selects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Key {
    /// Three `s16`, the only form Pulse ships.
    Short,
    /// Three `f32`: a translation in world units (quantum `1.0`, base at the
    /// origin on all 21 nodes that use it).
    Float3,
    /// Four `f32`: a quaternion with its `w` stored rather than reconstructed.
    Float4,
}

impl Key {
    fn width(self) -> usize {
        match self {
            Self::Short => 6,
            Self::Float3 => 12,
            Self::Float4 => 16,
        }
    }
}

/// `+0x34` bit 0: translation keys are `f32` triples rather than `s16` ones.
///
/// **Measured on HD by tiling, not read from an evaluator.** Under the `s16`
/// reading 97 of 5,518 nodes leave gaps between their six key arrays; under this
/// all tile contiguously from `0x50`. A wrong width shows as a gap on the first
/// file, not a plausible animation. Confidence 85.
pub const TRANSLATION_IS_FLOAT: u32 = 1;

/// `+0x34` bit 2: rotation keys are whole `f32` quaternions.
///
/// Measured as [`TRANSLATION_IS_FLOAT`] is, and always set with it on this disc
/// (all 97 carry `0x5`), so **nothing separates the two bits**; a file setting
/// only one would. The order `(x, y, z, w)` is a choice at confidence 60, not a
/// reading; confidence 85 on the width.
pub const ROTATION_IS_QUATERNION: u32 = 4;

/// One node's `Anim Transform`, with its `LoopEnd` and `FixedFrames`
/// attributes applied.
///
/// [`anim_transform`] reads the payload only, so it cannot see these node
/// attributes. Prefer this where a `Node` is at hand: on the 178 nodes that
/// author one the payload-only form loops 6,000 seconds late.
#[must_use]
pub fn anim_transform_of(data: &[u8], node: &Node) -> Option<AnimTransform> {
    let mut out = anim_transform(data.get(node.payload())?, super::byte_order(data))?;
    let attributes = super::node_attributes(data, node);
    let named = |want: &str| {
        attributes
            .iter()
            .find(|(name, _)| name == want)
            .map(|(_, value)| *value)
    };
    // The engine's lookup is a case-sensitive `strcmp`, and three nodes author
    // `Loopend`: matching them would depart from the original, which does not loop
    // them either. See `docs/rendering/scenery-animation.md`.
    if let Some(frames) = named("LoopEnd") {
        out.loop_seconds = frames * out.seconds_per_key;
    }
    out.step = named("FixedFrames").is_some_and(|v| v == 1.0);
    Some(out)
}

/// Every `Anim Transform` node's decoded payload, indexed the same way as
/// `nodes`, with `None` for every other class.
///
/// Beside [`anim_transform`] because [`super::world_transforms`] wants the whole
/// vector and a caller wanting one node already has the payload.
#[must_use]
pub fn anim_transforms(data: &[u8], nodes: &[Node]) -> Vec<Option<AnimTransform>> {
    let class = super::classes_of(data)
        .ok()
        .and_then(|classes| classes.anim_transform);
    nodes
        .iter()
        .map(|node| {
            if Some(node.class_id) != class {
                return None;
            }
            anim_transform_of(data, node)
        })
        .collect()
}

#[cfg(test)]
mod tests;
