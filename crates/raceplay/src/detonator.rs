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
}

/// How far above a section the probes start and how far below they reach.
const PROBE_UP: f32 = 8.0;
const PROBE_DOWN: f32 = 64.0;

/// Entries within this of the first are the start zone and take nothing.
const START_ZONE: f32 = 150.0;

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
        detonator.survey.point(mine, lane).map(|(at, _)| at)
    }

    /// Where a craft stands to shoot laid mine `index`: `distance` before it along the lap, with
    /// the lap's direction there and the section's up. For tests.
    #[must_use]
    pub fn detonator_mine_stand(&self, index: usize, distance: f32) -> Option<(Vec3, Vec3, Vec3)> {
        let detonator = self.detonator.as_ref()?;
        let mine = self.world.detonator.live_mines().get(index)?;
        let lane = detonator.layout.values.distance_offset_between_mines;
        let (at, up) = detonator.survey.point(mine, lane)?;
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

    /// Whether the person in slot 0 may work the cannon this tick: flying it themselves,
    /// racing, and past the countdown. **Chosen, not measured:** the original's gate on the
    /// green light is unread.
    fn cannon_handed(&self) -> bool {
        const SLOT: usize = 0;
        self.world.controllers[SLOT].is_human()
            && self.may_use_pickup(SLOT)
            && !self.flown_for_the_player(SLOT)
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
                EventKind::MineTouched
                | EventKind::MineShot { .. }
                | EventKind::MineChained { .. } => {
                    self.cues
                        .push(CueEvent::at_point(Cue::MineExplosion, event.at));
                }
                EventKind::BombTouched | EventKind::BombShot | EventKind::BombReplaced => {
                    self.cues
                        .push(CueEvent::at_point(Cue::BombExplosion, event.at));
                }
            }
        }
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
            // Square fires and Circle reloads, slot 0 only. **Chosen:** the reload is a press
            // edge; the original's edge-or-level is unread (`detonator-cannon.md`, "The reload").
            let handed = self.cannon_handed();
            let pad = inputs.get(0).buttons;
            let input = oag_weapons::detonator::cannon::Input {
                fire: handed && pad.is_held(oag_gameplay::input::Button::Square),
                reload: handed && pad.is_pressed(oag_gameplay::input::Button::Circle),
            };
            let collision = &self.collision;
            let sweep = |from, to| cannon_sweep(collision, from, to);
            let (field, owner) = (&mut self.world.detonator, &self.world.ships[0]);
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
