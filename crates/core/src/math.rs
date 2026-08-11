//! Canonical math types for the simulation.
//!
//! Everything is re-exported from `glam` built with `scalar-math`, so no SIMD
//! path is ever taken. SIMD implementations are free to reassociate operations
//! and to differ between targets and CPU feature levels; scalar code is not.
//!
//! Simulation code must use these re-exports rather than depending on `glam`
//! directly, so the choice of backing library stays a single decision in one
//! place.
//!
//! # Rules for simulation math
//!
//! - No `f32::mul_add` / `f64::mul_add`. Fused multiply-add keeps a wider
//!   intermediate, so `a.mul_add(b, c)` and `a * b + c` do not always agree.
//!   Whichever one the original used, we must pick deliberately, not let the
//!   optimiser or the target ISA pick for us.
//! - No fast-math. Rust does not enable it, but do not add it via build flags.
//! - No `HashMap` iteration feeding arithmetic. Iteration order is not stable.
//! - No wall-clock time or OS entropy. Use [`crate::tick`] and [`crate::rng`].

pub use glam::{Mat3, Mat4, Quat, Vec2, Vec3, Vec4, mat3, mat4, quat, vec2, vec3, vec4};

/// How many km/h one world unit per second is.
///
/// **Recovered.** The original multiplies by this before every speed test - the
/// exhaust's ramp and the HUD's readout both go through it - and divides by it
/// to turn an authored weapon speed into a velocity
/// (`docs/ghidra/functions/psp-pulse-usa/rocket-visuals.md`). One physical
/// conversion, used in both directions.
///
/// **It lives here because it has two consumers on opposite sides of a
/// dependency rule.** `oag_render::exhaust` needs it to display a speed and
/// `oag_gameplay::projectile` needs it to spend an authored one, and no
/// gameplay crate may depend on `oag-render`
/// (`scripts/check-dependency-rules.py`). `oag-core` is the crate both already
/// depend on, so it is the only place a single definition can sit. It was
/// duplicated for exactly one commit before this; a second copy of a constant
/// is a second thing to get wrong.
pub const SPEED_TO_KMH: f32 = 3.6;

/// Right-handed camera matrices with a 0..1 depth range.
///
/// That range is what wgpu, Metal and DX12 expect; the OpenGL -1..1 convention
/// would put everything at the wrong depth. Re-exported here so the renderer
/// takes its math from one place, like everything else.
pub mod camera {
    pub use glam::camera::rh::proj::directx::{perspective, perspective_infinite_reverse};
    pub use glam::camera::rh::view::look_at_mat4 as look_at;
}

/// The camera's view volume, for deciding what is worth submitting to the GPU
/// at all.
///
/// Pure and GPU-free on purpose - the same reasoning [`camera`] and
/// `oag_render::mesh_render::matrices` already follow: this is arithmetic a
/// test can check directly, not something only a screenshot can verify.
pub mod frustum {
    use super::{Mat4, Vec3, Vec4};

    /// The six half-spaces of a view-projection matrix's clip volume, each as
    /// `(normal, distance)` such that a world-space point `p` is inside this
    /// plane when `normal.dot(p) + distance >= 0`.
    ///
    /// Extracted with the standard Gribb-Hartmann method: each plane is a row
    /// of the combined view-projection matrix, added to or subtracted from the
    /// `w` row depending on which clip-space bound it represents. The near and
    /// far planes use the `z` row alone rather than `w +/- z`, because this
    /// project's projections put clip-space `z` in `0..w` (a right-handed,
    /// 0..1 depth range - see [`camera`]), not OpenGL's `-w..w`.
    #[derive(Debug, Clone, Copy)]
    pub struct Frustum {
        planes: [Vec4; 6],
    }

    impl Frustum {
        /// Builds a frustum from a combined view-projection matrix.
        #[must_use]
        pub fn from_view_projection(view_projection: Mat4) -> Self {
            // `Mat4` is column-major, so row `i` of the matrix that transforms
            // a column vector is built from element `i` of each column.
            let row = |i: usize| {
                Vec4::new(
                    view_projection.x_axis[i],
                    view_projection.y_axis[i],
                    view_projection.z_axis[i],
                    view_projection.w_axis[i],
                )
            };
            let (x, y, z, w) = (row(0), row(1), row(2), row(3));
            Self {
                planes: [
                    w + x, // left:   clip.x >= -clip.w
                    w - x, // right:  clip.x <=  clip.w
                    w + y, // bottom: clip.y >= -clip.w
                    w - y, // top:    clip.y <=  clip.w
                    z,     // near:   clip.z >= 0
                    w - z, // far:    clip.z <= clip.w
                ],
            }
        }

        /// Whether a world-space sphere touches or is inside this frustum.
        ///
        /// Conservative: a sphere is a looser bound than the mesh it stands in
        /// for, so this can return `true` for a mesh that is not actually
        /// visible (never wrongly culls one that is). Each plane is tested with
        /// its own un-normalised magnitude divided out, so the comparison is a
        /// true world-space distance regardless of the matrix's own scale.
        #[must_use]
        pub fn intersects_sphere(&self, centre: Vec3, radius: f32) -> bool {
            self.planes.iter().all(|p| {
                let normal = Vec3::new(p.x, p.y, p.z);
                let length = normal.length();
                if length <= 0.0 {
                    // A degenerate plane cannot exclude anything.
                    return true;
                }
                (normal.dot(centre) + p.w) / length >= -radius
            })
        }
    }
}

/// Fixed-point helpers.
///
/// The original PSP and PS2 builds are expected to use fixed-point in at least
/// some subsystems. Which ones is unknown until the relevant code is read in
/// Ghidra, so nothing here is wired into the simulation yet. These exist so
/// that when a fixed-point format is recovered it has an obvious home.
pub mod fixed {
    /// Converts a 16.16 fixed-point value to `f32`.
    ///
    /// **Exact only while `|v| < 2^24`.** An `f32` carries 24 significant bits
    /// and an `i32` up to 32, so the `as f32` cast rounds first; dividing by a
    /// power of two afterwards is exact and cannot recover what the cast lost.
    /// In 16.16 terms that means values above 256.0 lose low fractional bits,
    /// gradually: at 4096.0 the resolution is 2^-12 rather than 2^-16.
    ///
    /// This matters if a recovered format turns out to carry large coordinates
    /// in 16.16, because the loss is silent. Convert through `f64`, or keep the
    /// value in fixed point, if that day comes.
    #[must_use]
    pub fn from_16_16(v: i32) -> f32 {
        v as f32 / 65536.0
    }

    /// Converts an `f32` to 16.16 fixed-point, truncating toward zero.
    ///
    /// Saturates rather than wrapping on overflow. `as` casts in Rust already
    /// saturate, which is the behaviour we want, but relying on that silently
    /// would hide the decision.
    #[must_use]
    pub fn to_16_16(v: f32) -> i32 {
        (v * 65536.0) as i32
    }

    /// Converts a 12.4 fixed-point value to `f32`.
    #[must_use]
    pub fn from_12_4(v: i16) -> f32 {
        f32::from(v) / 16.0
    }
}

/// Converts a PSP-style 16-bit binary angle (65536 units per turn) to radians.
///
/// Binary angles are common in fixed-point engines because wrapping is free.
/// Whether Pulse uses this convention is unconfirmed; see
/// `docs/formats/README.md`.
#[must_use]
pub fn binary_angle_to_radians(units: u16) -> f32 {
    f32::from(units) * (core::f32::consts::TAU / 65536.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_16_16_round_trips_exact_values() {
        for raw in [0i32, 1, -1, 65536, -65536, 0x1234_5678, -0x1234_5678] {
            let f = fixed::from_16_16(raw);
            // 16.16 values with more than 24 significant bits are not exactly
            // representable in f32, so only check the ones that are.
            if raw.unsigned_abs() < (1 << 24) {
                assert_eq!(fixed::to_16_16(f), raw, "raw = {raw}");
            }
        }
    }

    /// Pins the limit the conversion's own documentation claims, since it is a
    /// silent loss rather than an error.
    #[test]
    fn fixed_16_16_loses_low_bits_above_the_mantissa() {
        // The largest exactly representable raw value, and the first that is
        // not: 2^24 + 1 rounds down to 2^24.
        let exact = 1i32 << 24;
        assert_eq!(fixed::to_16_16(fixed::from_16_16(exact)), exact);
        assert_eq!(fixed::to_16_16(fixed::from_16_16(exact + 1)), exact);

        // Which in 16.16 means the resolution at 256.0 is already 2^-16 * 2.
        assert_eq!(fixed::from_16_16(exact), 256.0);
        assert_eq!(fixed::from_16_16(exact + 1), 256.0);
    }

    #[test]
    fn binary_angle_covers_a_full_turn() {
        assert_eq!(binary_angle_to_radians(0), 0.0);
        assert!((binary_angle_to_radians(16384) - core::f32::consts::FRAC_PI_2).abs() < 1e-6);
        assert!((binary_angle_to_radians(32768) - core::f32::consts::PI).abs() < 1e-6);
    }

    mod frustum_tests {
        use super::*;
        use crate::math::frustum::Frustum;

        /// A camera at the origin looking down -z, 90-degree vertical field of
        /// view, matching the aspect so the horizontal field is 90 degrees too -
        /// chosen so the maths below are checkable by hand rather than only by
        /// trusting the library.
        fn test_frustum() -> Frustum {
            let projection = camera::perspective(core::f32::consts::FRAC_PI_2, 1.0, 1.0, 100.0);
            let view = camera::look_at(Vec3::ZERO, Vec3::NEG_Z, Vec3::Y);
            Frustum::from_view_projection(projection * view)
        }

        #[test]
        fn a_point_straight_ahead_is_inside() {
            let frustum = test_frustum();
            assert!(frustum.intersects_sphere(Vec3::new(0.0, 0.0, -10.0), 0.0));
        }

        #[test]
        fn a_point_behind_the_camera_is_outside() {
            let frustum = test_frustum();
            assert!(!frustum.intersects_sphere(Vec3::new(0.0, 0.0, 10.0), 0.0));
        }

        #[test]
        fn a_point_nearer_than_the_near_plane_is_outside() {
            let frustum = test_frustum();
            assert!(!frustum.intersects_sphere(Vec3::new(0.0, 0.0, -0.5), 0.0));
        }

        #[test]
        fn a_point_past_the_far_plane_is_outside() {
            let frustum = test_frustum();
            assert!(!frustum.intersects_sphere(Vec3::new(0.0, 0.0, -200.0), 0.0));
        }

        /// A 90-degree field of view means the side plane sits at 45 degrees, so
        /// a point twice as far to the side as it is deep is exactly on the
        /// boundary - just past it is outside, with zero radius to forgive it.
        #[test]
        fn a_point_outside_the_horizontal_field_of_view_is_outside() {
            let frustum = test_frustum();
            assert!(!frustum.intersects_sphere(Vec3::new(11.0, 0.0, -10.0), 0.0));
        }

        /// The same out-of-view point, forgiven by a sphere radius large enough
        /// to reach back inside the plane - proof the radius is read in real
        /// world units, not a raw, un-normalised clip-space quantity.
        #[test]
        fn a_large_enough_radius_brings_a_point_back_inside() {
            let frustum = test_frustum();
            assert!(frustum.intersects_sphere(Vec3::new(11.0, 0.0, -10.0), 2.0));
        }
    }
}
