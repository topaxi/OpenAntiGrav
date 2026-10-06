//! `sound` `0x3e1` and `soundcone` `0x3e9`: the sound sources a circuit
//! authors, and where they are.
//!
//! One 80-byte payload serves both classes. `VexSound_Init` (`0x089259a4`) reads
//! it, allocates a `0x70`-byte emitter, points it at the node's own world matrix
//! and hands `Sound_Play` (`0x089392b0`) a bank label and a cue name, so the
//! emitter is live from construction and plays one authored cue where the
//! transform chain puts it:
//!
//! ```text
//! payload +0x00  f32   cone angle, radians; -1.0 on a plain `sound`
//! payload +0x04  f32   second cone angle, radians; -1.0 on a plain `sound`
//! payload +0x08  u8    cone enabled: 0 on every `sound`, 1 on every `soundcone`
//! payload +0x0c  f32   copied to the emitter's +0x3c, a field nothing has mapped
//! payload +0x10  f32   radius, copied to the emitter's +0x38
//! payload +0x14  8 B   bank label, NUL-terminated
//! payload +0x1c  16 B  cue name, NUL-terminated, `~` included
//! payload +0x2c  u16   radius-curve key count
//! payload +0x30  u32   offset to the curve's u16 time array, node-relative
//! payload +0x34  u32   offset to the curve's u16 value array, node-relative
//! payload +0x38  f32   seconds per curve tick, 1/60 on every authored node
//! ```
//!
//! # The radius is a curve, and `+0x10` is only its first key
//!
//! `VexSound_Update` (`0x08925c4c`) does not read `+0x10`: it divides the
//! instance's elapsed time by `+0x38` to get curve ticks and resamples
//! [`RadiusCurve`] into the emitter's `+0x38` every frame. The `f32` alone is the
//! exporter's convenience copy, not what the game plays. Pulse authors exactly
//! one key on all 1,298 nodes, so the two agree to within the encoding's `0.075`
//! units; see [`SoundEmitter::radius`].
//!
//! # Placement is the transform chain
//!
//! Nothing about where an emitter *is* lives in its payload (`Init` copies the
//! node's world matrix from the scene graph), so placement comes from
//! [`vex::world_transforms`], as for a [`Speedup Pad`](crate::pads).
//!
//! Evidence and confidence scores:
//! `docs/ghidra/functions/psp-pulse-usa/track-sound-emitters.md`.

use crate::vex::{self, Node};
use oag_formats::ByteOrder;

/// The `sound` class: an omnidirectional authored emitter.
pub const CLASS_SOUND: u32 = 0x3e1;

/// The `soundcone` class: the same payload with its cone fields filled in.
pub const CLASS_SOUND_CONE: u32 = 0x3e9;

/// The `speaker` class, which Pulse registers and never authors. A named
/// constant because "zero instances on the disc" is a finding worth asserting.
pub const CLASS_SPEAKER: u32 = 0x3cc;

/// Smallest payload this module will read.
const MIN_PAYLOAD: usize = 0x40;

/// First cone angle, radians. `-1.0` where the node authors no cone.
const CONE_A: usize = 0x00;

/// Second cone angle, radians. `-1.0` where the node authors no cone.
const CONE_B: usize = 0x04;

/// `u8`, set on a `soundcone` and clear on a `sound`.
const CONE_ENABLED: usize = 0x08;

/// The `f32` that reaches the emitter's unmapped `+0x3c`.
const EMITTER_3C: usize = 0x0c;

/// The static radius, in world units.
const RADIUS: usize = 0x10;

/// The bank label field, and its width.
const BANK: usize = 0x14;
const BANK_LEN: usize = 8;

/// The cue name field, and its width.
const CUE: usize = 0x1c;
const CUE_LEN: usize = 16;

/// Number of keys in the radius curve.
const CURVE_KEYS: usize = 0x2c;

/// Node-relative offset of the curve's `u16` time array.
const CURVE_TIMES: usize = 0x30;

/// Node-relative offset of the curve's `u16` value array.
const CURVE_VALUES: usize = 0x34;

/// Seconds per curve tick.
const CURVE_TICK: usize = 0x38;

/// World units a stored curve value of `u16::MAX` decodes to: `0x459c4000` at
/// `0x08925d0c`, paired with the divide by [`CURVE_FULL_SCALE`] at `0x08925e08`,
/// both literals in `VexSound_SampleRadiusCurve`. Together they reproduce all
/// 1,164 stored `sound` keys; fitting the ratio to the data gives `13.10` and
/// misses two, so the encoding is a reading, not a curve fit.
const CURVE_RANGE: f32 = 5000.0;

/// What a stored curve value is divided by after scaling to [`CURVE_RANGE`].
const CURVE_FULL_SCALE: f32 = 65535.0;

/// One authored sound source, placed.
#[derive(Debug, Clone, PartialEq)]
pub struct SoundEmitter {
    /// The bank the cue lives in, by the bank's own 7-character label (`gentrak`
    /// for `Data\Sound\generaltrack.bnk`, `basilic` for `01_Track`'s).
    ///
    /// **Not guaranteed to name a bank that exists.** The field is 8 bytes and
    /// `07_Track` reversed fills all eight with `outpostf`, no terminator (the
    /// scene's name was longer than the field); a label tops out at 7 usable
    /// characters, so nothing on the disc answers to it.
    pub bank: String,
    /// The cue, spelled the way the bank's name table spells it, `~` included.
    pub cue: String,
    /// The radius as the `f32` at `+0x10`, what `Init` writes into the emitter
    /// before the first frame; afterwards [`SoundEmitter::sample_radius`] is what
    /// the game plays. Prefer the curve where the two could differ.
    pub radius: f32,
    /// The `f32` at `+0x0c`, which `Init` copies to the emitter's `+0x3c`.
    ///
    /// Unnamed beyond its destination: `positional-audio.md`'s emitter table has
    /// no `+0x3c` row and nothing observed reads it back. Equals
    /// [`SoundEmitter::radius`] on every `sound` node, `25.0` on every `soundcone`.
    pub emitter_field_3c: f32,
    /// The cone, where the node authors one.
    pub cone: Option<Cone>,
    /// The radius over time, which is what the emitter is actually driven from.
    pub radius_curve: RadiusCurve,
    /// Row-major world matrix, translation in row 3 - the emitter's placement.
    pub to_world: [f32; 16],
}

impl SoundEmitter {
    /// World-space position of the emitter.
    #[must_use]
    pub fn position(&self) -> [f32; 3] {
        [self.to_world[12], self.to_world[13], self.to_world[14]]
    }

    /// The radius the game would use `frame` curve ticks into the node's life;
    /// [`SoundEmitter::radius`] where the curve has no keys (the one case
    /// `VexSound_SampleRadiusCurve` leaves the emitter's field untouched).
    #[must_use]
    pub fn sample_radius(&self, frame: f32) -> f32 {
        self.radius_curve.sample(frame).unwrap_or(self.radius)
    }

    /// Reads one node's payload, with its world matrix already composed.
    #[must_use]
    pub fn parse(payload: &[u8], to_world: [f32; 16], order: ByteOrder) -> Option<Self> {
        if payload.len() < MIN_PAYLOAD {
            return None;
        }
        let cone = (payload[CONE_ENABLED] != 0).then(|| Cone {
            angle_a: order.f32(payload, CONE_A),
            angle_b: order.f32(payload, CONE_B),
        });
        Some(Self {
            bank: field_string(payload, BANK, BANK_LEN),
            cue: field_string(payload, CUE, CUE_LEN),
            radius: order.f32(payload, RADIUS),
            emitter_field_3c: order.f32(payload, EMITTER_3C),
            cone,
            radius_curve: RadiusCurve::parse(payload, order),
            to_world,
        })
    }
}

/// A `soundcone`'s two authored angles, in radians.
///
/// **Which one reaches the emitter's `+0x40` half-angle is read.**
/// `VexSoundCone_Init` (`0x08925ff4`, confidence 90; see
/// `docs/ghidra/functions/psp-pulse-usa/track-sound-emitters.md`) calls
/// `VexSound_Init` unchanged, then copies the node's `+0x00` to the emitter's
/// `+0x40` half-angle and `+0x08` to `+0x4c` (the enable byte). So
/// [`Cone::angle_a`] (`+0x00`, always the wider here) is the half-angle
/// `oag_audio::spatial` attenuates by, read off the decompile. Both are kept as
/// the offsets they came from: on all 134 cones [`Cone::angle_b`] is `40` degrees
/// and [`Cone::angle_a`] one of eight whole-degree values from `40` to `120`, so
/// [`Cone::wide`] and [`Cone::narrow`] are derived, `wide()` being `angle_a` on
/// every authored node.
///
/// **`angle_b` (`+0x04`) writes the emitter's `+0x44`**, which nothing observed
/// reads back (as `+0x3c`). Decoded and named, not used.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cone {
    /// Payload `+0x00`, radians.
    pub angle_a: f32,
    /// Payload `+0x04`, radians. `40` degrees on every Pulse cone.
    pub angle_b: f32,
}

impl Cone {
    /// The larger of the two angles, radians.
    #[must_use]
    pub fn wide(self) -> f32 {
        self.angle_a.max(self.angle_b)
    }

    /// The smaller of the two angles, radians.
    #[must_use]
    pub fn narrow(self) -> f32 {
        self.angle_a.min(self.angle_b)
    }
}

/// The emitter radius as a keyframed curve of `u16` times and `u16` values.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RadiusCurve {
    /// Key times, in curve ticks.
    pub times: Vec<u16>,
    /// Key values, still encoded; [`RadiusCurve::sample`] decodes them.
    pub values: Vec<u16>,
    /// Seconds per curve tick, payload `+0x38`.
    pub seconds_per_tick: f32,
}

impl RadiusCurve {
    /// Reads the curve out of a payload whose two offsets are still file-form:
    /// the game relocates `+0x30` and `+0x34` in place (`node[0x34] += (int)node`),
    /// so on disc they are payload-relative, and taking them as values gives `64`
    /// and `66`.
    fn parse(payload: &[u8], order: ByteOrder) -> Self {
        let keys = order.u16(payload, CURVE_KEYS) as usize;
        let read = |at: usize| -> Vec<u16> {
            let base = order.u32(payload, at) as usize;
            (0..keys)
                .map(|key| base + key * 2)
                .take_while(|off| off + 2 <= payload.len())
                .map(|off| order.u16(payload, off))
                .collect()
        };
        let times = read(CURVE_TIMES);
        let values = read(CURVE_VALUES);
        Self {
            // A truncated array means an early-ended payload; keep the pair the
            // same length so `sample` never indexes past one.
            times: times[..times.len().min(values.len())].to_vec(),
            values: values[..times.len().min(values.len())].to_vec(),
            seconds_per_tick: order.f32(payload, CURVE_TICK),
        }
    }

    /// The radius at `frame` curve ticks, in world units.
    ///
    /// Clamped at both ends, linear between keys, as `VexSound_SampleRadiusCurve`
    /// (`0x08925cf0`) does; `None` with no keys (the one path that writes nothing).
    #[must_use]
    pub fn sample(&self, frame: f32) -> Option<f32> {
        let (&first, &last) = (self.times.first()?, self.times.last()?);
        if frame < f32::from(first) {
            return Some(decode(f32::from(*self.values.first()?)));
        }
        let Some(key) = self
            .times
            .iter()
            .position(|&time| frame < f32::from(time))
            .filter(|_| frame < f32::from(last))
        else {
            return Some(decode(f32::from(*self.values.last()?)));
        };
        let span = f32::from(self.times[key]) - f32::from(self.times[key - 1]);
        if span == 0.0 {
            return Some(decode(f32::from(self.values[key])));
        }
        let t = (frame - f32::from(self.times[key - 1])) / span;
        let from = f32::from(self.values[key - 1]);
        let to = f32::from(self.values[key]);
        Some(decode(from + t * (to - from)))
    }
}

/// Turns a stored curve value into world units.
fn decode(value: f32) -> f32 {
    value * CURVE_RANGE / CURVE_FULL_SCALE
}

/// Encodes a radius the way the exporter did, for a round-trip check.
#[must_use]
pub fn encode_radius(radius: f32) -> u16 {
    let scaled = radius * CURVE_FULL_SCALE / CURVE_RANGE;
    if scaled < 0.0 {
        0
    } else if scaled >= CURVE_FULL_SCALE {
        u16::MAX
    } else {
        scaled as u16
    }
}

/// Every authored emitter in a `.vex`, `sound` and `soundcone` together.
///
/// An empty result is ordinary: a non-circuit `.vex` and a Zone circuit author
/// none.
#[must_use]
pub fn emitters(data: &[u8], nodes: &[Node]) -> Vec<SoundEmitter> {
    let chain = vex::world_transforms(data, nodes);
    let order = vex::byte_order(data);
    nodes
        .iter()
        .enumerate()
        .filter(|(_, node)| node.class_id == CLASS_SOUND || node.class_id == CLASS_SOUND_CONE)
        .filter_map(|(index, node)| {
            let to_world = chain.get(index).copied()?;
            SoundEmitter::parse(data.get(node.payload())?, to_world, order)
        })
        .collect()
}

/// A fixed-width, NUL-terminated field, with everything past the NUL dropped.
///
/// The exporter does not clear the bytes after a terminator, so a cue name is
/// followed by whatever was in its buffer (`~AIR_CON_FAN\0ape` on one node).
fn field_string(payload: &[u8], at: usize, len: usize) -> String {
    let field = payload.get(at..at + len).unwrap_or_default();
    let end = field.iter().position(|&b| b == 0).unwrap_or(field.len());
    String::from_utf8_lossy(&field[..end]).into_owned()
}

#[cfg(test)]
mod tests;
