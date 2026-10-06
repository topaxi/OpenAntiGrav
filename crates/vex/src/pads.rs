//! `Speedup Pad` `0x3bd` and `Weapon Pad` `0x3be`: the trigger volumes on the
//! track surface.
//!
//! A pad is a [`Mesh`](vex::CLASS_MESH) subclass: `Pad_Bind` (`0x089264f4`) calls
//! the `Mesh` bind first, so a pad carries its own geometry **and** its trigger
//! box, which is the mesh's bounding-box pair:
//!
//! ```text
//! payload +0x10  {x, y, z, _}  bounding-box minimum, pad-local space
//! payload +0x20  {x, y, z, _}  bounding-box maximum, pad-local space
//! ```
//!
//! The bind copies both into the object and expands the box vertically (`min.y
//! -= 2.0`, `max.y += 8.0`), turning a flat plate into something a craft can be
//! inside. The expansion is asymmetric in `+y`, world up (see
//! [`vex::transform`]), so a pad is entered from above.
//!
//! # The box is local, the placement is the transform chain
//!
//! All nine of `01_Track`'s `Speedup Pad` nodes share one byte-identical payload
//! under nine different [`Transform`](vex::CLASS_TRANSFORM) parents, so nothing
//! about where a pad *is* lives in its payload and the containment test runs in
//! pad-local space, as `Pad_ContainsPoint` (`0x088866bc`) does.
//!
//! # What the pad pushes along
//!
//! Row 2 of the pad's world matrix, its local `+Z` axis: `Ship_ApplySpeedupPad`
//! (`0x08848f9c`) copies `matrix+0x20` (floats 8 to 10) of the row-major,
//! translation-in-row-3 layout every `.vex` uses. See [`PadVolume::direction`].
//!
//! Evidence and confidence scores: `docs/formats/pads.md`.

use crate::vex::{self, Node};
use oag_formats::ByteOrder;

/// Where the bounding-box minimum sits in a pad's payload.
const MIN: usize = 0x10;

/// Where the bounding-box maximum sits.
const MAX: usize = 0x20;

/// Shortest payload that can carry the box pair.
const MIN_PAYLOAD: usize = 0x30;

/// How far the bind lowers the box floor.
///
/// A literal in `Pad_Bind`, in world units, meaningful only because a pad mesh's
/// quantisation scale is `1.0` (asserted in `pads_ground_truth.rs`; otherwise
/// these two literals would be the only thing in the layout that is not).
const EXPAND_DOWN: f32 = 2.0;

/// How far the bind raises the box ceiling.
const EXPAND_UP: f32 = 8.0;

/// One authored pad: where it is, how big it is, and which way it pushes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PadVolume {
    /// Pad-local to world: row-major, row-vector convention, composed from the
    /// node's [`Transform`](vex::CLASS_TRANSFORM) chain like any mesh's.
    pub to_world: [f32; 16],
    /// Box minimum in pad-local space, with [`EXPAND_DOWN`] already applied.
    pub min: [f32; 3],
    /// Box maximum in pad-local space, with [`EXPAND_UP`] already applied.
    pub max: [f32; 3],
    /// The original's own gate on a hit, at pad `+0x1a0`: `Pad_ContainsPoint`
    /// reports a hit only when this is `0.0`, so non-zero disables the pad.
    /// **Nothing here writes it.** The writer is identified (`WeaponPads_TestCraft`
    /// stamps a `Weapon Pad`'s `<WeaponPad refresh_time>`, or
    /// `elimination_refresh_time` under Eliminator, on pickup;
    /// `Pad_UpdateRefreshTimer` counts it down; see `pads.md` under
    /// `docs/ghidra/functions/`)
    /// but there is no pickup system to drive it. Carried inert: a term provably
    /// in the condition should be visible in its reimplementation, even at zero.
    pub disabled: f32,
}

impl PadVolume {
    /// Decodes one pad payload against a world matrix from its transform chain.
    ///
    /// `None` for a payload too short for the box pair, or a box not finite or
    /// inverted on any axis (an inverted box would make [`Self::distance`] report
    /// a positive distance from every point, reading as "no pad here").
    ///
    /// `order` is the containing file's ([`vex::byte_order`]): a mesh payload has
    /// no magic. This is the one HD `Mesh` field still *there*, the box pair
    /// having survived the geometry's move into `.rcsmodel`.
    #[must_use]
    pub fn parse(payload: &[u8], to_world: [f32; 16], order: ByteOrder) -> Option<Self> {
        if payload.len() < MIN_PAYLOAD {
            return None;
        }
        let f = |at: usize| -> Option<f32> {
            payload.get(at..at + 4)?;
            Some(order.f32(payload, at))
        };
        let mut min = [f(MIN)?, f(MIN + 4)?, f(MIN + 8)?];
        let mut max = [f(MAX)?, f(MAX + 4)?, f(MAX + 8)?];

        if !min.iter().chain(max.iter()).all(|c| c.is_finite()) {
            return None;
        }
        if (0..3).any(|i| max[i] < min[i]) {
            return None;
        }

        min[1] -= EXPAND_DOWN;
        max[1] += EXPAND_UP;

        Some(Self {
            to_world,
            min,
            max,
            disabled: 0.0,
        })
    }

    /// The direction this pad pushes a craft, normalised: row 2 of
    /// [`Self::to_world`], the pad's local `+Z`. `None` if degenerate (no shipped
    /// pad's is).
    #[must_use]
    pub fn direction(&self) -> Option<[f32; 3]> {
        let row = [self.to_world[8], self.to_world[9], self.to_world[10]];
        let length = dot(row, row).sqrt();
        if !length.is_finite() || length <= 0.0 {
            return None;
        }
        Some([row[0] / length, row[1] / length, row[2] / length])
    }

    /// Where this pad sits in world space: its box centre, transformed.
    #[must_use]
    pub fn centre(&self) -> [f32; 3] {
        let local = [
            (self.min[0] + self.max[0]) * 0.5,
            (self.min[1] + self.max[1]) * 0.5,
            (self.min[2] + self.max[2]) * 0.5,
        ];
        vex::transform_point(&self.to_world, local)
    }

    /// Transforms a world point into pad-local space (the inverse of
    /// [`vex::transform_point`]). Rows 0 to 2 are orthonormal in every shipped
    /// transform, so the inverse rotation is a dot against each row, as
    /// `Pad_ContainsPoint` does with one `vtfm3`.
    #[must_use]
    pub fn to_local(&self, world: [f32; 3]) -> [f32; 3] {
        let m = &self.to_world;
        let d = [world[0] - m[12], world[1] - m[13], world[2] - m[14]];
        [
            dot(d, [m[0], m[1], m[2]]),
            dot(d, [m[4], m[5], m[6]]),
            dot(d, [m[8], m[9], m[10]]),
        ]
    }

    /// Distance from `world` to this pad's box, `0.0` inside it: per axis, how far
    /// outside the slab, then the vector's length, as `Pad_ContainsPoint` does. The
    /// original keeps this per racer per pad and decrements it by the craft's
    /// movement, running the real test only once a craft could have reached the
    /// pad (`docs/formats/pads.md`).
    #[must_use]
    pub fn distance(&self, world: [f32; 3]) -> f32 {
        let local = self.to_local(world);
        let mut outside = [0.0f32; 3];
        for axis in 0..3 {
            if local[axis] < self.min[axis] {
                outside[axis] = self.min[axis] - local[axis];
            } else if local[axis] > self.max[axis] {
                outside[axis] = self.max[axis] - local[axis];
            }
        }
        dot(outside, outside).sqrt()
    }

    /// Whether `world` is inside this pad and this pad is armed.
    ///
    /// Both halves of `Pad_ContainsPoint`'s return condition: the distance and
    /// [`Self::disabled`] are both exactly zero.
    #[must_use]
    pub fn contains(&self, world: [f32; 3]) -> bool {
        self.distance(world) == 0.0 && self.disabled == 0.0
    }
}

/// Every pad of one class a `.vex` file authors, in node order.
///
/// Pass [`vex::CLASS_SPEEDUP_PAD`] or [`vex::CLASS_WEAPON_PAD`]. An empty result
/// is ordinary (a non-track `.vex`).
///
/// A pad's payload is a mesh payload, **not** a matrix, so placement comes from
/// [`vex::world_transforms`] (the pad contributing the identity).
/// [`vex::class_world_transforms`] would read the mesh header's first 64 bytes as
/// a matrix.
#[must_use]
pub fn volumes(data: &[u8], nodes: &[Node], class_id: u32) -> Vec<PadVolume> {
    let chain = vex::world_transforms(data, nodes);
    let order = vex::byte_order(data);
    nodes
        .iter()
        .enumerate()
        .filter(|(_, node)| node.class_id == class_id)
        .filter_map(|(index, node)| {
            let to_world = chain.get(index).copied()?;
            PadVolume::parse(data.get(node.payload())?, to_world, order)
        })
        .collect()
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The box `01_Track`'s pads actually author, rounded to what the hex dump
    /// shows, in a payload long enough to parse.
    fn payload() -> Vec<u8> {
        let mut p = vec![0u8; MIN_PAYLOAD];
        let mut put = |at: usize, v: f32| p[at..at + 4].copy_from_slice(&v.to_le_bytes());
        put(MIN, -4.775);
        put(MIN + 4, -0.15);
        put(MIN + 8, -3.754);
        put(MAX, 4.775);
        put(MAX + 4, -0.0016);
        put(MAX + 8, 4.968);
        p
    }

    #[test]
    fn the_bind_expands_the_box_vertically_and_asymmetrically() {
        let pad = PadVolume::parse(&payload(), vex::IDENTITY, ByteOrder::Little).expect("parse");
        assert_eq!(pad.min[1], -0.15 - EXPAND_DOWN);
        assert_eq!(pad.max[1], -0.0016 + EXPAND_UP);
        // The horizontal extents are untouched.
        assert_eq!(pad.min[0], -4.775);
        assert_eq!(pad.max[2], 4.968);
    }

    #[test]
    fn a_point_on_the_plate_is_inside_and_one_beside_it_is_not() {
        let pad = PadVolume::parse(&payload(), vex::IDENTITY, ByteOrder::Little).expect("parse");
        assert!(pad.contains([0.0, 0.0, 0.0]));
        assert!(pad.contains([0.0, 4.0, 0.0]), "the expanded ceiling");
        assert!(!pad.contains([0.0, 9.0, 0.0]), "above the expanded ceiling");
        assert!(!pad.contains([20.0, 0.0, 0.0]), "beside the plate");
    }

    #[test]
    fn distance_is_zero_inside_and_grows_outside() {
        let pad = PadVolume::parse(&payload(), vex::IDENTITY, ByteOrder::Little).expect("parse");
        assert_eq!(pad.distance([0.0, 0.0, 0.0]), 0.0);
        // 20.0 is 15.225 past the +x face, and nothing else is out of range.
        let d = pad.distance([20.0, 0.0, 0.0]);
        assert!((d - 15.225).abs() < 1e-3, "{d}");
    }

    #[test]
    fn the_disabled_gate_suppresses_a_hit_that_is_geometrically_inside() {
        let mut pad =
            PadVolume::parse(&payload(), vex::IDENTITY, ByteOrder::Little).expect("parse");
        assert!(pad.contains([0.0, 0.0, 0.0]));
        pad.disabled = 1.0;
        assert!(!pad.contains([0.0, 0.0, 0.0]));
        assert_eq!(pad.distance([0.0, 0.0, 0.0]), 0.0, "geometry is unchanged");
    }

    /// A pad rotated a quarter turn about `+y` and moved 100 units along `+x`.
    ///
    /// A pad rotated a quarter turn about `+y` and moved 100 units along `+x`.
    ///
    /// Containment is *not* a world-space box test: a point inside the rotated pad
    /// is outside the same box treated as axis-aligned in world space.
    fn rotated() -> PadVolume {
        let m = [
            0.0, 0.0, -1.0, 0.0, //
            0.0, 1.0, 0.0, 0.0, //
            1.0, 0.0, 0.0, 0.0, //
            100.0, 0.0, 0.0, 1.0, //
        ];
        PadVolume::parse(&payload(), m, ByteOrder::Little).expect("parse")
    }

    #[test]
    fn containment_runs_in_pad_local_space() {
        let pad = rotated();
        // Local +z is world +x, and the box runs to 4.968 along local z.
        assert!(pad.contains([104.0, 0.0, 0.0]));
        // Local +x is world -z, and the box runs to 4.775 along local x.
        assert!(pad.contains([100.0, 0.0, -4.0]));
    }

    /// The same box yawed 45 degrees about `+y`: a quarter turn maps a box onto
    /// itself and cannot tell a local-space test from a world-space one.
    #[test]
    fn a_rotated_pad_is_not_its_world_space_bounding_box() {
        const C: f32 = std::f32::consts::FRAC_1_SQRT_2;
        let m = [
            C, 0.0, -C, 0.0, //
            0.0, 1.0, 0.0, 0.0, //
            C, 0.0, C, 0.0, //
            0.0, 0.0, 0.0, 1.0, //
        ];
        let pad = PadVolume::parse(&payload(), m, ByteOrder::Little).expect("parse");

        // Local (4.7, 0, 4.9): inside near the far corner; its world x of 6.788 is
        // outside the box's `max.x`, so an axis-aligned world test would miss it.
        assert!(pad.contains([9.6 * C, 0.0, 0.2 * C]));

        // Local (0, 0, 6.0): outside, past `max.z` of 4.968, but both world
        // coordinates are inside the box's extents, so an axis-aligned test would hit.
        assert!(!pad.contains([6.0 * C, 0.0, 6.0 * C]));
    }

    #[test]
    fn the_push_direction_is_row_two_in_world_space() {
        let pad = rotated();
        let d = pad.direction().expect("direction");
        assert!((d[0] - 1.0).abs() < 1e-6, "{d:?}");
        assert!(d[1].abs() < 1e-6 && d[2].abs() < 1e-6, "{d:?}");
    }

    #[test]
    fn the_centre_is_the_box_centre_placed_by_the_transform() {
        let pad = rotated();
        let c = pad.centre();
        // Local centre z is (-3.754 + 4.968) / 2 = 0.607, which the rotation
        // sends to world +x, on top of the translation.
        assert!((c[0] - 100.607).abs() < 1e-3, "{c:?}");
    }

    #[test]
    fn a_payload_too_short_for_the_box_pair_is_not_a_pad() {
        assert!(PadVolume::parse(&[0u8; 0x2f], vex::IDENTITY, ByteOrder::Little).is_none());
    }

    #[test]
    fn an_inverted_box_is_rejected_rather_than_silently_never_hit() {
        let mut p = payload();
        p[MAX..MAX + 4].copy_from_slice(&(-100.0f32).to_le_bytes());
        assert!(PadVolume::parse(&p, vex::IDENTITY, ByteOrder::Little).is_none());
    }

    #[test]
    fn a_degenerate_direction_row_is_reported_rather_than_returned_as_nan() {
        let pad = PadVolume::parse(&payload(), [0.0; 16], ByteOrder::Little).expect("parse");
        assert!(pad.direction().is_none());
    }
}
