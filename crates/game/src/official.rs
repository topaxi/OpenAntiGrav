//! Whether a race may leave a record or a ghost behind.
//!
//! A race run with a chosen world seed, a weapon handed over from outside the simulation or a
//! craft placed by hand is a verification run, not a player's result. Every site that persists a
//! race's result asks [`unofficial`] and nothing else.

use oag_raceplay::{Options, PoseRequest};
use oag_tables::weapons::Weapon;

/// Whether the race `options` describe, flown with `give` as its handed-over weapon, is a
/// verification run: it saves no record, no medal, no loyalty and no ghost.
///
/// `options.seed` is the CLI's `--seed` here (the front end draws its fresh seed on a copy, in
/// `Session::launch_race`).
#[must_use]
pub fn unofficial(options: &Options, give: Option<Weapon>) -> bool {
    named(options.seed, options.pose.as_ref(), give)
}

fn named(seed: Option<u64>, pose: Option<&PoseRequest>, give: Option<Weapon>) -> bool {
    seed.is_some() || pose.is_some() || give.is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use oag_core::math::Vec3;

    #[test]
    fn a_plain_race_is_official() {
        assert!(!named(None, None, None));
    }

    #[test]
    fn a_seed_a_give_or_a_pose_make_it_unofficial() {
        assert!(named(Some(1), None, None));
        assert!(named(None, None, Some(Weapon::Rocket)));
        let pose = PoseRequest::SplineAligned {
            position: Vec3::new(0.0, 0.0, 0.0),
            yaw: 0.0,
        };
        assert!(named(None, Some(&pose), None));
    }
}
