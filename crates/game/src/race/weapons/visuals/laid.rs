//! Where a laid Mine or Bomb is drawn, and how it is turned.
//!
//! **Pulse's two poses are the executable's own, measured live on PPSSPP on
//! 2026-09-24** - see `docs/ghidra/functions/psp-pulse-usa/mine.md`, "a laid
//! mine spins, and the Bomb sits square to the world":
//!
//! - **The Mine** is re-posed every tick by `Mine_PoseNode` (`0x08859ce4`) as
//!   `0.6 x Rot(axis, -4 x fuse)` at its own position: a uniform scale of
//!   [`PULSE_MINE_SCALE`] (a `Mine_Construct` code literal) and a turn of
//!   [`PULSE_MINE_SPIN_RATE`] radians per second of fuse about a random unit
//!   axis `Mine_Init` rolled for that mine. The craft's pose `Mine_Init` also
//!   copies is overwritten before it is ever drawn. Confidence 92.
//! - **The Bomb** is posed once by `Bomb_Init` (`0x08863188`): up is the
//!   craft's own at the drop, and the yaw is world `+Z` orthogonalised against
//!   it - [`bomb_blast::bomb_blast_basis`]'s shape, at scale 1. Confidence 88.
//!
//! **The Mine's axis is ours.** The original rolls it from its own PRNG; this
//! draws the same shape (a normalised `U(-1, 1)^3`) from a hash of the mine's
//! slot and its fixed position, so the simulation's seeded stream never
//! advances for a picture - the same reason
//! `oag_render::weapon_quads::random` exists. Chosen, not measured.
//!
//! **Every other title keeps the frozen craft pose**, which is chosen, not
//! measured: HD's own laid-charge poses live in a different binary and were
//! not read. See `oag_gameplay::projectile::mine::frozen_pose`.

use super::super::*;

/// `Mine_Construct`'s `entity+0xd0`, the code literal `0x3f19999a` written at
/// `0x08859a30`: every laid mine is drawn at three-fifths of its authored
/// size. Measured: every row of the live node matrix has length `0.600`.
pub(in crate::race) const PULSE_MINE_SCALE: f32 = 0.6;

/// `Mine_PoseNode`'s `fuse * 4.0`: radians of turn per second of fuse. The
/// angle the matrix is built from is `-PULSE_MINE_SPIN_RATE * fuse` - the
/// negative sign is the one that fits the live matrices to every element;
/// the positive one misses by 0.4 to 1.1.
pub(in crate::race) const PULSE_MINE_SPIN_RATE: f32 = 4.0;

/// One laid charge's model matrix. `pulse` selects Pulse's own measured poses
/// over the frozen craft pose - see this module's doc comment.
pub(super) fn matrix(slot: usize, projectile: &oag_gameplay::Projectile, pulse: bool) -> Mat4 {
    use oag_tables::weapons::Weapon;
    match (pulse, projectile.kind) {
        (true, Some(Weapon::Mine)) => pulse_mine(slot, projectile.position, projectile.lifetime),
        (true, Some(Weapon::Bomb)) => bomb_blast::bomb_blast_basis(
            projectile.position,
            projectile.orientation * Vec3::Y,
        ),
        _ => Mat4::from_rotation_translation(projectile.orientation, projectile.position),
    }
}

/// `Mine_PoseNode`'s matrix for a mine with `fuse` seconds left.
///
/// A fuse that is not finite (a charge with none) holds the pose at angle
/// zero rather than spinning on a meaningless number.
pub(in crate::race) fn pulse_mine(slot: usize, position: Vec3, fuse: f32) -> Mat4 {
    let angle = if fuse.is_finite() {
        -PULSE_MINE_SPIN_RATE * fuse
    } else {
        0.0
    };
    Mat4::from_scale_rotation_translation(
        Vec3::splat(PULSE_MINE_SCALE),
        Quat::from_axis_angle(spin_axis(slot, position), angle),
        position,
    )
}

/// This mine's spin axis: a normalised `U(-1, 1)^3`, the shape `Mine_Init`
/// rolls, from a hash of the slot and the mine's fixed position rather than
/// from the simulation's own stream. Stable for the mine's whole life, since
/// a laid mine never moves.
pub(in crate::race) fn spin_axis(slot: usize, position: Vec3) -> Vec3 {
    let seed = (slot as u32).wrapping_mul(0x9e37_79b1)
        ^ position.x.to_bits().wrapping_mul(0x85eb_ca6b)
        ^ position.y.to_bits().wrapping_mul(0xc2b2_ae35)
        ^ position.z.to_bits().wrapping_mul(0x27d4_eb2f);
    let axis = Vec3::new(signed_unit(seed), signed_unit(seed ^ 1), signed_unit(seed ^ 2));
    axis.try_normalize().unwrap_or(Vec3::Y)
}

/// A cheap integer hash (xorshift-multiply, "lowbias32") remapped to
/// `[-1, 1)`.
fn signed_unit(mut x: u32) -> f32 {
    x ^= x >> 16;
    x = x.wrapping_mul(0x7feb_352d);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846c_a68b);
    x ^= x >> 16;
    (x >> 8) as f32 / (1u32 << 23) as f32 - 1.0
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every row of the live `Mine_PoseNode` matrix has length `0.6`, and the
    /// matrix is `0.6 x` the right-handed turn by `-4 x fuse` - one live
    /// sample's axis, fuse and rows (`mine.md`'s 2026-09-24 section), checked
    /// element by element through the same construction the draw uses.
    #[test]
    fn a_pulse_mine_reproduces_a_live_node_matrix() {
        let axis = Vec3::new(-0.034, 0.259, 0.965).normalize();
        let fuse = 7.0;
        let matrix = Mat4::from_scale_rotation_translation(
            Vec3::splat(PULSE_MINE_SCALE),
            Quat::from_axis_angle(axis, -PULSE_MINE_SPIN_RATE * fuse),
            Vec3::ZERO,
        );
        for column in [matrix.x_axis, matrix.y_axis, matrix.z_axis] {
            assert!((column.truncate().length() - 0.6).abs() < 1e-5);
        }
        // Rodrigues by hand, for `-4 x fuse`: the original's own row 0 is
        // `(t x^2 + c, t x y - s z, t x z + s y)` at angle `4 x fuse`, which
        // is column 0 of the right-handed turn by `-4 x fuse`.
        let theta = PULSE_MINE_SPIN_RATE * fuse;
        let (s, c) = (theta.sin(), theta.cos());
        let t = 1.0 - c;
        let row0 = Vec3::new(
            t * axis.x * axis.x + c,
            t * axis.x * axis.y - s * axis.z,
            t * axis.x * axis.z + s * axis.y,
        ) * 0.6;
        assert!(matrix.x_axis.truncate().abs_diff_eq(row0, 1e-4));
        assert!(matrix.determinant() > 0.0);
    }

    /// The axis is a unit vector, stable for one mine, and differs between
    /// two mines of one cluster.
    #[test]
    fn a_mines_spin_axis_is_stable_and_differs_per_mine() {
        let here = Vec3::new(-24.43, -49.83, -181.11);
        let a = spin_axis(3, here);
        assert!((a.length() - 1.0).abs() < 1e-5);
        assert_eq!(a, spin_axis(3, here));
        assert_ne!(a, spin_axis(4, here + Vec3::new(10.0, 0.0, 0.0)));
    }

    /// The fuse turns the mine: one second of fuse is four radians, and a
    /// charge with no fuse holds still rather than drawing a NaN.
    #[test]
    fn a_pulse_mine_turns_with_its_fuse_and_holds_without_one() {
        let here = Vec3::new(1.0, 2.0, 3.0);
        let now = pulse_mine(0, here, 7.0);
        let later = pulse_mine(0, here, 6.0);
        assert_ne!(now, later);
        assert!(pulse_mine(0, here, f32::INFINITY).is_finite());
        assert_eq!(now.w_axis.truncate(), here);
    }
}
