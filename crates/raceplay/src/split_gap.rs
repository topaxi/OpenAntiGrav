//! The per-crossing split table behind the HUD's lap-line time gap.
//!
//! The lap code keeps, for each crossing index, the race clock (as a whole
//! second) of the first craft to reach it and whether the player and/or an
//! opponent has been over it; the second craft over, when one of the two is the
//! player, raises the gap that `oag_hud::time_gap` displays. Pulse's is
//! `Craft_UpdateLapProgress` (`0x08842a18`), Pure's the same function at
//! `0x08927a00`-ish, and HD's `0x000d06e0`, which also stamps two splits inside
//! the lap. How many crossings the table holds and how many splits a lap has
//! are the title's ([`oag_title::lap_hud::SplitGapRule`]). Render-side, out of
//! every hash.

use oag_hud::time_gap::{GapEvents, TimeGap};
use oag_title::lap_hud::SplitGapRule;

#[derive(Debug, Clone, Copy, Default)]
struct Entry {
    /// Race clock in whole seconds of the first craft over, once there is one.
    first: Option<i32>,
    player: bool,
    opponent: bool,
}

/// One craft's reach of a table index, as the lap update reports it.
#[derive(Debug, Clone, Copy)]
pub struct Crossing {
    /// The table index reached: the crossing count times the splits per lap,
    /// plus the split within the lap.
    pub index: usize,
    /// It is the player's craft.
    pub player: bool,
    /// It is that craft's finishing crossing (`craft+0x912`).
    pub finished: bool,
}

/// The table, the indices already seen per slot, and the display.
#[derive(Debug, Clone)]
pub struct SplitGap {
    rule: SplitGapRule,
    table: Vec<Entry>,
    seen: Vec<usize>,
    /// The display state machine.
    pub display: TimeGap,
}

impl SplitGap {
    /// An empty table obeying one title's rule.
    #[must_use]
    pub fn new(rule: SplitGapRule) -> Self {
        let splits = rule.splits_per_lap.max(1) as usize;
        Self {
            rule,
            table: vec![Entry::default(); rule.table_crossings as usize * splits],
            seen: Vec::new(),
            display: TimeGap::new(rule),
        }
    }

    /// Splits per lap, at least one.
    fn splits(&self) -> usize {
        self.rule.splits_per_lap.max(1) as usize
    }

    /// The table index of a craft on crossing count `count` that is `fraction`
    /// of the way round the lap, or `None` before the split table applies: the
    /// line itself is stamped from the second crossing on, a split inside a lap
    /// from the first.
    #[must_use]
    pub fn index_of(&self, count: usize, fraction: f32) -> Option<usize> {
        let splits = self.splits();
        let split = if splits > 1 {
            ((fraction * splits as f32) as usize).min(splits - 1)
        } else {
            0
        };
        let needed = if split == 0 { 2 } else { 1 };
        (count >= needed && count < self.rule.table_crossings as usize)
            .then_some(count * splits + split)
    }

    /// Folds one craft's reach of an index into the table; the event it raises,
    /// if any.
    pub fn cross(&mut self, now: f32, crossing: Crossing) -> GapEvents {
        let mut events = GapEvents::default();
        let Some(entry) = self.table.get_mut(crossing.index) else {
            return events;
        };
        let stamp = now as i32;
        match entry.first {
            None => {
                entry.first = Some(stamp);
                if crossing.player && !crossing.finished {
                    entry.player = true;
                    events.led_at = Some(stamp as f32);
                } else {
                    entry.opponent = true;
                }
            }
            Some(first) if crossing.player => {
                if entry.opponent && !entry.player && !crossing.finished {
                    events.gap = Some((now - first as f32, false));
                }
                entry.player = true;
            }
            Some(first) => {
                if entry.player && !entry.opponent && !crossing.finished {
                    events.gap = Some((now - first as f32, true));
                }
                entry.opponent = true;
            }
        }
        events
    }

    /// The crossing counts last seen, per slot - grown on demand.
    pub fn seen(&mut self, slot: usize) -> &mut usize {
        if self.seen.len() <= slot {
            self.seen.resize(slot + 1, 0);
        }
        &mut self.seen[slot]
    }
}

impl super::Race {
    /// Folds this tick's lap-line and split crossings into the split table and
    /// steps the gap display. After `update_standings`, so every craft's count
    /// is fresh.
    pub(super) fn tick_split_gap(&mut self) {
        if self.view.split_gap.is_none() {
            return;
        }
        let player = self.player_slot();
        let now = oag_race::race_clock_ticks(self.sim.world.tick) as f32 / 60.0;
        let mut events = GapEvents::default();
        let mut live = 0;
        let ships = usize::from(self.ship_count());
        let places = self.places();
        let length = self.sim.course.as_ref().map(oag_race::Course::length);
        let reached: Vec<(usize, usize, f32, bool)> = (0..ships)
            .map(|slot| {
                let ship = &self.sim.world.ships[slot];
                if ship.active && ship.physics.craft_state != oag_physics::CraftState::Eliminated {
                    live += 1;
                }
                let standing = &ship.standing;
                let count = if standing.lap_start_tick.is_some() {
                    standing.lap as usize
                } else {
                    0
                };
                let fraction = match (standing.progress, length) {
                    (Some(progress), Some(length)) if length > 0.0 => progress / length,
                    _ => 0.0,
                };
                (slot, count, fraction, standing.finish_tick.is_some())
            })
            .collect();
        let timed = matches!(
            self.sim.world.mode(),
            oag_race::Mode::TimeTrial | oag_race::Mode::SpeedLap
        );
        let Some(split) = self.view.split_gap.as_mut() else {
            return;
        };
        for (slot, count, fraction, finished) in reached {
            let Some(index) = split.index_of(count, fraction) else {
                continue;
            };
            let seen = split.seen(slot);
            if index > *seen {
                *seen = index;
                let one = split.cross(
                    now,
                    Crossing {
                        index,
                        player: slot == player,
                        finished,
                    },
                );
                events.led_at = events.led_at.or(one.led_at);
                events.gap = events.gap.or(one.gap);
            }
        }
        split.display.advance(
            self.sim.dt,
            now,
            events,
            live,
            u32::from(places[player]),
            timed,
        );
    }
}

#[cfg(test)]
mod tests;
