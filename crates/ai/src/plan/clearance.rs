//! Where the plan passes a wall closely.
//!
//! A plan is learned from contacts, so it accepts a corner the neutral driver clears
//! by a hair, and a driver that is not the neutral one then finds that wall. HD's
//! Talon's Junction, line samples 3186-3190: the plan carries full throttle through
//! at 114-118 units/s with under a unit to spare, and an Ace spending 7 % of its line
//! bias on the plan sat 0.3-0.6 units wider and touched every lap
//! (`docs/gameplay/ai.md`, "Tight corners"). The verification run marks such samples
//! (`SpeedPlan::tight_within`) and a driver holds the plan's own line into them
//! (`driver::planned::plan_slack`); the plan's speeds do not change.
//!
//! The question is the physics' own: the ten hull probes
//! (`oag_physics::wall::hull_probes`), each lengthened by [`MARGIN`], and a hit counts
//! only where a contact would react (`oag_physics::wall::reacts`), which leaves out
//! floors.

use oag_physics::collide::{Ray, Raycaster, combine_friction};
use oag_physics::wall::{HULL_PROBES, MIN_HULL_EXTENT, SHIP_FRICTION, hull_probes, reacts};
use oag_physics::{Handling, ShipState};

/// How far outside the hull a wall still counts as too close, in world units.
/// **Chosen, not measured**: about twice the 0.3-0.6 units an Ace spending its share
/// of line bias sat wider than the neutral driver at Talon's Junction.
pub(super) const MARGIN: f32 = 1.0;

/// Whether the hull of a craft in `state` is within [`MARGIN`] of a wall.
pub(super) fn too_close<R: Raycaster + ?Sized>(
    state: &ShipState,
    handling: &Handling,
    self_collider: Option<u32>,
    raycaster: &R,
) -> bool {
    let probes: [_; HULL_PROBES] = hull_probes(&state.body, handling);
    probes.iter().any(|&(origin, direction, reach)| {
        if reach <= MIN_HULL_EXTENT {
            return false;
        }
        let mut hits = [None; 4];
        let count = raycaster.raycast_all(
            Ray::new(origin, direction, reach + MARGIN),
            self_collider,
            false,
            &mut hits,
        );
        hits.iter().flatten().take(count).any(|hit| {
            hit.normal.dot(direction) < 0.0
                && reacts(combine_friction(SHIP_FRICTION, hit.surface.friction()))
        })
    })
}
