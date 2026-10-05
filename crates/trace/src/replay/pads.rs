//! Speed pads, for a replay: the swept containment test that decides
//! [`oag_physics::Environment::pad_hit`], and the pad state a reseed has to
//! restore.
//!
//! The running game does this in `oag_raceplay`'s `test_speedup_pads`, which
//! reimplements `Pads_TestCraft` (`0x08887144`). This crate cannot depend on the
//! composition root, so the test is restated here, and only the part of it that
//! reaches the force law: which pad the hull is inside this tick and which way it
//! pushes. The entry edge (the Zone score, the exhaust flare, the sound) is
//! presentation and a replay has none of it.
//!
//! Until this existed a replay never armed step 15 at all, so every capture that
//! crossed a pad was compared against a craft that got no boost. On
//! `talons-junction-clean-lap.csv` that was eight crossings, the first at tick
//! 161, and it was most of the 44 units of position error the single-seeded
//! comparison used to read at tick 240.
//!
//! # What is left out, and why it cannot matter
//!
//! `Pads_TestCraft` keeps a per-racer, per-pad distance and skips a pad until
//! the craft has travelled that far. [`oag_vex::pads::PadVolume::distance`] is
//! a true lower bound on how far the hull is from the box, so that broadphase
//! only ever skips a pad the hull could not have reached; the answer is the same
//! with or without it, and a replay of one craft has no cost to save.

use oag_core::math::Vec3;
use oag_physics::{Handling, ShipControls, ShipState, clamp_dt, engine};
use oag_vex::pads::PadVolume;

use crate::trace::Frame;

/// `Pad_SweptTest`'s `25.0`: below this much movement in a tick the test is
/// swept, above it the destination alone is tested. Measured live at
/// confidence 88; see `docs/ghidra/functions/psp-pulse-usa/pads.md`.
const SWEEP_LIMIT: f32 = 25.0;

/// How many points the swept test checks: `t = 0.25, 0.5, 0.75, 1.0`, the
/// destination included and the origin not, because the origin was the
/// previous test's destination.
const SWEEP_STEPS: u32 = 4;

/// The swept pad test for one craft, carrying the one thing it needs between
/// ticks: where the previous test was made from.
#[derive(Debug, Clone, Default)]
pub(crate) struct PadSweep {
    previous: Option<Vec3>,
}

impl PadSweep {
    /// Forget where the previous test was made, or put it somewhere else - a
    /// reseed moves the craft without it having travelled there.
    pub(crate) fn restart_from(&mut self, previous: Option<Vec3>) {
        self.previous = previous;
    }

    /// The push direction of the first pad whose box the hull swept through
    /// on its way to `position`, or `None`.
    ///
    /// First pad in node order wins, matching `Pads_TestCraft`'s own loop.
    pub(crate) fn test(&mut self, pads: &[PadVolume], position: Vec3) -> Option<Vec3> {
        let previous = self.previous.replace(position);
        if pads.is_empty() {
            return None;
        }
        let moved = previous.map_or(f32::INFINITY, |from| position.distance(from));
        let sweep: Vec<Vec3> = match previous {
            Some(from) if moved > 0.0 && moved < SWEEP_LIMIT => (1..=SWEEP_STEPS)
                .map(|step| from.lerp(position, step as f32 / SWEEP_STEPS as f32))
                .collect(),
            _ => vec![position],
        };
        pads.iter()
            .filter(|pad| sweep.iter().any(|point| pad.contains(point.to_array())))
            .find_map(|pad| pad.direction().map(Vec3::from_array))
    }
}

/// The speed-pad state the original carried **into** each recorded tick,
/// derived from the recording's own positions.
///
/// A reseed rebuilds the ship from [`ShipState::default`] plus the recorded
/// columns, and no capture records `craft`'s pad timer. So a reseed that lands
/// inside a boost would start with no boost at all, and the window would read
/// the missing force as a force-law error. This walks the recorded positions
/// through the same swept test and the same timer rule
/// ([`engine::speedup_pad`] itself, on a scratch state, so the rule is not
/// restated) and returns `(pad_timer, pad_direction)` as they stood at the start
/// of each tick.
///
/// That is derivation rather than invention, the same move
/// `force_term_pose_walk_ground_truth.rs` makes for the landing timers: the
/// timer is a pure function of the recorded positions and the recorded `dt`.
pub(crate) fn recorded_pad_state(
    frames: &[Frame],
    pads: &[PadVolume],
    handling: &Handling,
) -> Vec<(f32, Vec3)> {
    let mut sweep = PadSweep::default();
    let mut scratch = ShipState::default();
    let mut out = Vec::with_capacity(frames.len());
    for frame in frames {
        out.push((scratch.pad_timer, scratch.pad_direction));
        let hit = sweep.test(pads, frame.position);
        // Only the timer and the direction are read back, so neither the input
        // nor the up axis matters: they decide the tilt of the force, not the
        // state this returns.
        let _ = engine::speedup_pad(
            &mut scratch,
            &ShipControls::default(),
            handling,
            frame.up,
            hit,
            clamp_dt(frame.dt),
        );
    }
    out
}
