//! One authored airbrake flap.
//!
//! Split out of `mesh.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change.

use oag_core::math::{Mat4, Vec3};
use oag_vex::vex;

use super::GpuVertex;

/// One authored airbrake flap: which vertices are its own, and what it hinges
/// about.
///
/// The flap is **not** a separate model. `Airbrake` (`0x3c5`) sits between a
/// `Transform` and a `Mesh` in `Ship.vex`, so its geometry is already in the
/// ship's buffers with everything else, and pulling it out into its own
/// `Drawable` would duplicate the ship's whole texture set for two meshes.
/// What it needs instead is for its own vertices to move, which is what
/// [`Self::vertices`] is for - see `Flap::deflect`.
#[derive(Debug, Clone, PartialEq)]
pub struct Flap {
    /// This flap's vertices, as a range into [`super::Model::vertices`].
    ///
    /// Contiguous because the builder appends one mesh node's vertices at a
    /// time and an `Airbrake`'s mesh children are consecutive: one on six
    /// teams, two on Feisar and Triakis (the flap, then its `underbrake_flash`),
    /// whose spans the builder joins. A child that did not follow its sibling
    /// would need a list here.
    pub vertices: std::ops::Range<u32>,
    /// The hinge's pose in model space: the enclosing `Transform`'s composed
    /// world matrix.
    ///
    /// The locator, not the flap. Its two instances are mirror images in X
    /// (`+1.5246` and `-1.5246` on Assegai), which is what makes the sides
    /// rotate oppositely without either angle being negated by hand.
    pub hinge: Mat4,
}

impl Flap {
    /// Records `node`'s vertices as part of an airbrake flap, when it is one.
    ///
    /// A flap is a `Mesh` whose parent is an `Airbrake` node, and its hinge is
    /// the `Airbrake`'s own parent: the locator `Transform` that carries the
    /// 4x4. `anchors` holds that pose (no ship authors an `Anim Transform`, so
    /// with no anchor above it `Anchored::local` *is* the world matrix).
    ///
    /// Left and right by the artists' own node names (`Airbrake_Left`,
    /// `Airbrake_Right`), not by the sign of a translation. `span` is the
    /// node's vertices in the model being built; a second mesh under the same
    /// `Airbrake` (Feisar's and Triakis's `underbrake_flash`) extends the
    /// first's span, since a builder appends a node at a time.
    ///
    /// Version-keyed through `classes.airbrake`, so a `.vex` version whose
    /// class id is unrecovered leaves a ship with no flaps.
    pub(super) fn collect(
        airbrakes: &mut [Option<Self>; 2],
        nodes: &[vex::Node],
        classes: vex::classes::Classes,
        anchors: &[vex::Anchored],
        node: &vex::Node,
        span: std::ops::Range<u32>,
    ) {
        let Some(brake) = node.parent else { return };
        if span.is_empty()
            || !nodes
                .get(brake)
                .is_some_and(|n| Some(n.class_id) == classes.airbrake)
        {
            return;
        }
        let Some(hinge) = nodes[brake].parent else {
            return;
        };
        // A Maya namespace (`andy_ship:AirBrake_left`, PS2's Triakis race hull) is not part of
        // the side.
        let name = nodes[brake].name.as_deref().unwrap_or_default();
        let name = name.rsplit(':').next().unwrap_or(name);
        let side = if name.eq_ignore_ascii_case("Airbrake_Left") {
            0
        } else if name.eq_ignore_ascii_case("Airbrake_Right") {
            1
        } else {
            return;
        };
        Self::record(
            &mut airbrakes[side],
            span,
            Mat4::from_cols_array(&anchors[hinge].local),
        );
    }

    /// Adds `span` to `slot`: the flap's first vertices, or an extension of
    /// them when it begins where they end (a second mesh under the same flap,
    /// since a builder appends one mesh at a time).
    pub(super) fn record(slot: &mut Option<Self>, span: std::ops::Range<u32>, hinge: Mat4) {
        match slot {
            Some(flap) if flap.vertices.end == span.start => flap.vertices.end = span.end,
            slot => {
                *slot = Some(Self {
                    vertices: span,
                    hinge,
                })
            }
        }
    }

    /// The model-space transform that swings this flap by `angle` radians.
    ///
    /// `hinge * R * hinge^-1`, because the builder has already baked every
    /// ancestor transform into the vertices ([`super::build_with_textures`]
    /// composes the tree through `vex::world_transforms`). Rotating the
    /// baked vertices directly would swing them about the model's origin
    /// instead of the hinge, which on Assegai is 6.2 units behind it.
    ///
    /// # The axis is recovered, from a live read
    ///
    /// The `Airbrake` node's payload is **zero bytes** and its class
    /// descriptor carries no handler, so the file does not say which way the
    /// flap turns and class-id dispatch has no per-class function to read it
    /// out of. What that search missed is a per-instance handler reached
    /// indirectly (the same shape `Exhaust_Update` has - no direct callers
    /// either): a live PPSSPP session with a read watchpoint on the graphics
    /// deflection state found `Airbrake_Update`, whose raw VFPU disassembly
    /// builds `(1,0,0,0)`/`(0,cos,sin,0)`/`(0,-sin,cos,0)`/`(0,0,0,1)` -
    /// local X, row-major. Confidence 90; see
    /// `docs/ghidra/functions/psp-pulse-usa/ship-parts.md`'s "recovered from
    /// a live read" section for the disassembly and the watchpoint counts.
    /// The mirrored hinges still take care of the sign, and everything else
    /// was already recovered - the hinge pose, the deflection in radians and
    /// both rates.
    #[must_use]
    pub fn deflect(&self, angle: f32) -> Mat4 {
        // Stowed is the common case and has to be *exact*: `H * I * H^-1` is
        // only identity to about `1e-7`, which would leave a closed flap
        // permanently a fraction of a unit off the hull it was authored
        // against. Cheap to rule out, and it is the case a parked ship spends
        // all its time in.
        if angle == 0.0 {
            return Mat4::IDENTITY;
        }
        self.hinge * Mat4::from_rotation_x(angle) * self.hinge.inverse()
    }

    /// This flap's vertices swung by `angle` radians, into `out` (cleared
    /// first), and where in `vertices` they belong - `None` when the flap's
    /// range is not inside `vertices`.
    ///
    /// **The one swing both draws of a flap use.** The hull's base draw
    /// (`Drawable::deflect_airbrakes`) and the extra pass's copy of the same
    /// vertices (`shine::write`) call this, because the original draws both
    /// under the *same node matrix*: in three recorded GE lists the flap's
    /// `TEXMAPMODE` 2 PRIM carries a world matrix equal, to the last digit, to
    /// its ordinary twin's and different from `shipShape`'s. Positions and
    /// normals are rotated from `vertices` as given - never from a previous
    /// frame - so a second caller reproduces the first bit for bit.
    pub fn swung(
        &self,
        vertices: &[GpuVertex],
        angle: f32,
        out: &mut Vec<GpuVertex>,
    ) -> Option<std::ops::Range<usize>> {
        let span = self.vertices.start as usize..self.vertices.end as usize;
        let base = vertices.get(span.clone())?;
        let swing = self.deflect(angle);
        out.clear();
        out.extend(base.iter().map(|v| moved(&swing, v)));
        Some(span)
    }

    /// Seats a flap whose vertices are in its hinge's own space: bakes `hinge`
    /// into them and records it as this flap's hinge, so the flap sits where
    /// the race model's does at rest and [`Self::swung`] turns it about the
    /// same pivot.
    ///
    /// For a hull whose file carries the flap with no locator above it: every
    /// Pulse `ship_FE.vex` hangs `AirBrake_Left`/`AirBrake_Right` straight
    /// under its turntable `Anim Transform`, so the builder had no hinge to
    /// bake and left the flap at the model origin, mid-hull. Positions and
    /// normals go through the same [`moved`] a swing uses. A no-op when the
    /// flap already has a hinge (the race hull's), so a second call cannot
    /// place it twice.
    pub fn seat(&mut self, vertices: &mut [GpuVertex], hinge: Mat4) {
        if self.hinge != Mat4::IDENTITY {
            return;
        }
        let span = self.vertices.start as usize..self.vertices.end as usize;
        if let Some(own) = vertices.get_mut(span) {
            for v in own {
                *v = moved(&hinge, v);
            }
        }
        self.hinge = hinge;
    }
}

impl super::Model {
    /// Gives this model's airbrake flaps `race`'s hinges and rest placement, side for side, and
    /// says how many sides it did (`0` to `2`).
    ///
    /// The front-end hull is the race hull's twin without the hinge locators, so its flaps
    /// carry no hinge. How their vertices were authored varies: Pulse's PSP files keep each flap
    /// in its hinge's own space ([`Flap::seat`] carries it through the race hinge, after which
    /// every vertex is within 2 mm of the race flap's), and some PS2 files already hold the flap
    /// in model space, where moving it again would throw it off the hull. Which of the two a
    /// flap is comes from where its centroid lies against the race flap's, before and after
    /// the hinge; an already placed flap only adopts the hinge, so a swing turns about the right
    /// pivot. A side either model lacks is left as it is.
    pub fn seat_flaps_on(&mut self, race: &Self) -> usize {
        let mut seated = 0;
        for (own, theirs) in self.airbrakes.iter_mut().zip(&race.airbrakes) {
            let (Some(own), Some(theirs)) = (own, theirs) else {
                continue;
            };
            if own.hinge != Mat4::IDENTITY {
                continue;
            }
            let (Some(have), Some(want)) = (
                centroid(&self.vertices, &own.vertices),
                centroid(&race.vertices, &theirs.vertices),
            ) else {
                continue;
            };
            if (theirs.hinge.transform_point3(have) - want).length() < (have - want).length() {
                own.seat(&mut self.vertices, theirs.hinge);
            } else {
                own.hinge = theirs.hinge;
            }
            seated += 1;
        }
        seated
    }
}

/// The mean position of `span` of `vertices`, or `None` for an empty or out-of-range span.
fn centroid(vertices: &[GpuVertex], span: &std::ops::Range<u32>) -> Option<Vec3> {
    let own = vertices.get(span.start as usize..span.end as usize)?;
    (!own.is_empty()).then(|| {
        own.iter()
            .map(|v| Vec3::from_array(v.position))
            .sum::<Vec3>()
            / own.len() as f32
    })
}

/// `vertex` with its position and normal carried through `matrix`: the one
/// transform a swing ([`Flap::swung`]) and a seating ([`Flap::seat`]) share.
fn moved(matrix: &Mat4, vertex: &GpuVertex) -> GpuVertex {
    let mut out = *vertex;
    out.position = matrix
        .transform_point3(Vec3::from_array(vertex.position))
        .to_array();
    out.normal = matrix
        .transform_vector3(Vec3::from_array(vertex.normal))
        .to_array();
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use oag_core::math::Vec3;

    /// A hinge a couple of units out to one side, tilted so the test cannot
    /// pass by accident on an axis-aligned matrix.
    fn flap(x: f32) -> Flap {
        Flap {
            vertices: 0..1,
            hinge: Mat4::from_translation(Vec3::new(x, -0.5, -6.0)) * Mat4::from_rotation_y(0.3),
        }
    }

    /// A stowed flap must be exactly where the builder put it.
    ///
    /// Exactly, not nearly: the rest position is written back every frame from
    /// the model's own vertices, so any drift here would be visible as a flap
    /// that never quite closes.
    #[test]
    fn a_zero_deflection_moves_nothing() {
        assert_eq!(flap(1.5).deflect(0.0), Mat4::IDENTITY);
    }

    /// The hinge itself is the one point that does not move.
    ///
    /// This is the whole reason `deflect` is `hinge * R * hinge^-1` rather than
    /// a bare rotation: the vertices arrive with every ancestor transform
    /// already baked in, so rotating them directly would swing the flap about
    /// the model's origin - six units away on a real ship - and tear it off the
    /// hull.
    #[test]
    fn the_hinge_point_stays_put() {
        let flap = flap(1.5);
        let pivot = flap.hinge.w_axis.truncate();
        let moved = flap.deflect(0.5).transform_point3(pivot);
        assert!(
            (moved - pivot).length() < 1e-5,
            "the hinge moved to {moved:?} from {pivot:?}"
        );
    }

    /// Mirrored hinges swing the two sides apart, with one angle for both.
    ///
    /// The sides are never negated by hand anywhere: `Race` feeds both flaps
    /// the same positive deflection and the opposition comes entirely from the
    /// locators being mirror images, which
    /// `tests/airbrake_flaps_ground_truth.rs` confirms on all eight teams.
    ///
    /// **The opposition is in `x` and only in `x`**, which is worth pinning
    /// rather than assuming - an earlier version of this test looked for it in
    /// `z` and failed. Conjugating by a reflection mirrors the whole motion, so
    /// two flaps swing *apart* sideways while rising and sweeping identically:
    /// exactly what a symmetric pair of airbrakes does, and not what a naive
    /// "negate the angle on one side" would give.
    #[test]
    fn mirrored_hinges_swing_the_two_sides_apart() {
        // The artists' mirror, as a conjugation, rather than a sign flip on the
        // translation alone: a real locator's rotation is mirrored too.
        let mirror = Mat4::from_scale(Vec3::new(-1.0, 1.0, 1.0));
        let left = flap(1.5);
        let right = Flap {
            vertices: left.vertices.clone(),
            hinge: mirror * left.hinge * mirror,
        };

        let point = Vec3::new(0.0, 0.0, -6.0);
        let moved_left = left.deflect(0.4).transform_point3(point) - point;
        let moved_right = right.deflect(0.4).transform_point3(point) - point;

        assert!(
            moved_left.x.abs() > 1e-3,
            "the fixture does not move sideways at all: {moved_left:?}"
        );
        assert!(
            moved_left.x.signum() != moved_right.x.signum(),
            "both sides swung the same way in x: {moved_left:?} and {moved_right:?}"
        );
        assert!(
            (moved_left.y - moved_right.y).abs() < 1e-5
                && (moved_left.z - moved_right.z).abs() < 1e-5,
            "a mirrored pair must rise and sweep alike: {moved_left:?} and {moved_right:?}"
        );
    }

    fn vertex(position: [f32; 3], normal: [f32; 3]) -> GpuVertex {
        GpuVertex {
            quake: 0.0,
            position,
            normal,
            colour: [1.0; 4],
            texcoord: [0.0; 2],
            lightmap_texcoord: [0.0; 2],
            lit: 1.0,
            anim: 0,
            xform: 0,
            sun_mask: 1.0,
            slots: crate::mesh::slots::DEFAULT,
            specular_exponent: crate::mesh::DEFAULT_SPECULAR_EXPONENT,
            glow: 0.0,
            texcoord2: [0.0; 2],
            tangent: 0,
        }
    }

    /// A flap authored in hinge space is carried through the hinge, normal included, and
    /// remembers the hinge; seating it again moves nothing.
    #[test]
    fn seating_carries_the_flap_through_its_hinge_once() {
        let hinge = Mat4::from_translation(Vec3::new(1.5, -0.5, -6.0)) * Mat4::from_rotation_x(3.0);
        let mut flap = Flap {
            vertices: 1..2,
            hinge: Mat4::IDENTITY,
        };
        let mut vertices = vec![
            vertex([9.0, 9.0, 9.0], [0.0, 1.0, 0.0]),
            vertex([0.0, 0.0, 1.0], [0.0, 1.0, 0.0]),
        ];
        flap.seat(&mut vertices, hinge);
        let want = hinge.transform_point3(Vec3::new(0.0, 0.0, 1.0));
        let got = Vec3::from_array(vertices[1].position);
        assert!((got - want).length() < 1e-5, "{got:?} is not {want:?}");
        let normal = Vec3::from_array(vertices[1].normal);
        assert!(
            normal.y < 0.0,
            "the 3 rad roll turns the normal over: {normal:?}"
        );
        assert_eq!(
            vertices[0].position,
            [9.0, 9.0, 9.0],
            "a vertex outside the span"
        );
        assert_eq!(flap.hinge, hinge);
        let once = vertices[1].position;
        flap.seat(&mut vertices, hinge);
        assert_eq!(
            vertices[1].position, once,
            "a seated flap is not placed twice"
        );
    }
}
