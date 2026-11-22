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
use oag_weapons::detonator::{Entry, Layout, MAX_LANES, Survey};

use crate::spline::Spline;

/// A lap's layout files and survey, shared by the race.
#[derive(Debug, Clone)]
pub(crate) struct Detonator {
    pub(crate) layout: Arc<Layout>,
    pub(crate) survey: Arc<Survey>,
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
    /// Lays the stages the player has reached: the opening one on the first tick, then one per
    /// lap, up to the fourteenth (`Detonator_UpdateRace` places while the new stage is at most
    /// `14`). Nothing happens off Detonator.
    pub(crate) fn advance_detonator(&mut self) {
        let Some(detonator) = self.detonator.clone() else {
            return;
        };
        let world = &mut self.world;
        let laps = world.race[world.primary_slot()].laps_completed();
        let stage = oag_race::Mode::detonator_stage(laps).min(STAGES_LAID);
        while u32::from(world.detonator.stages_laid) < stage {
            let old = u32::from(world.detonator.stages_laid);
            oag_weapons::detonator::place_stage(
                &mut world.detonator,
                &detonator.survey,
                &detonator.layout,
                &mut world.rng,
                old,
            );
        }
    }
}

/// The last stage that lays objects.
const STAGES_LAID: u32 = 14;
