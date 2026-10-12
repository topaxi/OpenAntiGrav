//! The per-crossing split table behind the HUD's lap-line time gap.
//!
//! `Craft_UpdateLapProgress` (`0x08842a18`) keeps, for each completed
//! crossing count `k`, the race clock (as a whole second) of the first craft to
//! reach it and whether the player and/or an opponent has been over it; the
//! second craft over the line, when one of the two is the player, raises the
//! gap that `oag_hud::time_gap` displays. Render-side, out of every hash.

use oag_hud::time_gap::{GapEvents, TimeGap};

/// Crossing counts the table holds (`uVar7 < 0x15`).
const ENTRIES: usize = 21;

#[derive(Debug, Clone, Copy, Default)]
struct Entry {
    /// Race clock in whole seconds of the first craft over, once there is one.
    first: Option<i32>,
    player: bool,
    opponent: bool,
}

/// One craft's crossing, as the lap update reports it.
#[derive(Debug, Clone, Copy)]
pub struct Crossing {
    /// The crossing count reached (`craft+0xac8`), at least 2.
    pub count: usize,
    /// It is the player's craft.
    pub player: bool,
    /// It is that craft's finishing crossing (`craft+0x912`).
    pub finished: bool,
}

/// The table, the crossings already seen per slot, and the display.
#[derive(Debug, Clone, Default)]
pub struct SplitGap {
    table: [Entry; ENTRIES],
    seen: Vec<usize>,
    /// The display state machine.
    pub display: TimeGap,
}

impl SplitGap {
    /// Folds one craft's crossing into the table; the event it raises, if any.
    pub fn cross(&mut self, now: f32, crossing: Crossing) -> GapEvents {
        let mut events = GapEvents::default();
        let Some(entry) = self.table.get_mut(crossing.count) else {
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
    /// Folds this tick's lap-line crossings into the split table and steps the
    /// gap display. After `update_standings`, so every craft's count is fresh.
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
        let crossings: Vec<(usize, usize, bool)> = (0..ships)
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
                (slot, count, standing.finish_tick.is_some())
            })
            .collect();
        let Some(split) = self.view.split_gap.as_mut() else {
            return;
        };
        for (slot, count, finished) in crossings {
            let seen = split.seen(slot);
            if count > *seen {
                *seen = count;
                if count >= 2 {
                    let one = split.cross(
                        now,
                        Crossing {
                            count,
                            player: slot == player,
                            finished,
                        },
                    );
                    events.led_at = events.led_at.or(one.led_at);
                    events.gap = events.gap.or(one.gap);
                }
            }
        }
        split
            .display
            .advance(self.sim.dt, now, events, live, u32::from(places[player]));
    }
}

#[cfg(test)]
mod tests;
