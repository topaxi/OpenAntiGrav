//! A miniature simulation used solely to prove determinism.
//!
//! This is not gameplay and never will be. It exists so the cross-platform
//! determinism gate has something to bite on before the real simulation
//! exists, exercising the operations the real one will lean on hardest:
//! accumulated float arithmetic, square roots, transcendentals, quaternion
//! composition and normalisation, and seeded random draws.
//!
//! If [`run`] produces a different hash on two platforms, the shared
//! foundation is not deterministic and no amount of care in the gameplay
//! crates will save the replay system.

use crate::hash::StateHasher;
use crate::math::{self, Quat, Vec3};
use crate::rng::Rng;
use crate::tick::TickRate;

/// Result of a probe run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProbeResult {
    /// Hash of the final state only.
    pub final_hash: u64,
    /// Hash of the state at every tick, folded together.
    ///
    /// Catches divergence that happens mid-run and then cancels out, which a
    /// final-state hash alone would miss.
    pub trajectory_hash: u64,
}

/// Runs the probe for `ticks` steps from `seed`.
#[must_use]
pub fn run(ticks: u32, seed: u64) -> ProbeResult {
    let dt = TickRate::DEFAULT.dt();
    let mut rng = Rng::new(seed);

    let mut position = Vec3::new(0.0, 1.0, 0.0);
    let mut velocity = Vec3::new(1.0, 0.0, 0.5);
    let mut orientation = Quat::IDENTITY;

    let mut trajectory = StateHasher::new();

    for tick in 0..ticks {
        // A jittered force, so the trajectory depends on the PRNG as well as
        // on the float pipeline.
        let jitter = Vec3::new(
            rng.next_f32() - 0.5,
            rng.next_f32() - 0.5,
            rng.next_f32() - 0.5,
        );

        // Gravity, drag and jitter. Written as separate terms rather than one
        // fused expression to keep the operation order obvious and stable.
        let gravity = Vec3::new(0.0, -9.81, 0.0);
        let drag = velocity * -0.02;
        let acceleration = gravity + drag + jitter;

        velocity += acceleration * dt;
        position += velocity * dt;

        // Bounce, which introduces a data-dependent branch. Branches are fine
        // for determinism as long as the condition itself is deterministic.
        if position.y < 0.0 {
            position.y = -position.y;
            velocity.y = -velocity.y * 0.8;
        }

        // sqrt is IEEE-754 correctly rounded, so normalize is safe. sin/cos are
        // not, so they come from `oag_core::math` (the `libm` crate, the same
        // code on every target) exactly as simulation code must take them: the
        // platform's own `sin` differed on macOS and Windows from the glibc one
        // the reference was first recorded with.
        let angle = (tick as f32) * 0.01;
        let spin = math::quat_from_axis_angle(Vec3::Y, math::sin_cos(angle).0 * 0.1);
        orientation = (orientation * spin).normalize();

        trajectory.write_vec3(position);
        trajectory.write_vec3(velocity);
        trajectory.write_quat(orientation);
    }

    let mut final_state = StateHasher::new();
    final_state.write_vec3(position);
    final_state.write_vec3(velocity);
    final_state.write_quat(orientation);
    final_state.write_u32(rng.next_u32());

    ProbeResult {
        final_hash: final_state.finish(),
        trajectory_hash: trajectory.finish(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_reproducible_within_a_single_process() {
        assert_eq!(run(1000, 42), run(1000, 42));
    }

    #[test]
    fn depends_on_the_seed() {
        assert_ne!(run(1000, 42), run(1000, 43));
    }

    #[test]
    fn depends_on_the_tick_count() {
        assert_ne!(run(1000, 42), run(1001, 42));
    }
}
