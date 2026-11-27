//! Detonator's mines and bombs: where a stage lays them.
//!
//! The law is `Detonator_PlaceStageObjects` (`0x000654e0`) and `Detonator_SpawnObject`
//! (`0x00064f70`) on HD/Fury, read off the disassembly
//! (`docs/ghidra/functions/ps3-hdfury-eu/detonator-placement.md`; 2048 and Omega match
//! structurally). The lap is a [`Survey`] of track [`Entry`]s, built once at load from the
//! circuit; [`place_stage`] picks groups from the layout's patterns and lays objects into a
//! [`Field`], which lives in the world.
//!
//! [`step_contacts`] is what a craft meeting one costs it
//! (`docs/ghidra/functions/ps3-hdfury-eu/detonator-mine.md`, `detonator-bomb.md`).
//!
//! # What is ours
//!
//! - The draws go through the seeded [`Rng`] (`Libc_Rand`'s sequence is the console's and is
//!   not reproduced), in the original's order: one per group attempt, two per top-up or bomb
//!   attempt.
//! - **A lane's occupancy is derived from the live objects** instead of kept as a second array:
//!   the original writes the lane marker on a spawn and clears it when the object goes, so the
//!   two agree. The original frees a mine's lane only if the mine is in some view's PVS when it
//!   goes off; here it is always freed.
//! - **A mine leaves on the tick of contact**, the original's next tick; and the 0.5 s grace
//!   a fresh object gives its owner is not modelled: nothing is laid within 150 entries of the
//!   player (the start zone), so no craft can be on a mine that young.
//! - **The bomb pool holds two**, the capacity `BombManager` reserves in Detonator
//!   (`detonator-placement.md`, "Pools"); the original's spawn checks no capacity, so a third
//!   live bomb is refused here rather than written past the pool.
//! - A cluster that runs past the lap's last entry stops; the original reads past the array.

use oag_core::Rng;
use oag_core::math::Vec3;
use oag_tables::detonator_layout::{GamePlayValues, PatternFile, Scoring};

pub mod cannon;
pub mod emp;

/// Mines the pool holds: `MineManager`'s capacity in Detonator.
pub const MAX_MINES: usize = 200;

/// Bombs the pool holds: `BombManager`'s capacity in Detonator.
pub const MAX_BOMBS: usize = 2;

/// The most lanes a usable entry has; a wider track is unusable.
pub const MAX_LANES: u8 = 20;

/// What the top-up and the bomb pass to the spawn as the spacing.
const SINGLE_SPACING: f32 = 5.0;

/// One spline section of the lap: where the lanes fan out and which of them are free.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Entry {
    /// Whether anything may be laid here.
    pub usable: bool,
    /// How many lanes the section has (odd; at most [`MAX_LANES`] when usable).
    pub lanes: u8,
    /// Bit `j` set: lane `j` is blocked by the ground under it.
    pub blocked: u32,
    /// The point lane `lanes / 2` sits on.
    pub centre: Vec3,
    /// The unit axis lanes step along.
    pub lateral: Vec3,
    /// The section's up axis.
    pub up: Vec3,
}

/// The lap's entries in driving order. Track data, not world state.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Survey {
    /// One per section of the ring, the route a lap drives. Every ring walk reads only these.
    pub entries: Vec<Entry>,
    /// The alternate paths off the ring, measured whole: the EMP wave's fork walks them.
    pub branches: Vec<SurveyBranch>,
    /// What placement lays into: the original's entry array, survey indices (as [`Self::entry`]
    /// takes them) in the order its walk stores them, a fork's primary route then its
    /// alternate's separated sections (`detonator-placement.md`, "The fork walk"). Empty: the
    /// ring alone, in order.
    pub lap: Vec<u16>,
}

/// One alternate path off the ring, measured like the ring.
///
/// Placement lays on the entries [`Survey::lap`] names; an object put on one is addressed by
/// [`Survey::branch_entry`]'s index, which [`Survey::point`] reads.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SurveyBranch {
    /// The ring entry the front steps onto at the split.
    pub split: u16,
    /// The ring entry the branch rejoins at.
    pub merge: u16,
    /// The branch's entries in driving order.
    pub entries: Vec<Entry>,
    /// The branch's path in the spline table: how the firing craft is located on it.
    pub path: u16,
    /// Per branch entry: the primary route's nearest ring entry. The primary route is the ring
    /// from [`Self::split`].
    pub to_primary: Vec<Option<Partner>>,
    /// Per primary-route entry, counted from [`Self::split`]: the nearest branch offset. Its
    /// length is the primary route's.
    pub from_primary: Vec<Option<Partner>>,
}

/// An entry's nearest section on the other route of a split (`Track_FindSiblingSection`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Partner {
    /// A ring entry, or a branch offset.
    pub at: u16,
    /// Within [`emp::SIBLING_REACH`]: close enough for the EMP's launch-time fork.
    pub near: bool,
}

/// What a race reads from the layout files.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Layout {
    /// `PatternFile.xml`.
    pub patterns: PatternFile,
    /// `GamePlayValues.xml`.
    pub values: GamePlayValues,
    /// `DetonatorScoring.xml`.
    pub scoring: Scoring,
}

/// One laid object: which entry and lane it sits on. Its position is the survey's, see
/// [`Survey::point`]; keeping only the address is what lets the field stay small in the world
/// snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Placed {
    /// The survey entry it sits on.
    pub entry: u16,
    /// Its lane offset from the centre lane (at most [`MAX_LANES`] / 2 either way).
    pub lane: i8,
    /// Laid this stage: a shot mine scores its multiplied value, a dark one the flat
    /// `old_mine_score` (`MineManager_DarkenAll` darkens every survivor at the stage step).
    pub live: bool,
}

impl Survey {
    /// The entry at `index`: the ring's, then each branch's in order.
    #[must_use]
    pub fn entry(&self, index: usize) -> Option<&Entry> {
        let Some(mut at) = index.checked_sub(self.entries.len()) else {
            return self.entries.get(index);
        };
        for branch in &self.branches {
            if at < branch.entries.len() {
                return branch.entries.get(at);
            }
            at -= branch.entries.len();
        }
        None
    }

    /// The branch and the offset into it that the index of a branch entry names, or `None` for
    /// a ring index or one past the survey: the inverse of [`Self::branch_entry`].
    #[must_use]
    pub fn branch_of(&self, index: usize) -> Option<(usize, usize)> {
        let mut at = index.checked_sub(self.entries.len())?;
        for (branch, b) in self.branches.iter().enumerate() {
            if at < b.entries.len() {
                return Some((branch, at));
            }
            at -= b.entries.len();
        }
        None
    }

    /// Whether the original holds a usable placement record for `index`: a section placement's
    /// walk stored (the ring's alone on an unforked lap) and did not mark unusable. The EMP's
    /// sweep matches only these.
    #[must_use]
    pub fn has_record(&self, index: usize) -> bool {
        self.entry(index).is_some_and(|e| e.usable)
            && if self.lap.is_empty() {
                index < self.entries.len()
            } else {
                u16::try_from(index).is_ok_and(|i| self.lap.contains(&i))
            }
    }

    /// The index of the nearest section on the other route of the split `index` lies beside, or
    /// `None` away from every split.
    #[must_use]
    pub fn sibling(&self, index: usize) -> Option<usize> {
        if let Some((branch, at)) = self.branch_of(index) {
            let partner = (*self.branches[branch].to_primary.get(at)?)?;
            return Some(usize::from(partner.at));
        }
        let total = self.entries.len();
        self.branches.iter().enumerate().find_map(|(branch, b)| {
            let offset = (index + total).checked_sub(usize::from(b.split))? % total.max(1);
            let partner = (*b.from_primary.get(offset)?)?;
            self.branch_entry(branch, usize::from(partner.at))
        })
    }

    /// How many entries placement lays into: [`Self::lap`]'s, or the ring's.
    #[must_use]
    pub fn lap_len(&self) -> usize {
        if self.lap.is_empty() {
            self.entries.len()
        } else {
            self.lap.len()
        }
    }

    /// The survey index of position `at` of the lap array, and its entry.
    #[must_use]
    pub fn lap_entry(&self, at: usize) -> Option<(usize, &Entry)> {
        let index = if self.lap.is_empty() {
            at
        } else {
            usize::from(*self.lap.get(at)?)
        };
        Some((index, self.entry(index)?))
    }

    /// The index [`Self::entry`] and [`Placed::entry`] address entry `offset` of branch
    /// `branch` by.
    #[must_use]
    pub fn branch_entry(&self, branch: usize, offset: usize) -> Option<usize> {
        let b = self.branches.get(branch)?;
        if offset >= b.entries.len() {
            return None;
        }
        let before: usize = self.branches[..branch]
            .iter()
            .map(|b| b.entries.len())
            .sum();
        Some(self.entries.len() + before + offset)
    }

    /// Where `placed` sits and the section's up axis there, or `None` for an entry the survey
    /// does not hold. `lane_width` is `DistanceOffsetBetweenMines`.
    #[must_use]
    pub fn point(&self, placed: &Placed, lane_width: f32) -> Option<(Vec3, Vec3)> {
        let entry = self.entry(usize::from(placed.entry))?;
        Some((
            entry.centre + entry.lateral * (lane_width * f32::from(placed.lane)),
            entry.up,
        ))
    }

    /// Where the mine `placed` stands: its lane point lifted [`MINE_LIFT`] along the section's up axis, and
    /// that axis. A bomb stands on [`Self::point`] (its own height is unmeasured).
    #[must_use]
    pub fn mine_point(&self, placed: &Placed, lane_width: f32) -> Option<(Vec3, Vec3)> {
        let (at, up) = self.point(placed, lane_width)?;
        Some((at + up * MINE_LIFT, up))
    }
}

/// How far above its lane point, along the section's up axis, a laid mine's object stands.
///
/// 80 mine objects read out of the original's heap on Talon's Junction (`MineManager`'s pool, the
/// position at `+0x80`) sat 3.00 above the point [`Survey::point`] gives for their entry and lane on 70
/// of 80, 2.5 to 2.9 on the rest, with the same point to 0.5 across the section on all 80
/// (`docs/ghidra/functions/ps3-hdfury-eu/detonator-mine.md`, "Where a mine stands"). Confidence 75.
pub const MINE_LIFT: f32 = 3.0;

/// Every mine and bomb laid, in fixed arrays.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Field {
    /// Stages laid so far: `0` before the opening stage, then the new stage's number.
    pub stages_laid: u8,
    /// The mines, the first `live_mines` of them live.
    pub mines: [Placed; MAX_MINES],
    /// How many of [`Self::mines`] are live.
    pub live_mines: u16,
    /// The bombs, the first `live_bombs` of them live.
    pub bombs: [Placed; MAX_BOMBS],
    /// How many of [`Self::bombs`] are live.
    pub live_bombs: u8,
    /// How many cannon hits each of [`Self::bombs`] has taken.
    pub bomb_hits: [u8; MAX_BOMBS],
    /// The pulse cannon and its score.
    pub cannon: cannon::Cannon,
    /// The EMP: the pads' charge, the gauge and the wave.
    pub emp: emp::Emp,
    /// The player craft's auto-speed ramp (`craft+0x2d4`), `0` to `1`, zeroed by a rescue
    /// (`docs/ghidra/functions/ps3-hdfury-eu/detonator-speed.md`, "The ramp and the rescue").
    pub ramp: f32,
}

impl Default for Field {
    fn default() -> Self {
        Self::new()
    }
}

impl Field {
    /// A field with nothing laid.
    #[must_use]
    pub const fn new() -> Self {
        const NONE: Placed = Placed {
            entry: 0,
            lane: 0,
            live: false,
        };
        Self {
            stages_laid: 0,
            mines: [NONE; MAX_MINES],
            live_mines: 0,
            bombs: [NONE; MAX_BOMBS],
            live_bombs: 0,
            bomb_hits: [0; MAX_BOMBS],
            cannon: cannon::Cannon::new(),
            emp: emp::Emp::new(),
            ramp: 0.0,
        }
    }

    /// Whether nothing was ever laid.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.stages_laid == 0
            && self.live_mines == 0
            && self.live_bombs == 0
            && !self.cannon.armed
            && !self.emp.armed
            && self.ramp == 0.0
    }

    /// Removes mine `index`, filling its place with the last one.
    pub fn remove_mine(&mut self, index: usize) {
        let last = usize::from(self.live_mines) - 1;
        self.mines[index] = self.mines[last];
        self.live_mines -= 1;
    }

    /// Removes bomb `index`, filling its place, and its hit count, with the last one's.
    pub fn remove_bomb(&mut self, index: usize) {
        let last = usize::from(self.live_bombs) - 1;
        self.bombs[index] = self.bombs[last];
        self.bomb_hits[index] = self.bomb_hits[last];
        self.bomb_hits[last] = 0;
        self.live_bombs -= 1;
    }

    /// The live mines.
    #[must_use]
    pub fn live_mines(&self) -> &[Placed] {
        &self.mines[..usize::from(self.live_mines)]
    }

    /// The live bombs.
    #[must_use]
    pub fn live_bombs(&self) -> &[Placed] {
        &self.bombs[..usize::from(self.live_bombs)]
    }

    fn occupied(&self, entry: usize, lane: i32) -> bool {
        self.live_mines()
            .iter()
            .chain(self.live_bombs())
            .any(|object| usize::from(object.entry) == entry && i32::from(object.lane) == lane)
    }
}

/// What the placement counts a call against: the mines alive, or the ones this call laid.
struct Tally {
    placed_this_call: i32,
}

/// Lays stage `old_stage + 1`'s objects (`old_stage` is `0` for the opening stage).
///
/// `Detonator_PlaceStageObjects(rm, oldStage)`: the groups, then single mines up to the stage's
/// count, then one bomb. Returns the bombs the step set off.
pub fn place_stage(
    field: &mut Field,
    survey: &Survey,
    layout: &Layout,
    rng: &mut Rng,
    old_stage: u32,
) -> Vec<Event> {
    field.stages_laid = u8::try_from(old_stage + 1).unwrap_or(u8::MAX);
    // `BombManager_DetonateAll`: the stage step sets off every live bomb harmlessly before the
    // new one is laid (`detonator-bomb.md`, "One bomb at a time").
    let lane = layout.values.distance_offset_between_mines;
    let replaced: Vec<Event> = field
        .live_bombs()
        .iter()
        .filter_map(|bomb| survey.point(bomb, lane))
        .map(|(at, _)| Event {
            kind: EventKind::BombReplaced,
            at,
            points: None,
        })
        .collect();
    field.live_bombs = 0;
    field.bomb_hits = [0; MAX_BOMBS];
    for mine in &mut field.mines[..usize::from(field.live_mines)] {
        mine.live = false;
    }
    let total = survey.lap_len();
    let Some(count) = layout.values.mine_count(old_stage as usize + 1) else {
        return replaced;
    };
    if total == 0 {
        return replaced;
    }
    let mut tally = Tally {
        placed_this_call: 0,
    };
    let groups = &layout.patterns.patterns;
    let eligible = |g: usize| i64::from(groups[g].stage_to_appear_from) - 1 <= i64::from(old_stage);

    // How many groups: the stage's target over the mean objects per eligible group.
    let eligible_groups: Vec<usize> = (0..groups.len()).filter(|&g| eligible(g)).collect();
    let objects: i32 = eligible_groups
        .iter()
        .flat_map(|&g| &groups[g].clusters)
        .map(|c| c.num_objects)
        .sum();
    let target = |field: &Field, tally: &Tally| {
        if layout.values.mine_counts_represent_max_total_to_appear {
            count - i32::from(field.live_mines)
        } else {
            count - tally.placed_this_call
        }
    };
    let average = i32::try_from(eligible_groups.len())
        .ok()
        .filter(|&n| n > 0)
        .map_or(0, |n| objects / n);
    let n_groups = if average > 0 {
        target(field, &tally) / average
    } else {
        0
    };

    // The groups, a chunk of the lap apart.
    if n_groups > 0 && !eligible_groups.is_empty() {
        let chunk = total / n_groups as usize;
        let mut base = chunk;
        if chunk > 0 {
            for _ in 0..n_groups {
                if total.saturating_sub(base) < chunk {
                    break;
                }
                let group = loop {
                    let g = rng.below(groups.len() as u32) as usize;
                    if eligible(g) {
                        break g;
                    }
                };
                let mut last = base;
                for cluster in &groups[group].clusters {
                    let position = base + cluster.distance_from_original_position.max(0) as usize;
                    last = position;
                    if position > total - 1 {
                        break;
                    }
                    if cluster.kind == 0 {
                        spawn(
                            field,
                            survey,
                            layout,
                            old_stage,
                            &mut tally,
                            position,
                            cluster.num_objects,
                            cluster.distance_between_objects as f32,
                            cluster.percentage_across_track,
                            &cluster.object_types,
                        );
                    }
                }
                base = last + chunk;
            }
        }
    }

    // Single mines up to the stage's count. The retry accepts a lane the spawn then offsets by
    // half the lane count, so these land on the centre lane and the positive side only.
    let mut left = target(field, &tally);
    // The original retries until a lane is free; a lap with none left would never end, so the
    // retries are bounded.
    let attempts = total.saturating_mul(64);
    'top_up: while left > 0 {
        let mut found = None;
        for _ in 0..attempts {
            let e = rng.below(total as u32) as usize;
            let Some((index, candidate)) = survey.lap_entry(e) else {
                continue;
            };
            let lanes = candidate.lanes.max(1);
            let lane = rng.below(u32::from(lanes)) as i32 - i32::from(lanes / 2);
            if candidate.usable && raw_lane_free(field, candidate, index, lane) && lane >= 0 {
                found = Some((e, lane));
                break;
            }
        }
        let Some((entry, lane)) = found else {
            break 'top_up;
        };
        spawn(
            field,
            survey,
            layout,
            old_stage,
            &mut tally,
            entry,
            1,
            SINGLE_SPACING,
            lane,
            &[0],
        );
        left -= 1;
    }

    // One bomb: the same draw, without the `lane >= 0` test, and one attempt only. A negative
    // lane reads before the original's array; here it lays nothing.
    let e = rng.below(total as u32) as usize;
    let Some((index, candidate)) = survey.lap_entry(e) else {
        return replaced;
    };
    let lanes = candidate.lanes.max(1);
    let lane = rng.below(u32::from(lanes)) as i32 - i32::from(lanes / 2);
    if candidate.usable && raw_lane_free(field, candidate, index, lane) {
        spawn(
            field,
            survey,
            layout,
            old_stage,
            &mut tally,
            e,
            1,
            SINGLE_SPACING,
            lane,
            &[1],
        );
    }
    replaced
}

/// The retry's own test: the lane array read at index `lane` as it stands, not at
/// `lanes / 2 + lane` where the spawn then places.
fn raw_lane_free(field: &Field, entry: &Entry, index: usize, lane: i32) -> bool {
    (0..i32::from(entry.lanes)).contains(&lane)
        && entry.blocked & (1 << lane) == 0
        && !field.occupied(index, lane - i32::from(entry.lanes / 2))
}

/// Whether lane `lane` of entry `index` (an offset from the centre lane) can take an object.
fn lane_free(field: &Field, entry: &Entry, index: usize, lane: i32) -> bool {
    let j = i32::from(entry.lanes / 2) + lane;
    entry.usable
        && (0..i32::from(entry.lanes)).contains(&j)
        && entry.blocked & (1 << j) == 0
        && !field.occupied(index, lane)
}

/// `Detonator_SpawnObject`: lays up to `count` objects of `kinds` down the lap array from
/// `position` in one lane, `spacing` array positions apart.
#[allow(clippy::too_many_arguments)]
fn spawn(
    field: &mut Field,
    survey: &Survey,
    layout: &Layout,
    old_stage: u32,
    tally: &mut Tally,
    mut position: usize,
    count: i32,
    spacing: f32,
    lane: i32,
    kinds: &[i32],
) {
    let step = (spacing.ceil() as usize).max(1);
    for &kind in kinds.iter().take(usize::try_from(count).unwrap_or(0)) {
        if kind == 0 {
            let cap = layout
                .values
                .mine_count(old_stage as usize + 1)
                .unwrap_or(0);
            let used = if layout.values.mine_counts_represent_max_total_to_appear {
                i32::from(field.live_mines)
            } else {
                tally.placed_this_call
            };
            if cap - used <= 0 {
                return;
            }
        }
        let Some((index, entry)) = survey.lap_entry(position) else {
            return;
        };
        if lane_free(field, entry, index, lane) {
            let (Ok(entry), Ok(lane)) = (u16::try_from(index), i8::try_from(lane)) else {
                return;
            };
            let placed = Placed {
                entry,
                lane,
                live: true,
            };
            match kind {
                0 if usize::from(field.live_mines) < MAX_MINES => {
                    field.mines[usize::from(field.live_mines)] = placed;
                    field.live_mines += 1;
                    tally.placed_this_call += 1;
                }
                1 if usize::from(field.live_bombs) < MAX_BOMBS => {
                    field.bombs[usize::from(field.live_bombs)] = placed;
                    field.bomb_hits[usize::from(field.live_bombs)] = 0;
                    field.live_bombs += 1;
                }
                _ => {}
            }
        }
        if spacing > 0.0 {
            position += step;
        }
    }
}

#[cfg(test)]
mod tests;

/// What a mine costs a craft that touches it, from the Detonator weapon table's Mine block.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MineContact {
    /// `trigger_radius`: the sphere about the mine that sets it off.
    pub trigger_radius: f32,
    /// `blastradius`: the craft must be inside it for the blast to land.
    pub blast_radius: f32,
    /// `damage`: energy lost.
    pub damage: f32,
    /// `slowdown_time`: seconds of slowdown credited.
    pub slowdown_time: f32,
    /// `blastforce`: the push at the centre.
    pub blast_force: f32,
    /// `velocity_reduction`: the factor the craft's velocity is multiplied by, once.
    pub velocity_reduction: f32,
}

/// What a bomb costs, from the Bomb block.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BombContact {
    /// `trigger_radius`.
    pub trigger_radius: f32,
    /// `blastradius`: every craft inside it is pushed.
    pub blast_radius: f32,
    /// `damage`: energy lost by the craft that touched it, and by no other.
    pub damage: f32,
    /// `blastforce`: the push at the centre.
    pub blast_force: f32,
}

/// Both halves, or `None` where the table authors no Mine or no Bomb.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Contact {
    /// The mine's.
    pub mine: MineContact,
    /// The bomb's.
    pub bomb: BombContact,
}

impl Contact {
    /// Reads both blocks of a Detonator weapon table.
    #[must_use]
    pub fn from_stats(stats: &oag_tables::weapons::WeaponStats) -> Option<Self> {
        let mine = stats.mine()?;
        let bomb = stats.bomb()?;
        let reduction = stats.detonator().and_then(|d| d.mine.velocity_reduction);
        Some(Self {
            mine: MineContact {
                trigger_radius: mine.trigger_radius,
                blast_radius: mine.blastradius,
                damage: mine.damage,
                slowdown_time: mine.slowdown_time,
                blast_force: mine.blastforce,
                velocity_reduction: reduction.unwrap_or(1.0),
            },
            bomb: BombContact {
                trigger_radius: bomb.trigger_radius,
                blast_radius: bomb.blastradius,
                damage: bomb.damage,
                blast_force: bomb.blastforce,
            },
        })
    }
}

/// What happened to a laid object or a round this tick, for the effects and sounds a view plays.
/// Out of the tick, never state: the simulation reads none of it back.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Event {
    /// What happened.
    pub kind: EventKind,
    /// Where, in the world.
    pub at: Vec3,
    /// What the event added to the score through `Hud_RaiseScoreFeedback`, or `None`
    /// where it adds none (`docs/ghidra/functions/ps3-hdfury-eu/detonator-cannon.md`,
    /// "The score").
    pub points: Option<i32>,
}

/// The things [`Event`] reports. Each names the original's effect or sound it stands behind in
/// `docs/ghidra/functions/ps3-hdfury-eu/detonator-{mine,bomb,cannon}.md`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EventKind {
    /// A craft ran into a mine, which the contact darkens first: always the dark explosion.
    MineTouched,
    /// A craft ran into a bomb.
    BombTouched,
    /// A cannon round hit a mine; `live` is whether the mine was laid this stage.
    MineShot {
        /// Laid this stage, and not yet darkened.
        live: bool,
    },
    /// A destroyed bomb's chain reaction set a mine off; `live` as for [`Self::MineShot`].
    MineChained {
        /// Laid this stage, and not yet darkened.
        live: bool,
    },
    /// A cannon destroyed a bomb with its last hit.
    BombShot,
    /// The stage step set a bomb off harmlessly before laying the new one.
    BombReplaced,
    /// A cannon round died on a wall, the wall's unit normal, and the frame the original
    /// spawns its spark in (`docs/ghidra/functions/ps3-hdfury-eu/detonator-cannon.md`,
    /// "The spark's matrix, read live").
    RoundWall {
        /// The wall's unit normal.
        normal: Vec3,
        /// Where the spark spawns: the hit lowered onto the road, `3.5` along the round's
        /// surface normal.
        spawn: Vec3,
        /// The spark's frame `+Y`: the round's velocity reflected off the wall, unit length.
        rebound: Vec3,
    },
    /// The cannon fired a volley, at the craft that fired it.
    Volley,
    /// The charge reached the EMP's threshold: the `EMPREADY` announcer line.
    EmpReady,
    /// The gauge filled: the `EMPFULL` announcer line.
    EmpFull,
    /// The EMP fired, at the survey entry its wave starts from: `QUAKELAUNCH`.
    EmpLaunch,
    /// The EMP's wave cleared a mine; `live` as for [`Self::MineShot`].
    MineEmped {
        /// Laid this stage, and not yet darkened.
        live: bool,
    },
    /// The EMP's wave cleared a bomb.
    BombEmped,
}

/// Sets off every mine and bomb a craft is touching, and removes them.
///
/// `MineManager_TestCraftContacts` and `BombManager_TestCraftContacts`: a sphere test on the
/// craft's position; a mine costs the toucher its `damage`, `slowdown_time` and one
/// `velocity_reduction` of its velocity, a bomb its `damage` and a push on every craft inside
/// its blast radius (the push law is the weapons' own, [`oag_physics::wall::post_blast_impulse`]).
/// Damage goes through [`oag_physics::damage::apply_weapon`], the same gate every weapon hit
/// takes. `lane_width` is `DistanceOffsetBetweenMines`.
pub fn step_contacts<S: crate::Craft>(
    field: &mut Field,
    survey: &Survey,
    lane_width: f32,
    contact: &Contact,
    ships: &mut [S],
    rules: oag_physics::DamageRules,
) -> Vec<Event> {
    let mut went = Vec::new();

    let mut i = 0;
    while i < usize::from(field.live_mines) {
        let Some((at, _)) = survey.mine_point(&field.mines[i], lane_width) else {
            i += 1;
            continue;
        };
        let toucher = ships.iter().position(|s| {
            s.active() && (s.physics().body.position - at).length() < contact.mine.trigger_radius
        });
        let Some(slot) = toucher else {
            i += 1;
            continue;
        };
        let mine = contact.mine;
        let ship = &mut ships[slot];
        if (ship.physics().body.position - at).length() < mine.blast_radius {
            *ship.pending_slowdown_mut() += mine.slowdown_time;
            let dimensions = ship.dimensions();
            oag_physics::damage::apply_weapon(ship.physics_mut(), &dimensions, mine.damage, rules);
            oag_physics::wall::post_blast_impulse(
                ship.physics_mut(),
                at,
                mine.blast_radius,
                mine.blast_force,
            );
            ship.physics_mut().body.linear_velocity *= mine.velocity_reduction;
        }
        went.push(Event {
            kind: EventKind::MineTouched,
            at,
            points: None,
        });
        field.remove_mine(i);
    }

    let mut i = 0;
    while i < usize::from(field.live_bombs) {
        let Some((at, _)) = survey.point(&field.bombs[i], lane_width) else {
            i += 1;
            continue;
        };
        let bomb = contact.bomb;
        let toucher = ships.iter().position(|s| {
            s.active() && (s.physics().body.position - at).length() < bomb.trigger_radius
        });
        let Some(slot) = toucher else {
            i += 1;
            continue;
        };
        {
            let ship = &mut ships[slot];
            let dimensions = ship.dimensions();
            oag_physics::damage::apply_weapon(ship.physics_mut(), &dimensions, bomb.damage, rules);
        }
        for ship in ships.iter_mut().filter(|s| s.active()) {
            let reach = (ship.physics().body.position - at).length();
            if reach < bomb.blast_radius {
                oag_physics::wall::post_blast_impulse(
                    ship.physics_mut(),
                    at,
                    bomb.blast_radius,
                    bomb.blast_force,
                );
            }
        }
        went.push(Event {
            kind: EventKind::BombTouched,
            at,
            points: None,
        });
        field.remove_bomb(i);
    }
    went
}
