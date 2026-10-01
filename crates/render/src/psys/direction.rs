//! The spawn direction laws: one unit vector per new particle, in an emitter frame whose
//! authored `+Y` maps to world-space `up`. Split out of `psys.rs`, which is baselined.

use oag_core::Rng;
use oag_core::math::Vec3;

use super::Direction;

/// One spawn direction for a [`Direction`] law, in an emitter frame whose
/// authored `+Y` maps to world-space `up`.
///
/// `up` is `Vec3::Y` for every caller but the collision sparks - see
/// [`System::advance`]'s own doc comment - in which case every branch below
/// reduces to exactly the world-axis arithmetic this function used before
/// `up` existed: [`horizontal_basis`] returns `(Vec3::X, Vec3::Z)` for that
/// input bit-for-bit, and reflecting a hemisphere sample across `Vec3::Y` is
/// the same float operations `d.y = d.y.abs()` was.
pub(super) fn direction_for(direction: Direction, up: Vec3, rng: &mut Rng) -> Vec3 {
    match direction {
        Direction::Radial { hemisphere } | Direction::Tangent { hemisphere } => {
            let mut d = sphere_direction(rng);
            if hemisphere {
                // `ParticleSystem_EmitSphere`'s shape-7 branch: `abs()` on
                // the emitter-local up - i.e. reflect the sample across the
                // plane the `up` axis is normal to, whenever it landed on
                // the wrong side.
                let along = d.dot(up);
                if along < 0.0 {
                    d -= up * (2.0 * along);
                }
            }
            if matches!(direction, Direction::Tangent { .. }) {
                // Mode 2 builds a random tangent to the spawn direction.
                let other = sphere_direction(rng);
                d.cross(other).try_normalize().unwrap_or(d)
            } else {
                d
            }
        }
        Direction::Aimed {
            elevation,
            azimuth,
            jitter,
        } => {
            // `ParticleSystem_AimedVelocity`: `y = sin(elevation ± jitter)`,
            // horizontal components scaled by the matching cosine, heading
            // taken from the spawn direction's - uniform here - rotated by
            // the authored azimuth, which uniform absorbs. `y` here is the
            // component along `up`, not necessarily world `Y`.
            let elev = elevation + jitter * signed_unit(rng);
            let heading = rng.next_f32() * std::f32::consts::TAU + azimuth;
            let (sin_e, cos_e) = elev.sin_cos();
            let (sin_a, cos_a) = heading.sin_cos();
            let (right, forward) = horizontal_basis(up);
            right * (cos_a * cos_e) + up * sin_e + forward * (sin_a * cos_e)
        }
        Direction::Cone { half_angle } => {
            // `ParticleSystem_ConeVelocity` draws `U(-a, a)` off the
            // emitter's axis and takes its sin/cos; the axis is the
            // emitter's up, uniform in azimuth about it.
            let tilt = half_angle * signed_unit(rng);
            let heading = rng.next_f32() * std::f32::consts::TAU;
            let (sin_t, cos_t) = tilt.sin_cos();
            let (sin_a, cos_a) = heading.sin_cos();
            let (right, forward) = horizontal_basis(up);
            right * (sin_t * cos_a) + up * cos_t + forward * (sin_t * sin_a)
        }
    }
}

/// Two vectors that, with `up`, form a right-handed orthonormal basis - the
/// horizontal reference [`Direction::Aimed`] and [`Direction::Cone`] spread
/// their uniformly-sampled azimuth around.
///
/// Which particular horizontal directions these are does not matter to
/// either caller: azimuth is drawn from a full `U(0, tau)` in both, so the
/// distribution this basis is built from is invariant to which perpendicular
/// pair is picked. What matters is that `up == Vec3::Y` reduces to exactly
/// `(Vec3::X, Vec3::Z)` - the world axes [`direction_for`] used before a
/// caller could supply anything else - so every effect that still passes
/// `Vec3::Y` (everything but the collision sparks) samples bit-identically
/// to before.
pub(super) fn horizontal_basis(up: Vec3) -> (Vec3, Vec3) {
    // A second axis to cross against, picked away from `up` so the cross
    // product never degenerates: `Vec3::Z` for every `up` that is not itself
    // close to `Vec3::Z`, `Vec3::Y` there instead. World up (`Vec3::Y`) hits
    // the ordinary `Z` branch, which is what keeps that case exact - see the
    // module doc above.
    let helper = if up.z.abs() > 0.9 { Vec3::Y } else { Vec3::Z };
    let right = up.cross(helper).try_normalize().unwrap_or(Vec3::X);
    let forward = right.cross(up);
    (right, forward)
}

/// `U(-1, 1)`.
pub(super) fn signed_unit(rng: &mut Rng) -> f32 {
    rng.next_f32() * 2.0 - 1.0
}

/// Uniform over the unit sphere, by the original's own construction
/// (`ParticleSystem_EmitSphere`): `z` uniform in `[-1, 1]`, azimuth uniform,
/// `sqrt(1 - z^2)` radius in the plane.
pub(super) fn sphere_direction(rng: &mut Rng) -> Vec3 {
    let z = signed_unit(rng);
    let phi = rng.next_f32() * std::f32::consts::TAU;
    let r = (1.0 - z * z).max(0.0).sqrt();
    let (sin_p, cos_p) = phi.sin_cos();
    Vec3::new(r * cos_p, r * sin_p, z)
}
