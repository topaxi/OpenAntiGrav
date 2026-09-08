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

/// How long the start-line countdown gates thrust, in ticks at the fixed 60 Hz
/// timestep.
///
/// **Measured, not authored.** Three live PPSSPP captures of `pulse-psp-usa.chd`
/// (Time Trial, Talon's Junction White) agree to the tick: `cross` held
/// continuously from before the track description screen through the whole
/// countdown, `throttleState` (`craft+0x2b8`) reads a flat `0.0` for 272 ticks
/// after the track description screen is dismissed and then steps straight to
/// the full held value - no ramp, no stall. See
/// `docs/gameplay/race-modes.md#the-countdown-is-measured`.
///
/// **Scope of the measurement, and of this constant.** Only thrust was held and
/// recorded, so only thrust is gated here - steering, braking and the airbrakes
/// are unmeasured through a countdown and this crate does not touch them. Only
/// Time Trial was captured; applied to every mode here for the same reason
/// [`RaceState::eliminate`] is mode-agnostic - a mode-gated countdown would be a
/// second, unverified claim (that another mode differs) rather than the
/// measured one. A grid race's countdown, Zone's own timing, and every other
/// title are still open - see the doc link above for exactly what "measured"
/// covers.
pub const COUNTDOWN_TICKS: u64 = 272;

/// How many completed laps [`RaceState::lap_splits`] and [`crate::Standing::lap_splits`]
/// keep a time for.
///
/// **Read off the authored data, not chosen.** HD/Fury's `HUD_lap_times.xml`
/// composes exactly four rows - `Lap1Image`-`Lap4Image`, each with its own
/// `Lap{n}Text`/`Lap{n}Time` pair - and no other shipped layout on any of the
/// four titles authors a fifth. A fixed array of this length is therefore the
/// same "no `Vec`, fixed-size arrays" shape [ADR-0003] already requires of
/// `World`, sized to the one consumer that exists rather than to a guess at how
/// long a race could run. See `docs/formats/hd-hud.md`.
///
/// Both of this crate's current lap targets ([`Mode::TIME_TRIAL_LAPS`],
/// [`Mode::SINGLE_RACE_LAPS`]) are `3`, so every bounded mode today fits with a
/// row to spare; a lap beyond the fourth - reachable only in an unbounded mode
/// like Speed Lap or Zone, or a longer configured race once one exists - simply
/// stops being recorded, the same way the original's own four rows would have
/// nowhere left to draw it.
///
/// [ADR-0003]: ../../../docs/architecture/adr/0003-no-ecs.md
pub const MAX_RECORDED_LAPS: usize = 4;

/// Records `ticks` as the time for lap `completed_lap`, if it fits.
///
/// Shared by [`RaceState::complete_lap`] and [`crate::Standing::update`] so the
/// bound and the indexing rule live in one place - the same reasoning
/// `wrapped_forward` and `LapGate` are shared for. `completed_lap` is 1-based,
/// matching [`RaceState::lap`] before it advances past the lap just finished.
///
/// **Overwrites rather than keeping the first or the best.** A lap only
/// records a second time at all after a backward crossing re-earns it
/// ([`RaceState::uncomplete_lap`]), which is already a wrong-way situation the
/// HUD warns about; showing the time from the drive that actually finished the
/// lap last is the reading with no data-side signal against it, kept for
/// symmetry with how [`RaceState::best_lap_ticks`] is threaded rather than
/// measured off the original.
pub(crate) fn record_split(
    splits: &mut [Option<u32>; MAX_RECORDED_LAPS],
    completed_lap: u32,
    ticks: u32,
) {
    if let Some(index) = (completed_lap as usize).checked_sub(1)
        && index < MAX_RECORDED_LAPS
    {
        splits[index] = Some(ticks);
    }
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
    /// Each completed lap's own time, in ticks, indexed by `lap - 1`.
    ///
    /// `None` for a lap not yet completed. Bounded at [`MAX_RECORDED_LAPS`] -
    /// see that constant for why - rather than a `Vec`, per [ADR-0003].
    ///
    /// [ADR-0003]: ../../../docs/architecture/adr/0003-no-ecs.md
    pub lap_splits: [Option<u32>; MAX_RECORDED_LAPS],
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
            lap_splits: [None; MAX_RECORDED_LAPS],
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
    /// **Mode-agnostic on purpose.** `oag_gameplay::damage_rules` clears the
    /// original's `Damage` option for a time trial and a speed lap, which floors
    /// their pool at 20, so those two cannot reach it. **Zone and a single race
    /// both can** - this doc used to say only Zone did, which was true only
    /// while nothing shot at the player. It stopped being true when the AI
    /// gained weapons on 2026-08-26 and nobody noticed, because the two tests
    /// that would have caught it are `#[ignore]`d and disc-backed. Gating this
    /// on the mode would encode the rules table twice.
    ///
    /// **Ending the race is the right answer for a single race too**, settled
    /// 2026-09-02 at confidence 75 off the disc's own text: the results
    /// screen's field list gives each scoreboard row one of three statuses -
    /// `ER_RACING` "Racing", `ER_SHIP_DES` "Ship destroyed", or
    /// `ER_1STP`..`ER_8THP` "1st place".."8th place". A destroyed craft takes
    /// the second *instead of* a place, so it does not come back to earn one.
    /// Respawn belongs to Eliminator, which is the mode with an `ER_DEATHS`
    /// "Deaths:" row. Full argument in `docs/gameplay/race-modes.md`.
    pub fn eliminate(&mut self) -> bool {
        if self.finished {
            return false;
        }
        self.finished = true;
        true
    }

    /// Ends an Eliminator event once *any* craft's kill count reaches
    /// `target`.
    ///
    /// **Both arguments are the caller's to supply, deliberately.** `target`
    /// is a `PI_Cell`'s own gold-medal figure on the real campaign grid -
    /// `10`, `7` or `5`, never one flat number - so this crate does not
    /// choose it; see [`Mode::ELIMINATOR_KILL_TARGET_DEFAULT`] for what a
    /// caller with no cell to read falls back to. `kills` is the **highest**
    /// [`crate::Standing::kills`] across the whole field, not only the
    /// player's own - `progress`'s Ghidra pass reads the original's own
    /// ending off `entity+0x8d8` on any craft, so the first ship to the
    /// target ends the event whichever one it is. Neither number is
    /// discovered here, which is what keeps this crate from needing to know
    /// which slot is the player, or which slot is a `PI_Cell`.
    ///
    /// A no-op outside Eliminator - callers checking `self.mode` first is a
    /// second, redundant guard against ending a race that has no kill count
    /// to reach, not a load-bearing one, since a non-Eliminator race's `kills`
    /// stays `0` for the whole race and never reaches a positive target.
    ///
    /// Same shape as [`Self::eliminate`]: idempotent, returns whether *this*
    /// call is the one that ended it.
    pub fn eliminator_finished(&mut self, target: u32, kills: u32) -> bool {
        if self.finished || self.mode != Mode::Eliminator || kills < target {
            return false;
        }
        self.finished = true;
        true
    }

    /// Whether the start-line countdown still gates thrust at `tick`.
    ///
    /// `tick` is the caller's own "ticks elapsed since the race began" counter
    /// (`World::tick` in `oag_gameplay`, which this crate does not depend on) -
    /// already zero at the standing start, so this needs no state of its own
    /// and takes the tick rather than reading a field, the same shape
    /// [`Self::lap_ticks`] has. See [`COUNTDOWN_TICKS`] for what is and is not
    /// measured about it.
    #[must_use]
    pub fn thrust_gated(tick: u64) -> bool {
        tick < COUNTDOWN_TICKS
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
        record_split(&mut self.lap_splits, self.lap, ticks);
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

    #[test]
    fn eliminator_ends_when_a_craft_reaches_the_given_target() {
        let target = 7; // one of the real, non-flat cell values - see `Mode`.
        let mut state = RaceState::new(Mode::Eliminator);
        assert!(!state.eliminator_finished(target, target - 1));
        assert!(!state.finished);
        assert!(state.eliminator_finished(target, target));
        assert!(state.finished);
        assert!(
            !state.eliminator_finished(target, target + 1),
            "already finished, must not re-fire"
        );
    }

    /// A kill count means nothing outside Eliminator - the field stays `0`
    /// there and nothing feeds it, but the method itself must refuse too, so
    /// a future caller cannot end the wrong mode by passing a stray number.
    #[test]
    fn a_kill_count_does_not_end_a_different_mode() {
        let mut state = RaceState::new(Mode::SingleRace);
        assert!(!state.eliminator_finished(10, 10));
        assert!(!state.finished);
    }
}
