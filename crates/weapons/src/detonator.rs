//! Detonator's mines and bombs: where a stage lays them.
//!
//! The law is `Detonator_PlaceStageObjects` (`0x000654e0`) and `Detonator_SpawnObject`
//! (`0x00064f70`) on HD/Fury, read off the disassembly
//! (`docs/ghidra/functions/ps3-hdfury-eu/detonator-placement.md`; 2048 and Omega match
//! structurally). The lap is a [`Survey`] of track [`Entry`]s, built once at load from the
//! circuit; [`place_stage`] picks groups from the layout's patterns and lays objects into a
//! [`Field`], which lives in the world.
//!
//! **Inert.** Nothing here moves, goes off or hurts: this lays objects and counts them.
//!
//! # What is ours
//!
//! - The draws go through the seeded [`Rng`] (`Libc_Rand`'s sequence is the console's and is
//!   not reproduced), in the original's order: one per group attempt, two per top-up or bomb
//!   attempt.
//! - **A lane's occupancy is derived from the live objects** instead of kept as a second array:
//!   the original writes the lane marker on a spawn and the objects never leave in this build,
//!   so the two agree.
//! - **The bomb pool holds two**, the capacity `BombManager` reserves in Detonator
//!   (`detonator-placement.md`, "Pools"); the original's spawn checks no capacity, so a third
//!   live bomb is refused here rather than written past the pool.
//! - A cluster that runs past the lap's last entry stops; the original reads past the array.

use oag_core::Rng;
use oag_core::math::Vec3;
use oag_tables::detonator_layout::{GamePlayValues, PatternFile};

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
    /// One per section.
    pub entries: Vec<Entry>,
}

/// What a race reads from the layout files.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Layout {
    /// `PatternFile.xml`.
    pub patterns: PatternFile,
    /// `GamePlayValues.xml`.
    pub values: GamePlayValues,
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
}

impl Survey {
    /// Where `placed` sits and the section's up axis there, or `None` for an entry the survey
    /// does not hold. `lane_width` is `DistanceOffsetBetweenMines`.
    #[must_use]
    pub fn point(&self, placed: &Placed, lane_width: f32) -> Option<(Vec3, Vec3)> {
        let entry = self.entries.get(usize::from(placed.entry))?;
        Some((
            entry.centre + entry.lateral * (lane_width * f32::from(placed.lane)),
            entry.up,
        ))
    }
}

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
        const NONE: Placed = Placed { entry: 0, lane: 0 };
        Self {
            stages_laid: 0,
            mines: [NONE; MAX_MINES],
            live_mines: 0,
            bombs: [NONE; MAX_BOMBS],
            live_bombs: 0,
        }
    }

    /// Whether nothing was ever laid.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.stages_laid == 0 && self.live_mines == 0 && self.live_bombs == 0
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
/// count, then one bomb.
pub fn place_stage(
    field: &mut Field,
    survey: &Survey,
    layout: &Layout,
    rng: &mut Rng,
    old_stage: u32,
) {
    field.stages_laid = u8::try_from(old_stage + 1).unwrap_or(u8::MAX);
    let total = survey.entries.len();
    let Some(count) = layout.values.mine_count(old_stage as usize + 1) else {
        return;
    };
    if total == 0 {
        return;
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
            let candidate = &survey.entries[e];
            let lanes = candidate.lanes.max(1);
            let lane = rng.below(u32::from(lanes)) as i32 - i32::from(lanes / 2);
            if candidate.usable && raw_lane_free(field, candidate, e, lane) && lane >= 0 {
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
    let candidate = &survey.entries[e];
    let lanes = candidate.lanes.max(1);
    let lane = rng.below(u32::from(lanes)) as i32 - i32::from(lanes / 2);
    if candidate.usable && raw_lane_free(field, candidate, e, lane) {
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

/// `Detonator_SpawnObject`: lays up to `count` objects of `kinds` down the track in one lane,
/// `spacing` entries apart.
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
        let Some(entry) = survey.entries.get(position) else {
            return;
        };
        if lane_free(field, entry, position, lane) {
            let (Ok(entry), Ok(lane)) = (u16::try_from(position), i8::try_from(lane)) else {
                return;
            };
            let placed = Placed { entry, lane };
            match kind {
                0 if usize::from(field.live_mines) < MAX_MINES => {
                    field.mines[usize::from(field.live_mines)] = placed;
                    field.live_mines += 1;
                    tally.placed_this_call += 1;
                }
                1 if usize::from(field.live_bombs) < MAX_BOMBS => {
                    field.bombs[usize::from(field.live_bombs)] = placed;
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
