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

/// Right-handed camera matrices with a 0..1 depth range.
///
/// That range is what wgpu, Metal and DX12 expect; the OpenGL -1..1 convention
/// would put everything at the wrong depth. Re-exported here so the renderer
/// takes its math from one place, like everything else.
pub mod camera {
    pub use glam::camera::rh::proj::directx::{perspective, perspective_infinite_reverse};
    pub use glam::camera::rh::view::look_at_mat4 as look_at;
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
    /// Exact for all inputs: an `i32` has at most 32 significant bits and the
    /// division is by a power of two, so this is a single correctly-rounded
    /// operation.
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

    #[test]
    fn binary_angle_covers_a_full_turn() {
        assert_eq!(binary_angle_to_radians(0), 0.0);
        assert!((binary_angle_to_radians(16384) - core::f32::consts::FRAC_PI_2).abs() < 1e-6);
        assert!((binary_angle_to_radians(32768) - core::f32::consts::PI).abs() < 1e-6);
    }
}
