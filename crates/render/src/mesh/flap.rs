//! One authored airbrake flap.
//!
//! Split out of `mesh.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change.

use oag_core::math::{Mat4, Vec3};

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
        out.extend(base.iter().map(|v| {
            let mut out = *v;
            out.position = swing
                .transform_point3(Vec3::from_array(v.position))
                .to_array();
            out.normal = swing
                .transform_vector3(Vec3::from_array(v.normal))
                .to_array();
            out
        }));
        Some(span)
    }
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
}
