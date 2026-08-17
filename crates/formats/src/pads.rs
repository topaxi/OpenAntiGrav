//! `Speedup Pad` `0x3bd` and `Weapon Pad` `0x3be`: the trigger volumes on the
//! track surface.
//!
//! A pad is a [`Mesh`](vex::CLASS_MESH) subclass. Its bind handler
//! (`Pad_Bind`, `0x089264f4`) calls the `Mesh` bind first and only then reads
//! its own fields, so a pad carries its own geometry **and** its own trigger
//! box, and the box is the mesh's own bounding-box pair:
//!
//! ```text
//! payload +0x10  {x, y, z, _}  bounding-box minimum, pad-local space
//! payload +0x20  {x, y, z, _}  bounding-box maximum, pad-local space
//! ```
//!
//! The bind copies both into the object and then expands the box vertically -
//! `min.y -= 2.0`, `max.y += 8.0` - which is what turns a flat plate into
//! something a craft can be inside. The expansion is asymmetric in `+y`, and
//! `+y` is world up (see [`vex::transform`]), so a pad is entered from above.
//!
//! # The box is local, the placement is the transform chain
//!
//! All nine of `01_Track`'s `Speedup Pad` nodes share one byte-identical
//! payload under nine different [`Transform`](vex::CLASS_TRANSFORM) parents. So
//! nothing about where a pad *is* lives in its payload, and the containment test
//! has to run in pad-local space rather than against a world-space box - which
//! is exactly what `Pad_ContainsPoint` (`0x088866bc`) does.
//!
//! # What the pad pushes along
//!
//! Row 2 of the pad's world matrix - its local `+Z` axis. `Ship_ApplySpeedupPad`
//! (`0x08848f9c`) copies `matrix+0x20`, and the matrix is the row-major,
//! translation-in-row-3 layout every `.vex` uses, so `+0x20` is floats 8 to 10:
//! row 2. See [`PadVolume::direction`].
//!
//! Evidence and confidence scores: `docs/formats/pads.md`.

use crate::ByteOrder;
use crate::vex::{self, Node};

/// Where the bounding-box minimum sits in a pad's payload.
const MIN: usize = 0x10;

/// Where the bounding-box maximum sits.
const MAX: usize = 0x20;

/// Shortest payload that can carry the box pair.
const MIN_PAYLOAD: usize = 0x30;

/// How far the bind lowers the box floor.
///
/// A literal in `Pad_Bind`. It is in world units, which is only meaningful
/// because a pad mesh's quantisation scale is `1.0` - asserted against the disc
/// in `pads_ground_truth.rs`, because if it were not, these two literals would
/// be the only thing in the layout that is not.
const EXPAND_DOWN: f32 = 2.0;

/// How far the bind raises the box ceiling.
const EXPAND_UP: f32 = 8.0;

/// One authored pad: where it is, how big it is, and which way it pushes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PadVolume {
    /// Pad-local to world. Row-major, row-vector convention, composed from the
    /// node's [`Transform`](vex::CLASS_TRANSFORM) chain the way any mesh's is.
    pub to_world: [f32; 16],
    /// Box minimum in pad-local space, with [`EXPAND_DOWN`] already applied.
    pub min: [f32; 3],
    /// Box maximum in pad-local space, with [`EXPAND_UP`] already applied.
    pub max: [f32; 3],
    /// The original's own gate on a hit, at pad `+0x1a0`.
    ///
    /// `Pad_ContainsPoint` reports a hit only when this is `0.0`, so a non-zero
    /// value disables the pad. **Nothing here ever writes it.** The writer is
    /// identified now - `WeaponPads_TestCraft` stamps a `Weapon Pad`'s own
    /// `<WeaponPad refresh_time>` (or `elimination_refresh_time` under a mode
    /// this project reads as Eliminator) on pickup, and `Pad_UpdateRefreshTimer`
    /// counts it back down - but there is still no pickup system to drive it
    /// from, `Weapon Pad` included. See `pads.md` in `docs/ghidra/functions/`.
    /// Carried inert rather than dropped: a term that is provably in the
    /// condition should be visible in the reimplementation of that condition,
    /// even at zero.
    pub disabled: f32,
}

impl PadVolume {
    /// Decodes one pad payload against a world matrix from its transform chain.
    ///
    /// Returns `None` for a payload too short to hold the box pair, or one whose
    /// box is not finite or is inverted on any axis. An inverted box would make
    /// [`Self::distance`] report a positive distance from every point including
    /// the ones inside it, which reads as "no pad here" rather than as a
    /// failure.
    /// `order` is the containing file's, from [`vex::byte_order`]. A mesh
    /// payload carries no magic, so it cannot say which way round it is - and
    /// this is the one HD `Mesh` field that is still *there*, the box pair
    /// having survived the move of the geometry into `.rcsmodel`.
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

    /// The direction this pad pushes a craft, normalised.
    ///
    /// Row 2 of [`Self::to_world`]: the pad's local `+Z` in world space.
    /// Returns `None` if the row is degenerate, which no shipped pad's is.
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

    /// Transforms a world point into pad-local space.
    ///
    /// The inverse of [`vex::transform_point`] for this matrix. Rows 0 to 2 are
    /// an orthonormal basis in every shipped transform, so the inverse rotation
    /// is a dot against each row rather than a general inverse - which is what
    /// `Pad_ContainsPoint` does with a single `vtfm3`.
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

    /// Distance from `world` to this pad's box, `0.0` inside it.
    ///
    /// Reimplements the body of `Pad_ContainsPoint`: per axis, how far outside
    /// the slab the point is; then the length of that vector. The original
    /// keeps this figure per racer per pad and decrements it by how far the
    /// craft moved, so it only runs the real test once a craft could plausibly
    /// have reached the pad - see `docs/formats/pads.md`.
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
    /// Both halves of `Pad_ContainsPoint`'s return condition: the distance is
    /// exactly zero **and** [`Self::disabled`] is exactly zero.
    #[must_use]
    pub fn contains(&self, world: [f32; 3]) -> bool {
        self.distance(world) == 0.0 && self.disabled == 0.0
    }
}

/// Every pad of one class a `.vex` file authors, in node order.
///
/// Pass [`vex::CLASS_SPEEDUP_PAD`] or [`vex::CLASS_WEAPON_PAD`]. An empty
/// result is ordinary: a `.vex` that is not a track authors neither.
///
/// A pad's own payload is a mesh payload, **not** a matrix, so its placement
/// comes from [`vex::world_transforms`] - the parent chain, with the pad itself
/// contributing the identity. Using [`vex::class_world_transforms`] here would
/// read the first 64 bytes of the mesh header as though they were a matrix.
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
    /// The point of the test is that containment is *not* a world-space box
    /// test: a point that is inside the rotated pad is outside the same box
    /// treated as axis-aligned in world space.
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

    /// The same box yawed 45 degrees about `+y`, so that it is genuinely not
    /// axis-aligned - a quarter turn maps a box onto itself and cannot tell a
    /// local-space test from a world-space one.
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

        // Local (4.7, 0, 4.9): inside, near the far corner. Its world x of
        // 6.788 is well outside the box's own `max.x`, so an axis-aligned test
        // against `min`/`max` in world space would miss it.
        assert!(pad.contains([9.6 * C, 0.0, 0.2 * C]));

        // Local (0, 0, 6.0): outside, past `max.z` of 4.968. Both its world
        // coordinates are inside the box's extents, so the same axis-aligned
        // test would report a hit.
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
