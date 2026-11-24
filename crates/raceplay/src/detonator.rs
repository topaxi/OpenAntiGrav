//! Detonator's mines and bombs in a race: the lap's survey, and laying each stage's objects.
//!
//! The placement law is `oag_weapons::detonator`; this builds the track data it needs and calls
//! it on the ticks the original does: once at the start, then on each stage step
//! (`docs/ghidra/functions/ps3-hdfury-eu/detonator-placement.md`, "When it runs").
//!
//! # The survey
//!
//! `Detonator_PlaceStageObjects` walks the lap once and measures each spline section with
//! collision sweeps. Its return codes (`0`, `2`, `4`, `5`, `127`) are unread, so the probes here
//! are **chosen, not measured**:
//!
//! - a horizontal sweep clamps a half-width where it meets any wall;
//! - a downward probe that meets no ground, or a `Reset` surface, blocks the lane under it;
//! - the same probe at the centre blanks the entry and the next `NumStepsToIgnoreAfterJump - 1`
//!   (the jump gap).
//!
//! Which half-width sits on which side of the lateral axis is taken from the original's order
//! (`sec+0x44` on the negative side) and not checked against a circuit.

use std::sync::Arc;

use oag_core::math::Vec3;
use oag_physics::{CollisionWorld, Ray, Raycaster, Surface};
use oag_tables::detonator_layout::GamePlayValues;
use oag_weapons::detonator::cannon::Sweep;
use oag_weapons::detonator::{Entry, Layout, MAX_LANES, Survey};

use crate::spline::Spline;

/// A lap's layout files and survey, shared by the race.
#[derive(Debug, Clone)]
pub(crate) struct Detonator {
    pub(crate) layout: Arc<Layout>,
    pub(crate) survey: Arc<Survey>,
    /// What a mine or a bomb costs a craft, from the Detonator weapon table; `None` where the
    /// table did not load, and then they are laid and harmless.
    pub(crate) contact: Option<oag_weapons::detonator::Contact>,
    /// The pulse cannon's tuning, from the same table and the layout files; `None` where the
    /// table did not load, and then the craft has no cannon.
    pub(crate) cannon: Option<oag_weapons::detonator::cannon::Config>,
    /// The EMP's tuning, from the table's `EMP` block; `None` where it authors none, and then
    /// nothing charges or fires.
    pub(crate) emp: Option<oag_weapons::detonator::emp::Config>,
    /// A speed pad charges the EMP in place of its boost ([`oag_title::DetonatorPads`]).
    pub(crate) speed_pads_charge: bool,
    /// A weapon pad charges the EMP in place of a pickup.
    pub(crate) weapon_pads_charge: bool,
}

/// How far above a section the probes start and how far below they reach.
const PROBE_UP: f32 = 8.0;
const PROBE_DOWN: f32 = 64.0;

/// Entries within this of the first are the start zone and take nothing.
const START_ZONE: f32 = 150.0;

/// How far across a section a carried mine may be from its nearest lane and still be snapped to it:
/// under one lane step.
const SNAP_RADIUS: f32 = 4.0;

/// How far a carried mine's height above a lane may differ from [`oag_weapons::detonator::MINE_LIFT`] (the
/// measured spread is 2.5 to 3.2): under one lane step, so a lane on another deck never takes the snap.
const LIFT_TOLERANCE: f32 = 1.5;

/// The share of the measured width the lanes fan across.
const USABLE_WIDTH: f32 = 0.9;

/// The survey of the lap, one entry per control point in driving order.
#[must_use]
pub(crate) fn survey(
    spline: &Spline,
    order: &[u32],
    collision: &CollisionWorld,
    values: &GamePlayValues,
) -> Survey {
    let lane_width = values.distance_offset_between_mines;
    let mut entries = Vec::new();
    let mut path = None;
    let mut within = 0usize;
    for &index in order {
        let this = spline.path_of(index as usize);
        if this != path {
            path = this;
            within = 0;
        }
        let at = within;
        within += 1;
        if !at.is_multiple_of(Spline::STEPS_PER_SEGMENT) {
            continue;
        }
        let Some(sample) = spline.sample(index as usize) else {
            continue;
        };
        entries.push(entry(sample, collision, lane_width));
    }
    blank_jumps(&mut entries, values.num_steps_to_ignore_after_jump);
    if let Some(first) = entries.first().map(|e| e.position) {
        for entry in &mut entries {
            if (entry.position - first).length() <= START_ZONE {
                entry.entry.usable = false;
            }
        }
    }
    Survey {
        entries: entries.into_iter().map(|e| e.entry).collect(),
    }
}

/// An entry and the section position it was measured at.
struct Measured {
    entry: Entry,
    position: Vec3,
    /// The centre probe found no ground: this entry starts a jump gap.
    jump: bool,
}

fn entry(sample: &oag_vex::track::Sample, collision: &CollisionWorld, lane_width: f32) -> Measured {
    let position = Vec3::from_array(sample.pos);
    let up = -Vec3::from_array(sample.down).normalize_or_zero();
    let lateral = Vec3::from_array(sample.lateral).normalize_or_zero();
    let start = position + up;
    let clamp = |direction: Vec3, width: f32| {
        collision
            .raycast(Ray::new(start, direction, width), None, false)
            .map_or(width, |hit| hit.distance.min(width))
    };
    let negative = clamp(-lateral, sample.half_width_left.max(0.0));
    let positive = clamp(lateral, sample.half_width_right.max(0.0));
    let width = USABLE_WIDTH * (negative + positive);
    let mut lanes = if lane_width > 0.0 {
        (width / lane_width) as i64
    } else {
        0
    };
    if lanes % 2 == 0 {
        lanes -= 1;
    }
    let usable = (1..=i64::from(MAX_LANES)).contains(&lanes);
    let centre = start - lateral * negative + lateral * (0.5 * width);
    let ground = |at: Vec3| {
        collision
            .raycast(
                Ray::new(at + up * PROBE_UP, -up, PROBE_UP + PROBE_DOWN),
                None,
                true,
            )
            .is_some_and(|hit| hit.surface != Surface::Reset)
    };
    let mut blocked = 0u32;
    if usable {
        let half = lanes as i32 / 2;
        for j in -half..=half {
            if !ground(centre + lateral * (lane_width * j as f32)) {
                blocked |= 1 << (j + half);
            }
        }
    }
    Measured {
        entry: Entry {
            usable,
            lanes: if usable { lanes as u8 } else { 0 },
            blocked,
            centre,
            lateral,
            up,
        },
        position,
        jump: usable && !ground(centre),
    }
}

/// A centre with no ground under it blanks that entry and the ones after it.
fn blank_jumps(entries: &mut [Measured], steps: i32) {
    let span = usize::try_from(steps.max(1)).unwrap_or(1);
    let mut blank_until = 0usize;
    for (index, measured) in entries.iter_mut().enumerate() {
        if measured.jump {
            blank_until = blank_until.max(index + span);
        }
        if index < blank_until {
            measured.entry.usable = false;
        }
    }
}

impl crate::sim::RaceSim {
    /// Where laid mine `index` sits, for tests that stand a craft on it.
    #[must_use]
    pub fn detonator_mine_position(&self, index: usize) -> Option<Vec3> {
        let detonator = self.detonator.as_ref()?;
        let mine = self.world.detonator.live_mines().get(index)?;
        let lane = detonator.layout.values.distance_offset_between_mines;
        detonator.survey.mine_point(mine, lane).map(|(at, _)| at)
    }

    /// Where a craft stands to shoot laid mine `index`: `distance` before it along the lap, with
    /// the lap's direction there and the section's up. For tests.
    #[must_use]
    pub fn detonator_mine_stand(&self, index: usize, distance: f32) -> Option<(Vec3, Vec3, Vec3)> {
        let detonator = self.detonator.as_ref()?;
        let mine = self.world.detonator.live_mines().get(index)?;
        let lane = detonator.layout.values.distance_offset_between_mines;
        let (at, up) = detonator.survey.mine_point(mine, lane)?;
        let entries = &detonator.survey.entries;
        let here = usize::from(mine.entry);
        let (from, to) = if here + 1 < entries.len() {
            (entries[here].centre, entries[here + 1].centre)
        } else {
            (entries[here - 1].centre, entries[here].centre)
        };
        let forward = (to - from).normalize_or_zero();
        Some((at - forward * distance, forward, up))
    }

    /// How much of Detonator's mines the cannon has cleared, as the end screen reports it
    /// (`destroyed / sum(MineCountStage1..14) * 100`), or `None` off Detonator.
    #[must_use]
    pub fn detonator_percentage(&self) -> Option<f32> {
        let detonator = self.detonator.as_ref()?;
        Some(oag_weapons::detonator::cannon::percentage(
            self.world.detonator.cannon.destroyed_mines,
            &detonator.layout.values,
        ))
    }

    /// The Detonator score the cannon has earned, or `None` off Detonator.
    #[must_use]
    pub fn detonator_score(&self) -> Option<i32> {
        self.detonator.as_ref()?;
        Some(self.world.detonator.cannon.score)
    }

    /// Mines the cannon has destroyed directly (`rm+0x6de3c`), or `None` off Detonator.
    #[must_use]
    pub fn detonator_destroyed_mines(&self) -> Option<u32> {
        self.detonator.as_ref()?;
        Some(self.world.detonator.cannon.destroyed_mines)
    }

    /// Bombs destroyed by the cannon or the EMP (`rm+0x6de40`), a count and not a percentage, or
    /// `None` off Detonator.
    #[must_use]
    pub fn detonator_destroyed_bombs(&self) -> Option<u32> {
        self.detonator.as_ref()?;
        Some(self.world.detonator.cannon.destroyed_bombs)
    }

    /// The streak awards paid over the race: fives, tens, fifteens (`rm+0x2de8`, `+0x2dea`,
    /// `+0x2dec`), or `None` off Detonator.
    #[must_use]
    pub fn detonator_chains(&self) -> Option<[u16; 3]> {
        self.detonator.as_ref()?;
        let cannon = &self.world.detonator.cannon;
        Some([cannon.fives, cannon.tens, cannon.fifteens])
    }

    /// Whether the person in `slot` may work the cannon this tick: flying it themselves,
    /// racing, and past the countdown. **Chosen, not measured:** the original's gate on the
    /// green light is unread.
    fn cannon_handed(&self, slot: usize) -> bool {
        self.world.controllers[slot].is_human()
            && self.may_use_pickup(slot)
            && !self.flown_for_the_player(slot)
            && !oag_race::RaceState::thrust_gated(self.world.tick)
    }

    /// The sounds this tick's [`Self::detonator_events`] make, queued as per-tick output
    /// (ADR-0018): `PulseBullet_Init` plays `CANNON` on the firing craft once per round, so a
    /// volley of two plays it twice; a round that dies on a wall plays `CANNONEXPLWALL` at the
    /// round (`detonator-cannon.md`, "The round"); a mine that goes off plays `MINEEXPL` and a
    /// bomb `BOMBEXPL` where it was (`detonator-mine.md`, `detonator-bomb.md`).
    pub(crate) fn detonator_cues(&mut self) {
        use oag_gameplay::cue::{Cue, CueEvent};
        use oag_weapons::detonator::EventKind;
        for event in &self.detonator_events {
            match event.kind {
                EventKind::Volley => {
                    for _ in 0..2 {
                        self.cues.push(CueEvent::new(Cue::Cannon, 0));
                    }
                }
                EventKind::RoundWall { .. } => {
                    self.cues
                        .push(CueEvent::at_point(Cue::CannonHitWall, event.at));
                }
                // An EMP'd mine and bomb go off through the same `DetonatorMine_Explode` and bomb
                // update as a shot one (`detonator-mine.md`, `detonator-bomb.md`).
                EventKind::MineTouched
                | EventKind::MineShot { .. }
                | EventKind::MineChained { .. }
                | EventKind::MineEmped { .. } => {
                    self.cues
                        .push(CueEvent::at_point(Cue::MineExplosion, event.at));
                }
                EventKind::BombTouched
                | EventKind::BombShot
                | EventKind::BombReplaced
                | EventKind::BombEmped => {
                    self.cues
                        .push(CueEvent::at_point(Cue::BombExplosion, event.at));
                }
                EventKind::EmpReady => self.cues.push(CueEvent::new(Cue::EmpReady, 0)),
                EventKind::EmpFull => self.cues.push(CueEvent::new(Cue::EmpFull, 0)),
                // `EMP_Launch` plays `QUAKELAUNCH` on the firing ship's emitter.
                EventKind::EmpLaunch => self.cues.push(CueEvent::new(Cue::QuakeLaunch, 0)),
            }
        }
    }

    /// Puts the opening stage's mines where the original's were, all live, in place of the seeded layout
    /// (`--detonator-mines`): each position is snapped to the free lane of ours' survey nearest to it
    /// across the section (the offset along the section's up axis is the mine's hover height and is not
    /// compared). The stage is laid first so the bomb and the stage counter are the race's own. Returns how
    /// many positions had no lane within [`SNAP_RADIUS`] across and [`LIFT_TOLERANCE`] of the mine's height, or
    /// a lane another carried mine or the bomb already holds, and were dropped. Nothing happens off Detonator.
    ///
    /// A comparison against the original carries its mines this way: the placer's `Libc_Rand` stream is not
    /// carried, and the original numbers its entries from the player's start cursor where ours starts at the
    /// spline's origin (7 apart on Talon's Junction), so a mine's own entry number does not transfer; its
    /// position does (`docs/tools/compare-page.md`, "Detonator pairs").
    pub fn carry_detonator_mines(&mut self, positions: &[Vec3]) -> usize {
        use oag_weapons::detonator::{MAX_MINES, MINE_LIFT, Placed};
        let Some(detonator) = self.detonator.clone() else {
            return 0;
        };
        if self.world.detonator.stages_laid == 0 {
            let _ = oag_weapons::detonator::place_stage(
                &mut self.world.detonator,
                &detonator.survey,
                &detonator.layout,
                &mut self.world.rng,
                0,
            );
        }
        let lane_width = detonator.layout.values.distance_offset_between_mines;
        let bombs: Vec<Placed> = self.world.detonator.live_bombs().to_vec();
        let mut laid: Vec<Placed> = Vec::new();
        let mut dropped = 0;
        for &position in positions {
            let mut best: Option<(f32, Placed)> = None;
            for (index, entry) in detonator.survey.entries.iter().enumerate() {
                if !entry.usable {
                    continue;
                }
                for j in 0..entry.lanes {
                    if entry.blocked & (1 << j) != 0 {
                        continue;
                    }
                    let placed = Placed {
                        entry: index as u16,
                        lane: j as i8 - (entry.lanes / 2) as i8,
                        live: true,
                    };
                    let Some((at, up)) = detonator.survey.point(&placed, lane_width) else {
                        continue;
                    };
                    let apart = position - at;
                    let along = apart.dot(up);
                    // A lane on another deck (an overpass) has the right place across and the wrong height.
                    if (along - MINE_LIFT).abs() > LIFT_TOLERANCE {
                        continue;
                    }
                    let across = (apart - up * along).length();
                    if best.is_none_or(|(nearest, _)| across < nearest) {
                        best = Some((across, placed));
                    }
                }
            }
            match best {
                Some((across, placed))
                    if across <= SNAP_RADIUS
                        && laid.len() < MAX_MINES
                        && !laid
                            .iter()
                            .any(|l| (l.entry, l.lane) == (placed.entry, placed.lane))
                        && !bombs
                            .iter()
                            .any(|b| (b.entry, b.lane) == (placed.entry, placed.lane)) =>
                {
                    laid.push(placed);
                }
                _ => dropped += 1,
            }
        }
        let field = &mut self.world.detonator;
        for (slot, placed) in field.mines.iter_mut().zip(&laid) {
            *slot = *placed;
        }
        field.live_mines = laid.len() as u16;
        dropped
    }

    /// Lays the stages the player has reached: the opening one on the first tick, then one per
    /// lap, up to the fourteenth (`Detonator_UpdateRace` places while the new stage is at most
    /// `14`). Nothing happens off Detonator.
    pub(crate) fn advance_detonator(&mut self, inputs: &oag_gameplay::PlayerInputs) {
        self.detonator_events.clear();
        let Some(detonator) = self.detonator.clone() else {
            return;
        };
        let world = &mut self.world;
        if let Some(contact) = &detonator.contact {
            // **Chosen, not measured:** weapons-on damage rules. The original halves every hit
            // while `g_GameState+0x18` is clear, a byte read here as the weapons switch at
            // confidence 60, and Detonator's campaign cells author `Weapons="on"`
            // (`detonator-mine.md`, "Damage").
            let rules = oag_physics::DamageRules {
                weapons: true,
                ..oag_gameplay::damage_rules(world.mode())
            };
            let count = usize::from(world.ship_count);
            let lane_width = detonator.layout.values.distance_offset_between_mines;
            self.detonator_events = oag_weapons::detonator::step_contacts(
                &mut world.detonator,
                &detonator.survey,
                lane_width,
                contact,
                &mut world.ships[..count],
                rules,
            );
        }
        if let Some(config) = &detonator.cannon {
            // Square fires and Circle reloads, the primary slot only (the one slot the EMP's
            // charge and fire use). **Chosen:** the reload is a press edge; the original's
            // edge-or-level is unread (`detonator-cannon.md`, "The reload").
            let slot = self.world.primary_slot();
            let handed = self.cannon_handed(slot);
            let pad = inputs.get(slot).buttons;
            let input = oag_weapons::detonator::cannon::Input {
                fire: handed && pad.is_held(oag_gameplay::input::Button::Square),
                reload: handed && pad.is_pressed(oag_gameplay::input::Button::Circle),
            };
            let collision = &self.collision;
            let sweep = |from, to| cannon_sweep(collision, from, to);
            let (field, owner) = (&mut self.world.detonator, &self.world.ships[slot]);
            let fired = oag_weapons::detonator::cannon::step_cannon(
                field,
                &detonator.survey,
                config,
                input,
                owner,
                self.dt,
                &sweep,
            );
            self.detonator_events.extend(fired);
        }
        if let Some(config) = &detonator.emp {
            self.advance_emp(&detonator, config, inputs);
        }
        let world = &mut self.world;
        let laps = world.race[world.primary_slot()].laps_completed();
        let stage = oag_race::Mode::detonator_stage(laps).min(STAGES_LAID);
        while u32::from(world.detonator.stages_laid) < stage {
            let old = u32::from(world.detonator.stages_laid);
            let replaced = oag_weapons::detonator::place_stage(
                &mut world.detonator,
                &detonator.survey,
                &detonator.layout,
                &mut world.rng,
                old,
            );
            self.detonator_events.extend(replaced);
        }
        self.detonator_cues();
    }
}

impl crate::sim::RaceSim {
    /// The EMP's tick: spend the charge a pad raised, fire on the EMP button, run the wave.
    ///
    /// The button is L1 (`Ship_UpdateCannonTrigger` reads the controls' `+0x1c`, live), which
    /// the pad raises as [`oag_gameplay::input::Button::Flip`]. **Chosen:** a press edge; the
    /// original's edge-or-level is the controls' and unread (`detonator-emp.md`, "Fire").
    fn advance_emp(
        &mut self,
        detonator: &Detonator,
        config: &oag_weapons::detonator::emp::Config,
        inputs: &oag_gameplay::PlayerInputs,
    ) {
        use oag_weapons::detonator::emp;
        let charged = emp::step_charge(&mut self.world.detonator, config);
        self.detonator_events.extend(charged);
        let slot = self.world.primary_slot();
        let pressed = inputs
            .get(slot)
            .buttons
            .is_pressed(oag_gameplay::input::Button::Flip);
        if pressed && self.cannon_handed(slot) {
            let at = self.world.ships[slot].physics.body.position;
            let from = nearest_entry(&detonator.survey, at);
            let fired = emp::fire(&mut self.world.detonator, &detonator.survey, config, from);
            self.detonator_events.extend(fired);
        }
        let swept = emp::step_wave(
            &mut self.world.detonator,
            &detonator.survey,
            config,
            self.dt,
        );
        self.detonator_events.extend(swept);
    }

    /// Detonator's auto-speed target for `slot` this tick, or `None` off Detonator. Steps the
    /// ramp once a tick for the player craft, then reads [`Self::detonator_speed_target`]; any
    /// other slot flies the whole target.
    pub(crate) fn detonator_auto_speed(&mut self, slot: usize) -> Option<f32> {
        self.detonator.as_ref()?;
        self.zone?;
        if slot != self.world.primary_slot() {
            return Some(self.unramped_target());
        }
        let ramp = oag_weapons::detonator::emp::step_ramp(&mut self.world.detonator.ramp, self.dt);
        Some(ramp * self.unramped_target())
    }

    /// The engine target the player craft flies at, or `None` off Detonator or without the
    /// `<Zone>` numbers: Zone's `start + increment * n` with the Detonator stage as `n` (`0` on
    /// lap 1, then the HUD stage `2..14`), times the ramp as it stands
    /// (`docs/ghidra/functions/ps3-hdfury-eu/detonator-speed.md`). The craft adds the shared
    /// tail (the doubling) on top.
    #[must_use]
    pub fn detonator_speed_target(&self) -> Option<f32> {
        self.detonator.as_ref()?;
        self.zone?;
        let r = self.world.detonator.ramp;
        let ramp = if r >= 1.0 {
            1.0
        } else {
            oag_weapons::detonator::emp::ramp_of(r)
        };
        Some(ramp * self.unramped_target())
    }

    fn unramped_target(&self) -> f32 {
        let Some(zone) = self.zone else {
            return 0.0;
        };
        let stage = u32::from(self.world.detonator.stages_laid);
        let n = if stage <= 1 {
            0
        } else {
            stage.min(STAGES_LAID)
        };
        oag_race::zone::thrust(zone.start, zone.increment, n as u16)
    }

    /// A pad entry by `slot` that charges the EMP: raises the charge flag the next EMP tick
    /// spends. Only the player craft charges (`Detonator_UpdateRace` reads its own ship).
    pub(crate) fn charge_emp(&mut self, slot: usize) {
        if slot == self.world.primary_slot() {
            self.world.detonator.emp.pending = true;
        }
    }
}

/// The survey entry nearest `at`: where the EMP's wave starts.
fn nearest_entry(survey: &Survey, at: Vec3) -> u16 {
    survey
        .entries
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| {
            (a.centre - at)
                .length_squared()
                .total_cmp(&(b.centre - at).length_squared())
        })
        .and_then(|(index, _)| u16::try_from(index).ok())
        .unwrap_or(0)
}

/// What a round's segment meets in the track: a floor to ride, a wall to bounce off, or nothing.
/// A `Reset` surface is ignored, as every ordinary segment query ignores it.
fn cannon_sweep(collision: &CollisionWorld, from: Vec3, to: Vec3) -> Sweep {
    let along = to - from;
    let length = along.length();
    if length <= 0.0 {
        return Sweep::None;
    }
    let Some(hit) = collision.raycast(Ray::new(from, along / length, length), None, false) else {
        return Sweep::None;
    };
    match hit.surface {
        Surface::Wall => Sweep::Wall {
            point: hit.point,
            normal: hit.normal,
        },
        Surface::Floor | Surface::MagFloor => Sweep::Floor {
            point: hit.point,
            normal: hit.normal,
        },
        Surface::Reset => Sweep::None,
    }
}

/// The last stage that lays objects.
const STAGES_LAID: u32 = 14;

impl crate::sim::RaceSim {
    /// The pulse cannon's state, or `None` off Detonator.
    #[must_use]
    pub fn detonator_cannon(&self) -> Option<&oag_weapons::detonator::cannon::Cannon> {
        self.detonator.as_ref()?;
        Some(&self.world.detonator.cannon)
    }

    /// The EMP's charge and whether it has reached the threshold the EMP fires at, or `None`
    /// off Detonator. The HUD's `Emp_0..7` pips and `EmpReady` read them.
    #[must_use]
    pub fn detonator_emp(&self) -> Option<(u32, bool)> {
        let detonator = self.detonator.as_ref()?;
        let emp = &self.world.detonator.emp;
        Some((
            emp.charge,
            detonator
                .emp
                .as_ref()
                .is_some_and(|config| emp.ready(config)),
        ))
    }

    /// The EMP's gauge, `0..=1` (`craft+0x7da0`), or `None` off Detonator: what the HUD's
    /// `EmpBarEffect` would read, had the layout reader a model for it.
    #[must_use]
    pub fn detonator_emp_gauge(&self) -> Option<f32> {
        self.detonator.as_ref()?;
        Some(self.world.detonator.emp.gauge)
    }

    /// The stage the HUD counts: stages laid, which is `1` at the opening stage and one more per
    /// lap up to the fourteenth (`detonator-hud.md`, "The stage counter"), or `None` off
    /// Detonator.
    #[must_use]
    pub fn detonator_stage(&self) -> Option<u32> {
        self.detonator.as_ref()?;
        Some(u32::from(self.world.detonator.stages_laid).clamp(1, STAGES_LAID))
    }
}
