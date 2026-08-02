//! `fogCube` `0x3d3`: a track's fog volume and the parameters it grades across.
//!
//! The payload is 128 bytes and every field in it is recovered - the layout from
//! shipped data, the meaning from the runtime. See
//! [`skycube.md`](../../../docs/formats/skycube.md) for the data side and
//! [`fog.md`](../../../docs/ghidra/functions/psp-pulse/fog.md) for
//! `FogCube_Init` (`0x089068f8`) and `FogCube_Sample` (`0x089069b0`), which this
//! module reimplements.
//!
//! ```text
//! +0x00  row-major 4x4, world -> volume space
//! +0x40  {r, g, b, a, near, far}   parameters at the -Z end
//! +0x58  {r, g, b, a, near, far}   parameters at the +Z end
//! +0x70  f32  tiebreak: the smallest wins where volumes nest
//! +0x74  f32  the cube's edge length, 500.0 on every shipped track
//! +0x78  eight zero bytes
//! ```
//!
//! # The two parameter sets are a gradient, not a pair of presets
//!
//! This is the part that is not guessable from the bytes. `FogCube_Sample`
//! normalises the sample point's **local Z** across the box and interpolates all
//! six floats between the two sets - so colour, near *and* far slide as the
//! camera moves through the volume. 28 of the 36 shipped `fogCube` nodes carry
//! byte-identical sets, which is uniform fog; the other 8 are genuinely graded.
//!
//! It is also why the original's fog is described as "a specific curve rather
//! than a linear ramp" while `Gu_Fog` (`0x08811748`) is flatly linear. **Both
//! are true.** The hardware ramp is linear within a frame; what varies is the
//! parameters fed to it, re-sampled every frame from the camera's position. A
//! shaping function on the ramp would be the wrong fix.

use crate::vex::{self, Node, transform_point};

/// Length of a `fogCube` payload.
///
/// `FogCube_Init` advances the bind cursor by exactly this, which is
/// independent confirmation of the figure measured across all 36 shipped nodes.
pub const PAYLOAD_LEN: usize = 0x80;

/// Where the first parameter set starts.
const SET_A: usize = 0x40;

/// Where the second starts.
const SET_B: usize = 0x58;

/// The tiebreak float.
const TIEBREAK: usize = 0x70;

/// The cube's edge length.
const EDGE: usize = 0x74;

/// Fog parameters at one point: what [`Gu_Fog`] would be handed.
///
/// [`Gu_Fog`]: https://example.invalid
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FogParams {
    /// Linear RGB in `0..=1`, as authored.
    pub colour: [f32; 3],
    /// The alpha slot. Zero on every shipped node, and the runtime zeroes it
    /// again before packing the colour, so nothing reads it. Kept because a
    /// field that is provably unused is worth being able to assert.
    pub alpha: f32,
    /// Distance at which fog starts.
    pub near: f32,
    /// Distance at which fog is total.
    pub far: f32,
}

impl FogParams {
    fn read(payload: &[u8], at: usize) -> Option<Self> {
        let f = |i: usize| -> Option<f32> {
            let b = payload.get(at + i * 4..at + i * 4 + 4)?;
            Some(f32::from_le_bytes(b.try_into().ok()?))
        };
        Some(Self {
            colour: [f(0)?, f(1)?, f(2)?],
            alpha: f(3)?,
            near: f(4)?,
            far: f(5)?,
        })
    }

    fn lerp(self, other: Self, t: f32) -> Self {
        let mix = |a: f32, b: f32| a + t * (b - a);
        Self {
            colour: [
                mix(self.colour[0], other.colour[0]),
                mix(self.colour[1], other.colour[1]),
                mix(self.colour[2], other.colour[2]),
            ],
            alpha: mix(self.alpha, other.alpha),
            near: mix(self.near, other.near),
            far: mix(self.far, other.far),
        }
    }
}

/// One authored fog volume.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FogVolume {
    /// World to volume space. Row-major, row-vector convention, the same as
    /// every other matrix in a `.vex`.
    pub to_local: [f32; 16],
    /// Parameters at the `-Z` face.
    pub near_end: FogParams,
    /// Parameters at the `+Z` face.
    pub far_end: FogParams,
    /// Smallest wins where volumes nest. Units unestablished - see the docs.
    pub tiebreak: f32,
    /// The cube's edge length in volume space.
    pub edge: f32,
}

impl FogVolume {
    /// Decodes one `fogCube` payload.
    ///
    /// Returns `None` for a payload that is not [`PAYLOAD_LEN`] or whose edge
    /// length is not positive - the latter because the runtime divides by it,
    /// and a zero would make every sample infinite rather than merely wrong.
    #[must_use]
    pub fn parse(payload: &[u8]) -> Option<Self> {
        if payload.len() < PAYLOAD_LEN {
            return None;
        }
        let edge = f32::from_le_bytes(payload.get(EDGE..EDGE + 4)?.try_into().ok()?);
        // Finite *and* positive, spelled out: the runtime divides by this, so a
        // NaN would propagate into every sample rather than fail here, and a
        // plain `edge <= 0.0` lets NaN through.
        if !edge.is_finite() || edge <= 0.0 {
            return None;
        }
        Some(Self {
            to_local: vex::transform(&payload[..0x40])?,
            near_end: FogParams::read(payload, SET_A)?,
            far_end: FogParams::read(payload, SET_B)?,
            tiebreak: f32::from_le_bytes(payload.get(TIEBREAK..TIEBREAK + 4)?.try_into().ok()?),
            edge,
        })
    }

    /// Half the edge length: the box test's bound, as `FogCube_Init` derives it.
    #[must_use]
    pub fn half_extent(&self) -> f32 {
        self.edge * 0.5
    }

    /// Fog at `world` if this volume contains it, `None` if it does not.
    ///
    /// Reimplements `FogCube_Sample`: transform into volume space, reject
    /// outside the box on any axis, then normalise **local Z** across the box
    /// and interpolate.
    #[must_use]
    pub fn sample(&self, world: [f32; 3]) -> Option<FogParams> {
        let local = transform_point(&self.to_local, world);
        let half = self.half_extent();
        if local.iter().any(|c| c.abs() > half) {
            return None;
        }
        // The runtime stores `1.0 / edge` and multiplies; dividing here is the
        // same value and keeps the field that was actually authored on show.
        let t = local[2] / self.edge + 0.5;
        Some(self.near_end.lerp(self.far_end, t))
    }
}

/// Every `fogCube` a `.vex` file authors, in node order.
///
/// A track has one or none - `06_Track` authors none, so an empty result is
/// ordinary and not a decode failure.
#[must_use]
pub fn volumes(data: &[u8], nodes: &[Node]) -> Vec<FogVolume> {
    vex::nodes_by_class(nodes, vex::CLASS_FOGCUBE)
        .filter_map(|node| FogVolume::parse(data.get(node.payload())?))
        .collect()
}

/// The volume containing `world`, or `None` outside all of them.
///
/// Where volumes nest, the smallest [`FogVolume::tiebreak`] wins, which is what
/// `Fog_FindVolume` (`0x08887764`) does. No shipped PSP track authors more than
/// one, so that rule is reimplemented rather than exercised.
#[must_use]
pub fn sample(volumes: &[FogVolume], world: [f32; 3]) -> Option<FogParams> {
    volumes
        .iter()
        .filter_map(|v| v.sample(world).map(|p| (v.tiebreak, p)))
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, p)| p)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A volume centred on the origin with no rotation and a 100-unit edge,
    /// grading from black at `-Z` to white at `+Z`.
    fn payload() -> Vec<u8> {
        let mut p = vec![0u8; PAYLOAD_LEN];
        let mut put = |at: usize, v: f32| p[at..at + 4].copy_from_slice(&v.to_le_bytes());
        // Identity 4x4: the box is already axis-aligned about the origin.
        for i in 0..4 {
            put(i * 16 + i * 4, 1.0);
        }
        put(SET_A, 0.0);
        put(SET_A + 4, 0.0);
        put(SET_A + 8, 0.0);
        put(SET_A + 16, 10.0);
        put(SET_A + 20, 100.0);
        put(SET_B, 1.0);
        put(SET_B + 4, 1.0);
        put(SET_B + 8, 1.0);
        put(SET_B + 16, 30.0);
        put(SET_B + 20, 300.0);
        put(TIEBREAK, 5.0);
        put(EDGE, 100.0);
        p
    }

    #[test]
    fn the_edge_length_sets_the_box() {
        let v = FogVolume::parse(&payload()).expect("parses");
        assert_eq!(v.edge, 100.0);
        assert_eq!(v.half_extent(), 50.0);
    }

    #[test]
    fn a_point_outside_the_box_is_not_in_the_volume() {
        let v = FogVolume::parse(&payload()).expect("parses");
        assert!(v.sample([0.0, 0.0, 0.0]).is_some());
        // Just past the face on each axis in turn, so a missing axis in the
        // test shows up rather than being masked by the others.
        assert!(v.sample([50.1, 0.0, 0.0]).is_none());
        assert!(v.sample([0.0, -50.1, 0.0]).is_none());
        assert!(v.sample([0.0, 0.0, 50.1]).is_none());
    }

    #[test]
    fn the_parameters_grade_across_local_z() {
        let v = FogVolume::parse(&payload()).expect("parses");
        // The -Z face is set A exactly, the +Z face set B, the centre halfway.
        let lo = v.sample([0.0, 0.0, -50.0]).expect("inside");
        let mid = v.sample([0.0, 0.0, 0.0]).expect("inside");
        let hi = v.sample([0.0, 0.0, 50.0]).expect("inside");

        assert_eq!(lo.colour, [0.0, 0.0, 0.0]);
        assert_eq!(lo.near, 10.0);
        assert_eq!(hi.colour, [1.0, 1.0, 1.0]);
        assert_eq!(hi.far, 300.0);
        assert_eq!(mid.colour, [0.5, 0.5, 0.5]);
        assert_eq!(mid.near, 20.0);
        assert_eq!(mid.far, 200.0);
    }

    #[test]
    fn identical_sets_grade_to_nothing() {
        // The shipped common case: 28 of 36 nodes carry the same six floats
        // twice, and must then read the same everywhere in the box.
        let mut p = payload();
        p.copy_within(SET_A..SET_A + 24, SET_B);
        let v = FogVolume::parse(&p).expect("parses");
        for z in [-50.0, -12.5, 0.0, 33.0, 50.0] {
            assert_eq!(v.sample([0.0, 0.0, z]), Some(v.near_end));
        }
    }

    #[test]
    fn the_smallest_tiebreak_wins_where_volumes_nest() {
        let outer = FogVolume::parse(&payload()).expect("parses");
        let mut inner_bytes = payload();
        inner_bytes[TIEBREAK..TIEBREAK + 4].copy_from_slice(&1.0f32.to_le_bytes());
        inner_bytes[SET_A..SET_A + 4].copy_from_slice(&0.25f32.to_le_bytes());
        inner_bytes[SET_B..SET_B + 4].copy_from_slice(&0.25f32.to_le_bytes());
        let inner = FogVolume::parse(&inner_bytes).expect("parses");

        // Order in the list must not decide it; only the tiebreak.
        for list in [vec![outer, inner], vec![inner, outer]] {
            let got = sample(&list, [0.0, 0.0, 0.0]).expect("inside both");
            assert_eq!(got.colour[0], 0.25, "the smaller tiebreak did not win");
        }
    }

    #[test]
    fn a_zero_edge_is_rejected_rather_than_dividing_by_it() {
        let mut p = payload();
        p[EDGE..EDGE + 4].copy_from_slice(&0.0f32.to_le_bytes());
        assert_eq!(FogVolume::parse(&p), None);
    }

    #[test]
    fn a_short_payload_is_rejected() {
        assert_eq!(FogVolume::parse(&[0u8; PAYLOAD_LEN - 1]), None);
    }
}
