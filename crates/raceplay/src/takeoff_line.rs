//! Moving the AI line off a magstrip on the run-up to a jump.
//!
//! **Ours, chosen, not measured.** `05_Track`'s first jump (samples 162-210 of
//! the line are over a gap) is climbed on a ramp whose left two thirds are
//! `Mag Floor Collision`, and the authored racing line runs up it on the
//! magstrip. `oag_physics::maglock` holds a craft to the surface it is over by
//! projecting the normal component out of its velocity, so a craft riding the
//! strip over the convex top of the ramp leaves it along the road rather than
//! with the climb it carried: measured in our own physics, full throttle at
//! VENOM, 78 units/s of climb at the lip on the line against 87 for the race's
//! lone Ace six units to the right on plain floor, which clears where the line
//! falls short and drops into the pit under the upper road. See
//! `docs/gameplay/ai.md`, "de Konstruct Black: the first jump is a magstrip".
//!
//! The rule reads only the track's own collision: on every sample the line
//! marks as a takeoff run-up (`oag_ai::Line::is_takeoff`), cast down at the
//! line; where that lands on a magstrip, find the nearest lateral offset inside
//! the corridor whose surface, and the surface [`CLEARANCE`] either side of it,
//! is hoverable and not a magstrip, and move the line there. A run-up with no
//! such room is left alone.

use oag_core::math::Vec3;
use oag_physics::{CollisionWorld, Ray, Raycaster, Surface};

/// How far either side of the chosen offset must also be plain floor: about a
/// hull's half-width plus a little. **Chosen, not measured.**
const CLEARANCE: f32 = 2.5;

/// Lateral search step. **Chosen.**
const STEP: f32 = 0.5;

/// Travel over which the move eases in before the first marked sample and out
/// after the last, so the line's heading changes gently. **Chosen, not
/// measured.**
const EASE: f32 = 120.0;

/// The surface straight under `point + lateral * offset`, along `-up`, within
/// `reach` either side.
fn surface_at(
    collision: &CollisionWorld,
    point: Vec3,
    up: Vec3,
    lateral: Vec3,
    offset: f32,
    reach: f32,
) -> Option<Surface> {
    let origin = point + lateral * offset + up * reach;
    collision
        .raycast(Ray::new(origin, -up, reach * 2.0), None, false)
        .map(|hit| hit.surface)
}

fn plain(surface: Option<Surface>) -> bool {
    surface == Some(Surface::Floor)
}

/// Moves `points` off magstrips on every takeoff run-up, rebasing `corridor`.
/// Returns how many samples asked for a move.
pub(super) fn off_magstrips(
    points: &mut [Vec3],
    corridor: &mut [oag_ai::Frame],
    ups: &[Vec3],
    takeoff: &[bool],
    collision: &CollisionWorld,
    reach: f32,
) -> usize {
    let n = points.len();
    if corridor.len() != n || ups.len() != n || takeoff.len() != n {
        return 0;
    }
    let mut targets = Vec::new();
    for i in (0..n).filter(|&i| takeoff[i]) {
        let frame = corridor[i];
        let probe =
            |offset: f32| surface_at(collision, points[i], ups[i], frame.lateral, offset, reach);
        if probe(0.0) != Some(Surface::MagFloor) {
            continue;
        }
        let fits = |offset: f32| {
            offset - CLEARANCE >= frame.left
                && offset + CLEARANCE <= frame.right
                && plain(probe(offset))
                && plain(probe(offset - CLEARANCE))
                && plain(probe(offset + CLEARANCE))
        };
        let reach_out = frame.right.max(-frame.left);
        let mut k = 1;
        let found = loop {
            let d = STEP * k as f32;
            if d > reach_out {
                break None;
            }
            // Right first on a tie, deterministically.
            if fits(d) {
                break Some(d);
            }
            if fits(-d) {
                break Some(-d);
            }
            k += 1;
        };
        if let Some(offset) = found {
            targets.push(oag_ai::line_shift::Target { index: i, offset });
        }
    }
    let before = points.to_vec();
    oag_ai::line_shift::toward(points, corridor, &targets, 0.0, EASE, 0.0);
    // And the room a driver may stray into stops short of the strip, on every
    // sample the move reached: a pilot's own line, or one handed back in
    // traffic, otherwise wanders straight back onto it. Measured: in a field
    // of Aces at VENOM the craft that still fell were the ones a rival had
    // pushed back across, at -5 to -8 units, on the run-up.
    for i in 0..n {
        let moved = (points[i] - before[i]).dot(corridor[i].lateral);
        if moved == 0.0 {
            continue;
        }
        let frame = corridor[i];
        let probe =
            |offset: f32| surface_at(collision, points[i], ups[i], frame.lateral, offset, reach);
        // The strip lies the way the line came from.
        let toward_strip = -moved.signum();
        let room = if toward_strip < 0.0 {
            -frame.left
        } else {
            frame.right
        };
        let mut edge = room;
        let mut d = 0.0;
        while d <= room {
            if probe(toward_strip * d) == Some(Surface::MagFloor) {
                edge = (d - CLEARANCE).max(0.0);
                break;
            }
            d += STEP;
        }
        if toward_strip < 0.0 {
            corridor[i].left = corridor[i].left.max(-edge);
        } else {
            corridor[i].right = corridor[i].right.min(edge);
        }
    }
    targets.len()
}
