//! Where each craft is in the race, and therefore who is winning.
//!
//! [`RaceState`] is the *player's* race - its mode, its clock, its Zone counters,
//! its best lap. This is the much smaller thing every craft on the grid needs: a
//! lap count, a place on the circuit, and whether it has finished. Eight of these
//! ordered against each other is a results table.
//!
//! **The lap rule is not reimplemented here.** A wrap of the distance-along is
//! what counts a lap, and the two-half [`LapGate`] is what stops a craft rocking
//! over the line and scoring one - both live in [`crate::state`] and both are
//! reached through [`wrapped_forward`] and [`LapGate::advanced`], so there is one
//! copy of the rule and not two. That matters more than the duplication it saves:
//! two lap counters that disagree would put a craft in a position it is not in.
//!
//! [`RaceState`]: crate::RaceState

use oag_core::math::Vec3;

use crate::course::Course;
use crate::state::{LapGate, wrapped_backward, wrapped_forward};

/// One craft's place in the race.
///
/// `Copy` and free of allocation, so it sits on a `Ship` inside the world
/// snapshot and a replay reproduces the finishing order. See
/// `docs/architecture/adr/0003-no-ecs.md`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Standing {
    /// The lap this craft is on, 1-based, exactly as [`RaceState::lap`] means it.
    ///
    /// [`RaceState::lap`]: crate::RaceState::lap
    pub lap: u32,
    /// The two-half gate that stops rocking over the line counting as a lap.
    pub gate: LapGate,
    /// Distance along the circuit, or `None` before the first fix.
    ///
    /// Stored as the raw distance rather than a fraction so it can be compared
    /// against another craft's directly.
    pub progress: Option<f32>,
    /// Which course segment the last fix landed on, as the next search's hint.
    pub course_index: Option<u32>,
    /// The tick this craft crossed the line for the last time, once it has.
    ///
    /// `None` while it is still racing. **The ordering key for finishers**: two
    /// craft that both finished are placed by who got there first, not by where
    /// they stopped.
    pub finish_tick: Option<u64>,
}

impl Default for Standing {
    fn default() -> Self {
        Self {
            // Lap 1, like `RaceState`: a craft on the grid has not completed a
            // lap, it is *on* its first.
            lap: 1,
            gate: LapGate::default(),
            progress: None,
            course_index: None,
            finish_tick: None,
        }
    }
}

impl Standing {
    /// Whether this craft has finished.
    #[must_use]
    pub fn finished(&self) -> bool {
        self.finish_tick.is_some()
    }

    /// Advances the standing from where the craft now is.
    ///
    /// Returns `true` on the tick a lap is completed. `laps_target` of `None` is
    /// a mode that never ends on its own, and such a craft never finishes.
    pub fn update(
        &mut self,
        course: &Course,
        position: Vec3,
        tick: u64,
        laps_target: Option<u32>,
    ) -> bool {
        if self.finished() {
            return false;
        }

        let hint = self.course_index.map(|index| index as usize);
        let Some(located) = course.locate(position, hint) else {
            return false;
        };
        self.course_index = Some(located.index as u32);

        let previous = self.progress.replace(located.progress);
        // First fix of the race: nothing to have wrapped away from, however far
        // from the line the craft happens to have spawned.
        let Some(previous) = previous else {
            return false;
        };

        let half = course.length() * 0.5;
        self.gate = self.gate.advanced(located.progress, half);

        let delta = located.progress - previous;
        if wrapped_backward(delta, half) {
            self.lap = self.lap.saturating_sub(1).max(1);
            return false;
        }
        if !wrapped_forward(delta, half) || self.gate != LapGate::Ready {
            return false;
        }

        self.lap = self.lap.saturating_add(1);
        self.gate = LapGate::NeedsNearHalf;
        if let Some(target) = laps_target
            && self.lap > target
        {
            self.finish_tick = Some(tick);
        }
        true
    }

    /// How far round the race this craft is, for ordering.
    ///
    /// Laps times the circuit plus the distance into the current one, so one
    /// number orders the whole field. A craft with no fix yet reads as being at
    /// the very start, which is where it is.
    #[must_use]
    pub fn distance(&self, course: &Course) -> f32 {
        (self.lap.saturating_sub(1)) as f32 * course.length() + self.progress.unwrap_or(0.0)
    }
}

/// Race positions, `1`-based, in the same slot order as the input.
///
/// Ordering, in this order:
///
/// 1. **Finishers before racers.** A craft that has crossed for the last time is
///    ahead of one still going round, whatever the distances say.
/// 2. **Finishers by when they finished**, earliest first.
/// 3. **Racers by distance covered**, furthest first.
/// 4. **Ties by slot index**, lowest first.
///
/// Rule 4 is not cosmetic. Two craft can be at the same distance to the bit -
/// they start that way, on the grid, before anyone has a fix - and a comparison
/// that left the order to chance would feed an arbitrary result into simulation
/// state. See `docs/architecture/determinism.md`.
#[must_use]
pub fn places<const N: usize>(standings: &[Standing; N], course: &Course) -> [u8; N] {
    let mut order: [usize; N] = std::array::from_fn(|slot| slot);
    order.sort_by(|&a, &b| {
        let (first, second) = (&standings[a], &standings[b]);
        match (first.finish_tick, second.finish_tick) {
            (Some(a_tick), Some(b_tick)) => a_tick.cmp(&b_tick),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => second.distance(course).total_cmp(&first.distance(course)),
        }
        .then(a.cmp(&b))
    });

    let mut places = [0u8; N];
    for (place, &slot) in order.iter().enumerate() {
        places[slot] = u8::try_from(place + 1).unwrap_or(u8::MAX);
    }
    places
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing;

    fn course() -> Course {
        testing::ring_course()
    }

    fn at(lap: u32, progress: f32) -> Standing {
        Standing {
            lap,
            progress: Some(progress),
            ..Standing::default()
        }
    }

    #[test]
    fn a_fresh_standing_is_on_lap_one_and_has_not_finished() {
        let standing = Standing::default();
        assert_eq!(standing.lap, 1);
        assert!(!standing.finished());
        assert_eq!(standing.progress, None);
    }

    #[test]
    fn further_round_is_a_better_place() {
        let course = course();
        let standings = [at(1, 10.0), at(1, 90.0), at(2, 5.0)];
        assert_eq!(places(&standings, &course), [3, 2, 1]);
    }

    #[test]
    fn a_finisher_beats_anyone_still_racing() {
        let course = course();
        let mut done = at(4, 0.0);
        done.finish_tick = Some(500);
        // Further round the circuit, and still racing, so still second.
        let standings = [at(9, 0.0), done];
        assert_eq!(places(&standings, &course), [2, 1]);
    }

    #[test]
    fn finishers_are_placed_by_when_they_finished() {
        let course = course();
        let early = Standing {
            finish_tick: Some(100),
            ..Standing::default()
        };
        let late = Standing {
            finish_tick: Some(200),
            ..Standing::default()
        };
        assert_eq!(places(&[late, early], &course), [2, 1]);
    }

    /// Eight craft on the grid have no fix at all and are all at distance zero.
    /// The order has to be *something*, and it has to be the same something every
    /// run.
    #[test]
    fn craft_that_are_exactly_level_are_placed_by_slot() {
        let course = course();
        let standings = [Standing::default(); 8];
        assert_eq!(places(&standings, &course), [1, 2, 3, 4, 5, 6, 7, 8]);
    }
}
