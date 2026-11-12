//! A wall met on the landing after a held run-up: the takeoff was too fast.
//!
//! A run-up is held ([`SpeedPlan::hold_run_up`]) when the craft fell short of
//! a gap after the plan braked on it, and a held sample has no ceiling, so
//! the craft takes off at whatever full throttle gives it. Where the road
//! after the landing turns, that can carry it past the turn: Pulse `06_Track`
//! RAPIER falls into the gap at 1196-1200 from a lip speed of 34-37.5 u/s,
//! lands at 1214 and stays on the road from about 41, and from the 72-89 a
//! held run-up gives lands at 1225 at about 100 u/s and meets the wall at
//! 1232-1242, which no ceiling after the landing brings it down from
//! (`docs/gameplay/ai.md`, "06_Track RAPIER with the body damping").
//!
//! So a wall on the ground (or after a hop) soon after such a flight lowers
//! the held run-up's own ceilings by [`LOWER`] a try, keeping the hold so a
//! corner after the landing still does not brake through it; and a fall
//! short of the gap off a held run-up with a ceiling raises it by the same
//! step. The two meet in the window between. **Chosen, not measured.**

use std::collections::BTreeMap;

use crate::Line;

use super::{LOOKBACK_TICKS, STALL_SPEED, SpeedPlan, Trace, over_a_gap};

/// The step a long landing lowers a held ceiling by and a short one raises
/// it by: flat, unlike [`SpeedPlan::lower`]'s, which steepens with the tries
/// and would step over the window. **Chosen, not measured.**
const LOWER: f32 = 0.96;

/// Where the run-up of the flight a craft landed from ends in `recent` (the
/// index of the flight's first tick), if that flight ended inside the last
/// [`LOOKBACK_TICKS`], crossed a gap the line crosses, and took off from a
/// run-up with a held sample in it.
pub(super) fn long_landing(recent: &[Trace], line: &Line, hold: &[bool]) -> Option<usize> {
    let from = recent.len().saturating_sub(LOOKBACK_TICKS);
    let last_air = from + recent[from..].iter().rposition(|t| t.airborne)?;
    let first_air = recent[..last_air]
        .iter()
        .rposition(|t| !t.airborne)
        .map_or(0, |grounded| grounded + 1);
    let run_up = &recent[first_air.saturating_sub(LOOKBACK_TICKS * 2)..first_air];
    let held = run_up
        .iter()
        .any(|t| hold.get(t.index as usize).copied().unwrap_or(false));
    (held && over_a_gap(run_up.iter().chain(&recent[first_air..=last_air]), line))
        .then_some(first_air)
}

/// One step up from a held ceiling a long landing brought down, back to none
/// once it passes what any class reaches.
pub(super) fn raised(ceiling: f32) -> f32 {
    let up = ceiling / LOWER;
    if up < super::UNLIMITED {
        up
    } else {
        f32::INFINITY
    }
}

impl SpeedPlan {
    /// Lowers each held sample of `run_up` (which ends at the last grounded
    /// tick before a takeoff) once, to [`LOWER`] of the lower of the slowest
    /// the craft went there and what it was allowed: a sample the craft sits
    /// on for many ticks (the lip) comes down one step, not one per tick.
    /// Returns how many moved.
    pub(super) fn slow_held_run_up(&mut self, run_up: &[Trace]) -> u32 {
        let from = run_up.len().saturating_sub(LOOKBACK_TICKS * 2);
        let mut slowest: BTreeMap<usize, f32> = BTreeMap::new();
        for trace in &run_up[from..] {
            let i = trace.index as usize;
            if self.hold[i] && !trace.airborne {
                let speed = slowest.entry(i).or_insert(trace.speed);
                *speed = speed.min(trace.speed);
            }
        }
        let mut moved = 0;
        for (i, speed) in slowest {
            let lowered = (speed.min(self.ceiling[i]) * LOWER).max(STALL_SPEED * 4.0);
            if lowered < self.ceiling[i] {
                self.ceiling[i] = lowered;
                moved += 1;
            }
        }
        moved
    }
}
