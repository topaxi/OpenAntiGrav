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
use crate::state::{LapGate, MAX_RECORDED_LAPS, record_split, wrapped_backward, wrapped_forward};

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
    /// The tick the lap being driven started on, once one has.
    ///
    /// `None` until this craft first crosses the line, which is **not** the tick
    /// the race started: the grid is laid out from the authored `Start Position`
    /// node, `Course::START_LINE_OFFSET` behind the line, so a craft on the grid
    /// has not begun a lap however long it has been driving. See
    /// [`Self::update`]'s spawn-to-line arm.
    ///
    /// [`RaceState`] holds the same quantity as a bare `u64` defaulting to `0`,
    /// which works there only because `best_lap_ticks.is_none()` doubles as the
    /// "clock not started" flag. Keeping the two coupled is what the `Option`
    /// avoids; a craft that had genuinely started a lap on tick `0` would be
    /// indistinguishable from one that had not.
    ///
    /// [`RaceState`]: crate::RaceState
    pub lap_start_tick: Option<u64>,
    /// The quickest lap this craft has driven, in ticks.
    ///
    /// Ticks and not seconds, exactly as [`RaceState::best_lap_ticks`] is, so the
    /// two are directly comparable - which is the invariant
    /// `crates/game/tests/lap_times_ground_truth.rs` asserts of slot 0, where both
    /// clocks time the same craft.
    ///
    /// [`RaceState::best_lap_ticks`]: crate::RaceState::best_lap_ticks
    pub best_lap_ticks: Option<u32>,
    /// Each completed lap's own time, in ticks, indexed by `lap - 1`.
    ///
    /// The same field and the same bound [`RaceState::lap_splits`] carries,
    /// recorded through the shared [`record_split`] so the two never disagree
    /// about which lap an index means.
    ///
    /// [`RaceState::lap_splits`]: crate::RaceState::lap_splits
    pub lap_splits: [Option<u32>; MAX_RECORDED_LAPS],
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
            lap_start_tick: None,
            best_lap_ticks: None,
            lap_splits: [None; MAX_RECORDED_LAPS],
        }
    }
}

impl Standing {
    /// Whether this craft has finished.
    #[must_use]
    pub fn finished(&self) -> bool {
        self.finish_tick.is_some()
    }

    /// Ticks the lap being driven has been running, as of `tick`.
    ///
    /// `None` before this craft's clock started, which is a different answer from
    /// `Some(0)` and the reason [`Self::lap_start_tick`] is an `Option`.
    /// Saturating for the same reason [`RaceState::lap_ticks`] is: a caller that
    /// passes a tick from before the lap started gets zero, not a lap that has run
    /// for half an eternity.
    ///
    /// [`RaceState::lap_ticks`]: crate::RaceState::lap_ticks
    #[must_use]
    pub fn lap_ticks(&self, tick: u64) -> Option<u64> {
        self.lap_start_tick
            .map(|started| tick.saturating_sub(started))
    }

    /// Advances the standing from where the craft now is.
    ///
    /// Returns `true` on the tick a lap is completed. `laps_target` of `None` is
    /// a mode that never ends on its own, and such a craft never finishes.
    ///
    /// # The clock, and why lap 1 needs a rule of its own
    ///
    /// A craft crosses the start line **twice** on its way to completing lap 1:
    /// once on the way off the grid, and once to finish the lap. The grid is laid
    /// out from the authored `Start Position` node, which sits
    /// `Course::START_LINE_OFFSET` behind the line, so timing lap 1 from the
    /// standing start would make it longer than every other lap by however far the
    /// craft's own slot is back - a different amount per slot, so the field's lap
    /// times would not even be comparable with each other.
    ///
    /// The first crossing is therefore where the clock starts, and it is
    /// identified the same way [`RaceState::update`] identifies it: a forward wrap
    /// that the gate refuses, on lap 1, with no best lap yet. This is
    /// [`RaceState`]'s rule ported rather than a second invention; the lap
    /// *counting* around it is untouched.
    ///
    /// [`RaceState`]: crate::RaceState
    /// [`RaceState::update`]: crate::RaceState::update
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
            // **The clock is deliberately not restored**, exactly as
            // `RaceState::uncomplete_lap` does not restore it: what it read when
            // the line was last crossed forwards is not kept, and inventing a
            // value would put a wrong time on a results table. Driving backwards
            // over the line is already a wrong-way situation.
            //
            // **The gate has to go back too, and until 2026-09-07 it did not.**
            // `self.gate` was already advanced above using *this* tick's
            // (post-wrap) progress, which for a backward crossing sits past the
            // far half - so a craft shoved backwards over the line landed on
            // `LapGate::Ready` and stayed there, with nothing here to undo it.
            // The very next forward crossing then read `Ready` as "the lap is
            // earned" and completed it on the spot, against the clock still
            // reading the lap this craft had *not* re-driven - a lap of a
            // handful of ticks that never happened. `RaceState::uncomplete_lap`
            // resets its own gate for exactly this reason; this arm has to
            // match it, per this module's own rule that the two must not
            // disagree about where a craft is. Found on a real race where the
            // corrected craft-pair narrowphase
            // (`docs/ghidra/functions/psp-pulse-usa/contact-response.md`) threw
            // a craft back across the line in one tick - see
            // `crates/game/tests/lap_times_ground_truth.rs`'s
            // `every_opponent_that_laps_has_a_lap_time`, which is what caught
            // it, and `an_immediate_re_crossing_after_a_backward_wrap_earns_no_lap`
            // below, which pins the rule directly.
            self.gate = LapGate::NeedsNearHalf;
            return false;
        }
        if !wrapped_forward(delta, half) {
            return false;
        }
        if self.gate != LapGate::Ready {
            // **The spawn-to-line crossing**, and the only place the clock ever
            // starts. See this function's doc comment for why lap 1 cannot be
            // timed from the standing start.
            if self.lap == 1 && self.best_lap_ticks.is_none() {
                self.lap_start_tick = Some(tick);
            }
            return false;
        }

        // A lap, so it has a time - unless the clock never started, which happens
        // only when the craft's first located fix was already past the line and
        // its first wrap is this one. `None` rather than a time measured from a
        // start that was never observed.
        if let Some(ticks) = self.lap_ticks(tick) {
            let ticks = u32::try_from(ticks).unwrap_or(u32::MAX);
            self.best_lap_ticks = Some(match self.best_lap_ticks {
                Some(best) => best.min(ticks),
                None => ticks,
            });
            record_split(&mut self.lap_splits, self.lap, ticks);
        }
        self.lap_start_tick = Some(tick);

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
    ///
    /// # The grid straddles the start line, so lap 1 needs one more rule
    ///
    /// **A craft that has not yet reached the line for the first time is *behind*
    /// it, not a whole lap ahead of it**, and without that the whole first lap is
    /// ordered backwards. This is not a corner case, it is every race: the grid is
    /// laid out from the authored `Start Position` node, which on `16_Track` sits
    /// ~138 units *before* the line, and the eight slots run ~19.8 units apart
    /// forward from it - so the field spans the line at the moment the lights go
    /// out. Measured, on a real load: the parked player read `progress 4956` of a
    /// 5,094-unit circuit while seven craft that had crossed and driven on read
    /// `1500`, which made the craft in last place first.
    ///
    /// The marker for "has not crossed yet" is the lap gate rather than the
    /// distance: [`LapGate::NeedsNearHalf`] means this craft has never been seen in
    /// the half of the circuit that follows the line, which is exactly the state a
    /// craft sitting behind it on the grid is in, and which it leaves on its first
    /// crossing. Keying on the distance alone would also catch a craft on its way
    /// round lap 1 having already crossed once - a craft the gate has moved on.
    ///
    /// One tick is still not ordered: before the first fix nobody has a progress at
    /// all, every craft reads `0`, and the field is ordered by slot. That is the
    /// tick before the first physics step, and the original spends it in a
    /// countdown.
    ///
    /// [`LapGate::NeedsNearHalf`]: crate::LapGate::NeedsNearHalf
    #[must_use]
    pub fn distance(&self, course: &Course) -> f32 {
        let progress = self.progress.unwrap_or(0.0);
        if self.lap == 1 && self.gate == LapGate::NeedsNearHalf && progress > course.length() * 0.5
        {
            return progress - course.length();
        }
        (self.lap.saturating_sub(1)) as f32 * course.length() + progress
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
mod tests;
