//! Detecting contact with `Reset Collision` geometry.
//!
//! `docs/ghidra/functions/psp-pulse/collision.md` records two facts about the
//! `Reset` class at confidence **86**: it is excluded from ordinary raycasts, and
//! **a contact with it triggers a respawn**. This module is the first half of
//! that - the detection. It deliberately does nothing about the respawn, because
//! respawning needs a track spline and this crate does not know one exists.
//!
//! # What is not recovered, and is therefore not here
//!
//! **Where the ship goes.** The evidence says a respawn happens; nothing found
//! says where to. That decision lives in `crates/game/src/race.rs`, which owns
//! the spline, and is scored there as the low-confidence guess it is. Keeping the
//! split here rather than passing a recovery pose down means this module cannot
//! quietly acquire an invented constant.
//!
//! # Why this does not go through [`Raycaster`]
//!
//! [`Raycaster::raycast`] returns the **nearest** hit across every collider, and
//! `include_reset` only decides whether `Reset` is a *candidate*. A reset volume
//! is almost always below or outside the track, so a probe aimed at one passes
//! through the floor first and the floor wins. Asking through the trait would
//! produce a trigger that essentially never fires.
//!
//! So this walks [`CollisionWorld::colliders`] itself and queries only the
//! `Reset` ones - each [`TriangleSoup`] is a [`Raycaster`] in its own right, so
//! no new query code exists here and the narrowphase is the same one everything
//! else uses. Iteration is over an ordered `Vec`, which is what
//! `docs/architecture/determinism.md` requires of anything feeding simulation
//! state.
//!
//! # The probes are the hull's, not a new set
//!
//! Detection uses [`crate::wall::hull_probes`] plus one swept ray along the
//! frame's displacement, exactly as [`crate::wall`] does. Two probe sets would
//! drift, and the drift would show as a trigger that fires for one side of the
//! hull and not the other.
//!
//! Unlike a wall contact, **no penetration depth is required**: a reset volume is
//! a trigger, not a surface, so touching it at all is the event. Nothing here
//! reads restitution, and `Surface::Reset`'s is the never-bounce sentinel anyway.

use oag_core::math::Vec3;

use crate::collide::{CollisionWorld, Ray, Raycaster, Surface};
use crate::forces::Environment;
use crate::params::Handling;
use crate::ship::ShipState;
use crate::wall::{self, MIN_HULL_EXTENT};

/// A touch of `Reset` geometry.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResetContact {
    /// Where the probe met the triangle, in world space.
    pub point: Vec3,
    /// The triangle's normal, flipped to face the probe it came back along.
    ///
    /// Carried for a debug view rather than for the response: nothing about a
    /// respawn depends on which way the trigger's surface faces.
    pub normal: Vec3,
    /// Index of the reset collider that was touched.
    pub collider: u32,
    /// Set when the swept ray, rather than a hull probe, found it.
    ///
    /// A ship falling off the track crosses a reset volume's top face at speed,
    /// which is precisely the case a probe from the end position cannot see.
    pub swept: bool,
}

/// Whether the ship touched any `Reset` geometry this frame.
///
/// `previous_position` is where the body was before this frame moved it. Passing
/// the position from before force evaluation rather than before integration
/// sweeps slightly further, which for a trigger volume is the safe direction: a
/// missed reset strands a ship, a marginally early one costs nothing.
#[must_use]
pub fn contact(
    state: &ShipState,
    handling: &Handling,
    env: &Environment,
    world: &CollisionWorld,
    previous_position: Vec3,
) -> Option<ResetContact> {
    let body = &state.body;

    // The swept ray first: it is the one that catches a ship dropping through a
    // volume's face between two frames.
    let displacement = body.position - previous_position;
    let travelled = displacement.length();
    let moved = travelled > MIN_HULL_EXTENT;
    if moved {
        let direction = displacement / travelled;
        if let Some(found) = nearest_reset(
            world,
            Ray::new(previous_position, direction, travelled),
            env.self_collider,
            direction,
        ) {
            return Some(ResetContact {
                swept: true,
                ..found
            });
        }
    }

    for (origin, direction, reach) in wall::hull_probes(body, handling) {
        let probeable = reach > MIN_HULL_EXTENT;
        if !probeable {
            continue;
        }
        if let Some(found) = nearest_reset(
            world,
            Ray::new(origin, direction, reach),
            env.self_collider,
            direction,
        ) {
            return Some(found);
        }
    }

    None
}

/// The nearest `Reset` hit along one ray, ignoring every other class.
fn nearest_reset(
    world: &CollisionWorld,
    ray: Ray,
    skip: Option<u32>,
    direction: Vec3,
) -> Option<ResetContact> {
    let mut nearest: Option<(f32, ResetContact)> = None;

    for collider in world.colliders() {
        if collider.surface() != Surface::Reset {
            continue;
        }
        // `include_reset` is set because the collider would otherwise skip
        // itself; this is the "explicitly requested" case the evidence names.
        let Some(hit) = collider.raycast(ray, skip, true) else {
            continue;
        };
        let nearer = nearest.is_none_or(|(best, _)| hit.distance < best);
        if nearer {
            nearest = Some((
                hit.distance,
                ResetContact {
                    point: hit.point,
                    normal: wall::facing(hit.normal, direction),
                    collider: hit.collider,
                    swept: false,
                },
            ));
        }
    }

    nearest.map(|(_, contact)| contact)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collide::TriangleSoup;
    use crate::params::Dimensions;
    use crate::ship::Body;

    fn handling() -> Handling {
        Handling {
            dimensions: Dimensions {
                width: 2.0,
                height: 1.0,
                length: 4.0,
                ..Handling::ZERO.dimensions
            },
            ..Handling::ZERO
        }
    }

    /// A large quad in the plane `x = at`.
    fn quad(at: f32, surface: Surface, collider: u32) -> TriangleSoup {
        TriangleSoup::new(
            vec![
                [at, -100.0, -100.0],
                [at, 100.0, -100.0],
                [at, 100.0, 100.0],
                [at, -100.0, 100.0],
            ],
            vec![[0, 1, 2], [0, 2, 3]],
            Vec::new(),
            surface,
            collider,
        )
    }

    fn ship_at(x: f32) -> ShipState {
        ShipState {
            body: Body {
                position: Vec3::new(x, 0.0, 0.0),
                mass: 1.0,
                ..Body::default()
            },
            ..ShipState::default()
        }
    }

    fn world(colliders: Vec<TriangleSoup>) -> CollisionWorld {
        let mut world = CollisionWorld::new();
        for c in colliders {
            world.push(c);
        }
        world
    }

    #[test]
    fn a_hull_overlapping_a_reset_volume_reports_a_contact() {
        let state = ship_at(1.0);
        let found = contact(
            &state,
            &handling(),
            &Environment::default(),
            &world(vec![quad(1.6, Surface::Reset, 0)]),
            state.body.position,
        )
        .expect("a reset contact");
        assert_eq!(found.collider, 0);
        assert!(!found.swept);
    }

    /// The masking case, and the whole reason this does not go through
    /// `Raycaster`: a reset volume sits behind a floor, which every ordinary
    /// nearest-hit query would return instead. Delete the per-collider filter and
    /// this fails.
    #[test]
    fn a_reset_volume_behind_a_floor_is_still_found() {
        let state = ship_at(1.0);
        let found = contact(
            &state,
            &handling(),
            &Environment::default(),
            &world(vec![
                // Nearer, and not a reset: exactly what would win a nearest-hit.
                quad(1.4, Surface::Floor, 0),
                quad(1.6, Surface::Reset, 1),
            ]),
            state.body.position,
        )
        .expect("the reset behind the floor");
        assert_eq!(found.collider, 1);
    }

    /// A ship dropping through a volume's face between two frames must still
    /// trigger. Without the swept ray it lands clear on the far side and no hull
    /// probe reaches back to a zero-thickness shell.
    #[test]
    fn a_reset_volume_crossed_within_one_frame_is_caught_by_the_sweep() {
        let mut state = ship_at(20.0);
        state.body.linear_velocity = Vec3::new(200.0, 0.0, 0.0);
        let found = contact(
            &state,
            &handling(),
            &Environment::default(),
            &world(vec![quad(5.0, Surface::Reset, 3)]),
            Vec3::new(-5.0, 0.0, 0.0),
        )
        .expect("the swept reset contact");
        assert!(found.swept);
        assert_eq!(found.collider, 3);
    }

    /// Nothing but reset geometry may trigger a respawn, or a race ends the
    /// moment a ship touches the ground.
    #[test]
    fn no_other_surface_class_triggers_a_reset() {
        for surface in [Surface::Wall, Surface::Floor, Surface::MagFloor] {
            let state = ship_at(1.0);
            assert_eq!(
                contact(
                    &state,
                    &handling(),
                    &Environment::default(),
                    &world(vec![quad(1.6, surface, 0)]),
                    state.body.position,
                ),
                None,
                "{surface:?} triggered a reset"
            );
        }
    }

    #[test]
    fn open_space_triggers_nothing() {
        let state = ship_at(0.0);
        assert_eq!(
            contact(
                &state,
                &handling(),
                &Environment::default(),
                &CollisionWorld::new(),
                state.body.position,
            ),
            None
        );
    }

    /// A reset volume out of the hull's reach is not a contact: the trigger is
    /// touching it, not being near it.
    #[test]
    fn a_reset_volume_out_of_reach_is_not_a_contact() {
        let state = ship_at(0.0);
        assert_eq!(
            contact(
                &state,
                &handling(),
                &Environment::default(),
                &world(vec![quad(50.0, Surface::Reset, 0)]),
                state.body.position,
            ),
            None
        );
    }

    /// A zero hull cannot probe, and a ship that has not moved has nothing to
    /// sweep, so a degenerate parameter set is a quiet no-op rather than a panic.
    #[test]
    fn a_degenerate_hull_that_has_not_moved_finds_nothing() {
        let state = ship_at(1.0);
        assert_eq!(
            contact(
                &state,
                &Handling::ZERO,
                &Environment::default(),
                &world(vec![quad(1.6, Surface::Reset, 0)]),
                state.body.position,
            ),
            None
        );
    }
}
