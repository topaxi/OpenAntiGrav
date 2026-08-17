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
/// Sized for the single-ship modes, and it stays that way on purpose. Everything
/// that is *per craft* lives on [`crate::Standing`] instead - the lap count, the
/// place on the circuit, the finish, and since 2026-08-17 the lap clock and best
/// lap as well. This struct keeps what belongs to the player's race rather than
/// to a craft: the mode, the Zone counters and the finish condition.
///
/// The two therefore hold the same clock for slot 0, and
/// `standing::tests::the_standings_clock_agrees_with_the_players` is what keeps
/// them from drifting apart.
/// Whether a one-tick change in distance-along is a **forward** wrap of the ring.
///
/// A lap is a wrap, not a plane crossing. Anything that moves more than half the
/// circuit in one tick has wrapped rather than travelled: at 60 Hz even a
/// Phantom-class craft covers a few units per tick against a circuit thousands of
/// units round.
///
/// Shared with [`crate::standing`] so the field and the player count laps by one
/// rule. Two counters that disagreed would put a craft in a position it is not
/// in.
#[must_use]
pub fn wrapped_forward(delta: f32, half_length: f32) -> bool {
    delta < -half_length
}

/// The same test for a craft that has gone backwards over the line.
#[must_use]
pub fn wrapped_backward(delta: f32, half_length: f32) -> bool {
    delta > half_length
}

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
    /// Only a time trial ever sets this today. Speed lap has no end. **Zone's
    /// end condition is recovered** - a run ends when the ship is destroyed,
    /// which the disc says outright in `ER_ZONE_DEST` ("Ship destroyed on
    /// zone"), confidence 80. It is unimplemented because nothing depletes
    /// shield yet, so what blocks it is collision damage rather than reverse
    /// engineering. See `docs/gameplay/race-modes.md`.
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
    /// End the race because the craft was destroyed.
    ///
    /// **This is what Zone has been missing**, and it is the original's own
    /// ending rather than an invention: `Zone_UpdateRacing` moves the mode out
    /// of its racing state on bit 12 (`0x1000`) of `entity+0x860`, and
    /// `Ship_SetState`'s case 5 is what sets that bit - reached half a second
    /// after `Ship_Damage` drives the energy pool to zero. The disc's own
    /// `ER_ZONE_DEST` reads *"Ship destroyed on zone"*. See
    /// [`zone-mode.md`](../../../docs/ghidra/functions/psp-pulse-usa/zone-mode.md)
    /// and [`shield.md`](../../../docs/ghidra/functions/psp-pulse-usa/shield.md).
    ///
    /// A method rather than a sixth argument to [`Self::update`] because that is
    /// the shape the original has: the mode *watches a flag on the craft* rather
    /// than being told each tick whether one is set. The caller does the same -
    /// it holds the craft and this crate deliberately does not
    /// (`oag-race`'s `Cargo.toml` says why it cannot see `oag-physics`).
    ///
    /// Returns whether this call is the one that ended the race, so a caller can
    /// fire a sound or a screen on the edge. Idempotent: calling it every tick
    /// of a wrecked craft returns `true` once.
    ///
    /// **Mode-agnostic on purpose.** Only Zone can reach it in practice, because
    /// a time trial and a speed lap race with the original's `Damage` option off
    /// and their pool floors at 20 - see `oag_gameplay::damage_rules`. Gating it
    /// on the mode would encode that twice, and wrongly the day a mode with
    /// damage on arrives.
    pub fn eliminate(&mut self) -> bool {
        if self.finished {
            return false;
        }
        self.finished = true;
        true
    }

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
        if wrapped_forward(delta, half) {
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
        } else if wrapped_backward(delta, half) {
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
mod tests;

#[cfg(test)]
mod elimination_tests {
    use super::RaceState;
    use crate::Mode;

    /// The edge, which is what a caller hangs a sound or an end screen on.
    #[test]
    fn eliminating_a_craft_ends_the_race_once() {
        let mut state = RaceState::new(Mode::Zone);
        assert!(!state.finished);
        assert!(state.eliminate(), "the first call should end the race");
        assert!(state.finished);
        assert!(
            !state.eliminate(),
            "a wrecked craft calling every tick must not re-fire"
        );
    }

    /// A race already over for another reason is not ended twice.
    #[test]
    fn a_finished_race_is_not_eliminated_again() {
        let mut state = RaceState::new(Mode::TimeTrial);
        state.finished = true;
        assert!(!state.eliminate());
    }
}
