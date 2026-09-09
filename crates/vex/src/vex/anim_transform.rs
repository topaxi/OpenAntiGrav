//! The `Anim Transform` node (class `0x3c0`): a keyframed local matrix.
//!
//! A scene-graph transform whose translation, rotation and scale are each an
//! authored keyframe track rather than a constant. It is what makes trackside
//! scenery *move*, as distinct from the per-material texture transform in
//! [`super::mesh_tex_transforms`] that makes a surface *scroll*.
//!
//! **Every Pulse circuit authors it** - 393 nodes over the twelve, with 474
//! meshes below them - so a reader that treats the class as the identity, the
//! way [`super::world_transforms`] used to, does not merely fail to animate:
//! it drops the node's placement too, and 245 of those meshes composed to the
//! world origin. See `docs/rendering/scenery-animation.md`.
//!
//! # Where the layout comes from
//!
//! Read at instruction level from the class's own binder and evaluators in
//! `psp-pulse-usa/BOOT.BIN`, reached through the registration site
//! `AnimTransform_Register` (`0x0890009c`), which is the call passing class id
//! `0x3c0` to `Vex_RegisterClass`. Full evidence, including how each field was
//! pinned, is in
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
//! exactly on every one of the 393 - which is what pins the field map rather
//! than making it plausible.
//!
//! # Wipeout HD writes the same layout big-endian
//!
//! Field for field, with only the byte order changed: 5,518 nodes across all
//! seven of its archives decode, their six arrays tile contiguously from
//! `0x50` with at most 15 bytes of alignment padding after (Pulse leaves none),
//! and `seconds_per_key` is `1/60` and `flags` `0` on every one - the two
//! values the evaluators above were read against. That is why
//! [`anim_transform`] takes the order rather than sniffing it: a payload has no
//! magic. See `docs/formats/hd-status.md`.

use super::Node;

/// One channel's authored keys: `u16` times in frames, and a value per key
/// whose meaning and units are the channel's.
///
/// **Values are held widened to `f32` and otherwise unscaled** - an `s16` key
/// of `256` is `256.0` here, not `1.0` - because the scaling is the channel's
/// own (`1/32767` on a rotation, `1/256` on a scale, the node's per-axis
/// quantum on a translation) and [`AnimTransform::sample`] is where it belongs.
/// Widening rather than keeping the `s16` is what lets [`wide`](Self::wide)
/// keys share every line of the evaluator; see the module doc.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AnimChannel {
    /// Key times, in key units (frames at 1/60 s).
    pub times: Vec<u16>,
    /// `(x, y, z)` at each key, raw.
    pub values: Vec<[f32; 3]>,
    /// The `w` of each key, on a [`wide`](Self::wide) rotation channel that
    /// stores a whole quaternion. Empty on every other channel, where `w` is
    /// reconstructed from the other three instead.
    pub w: Vec<f32>,
    /// Whether the file stored `f32` keys rather than `s16` ones - the widened
    /// form flagged by the node's `+0x34` word.
    pub wide: bool,
}

impl AnimChannel {
    /// Whether this channel has anything to evaluate.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.times.is_empty() || self.values.is_empty()
    }

    /// The pair of key indices and the blend factor at `t` frames, the way all
    /// three evaluators pick one.
    ///
    /// They share the shape exactly: below the first key time, hold the first
    /// key; at or past the last, hold the last; otherwise find the first key
    /// whose time is above `t` and blend from the one before it. `step` is the
    /// node's authored `FixedFrames` attribute, which snaps to the preceding
    /// key instead of blending - the same choice the texture-transform block's
    /// step flag makes.
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
    /// Scale keys, in 1/256 units - `256` is `1.0`, the same fixed point the
    /// texture-transform block uses.
    pub scale: AnimChannel,
    /// Seconds per key-time unit, from `+0x3c`. `1/60` on every node read, so
    /// key times are 60 Hz frames.
    pub seconds_per_key: f32,
    /// The word at `+0x34`: which key *width* each channel uses. See
    /// [`TRANSLATION_IS_FLOAT`] and [`ROTATION_IS_QUATERNION`].
    ///
    /// **Zero on all 393 of Pulse's nodes**, which is why the widened forms
    /// were unknown until Wipeout HD was read: 97 of its 5,518 carry `0x5`,
    /// and they are the grid-camera paths and one billboard's fish. Any bit
    /// outside those two is still unaccounted for and appears nowhere on
    /// either disc.
    pub flags: u32,
    /// The float at `+0x30`: the denominator of the rate multiplier
    /// `AnimTransform_Update` (`0x088fe0a8`) applies to its delta time,
    /// `max(1.0, node+0x5c / this)`.
    ///
    /// **Inert on this disc.** The multiplier is guarded on both terms being
    /// non-zero and nothing read writes `node+0x5c`, which is consistent with
    /// this field being `0.0` on the median node. Carried so a later reader does
    /// not re-derive that it exists; its *units* are still unknown, so do not
    /// compute with it.
    pub rate_denominator: f32,
    /// The node's `LoopEnd` attribute, converted to seconds, or
    /// [`DEFAULT_LOOP_SECONDS`] where the node authors none.
    ///
    /// The wrap is the engine's own: `AnimTransform_Update` (`0x088fe0a8`)
    /// advances the node's clock and does `fmodf(t, LoopEnd)` once `t` reaches
    /// it. [`AnimTransform::sample`] evaluates `seconds % loop` directly
    /// instead of integrating a per-node clock; the two agree while a node
    /// starts at zero and is never paused, which is every node on the disc.
    ///
    /// **Not in the payload** - it is a node attribute, so
    /// [`anim_transform`] cannot fill it and [`anim_transforms`] does.
    pub loop_seconds: f32,
    /// The node's `FixedFrames` attribute: snap to the preceding key instead of
    /// blending toward the next.
    ///
    /// 16 nodes over the twelve circuits set it, and they are the ones authored
    /// as key *pairs* one frame apart - a teleport, which blending would turn
    /// into a smear. Same role as the texture-transform block's step flag.
    pub step: bool,
}

/// What `LoopEnd` defaults to when a node authors none: the binder's own
/// `6000.0 / seconds_per_key` key units, which at 1/60 is 6,000 seconds.
///
/// Long enough that nothing on the disc reaches it, which is the point - a node
/// with no `LoopEnd` is one the artists did not intend to loop.
pub const DEFAULT_LOOP_SECONDS: f32 = 6000.0;

impl AnimTransform {
    /// The local matrix at `seconds`, in [`super::transform`]'s row-major
    /// convention: rows 0 to 2 the basis, row 3 the translation.
    ///
    /// Reproduces `AnimTransform_Evaluate` (`0x088fe400`): rotation writes the
    /// basis, translation writes row 3, and scale then multiplies rows 0 to 2 -
    /// so the composition is scale, then rotate, then translate.
    ///
    /// `seconds` wraps at [`loop_seconds`](Self::loop_seconds) first, and
    /// [`step`](Self::step) decides whether a key blends or snaps - both read
    /// off the node's attribute list rather than out of the payload, so a
    /// transform built by hand gets the engine's own defaults for them.
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
            // The widened form stores the quaternion outright, in whatever
            // units it is already a unit quaternion in; the `s16` form stores
            // three components in 1/32767 and leaves `w` to be reconstructed.
            let [x, y, z] = if self.rotation.wide {
                q
            } else {
                [q[0] / 32767.0, q[1] / 32767.0, q[2] / 32767.0]
            };
            let w = self.rotation.sample_w(t, step).unwrap_or_else(|| {
                // The evaluator's own reconstruction. A blended pair can leave
                // the sum a hair over 1, which would root a negative.
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
/// Returns `None` for a payload too short to hold the header, or one whose key
/// arrays run past its end - never a partly-filled transform, since a channel
/// read off the end of the payload would place a mesh somewhere arbitrary
/// rather than fail visibly.
///
/// `order` is the containing file's, from [`super::byte_order`], for the same
/// reason [`super::transform`] has to be told: a payload carries no magic of
/// its own. **Wipeout HD writes this class big-endian and authors 5,518 of
/// them**, and reading those little-endian does not fail loudly - the counts
/// come out in the tens of thousands, the key arrays then run past the payload,
/// and every node decodes to `None` and falls back to the identity. That is the
/// Pulse defect exactly: placement dropped along with the animation.
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

    // A count of zero still stores one key - the evaluators read `values[0]`
    // through the "before the first key" branch whatever the count says - so a
    // channel is empty only when the *evaluator* skips it, which it does on a
    // count of zero. The stored key is still parsed, and `is_empty` is what
    // decides, so the arrays tile whatever the counts are.
    //
    // `width` is the key's stride in bytes and is what the flag word selects:
    // 6 for an `s16` triple, 12 for an `f32` triple, 16 for an `f32`
    // quaternion. See [`Key`].
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
    /// Three `f32`: a translation in world units, with the node's quantum at
    /// `1.0` and its base at the origin on all 21 nodes that use it.
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
/// **Measured on the Wipeout HD disc by tiling, not read from an evaluator.**
/// Under the `s16` reading, 97 of its 5,518 nodes leave gaps between their six
/// key arrays; under this one all 5,518 tile contiguously from `0x50`. The
/// widths and the counts are separate fields, so a wrong width shows up as a
/// gap on the first file rather than as a plausible animation. Confidence 85.
pub const TRANSLATION_IS_FLOAT: u32 = 1;

/// `+0x34` bit 2: rotation keys are whole `f32` quaternions.
///
/// Measured the same way as [`TRANSLATION_IS_FLOAT`], and always set with it on
/// this disc - every one of the 97 carries `0x5` - so **nothing here separates
/// the two bits**, and a file setting only one would be the test that does.
/// The component order is taken as `(x, y, z, w)`, continuing the `s16` form's
/// own `(x, y, z)` with the `w` it reconstructs; that ordering is a choice at
/// confidence 60, not a reading. Confidence 85 on the width.
pub const ROTATION_IS_QUATERNION: u32 = 4;

/// One node's `Anim Transform`, with its `LoopEnd` and `FixedFrames`
/// attributes applied.
///
/// [`anim_transform`] reads the payload and nothing else, so it cannot see
/// either - they are node attributes. Prefer this wherever a `Node` is at hand;
/// on the 178 nodes that author one, the payload-only form loops 6,000 seconds
/// later than the artists asked for.
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
    // The engine's own lookup is a `strcmp`, so it is case sensitive, and three
    // nodes author `Loopend`. Matching those would be a *departure*: the
    // original does not loop them either. See
    // `docs/rendering/scenery-animation.md`.
    if let Some(frames) = named("LoopEnd") {
        out.loop_seconds = frames * out.seconds_per_key;
    }
    out.step = named("FixedFrames").is_some_and(|v| v == 1.0);
    Some(out)
}

/// Every `Anim Transform` node's decoded payload, indexed the same way as
/// `nodes`, with `None` for every other class.
///
/// Kept beside [`anim_transform`] because [`super::world_transforms`] wants the
/// whole vector and a caller that wants one node has the payload already.
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
