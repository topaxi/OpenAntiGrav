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
use oag_race::Course;
use oag_tables::detonator_layout::GamePlayValues;
use oag_weapons::detonator::cannon::Sweep;
use oag_weapons::detonator::{Entry, Layout, MAX_LANES, Partner, Survey, SurveyBranch};

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

/// The survey of the lap, one entry per control point in driving order, and of each alternate
/// path `course` branches onto, with the lap array placement lays into
/// (`docs/ghidra/functions/ps3-hdfury-eu/detonator-placement.md`, "The fork walk").
#[must_use]
pub(crate) fn survey(
    spline: &Spline,
    order: &[u32],
    course: Option<&Course>,
    collision: &CollisionWorld,
    values: &GamePlayValues,
) -> Survey {
    let lane_width = values.distance_offset_between_mines;
    let mut all = measure(spline, order, collision, lane_width);
    let ring = all.len();
    let mut branches = Vec::new();
    let mut forks = Vec::new();
    for branch in course.into_iter().flat_map(Course::branches) {
        let along: Vec<u32> = (0..spline.len() as u32)
            .filter(|&index| spline.path_of(index as usize) == Some(branch.path))
            .collect();
        let measured = measure(spline, &along, collision, lane_width);
        let at = |ring_index: usize| {
            u16::try_from(ring_index / Spline::STEPS_PER_SEGMENT)
                .ok()
                .filter(|&e| usize::from(e) < ring)
        };
        let (Some(split), Some(merge)) = (at(branch.split), at(branch.merge)) else {
            continue;
        };
        if measured.is_empty() {
            continue;
        }
        let first = all.len();
        let primary = all[usize::from(split)].path;
        let len = measured.len();
        let kept = separated(&all[..ring], primary, &measured);
        forks.push(Fork {
            split: usize::from(split),
            merge: usize::from(merge),
            alternate: kept.into_iter().map(|k| first + k).collect(),
        });
        let (to_primary, from_primary) =
            siblings(&all[..ring], primary, usize::from(split), &measured);
        branches.push((split, merge, branch.path, len, to_primary, from_primary));
        all.extend(measured);
    }
    let lap = lap_order(ring, &mut forks);
    blank_jumps(&mut all, &lap, values.num_steps_to_ignore_after_jump);
    if let Some(first) = all.first().map(|e| e.position) {
        for entry in &mut all {
            if (entry.position - first).length() <= START_ZONE {
                entry.entry.usable = false;
            }
        }
    }
    let mut entries = all.into_iter().map(|e| e.entry);
    let ring_entries: Vec<Entry> = entries.by_ref().take(ring).collect();
    let branches = branches
        .into_iter()
        .map(
            |(split, merge, path, len, to_primary, from_primary)| SurveyBranch {
                split,
                merge,
                entries: entries.by_ref().take(len).collect(),
                path,
                to_primary,
                from_primary,
            },
        )
        .collect();
    let lap = if lap.iter().copied().eq(0..ring) {
        Vec::new()
    } else {
        lap.into_iter()
            .filter_map(|index| u16::try_from(index).ok())
            .collect()
    };
    Survey {
        entries: ring_entries,
        branches,
        lap,
    }
}

/// One fork as the lap array takes it: the ring entries where the routes part and meet again,
/// and the survey indices of the alternate's sections that enter.
struct Fork {
    split: usize,
    merge: usize,
    alternate: Vec<usize>,
}

/// The lap array in the original's walk order. At each fork the walk steps onto the primary
/// route with selector `2` and stores every section after the first up to the merge, steps
/// onto the alternate with selector `3` and stores its separated sections after the first,
/// then passes the merge with selector `1`; the first section past the merge is stored as
/// usual (`0x0006675c`-`0x00066b38` counting, `0x00066300`-`0x00066758` storing). A merge at
/// the ring's first entry closes the lap there; a fork that otherwise wraps the spline's origin
/// is left to the ring.
fn lap_order(ring: usize, forks: &mut [Fork]) -> Vec<usize> {
    for fork in forks.iter_mut() {
        if fork.merge == 0 {
            fork.merge = ring;
        }
    }
    forks.sort_by_key(|f| f.split);
    let mut lap = Vec::with_capacity(ring);
    let mut next = 0usize;
    for fork in forks.iter() {
        if fork.split < next || fork.split >= fork.merge || fork.merge > ring {
            continue;
        }
        lap.extend(next..fork.split);
        lap.extend(fork.split + 1..fork.merge);
        lap.extend(fork.alternate.iter().copied());
        next = fork.merge;
    }
    lap.extend(next..ring);
    lap
}

/// How far `Track_FindSiblingSection` searches the other route (`0x000a96d8`).
const SIBLING_SEARCH: f32 = 500.0;

/// Each route's partner on the other (`oag_weapons::detonator::Partner`): per branch section the
/// primary route's nearest ring entry, and per primary section (counted from `split`) the
/// branch's nearest offset, marked where it lies within the launch fork's reach.
///
/// The route is compared section position to section position, as [`separated`] reads
/// `0x000a96d8`. `0x000a5290`, the two spans' neighbour listing, is taken as always.
fn siblings(
    ring: &[Measured],
    primary: Option<u16>,
    split: usize,
    alternate: &[Measured],
) -> (Vec<Option<Partner>>, Vec<Option<Partner>>) {
    let total = ring.len();
    let count = ring.iter().filter(|m| m.path == primary).count();
    let route: Vec<usize> = (0..count).map(|o| (split + o) % total.max(1)).collect();
    let reach = oag_weapons::detonator::emp::SIBLING_REACH;
    let nearest = |at: Vec3, among: &mut dyn Iterator<Item = (usize, Vec3)>| {
        among
            .map(|(index, p)| (index, (p - at).length_squared()))
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .filter(|&(_, d)| d < SIBLING_SEARCH * SIBLING_SEARCH)
            .and_then(|(index, d)| {
                Some(Partner {
                    at: u16::try_from(index).ok()?,
                    near: d < reach * reach,
                })
            })
    };
    let to_primary = alternate
        .iter()
        .map(|a| {
            nearest(
                a.position,
                &mut route.iter().map(|&i| (i, ring[i].position)),
            )
        })
        .collect();
    let from_primary = route
        .iter()
        .map(|&i| {
            nearest(
                ring[i].position,
                &mut alternate.iter().enumerate().map(|(k, m)| (k, m.position)),
            )
        })
        .collect();
    (to_primary, from_primary)
}

/// Which of an alternate route's sections enter the lap array: from the second on, those whose
/// lane centre lies further from the nearest section of the primary route than the two
/// sections' half-widths together, so where the routes still overlap nothing is stored twice.
///
/// `0x00066430`-`0x000666c0`: `0x000a96d8` finds the section of the sibling span nearest the
/// alternate's, `0x000a5290` checks the two spans are listed as neighbours (taken as always
/// here), and the vector block compares `|cA - cB|` against `hA + hB`, with
/// `c = pos - v30 * w44 + v30 * 0.5 * (w44 + w48)` and `h = |v30 * 0.5 * (w44 + w48)|`.
fn separated(ring: &[Measured], primary: Option<u16>, alternate: &[Measured]) -> Vec<usize> {
    let sibling: Vec<&Measured> = ring.iter().filter(|m| m.path == primary).collect();
    alternate
        .iter()
        .enumerate()
        .skip(1)
        .filter(|(_, b)| {
            sibling
                .iter()
                .min_by(|x, y| {
                    let dx = (x.position - b.position).length();
                    let dy = (y.position - b.position).length();
                    dx.total_cmp(&dy)
                })
                .is_some_and(|a| {
                    (a.lane_centre - b.lane_centre).length() > a.half_span + b.half_span
                })
        })
        .map(|(k, _)| k)
        .collect()
}

/// One entry per control point of `order`, which walks whole paths.
fn measure(
    spline: &Spline,
    order: &[u32],
    collision: &CollisionWorld,
    lane_width: f32,
) -> Vec<Measured> {
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
        entries.push(entry(sample, this, collision, lane_width));
    }
    entries
}

/// An entry and the section it was measured at.
struct Measured {
    entry: Entry,
    position: Vec3,
    /// The path the section lies on.
    path: Option<u16>,
    /// The section's centre between its spline widths, unclamped by any probe.
    lane_centre: Vec3,
    /// Half the section's spline width along its lateral axis as the track authors it.
    half_span: f32,
    /// The centre probe found no ground: this entry starts a jump gap.
    jump: bool,
}

fn entry(
    sample: &oag_vex::track::Sample,
    path: Option<u16>,
    collision: &CollisionWorld,
    lane_width: f32,
) -> Measured {
    let position = Vec3::from_array(sample.pos);
    let raw_lateral = Vec3::from_array(sample.lateral);
    let half = 0.5 * (sample.half_width_left + sample.half_width_right);
    let lane_centre = position - raw_lateral * sample.half_width_left + raw_lateral * half;
    let half_span = (raw_lateral * half).length();
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
        path,
        lane_centre,
        half_span,
        jump: usable && !ground(centre),
    }
}

/// A centre with no ground under it blanks that entry and the ones after it in the lap array.
fn blank_jumps(entries: &mut [Measured], lap: &[usize], steps: i32) {
    let span = usize::try_from(steps.max(1)).unwrap_or(1);
    let mut blank_until = 0usize;
    for (index, &at) in lap.iter().enumerate() {
        let measured = &mut entries[at];
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
        let survey = &detonator.survey;
        let here = usize::from(mine.entry);
        let centre = |index: usize| survey.entry(index).map(|e| e.centre);
        let (from, to) = match (centre(here), centre(here + 1)) {
            (Some(from), Some(to)) => (from, to),
            _ => (centre(here.checked_sub(1)?)?, centre(here)?),
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

    /// Sets the EMP's charge to `n` and its gauge to what `n` charges give (`--detonator-emp-charge`):
    /// a comparison's stand-in for the pads the original collected, as the live runs on
    /// `docs/ghidra/functions/ps3-hdfury-eu/detonator-emp.md` write `rm+0x2dd8`. Returns whether the
    /// race runs an EMP.
    pub fn charge_detonator_emp(&mut self, n: u32) -> bool {
        let Some(config) = self.detonator.as_ref().and_then(|d| d.emp) else {
            return false;
        };
        let emp = &mut self.world.detonator.emp;
        emp.armed = true;
        emp.charge = n;
        emp.gauge = oag_weapons::detonator::emp::gauge(n, config.lo, config.hi);
        true
    }

    /// Puts the opening stage's mines where the original's were, all live, in place of the seeded layout
    /// (`--detonator-mines`): each position is snapped to the free lane of ours' survey nearest to it
    /// across the section (the offset along the section's up axis is the mine's hover height and is not
    /// compared). The stage is laid first so the bomb and the stage counter are the race's own. Returns
    /// each position's lane as (survey entry, lane), `None` for a position with no lane within [`SNAP_RADIUS`]
    /// across and [`LIFT_TOLERANCE`] of the mine's height, or on a lane another carried mine or the bomb already
    /// holds (dropped). Nothing happens off Detonator (every position is `None`).
    ///
    /// A comparison against the original carries its mines this way: the placer's `Libc_Rand` stream is not
    /// carried, and the original numbers its entries from the player's start cursor where ours starts at the
    /// spline's origin (7 apart on Talon's Junction), so a mine's own entry number does not transfer; its
    /// position does (`docs/tools/compare-page.md`, "Detonator pairs").
    pub fn carry_detonator_mines(&mut self, positions: &[Vec3]) -> Vec<Option<(u16, i8)>> {
        use oag_weapons::detonator::{MAX_MINES, MINE_LIFT, Placed};
        let Some(detonator) = self.detonator.clone() else {
            return vec![None; positions.len()];
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
        let mut lanes: Vec<Option<(u16, i8)>> = Vec::with_capacity(positions.len());
        for &position in positions {
            let mut best: Option<(f32, Placed)> = None;
            let survey = &detonator.survey;
            for (index, entry) in (0..survey.lap_len()).filter_map(|at| survey.lap_entry(at)) {
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
                    lanes.push(Some((placed.entry, placed.lane)));
                }
                _ => lanes.push(None),
            }
        }
        let field = &mut self.world.detonator;
        for (slot, placed) in field.mines.iter_mut().zip(&laid) {
            *slot = *placed;
        }
        field.live_mines = laid.len() as u16;
        lanes
    }

    /// Lays the stages the player has reached: the opening one on the first tick, then one per
    /// lap, up to the fourteenth (`Detonator_UpdateRace` places while the new stage is at most
    /// `14`). Nothing happens off Detonator.
    pub(crate) fn advance_detonator(&mut self, inputs: &oag_gameplay::PlayerInputs) {
        self.detonator_events.clear();
        let Some(detonator) = self.detonator.clone() else {
            return;
        };
        // The record and the rows are taken at the blast: nothing scores while the end plays.
        if self.detonator_ending(self.world.primary_slot()) {
            return;
        }
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
            let from = firer_entry(&detonator.survey, &self.spline, at);
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

/// Where the EMP's wave starts: the entry nearest `at` on the path the craft is on, a branch's
/// or else the ring's. The original copies the firing ship's own AI-track cursor
/// (`ship+0x7820`); the craft's path is read here off its nearest spline sample.
fn firer_entry(survey: &Survey, spline: &Spline, at: Vec3) -> u16 {
    let on = spline
        .nearest(at)
        .and_then(|(index, _, _)| spline.path_of(index));
    survey
        .branches
        .iter()
        .position(|b| Some(b.path) == on)
        .and_then(|branch| {
            let offset = nearest_of(&survey.branches[branch].entries, at)?;
            u16::try_from(survey.branch_entry(branch, offset)?).ok()
        })
        .or_else(|| u16::try_from(nearest_of(&survey.entries, at)?).ok())
        .unwrap_or(0)
}

/// The index of the entry of `entries` nearest `at`.
fn nearest_of(entries: &[Entry], at: Vec3) -> Option<usize> {
    entries
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| {
            (a.centre - at)
                .length_squared()
                .total_cmp(&(b.centre - at).length_squared())
        })
        .map(|(index, _)| index)
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

    /// The EMP's charge with its ready and full counts, or `None` off Detonator or without the
    /// table: what the hull's plasma shells map (`hull_charge`).
    #[must_use]
    pub fn detonator_emp_counts(&self) -> Option<(u32, u32, u32)> {
        let config = self.detonator.as_ref()?.emp.as_ref()?;
        Some((self.world.detonator.emp.charge, config.lo, config.hi))
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
