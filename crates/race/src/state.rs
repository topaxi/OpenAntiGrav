//! The race's own state: what lap it is, how long it has taken, which zone.
//!
//! This is the struct [ADR-0003] sketched as `World.race` back when the world
//! was first laid out, and it is plain `Copy` data for the reason that ADR
//! gives: the whole world has to snapshot in one `memcpy`-shaped operation for
//! replays and golden tests. No `Vec`, no `String`, no allocation.
//!
//! [ADR-0003]: ../../../docs/architecture/adr/0003-no-ecs.md

use crate::{Course, Mode, zone};
use oag_core::math::Vec3;

/// Everything the race layer knows, for one ship.
///
/// Sized for the single-ship modes. Positions, grid order and per-opponent
/// timing are M5 work and are deliberately not modelled here yet: a `place`
/// field that always reads 1 is worse than no field, because it looks like an
/// answer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RaceState {
    /// Which mode's rules are running.
    pub mode: Mode,
    /// The lap the ship is on, 1-based.
    pub lap: u32,
    /// Laps the race finishes after, or `None` when it never ends on its own.
    pub laps_target: Option<u32>,
    /// Distance along the course, in track units, from the start line.
    ///
    /// `None` until the ship has been located on the course at least once. A
    /// ship far enough off the course to have no nearest point keeps its last
    /// value rather than dropping to `None`, so a big jump cannot be read as a
    /// lap.
    pub progress: Option<f32>,
    /// The tick the current lap began on.
    pub lap_start_tick: u64,
    /// The fastest completed lap so far, in ticks.
    pub best_lap_ticks: Option<u32>,
    /// The zone number, Zone mode only. Zero before the first step.
    ///
    /// A `u16` because that is what the original stores it as, and it is
    /// **assigned** into the craft rather than incremented there.
    pub zone: u16,
    /// Seconds accumulated toward the next zone step.
    pub zone_timer: f32,
    /// Zone mode's score.
    pub score: i32,
    /// How much of the current lap the ship has actually driven.
    ///
    /// **The guard that makes a wrap mean a lap.** Two separate failures need it,
    /// and one of them was found by driving a time trial on Moa Therma:
    ///
    /// - A ship spawns on the grid slot, which is *upstream* of the start line by
    ///   [`Course::START_LINE_OFFSET`]. It therefore crosses the line seconds
    ///   into the race - a genuine wrap of the distance-along, and not a lap.
    /// - A ship reversed back over the line and driven forward again re-crosses
    ///   it. Counting that would also reset the lap clock, so a few seconds of
    ///   rocking would record an unbeatable best lap.
    ///
    /// Requiring the near half *and then* the far half, in that order, rules out
    /// both: the first has not driven the near half, and the second has not
    /// driven it *since* the crossing.
    pub lap_gate: LapGate,
    /// Whether the current zone has had a wall contact.
    ///
    /// Clears at every zone step. A zone that ends with this still clear is a
    /// "perfect zone" and pays a bonus.
    pub zone_dirty: bool,
    /// Whether the race has reached its finish condition.
    ///
    /// Only a time trial ever sets this today. Speed lap has no end, and Zone's
    /// end condition is unrecovered - see `docs/gameplay/race-modes.md`.
    pub finished: bool,
    /// The ring point the ship was nearest last tick.
    ///
    /// A cursor, and the original keeps one too: `AiTrack_LocatePosition` gives
    /// every ship and the camera its own `{track, path_index, point_index}` and
    /// seeds each search from last frame's answer
    /// (`docs/formats/track.md:389-399`). It is simulation state rather than a
    /// cache because the search result depends on it: a locator that starts from
    /// a different place can settle on a different point where the track passes
    /// over itself.
    pub course_index: Option<u32>,
}

impl Default for RaceState {
    fn default() -> Self {
        Self::new(Mode::TimeTrial)
    }
}

impl RaceState {
    /// A race about to start, on lap 1, with the clock at zero.
    #[must_use]
    pub fn new(mode: Mode) -> Self {
        Self {
            mode,
            lap: 1,
            laps_target: mode.laps_target(),
            progress: None,
            lap_start_tick: 0,
            best_lap_ticks: None,
            zone: 0,
            zone_timer: 0.0,
            score: 0,
            lap_gate: LapGate::NeedsNearHalf,
            zone_dirty: false,
            finished: false,
            course_index: None,
        }
    }

    /// Ticks the current lap has been running, as of `tick`.
    ///
    /// Saturating rather than wrapping: a caller that passes a tick from before
    /// the lap started gets zero, not a lap that has run for half an eternity.
    #[must_use]
    pub fn lap_ticks(&self, tick: u64) -> u64 {
        tick.saturating_sub(self.lap_start_tick)
    }

    /// Laps completed, which is one less than the lap being driven.
    #[must_use]
    pub fn laps_completed(&self) -> u32 {
        self.lap.saturating_sub(1)
    }

    /// Advances the race by one tick.
    ///
    /// Call this **after** the physics step, on the position that step produced.
    /// Running it first would test last tick's position against this tick's
    /// clock, which is a tick of error on a quantity whose whole job is to be
    /// exact at one instant.
    ///
    /// `dt` is the fixed timestep, and `wall_contact` is whether the ship
    /// touched track geometry this tick - a flag rather than a magnitude,
    /// because the original's own perfect-zone test reads a single bit
    /// (`entity+0x860 & 0x400000`) and not an impulse.
    pub fn update(
        &mut self,
        course: &Course,
        position: Vec3,
        tick: u64,
        dt: f32,
        wall_contact: bool,
    ) -> Outcome {
        let mut outcome = Outcome::default();
        if self.finished {
            return outcome;
        }

        if wall_contact {
            self.zone_dirty = true;
        }
        if self.mode == Mode::Zone {
            self.advance_zone(dt, &mut outcome);
        }

        let hint = self.course_index.map(|index| index as usize);
        let Some(located) = course.locate(position, hint) else {
            return outcome;
        };
        self.course_index = Some(located.index as u32);

        let previous = self.progress;
        self.progress = Some(located.progress);
        let Some(previous) = previous else {
            // First fix of the race. There is no previous reading to have
            // wrapped away from, so this cannot be a lap however far from the
            // line the ship happens to have spawned.
            return outcome;
        };

        // A lap is a wrap of the distance-along, not a plane crossing. Anything
        // that moves more than half the circuit in one tick has wrapped rather
        // than travelled: at 60 Hz even a Phantom-class craft covers a few units
        // per tick against a circuit thousands of units round.
        let half = course.length() * 0.5;
        self.lap_gate = self.lap_gate.advanced(located.progress, half);

        let delta = located.progress - previous;
        if delta < -half {
            // A wrap. Whether it is also a *lap* is what the gate decides.
            if self.lap_gate == LapGate::Ready {
                self.complete_lap(tick, &mut outcome);
            } else if self.lap == 1 && self.best_lap_ticks.is_none() {
                // The first time the race crosses the line, and it did not
                // complete a lap doing it: this is the spawn-to-line crossing.
                //
                // Start lap 1's clock here rather than at the standing start.
                // The ship spawns on the authored grid slot, which is
                // `Course::START_LINE_OFFSET` *behind* the line, while the
                // original's craft starts on the line itself
                // (`crates/game/tests/race_ground_truth.rs`). Timing from the
                // standing start would make our lap 1 that much longer than
                // every other lap and than the original's.
                self.lap_start_tick = tick;
            }
            self.lap_gate = LapGate::NeedsNearHalf;
        } else if delta > half {
            self.uncomplete_lap();
        }
        outcome
    }

    /// Steps the zone counter, and pays out a zone that ended clean.
    fn advance_zone(&mut self, dt: f32, outcome: &mut Outcome) {
        self.score = self.score.saturating_add(zone::SCORE_PER_TICK);
        self.zone_timer += dt;
        if self.zone_timer < zone::STEP_SECONDS {
            return;
        }

        if self.zone_dirty {
            self.zone_dirty = false;
        } else {
            self.score = self.score.saturating_add(zone::CLEAN_ZONE_BONUS);
            outcome.perfect_zone = true;
        }
        self.zone = self.zone.saturating_add(1);
        // Reset rather than subtract the step. The original stores `0.0` here,
        // so it drops whatever the frame overshot by and the zone clock runs
        // fractionally slow. That is recovered behaviour, not a rounding
        // artefact to be tidied up. See `docs/gameplay/race-modes.md`.
        self.zone_timer = 0.0;
        outcome.zone_advanced = true;
    }

    /// Crosses the line forwards.
    fn complete_lap(&mut self, tick: u64, outcome: &mut Outcome) {
        let ticks = u32::try_from(self.lap_ticks(tick)).unwrap_or(u32::MAX);
        self.best_lap_ticks = Some(match self.best_lap_ticks {
            Some(best) => best.min(ticks),
            None => ticks,
        });
        self.lap = self.lap.saturating_add(1);
        self.lap_start_tick = tick;
        outcome.lap_completed = true;
        outcome.lap_ticks = Some(ticks);

        if self.laps_target.is_some_and(|target| self.lap > target) {
            self.finished = true;
            outcome.finished = true;
        }
    }

    /// Crosses the line backwards.
    ///
    /// The lap count goes back down, and the lap clock is deliberately **not**
    /// restored: what it read when the line was last crossed forwards is not
    /// kept, and inventing a value would put a wrong time on the HUD. Driving
    /// backwards over the line is already a wrong-way situation the HUD warns
    /// about.
    fn uncomplete_lap(&mut self) {
        self.lap = self.lap.saturating_sub(1).max(1);
        // The lap being re-entered has to be earned again. Strict - the ship did
        // drive it once - but the alternative lets a ship rock over the line and
        // record a lap time of a few ticks, and reversing over a start line is
        // already a wrong-way situation the HUD warns about.
        self.lap_gate = LapGate::NeedsNearHalf;
    }
}

/// How much of the current lap has been driven, in order.
///
/// A lap is only counted from [`Self::Ready`]. See [`RaceState::lap_gate`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LapGate {
    /// Nothing yet. The ship has not been seen in the half of the lap that
    /// follows the start line.
    #[default]
    NeedsNearHalf,
    /// The near half is done; the far half is not.
    NeedsFarHalf,
    /// Both halves driven. The next forward crossing is a lap.
    Ready,
}

impl LapGate {
    /// This gate after seeing `progress`, given the lap's half-way distance.
    #[must_use]
    pub fn advanced(self, progress: f32, half: f32) -> Self {
        match self {
            Self::NeedsNearHalf if progress < half => Self::NeedsFarHalf,
            Self::NeedsFarHalf if progress >= half => Self::Ready,
            other => other,
        }
    }
}

/// What one [`RaceState::update`] did, for a caller that needs to react.
///
/// A caller that only draws a HUD can ignore this and read the state; this
/// exists for the things that happen *at* an edge - a sound, a banner, a
/// recorded lap - which cannot be recovered by looking at the state afterwards.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Outcome {
    /// The ship crossed the line forwards this tick.
    pub lap_completed: bool,
    /// How long that lap took, in ticks.
    pub lap_ticks: Option<u32>,
    /// The race reached its finish condition this tick.
    pub finished: bool,
    /// The zone number stepped this tick.
    pub zone_advanced: bool,
    /// That zone was completed without touching anything.
    pub perfect_zone: bool,
}

#[cfg(test)]
mod tests {
    use super::{Outcome, RaceState};
    use crate::testing::square_track;
    use crate::{Course, Mode, zone};

    /// The fixed timestep, from ADR-0007.
    const DT: f32 = 1.0 / 60.0;

    fn course() -> Course {
        Course::from_track(&square_track(2), None).expect("a ring")
    }

    /// Drives one full lap by stepping the ship along every ring point in order.
    ///
    /// Returns the outcomes, one per tick, so a test can assert on the edge
    /// rather than only on the state that survives it.
    fn drive_lap(state: &mut RaceState, course: &Course, from_tick: u64) -> Vec<Outcome> {
        (0..course.len())
            .map(|index| {
                let position = course.position(index).expect("in range");
                state.update(course, position, from_tick + index as u64, DT, false)
            })
            .collect()
    }

    /// The bug this guards, found by driving a time trial on Moa Therma: the
    /// ship spawns on the grid slot, the slot is upstream of the start line, so
    /// the ship crosses the line seconds into the race. That is a real wrap of
    /// the distance-along and a bare wrap test counted it as a completed lap.
    #[test]
    fn crossing_the_line_just_after_the_start_is_not_a_lap() {
        let course = course();
        let mut state = RaceState::new(Mode::TimeTrial);

        // Spawn a few points *before* the line and drive over it.
        let before_line = course.len() - 3;
        for (tick, index) in (0..).zip((before_line..course.len()).chain(0..6)) {
            let position = course.position(index).expect("in range");
            let outcome = state.update(&course, position, tick, DT, false);
            assert!(
                !outcome.lap_completed,
                "counted a lap {tick} ticks into the race, at ring point {index}"
            );
        }
        assert_eq!(state.lap, 1, "the race left the line already on lap 2");
    }

    /// And having done that, a real lap still counts - the guard must not eat it.
    #[test]
    fn the_first_real_lap_after_an_early_crossing_still_counts() {
        let course = course();
        let mut state = RaceState::new(Mode::TimeTrial);

        let before_line = course.len() - 3;
        let mut tick = 0;
        for index in (before_line..course.len()).chain(0..course.len()) {
            let position = course.position(index).expect("in range");
            state.update(&course, position, tick, DT, false);
            tick += 1;
        }
        // Back over the line after a full circuit.
        let outcome = state.update(&course, course.position(0).unwrap(), tick, DT, false);
        assert!(outcome.lap_completed, "a genuine lap did not count");
        assert_eq!(state.lap, 2);
    }

    #[test]
    fn a_ship_nudged_back_and_forth_over_the_line_cannot_ratchet_the_counter() {
        let course = course();
        let mut state = RaceState::new(Mode::SpeedLap);
        for tick in 0..20 {
            let index = if tick % 2 == 0 { course.len() - 1 } else { 0 };
            state.update(&course, course.position(index).unwrap(), tick, DT, false);
        }
        assert_eq!(state.lap, 1, "rocking over the line counted laps");
    }

    #[test]
    fn the_first_fix_of_a_race_is_never_a_lap() {
        let course = course();
        let mut state = RaceState::new(Mode::TimeTrial);
        // Spawn most of the way round, so the very first reading is a large
        // distance. Without the "no previous fix" guard this reads as a wrap.
        let position = course.position(course.len() - 1).expect("in range");
        let outcome = state.update(&course, position, 0, DT, false);
        assert!(!outcome.lap_completed);
        assert_eq!(state.lap, 1);
    }

    #[test]
    fn driving_all_the_way_round_completes_exactly_one_lap() {
        let course = course();
        let mut state = RaceState::new(Mode::TimeTrial);
        let outcomes = drive_lap(&mut state, &course, 0);
        let laps = outcomes.iter().filter(|o| o.lap_completed).count();
        assert_eq!(
            laps, 0,
            "the lap only closes when the line is crossed again"
        );

        // One more step, back onto the first point: that is the wrap.
        let position = course.position(0).expect("in range");
        let outcome = state.update(&course, position, course.len() as u64, DT, false);
        assert!(outcome.lap_completed);
        assert_eq!(state.lap, 2);
        assert_eq!(state.laps_completed(), 1);
    }

    #[test]
    fn the_lap_time_is_recorded_and_the_best_is_kept() {
        let course = course();
        let mut state = RaceState::new(Mode::SpeedLap);
        let mut tick = 0;

        // Three laps. The clock is driven by the tick we pass in, not by how
        // many points we stepped through, so the laps can be given different
        // durations without moving the ship differently.
        let mut lap_times = Vec::new();
        for gap in [0u64, 100, 40] {
            drive_lap(&mut state, &course, tick);
            tick += course.len() as u64 + gap;
            let position = course.position(0).expect("in range");
            let outcome = state.update(&course, position, tick, DT, false);
            assert!(outcome.lap_completed, "lap did not close");
            lap_times.push(outcome.lap_ticks.expect("a completed lap has a time"));
            tick += 1;
        }

        assert_eq!(
            state.best_lap_ticks,
            Some(*lap_times.iter().min().expect("three laps")),
            "best lap is not the fastest of {lap_times:?}"
        );
    }

    #[test]
    fn a_time_trial_finishes_after_three_laps_and_a_speed_lap_never_does() {
        for (mode, expect_finish) in [(Mode::TimeTrial, true), (Mode::SpeedLap, false)] {
            let course = course();
            let mut state = RaceState::new(mode);
            let mut tick = 0;
            let mut finished_on = None;

            for lap in 0..4 {
                drive_lap(&mut state, &course, tick);
                tick += course.len() as u64;
                let position = course.position(0).expect("in range");
                let outcome = state.update(&course, position, tick, DT, false);
                tick += 1;
                if outcome.finished {
                    finished_on = Some(lap + 1);
                    break;
                }
            }

            if expect_finish {
                assert_eq!(finished_on, Some(3), "{mode:?} should end on lap 3");
                assert!(state.finished);
            } else {
                assert_eq!(finished_on, None, "{mode:?} should never end on its own");
                assert!(!state.finished);
            }
        }
    }

    #[test]
    fn a_finished_race_stops_counting() {
        let course = course();
        let mut state = RaceState::new(Mode::TimeTrial);
        state.finished = true;
        let before = state;
        let outcome = state.update(&course, course.position(5).unwrap(), 999, DT, false);
        assert_eq!(outcome, Outcome::default());
        assert_eq!(state, before, "a finished race kept simulating");
    }

    #[test]
    fn crossing_the_line_backwards_takes_the_lap_back() {
        let course = course();
        let mut state = RaceState::new(Mode::SpeedLap);
        drive_lap(&mut state, &course, 0);
        let position = course.position(0).expect("in range");
        state.update(&course, position, course.len() as u64, DT, false);
        assert_eq!(state.lap, 2);

        // Reverse back over the line.
        let first_tick = course.len() as u64 + 1;
        for (tick, index) in (first_tick..).zip((course.len() - 4..course.len()).rev()) {
            let position = course.position(index).expect("in range");
            state.update(&course, position, tick, DT, false);
        }
        assert_eq!(state.lap, 1, "reversing over the line did not undo the lap");
    }

    #[test]
    fn the_lap_count_never_goes_below_one() {
        let course = course();
        let mut state = RaceState::new(Mode::SpeedLap);
        // Sit just past the line, then reverse over it repeatedly.
        for _ in 0..3 {
            state.update(&course, course.position(1).unwrap(), 0, DT, false);
            state.update(
                &course,
                course.position(course.len() - 1).unwrap(),
                1,
                DT,
                false,
            );
        }
        assert_eq!(state.lap, 1);
    }

    #[test]
    fn a_zone_steps_every_ten_seconds_and_the_timer_resets_to_zero() {
        let course = course();
        let mut state = RaceState::new(Mode::Zone);
        let position = course.position(0).expect("in range");

        // Ten seconds at the fixed 60 Hz of ADR-0007 is 600 ticks, so the 600th
        // accumulation is the one that steps. Counting the ticks it actually
        // took says that directly, and does not quietly pass if `dt` accumulates
        // to slightly under 10.0 and the step slips a tick.
        let mut stepped_after = None;
        for tick in 0..2_000 {
            if state
                .update(&course, position, tick, DT, false)
                .zone_advanced
            {
                stepped_after = Some(tick + 1);
                break;
            }
        }
        assert_eq!(stepped_after, Some(600), "a zone is not 600 ticks long");
        assert_eq!(state.zone, 1);

        // Reset to zero, not `-= STEP_SECONDS`. The original stores 0.0 and so
        // drops the frame overshoot; keeping the remainder would be a different
        // clock. See `zone::STEP_SECONDS`.
        assert_eq!(state.zone_timer, 0.0);
    }

    #[test]
    fn only_zone_mode_scores_or_steps() {
        let course = course();
        for mode in [Mode::TimeTrial, Mode::SpeedLap] {
            let mut state = RaceState::new(mode);
            let position = course.position(0).expect("in range");
            for tick in 0..1_000 {
                state.update(&course, position, tick, DT, false);
            }
            assert_eq!(state.zone, 0, "{mode:?} stepped a zone");
            assert_eq!(state.score, 0, "{mode:?} scored");
        }
    }

    #[test]
    fn a_clean_zone_pays_the_bonus_and_a_dirty_one_does_not() {
        let course = course();
        let position = course.position(0).expect("in range");
        let ticks_per_zone = (zone::STEP_SECONDS * 60.0) as u64;

        let mut clean = RaceState::new(Mode::Zone);
        let mut dirty = RaceState::new(Mode::Zone);
        let mut perfect = (false, false);
        for tick in 0..=ticks_per_zone {
            perfect.0 |= clean
                .update(&course, position, tick, DT, false)
                .perfect_zone;
            // One wall contact anywhere in the zone spoils it.
            let hit = tick == 3;
            perfect.1 |= dirty.update(&course, position, tick, DT, hit).perfect_zone;
        }

        assert!(perfect.0, "a clean zone did not pay out");
        assert!(!perfect.1, "a zone with a wall contact paid out anyway");
        assert_eq!(
            clean.score - dirty.score,
            zone::CLEAN_ZONE_BONUS,
            "the bonus is not the recovered 500"
        );
    }

    #[test]
    fn the_dirty_flag_clears_at_each_zone_step() {
        let course = course();
        let position = course.position(0).expect("in range");
        let ticks_per_zone = (zone::STEP_SECONDS * 60.0) as u64;
        let mut state = RaceState::new(Mode::Zone);

        // Dirty the first zone only.
        state.update(&course, position, 0, DT, true);
        assert!(state.zone_dirty);
        for tick in 1..=ticks_per_zone {
            state.update(&course, position, tick, DT, false);
        }
        assert_eq!(state.zone, 1);
        assert!(!state.zone_dirty, "the flag survived the step");

        // The second zone is clean, so it pays.
        let mut paid = false;
        for tick in ticks_per_zone + 1..=ticks_per_zone * 2 + 1 {
            paid |= state
                .update(&course, position, tick, DT, false)
                .perfect_zone;
        }
        assert!(paid, "the zone after a dirty one could not be perfect");
    }

    #[test]
    fn a_new_race_starts_on_lap_one_with_no_best() {
        let state = RaceState::new(Mode::TimeTrial);
        assert_eq!(state.lap, 1);
        assert_eq!(state.laps_completed(), 0);
        assert_eq!(state.best_lap_ticks, None);
        assert_eq!(state.progress, None);
        assert!(!state.finished);
    }

    #[test]
    fn the_lap_target_comes_from_the_mode() {
        assert_eq!(RaceState::new(Mode::TimeTrial).laps_target, Some(3));
        assert_eq!(RaceState::new(Mode::SpeedLap).laps_target, None);
        assert_eq!(RaceState::new(Mode::Zone).laps_target, None);
    }

    #[test]
    fn the_default_is_a_time_trial() {
        assert_eq!(RaceState::default(), RaceState::new(Mode::TimeTrial));
    }

    #[test]
    fn a_lap_clock_before_its_own_start_reads_zero() {
        let mut state = RaceState::new(Mode::TimeTrial);
        state.lap_start_tick = 100;
        assert_eq!(state.lap_ticks(140), 40);
        assert_eq!(state.lap_ticks(100), 0);
        assert_eq!(state.lap_ticks(99), 0);
    }
}
