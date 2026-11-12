//! A race's XP tally: what `Profile_AwardXP` (`0x810134ce`) pays into the pilot's race total as
//! the race goes (`docs/ghidra/functions/vita-2048-eu-v104/xp-award.md`, "In-race awards").
//!
//! **Not `World` state and never read back by the simulation.** It is fed from edges the
//! simulation already reports (a zone step, a kill, the player's place) and is absent from the
//! state hash, like the cue queue. A rewind restores it with the rest of [`crate::RaceSim`].
//!
//! The chain rule is the original's code: an award whose column pays nothing is dropped; one that
//! lands while the 2.0 s window is open raises the chain counter, and every award reopens the
//! window; the counter reaching five pays `chain * 2` as `AWARD_GROUP_CHAIN` and resets. The
//! window is counted in fixed 60 Hz ticks, standing for the original's variable timestep
//! (`docs/architecture/adr/0007-fixed-timestep-vs-original.md`).

use std::collections::BTreeMap;

use oag_tables::xp::{Awards, Column, award};

/// The chain window, 2.0 s at the fixed 60 Hz timestep.
pub const CHAIN_WINDOW_TICKS: u32 = 120;
/// The chain counter value that pays the bonus.
pub const CHAIN_LENGTH: u32 = 5;
/// `Hit Zone 10` is award 48; zone `10 * n` pays `47 + n`.
const HIT_ZONE_BASE: u32 = 47;
/// `Held 1st for 30sec`, `20sec`, `10sec` are awards 41, 42, 43.
const HELD_FIRST_30: u32 = 41;
const HELD_FIRST_20: u32 = 42;
const HELD_FIRST_10: u32 = 43;
/// Opponent destroyed.
const OPPONENT_DESTROYED: u32 = 6;
/// `Zone` (a zone with a wall contact) and `Perfect Zone`.
const ZONE: u32 = 9;
const PERFECT_ZONE: u32 = 10;

/// One race's running XP.
#[derive(Debug, Clone, PartialEq)]
pub struct XpTally {
    awards: Awards,
    column: Column,
    total: u32,
    chain: u32,
    window: u32,
    kills_seen: u32,
    held_place: u8,
    held_ticks: u32,
    paid: BTreeMap<u32, u32>,
}

impl XpTally {
    /// An empty tally paying `column` of `awards`.
    #[must_use]
    pub fn new(awards: Awards, column: Column) -> Self {
        Self {
            awards,
            column,
            total: 0,
            chain: 0,
            window: 0,
            kills_seen: 0,
            held_place: 0,
            held_ticks: 0,
            paid: BTreeMap::new(),
        }
    }

    /// XP gathered and not yet banked, chain bonuses included.
    #[must_use]
    pub fn total(&self) -> u32 {
        self.total
    }

    /// How many times award `index` has paid.
    #[must_use]
    pub fn times_paid(&self, index: u32) -> u32 {
        self.paid.get(&index).copied().unwrap_or(0)
    }

    /// Hands the gathered XP over for banking and starts again from nothing.
    pub fn take(&mut self) -> u32 {
        self.chain = 0;
        self.window = 0;
        std::mem::take(&mut self.total)
    }

    /// One tick of the chain window (`Profile_Update`, `0x81013708`): while it is open it runs
    /// down, and closing it clears the chain.
    pub fn advance_window(&mut self) {
        if self.window > 0 {
            self.window -= 1;
            if self.window == 0 {
                self.chain = 0;
            }
        }
    }

    /// Pays award `index` (`Profile_AwardXP`): its value in this event's column, then the chain.
    /// Returns what was added. An award the column does not pay is dropped whole and does not
    /// touch the chain. The multipliers are 1.0 on the shipped disc and are not applied.
    pub fn award(&mut self, index: u32) -> u32 {
        let xp = self.awards.xp(index, self.column);
        if xp == 0 {
            return 0;
        }
        self.total = self.total.saturating_add(xp);
        *self.paid.entry(index).or_insert(0) += 1;
        let mut added = xp;
        if self.window > 0 {
            self.chain += 1;
        }
        self.window = CHAIN_WINDOW_TICKS;
        if self.chain == CHAIN_LENGTH {
            let bonus = self.chain * 2;
            self.total = self.total.saturating_add(bonus);
            added += bonus;
            self.chain = 0;
        }
        added
    }

    /// The player's zone count stepped to `zone`: `Perfect Zone` when no wall was touched in it,
    /// else `Zone`, then `Hit Zone N` on every tenth zone up to 100.
    pub fn zone_cleared(&mut self, zone: u16, perfect: bool) {
        self.award(if perfect { PERFECT_ZONE } else { ZONE });
        if zone > 0 && zone <= 100 && zone % 10 == 0 {
            self.award(HIT_ZONE_BASE + u32::from(zone) / 10);
        }
    }

    /// The player's kill count this tick: one `Opponent destroyed` per new kill.
    pub fn observe_kills(&mut self, kills: u32) {
        for _ in self.kills_seen..kills {
            self.award(OPPONENT_DESTROYED);
        }
        self.kills_seen = self.kills_seen.max(kills);
    }

    /// The player's place this tick (`1` is first) among `field` craft. A place held without a
    /// change for 10, 20 and then 30 s pays `Held 1st for N sec`, only in first place, only with
    /// rivals, and not in elimination, where the original skips it
    /// (`GameMode_IsEliminatorFamily_q`).
    pub fn observe_place(&mut self, place: u8, field: usize) {
        if place == self.held_place {
            self.held_ticks += 1;
        } else {
            self.held_place = place;
            self.held_ticks = 0;
            return;
        }
        let ticks = self.held_ticks;
        if place != 1 || field < 2 || self.column == Column::Elim {
            return;
        }
        let (ten, twenty, thirty) = (10 * 60, 20 * 60, 30 * 60);
        if ticks == thirty {
            self.award(HELD_FIRST_30);
        } else if ticks == twenty {
            self.award(HELD_FIRST_20);
        } else if ticks == ten {
            self.award(HELD_FIRST_10);
        }
    }

    /// The race-end awards: `Event complete` (Elite or Pass; `None` pays none) and `Finished Nth`.
    pub fn finish(&mut self, completion: Option<bool>, place: Option<u8>) {
        if let Some(elite) = completion {
            self.award(if elite {
                award::EVENT_ELITE
            } else {
                award::EVENT_PASS
            });
        }
        if let Some(p) = place.filter(|p| (1..=8).contains(p)) {
            self.award(award::FINISHED_1ST + u32::from(p) - 1);
        }
    }
}

impl crate::Race {
    /// Starts counting this race's in-race XP into `tally`.
    pub fn arm_xp(&mut self, tally: XpTally) {
        self.sim.xp = Some(tally);
    }

    /// Arms the tally an event's own `XPAwards.xml` and mode column call for; nothing when the
    /// package carried no table.
    pub fn arm_event_xp(&mut self, progress: &crate::Campaign2048Progress) {
        if let Some(awards) = &progress.xp_awards {
            self.arm_xp(XpTally::new(awards.clone(), progress.xp_column));
        }
    }

    /// The race's XP tally, when the event pays XP.
    #[must_use]
    pub fn xp_tally(&self) -> Option<&XpTally> {
        self.sim.xp.as_ref()
    }

    /// The race's XP tally, to bank it at the finish.
    pub fn xp_tally_mut(&mut self) -> Option<&mut XpTally> {
        self.sim.xp.as_mut()
    }

    /// Feeds the tally what the player did this tick: kills and place. Counts only while the
    /// player is racing (`FUN_8100d2a6` is false on the grid and after the finish); the zone
    /// step is fed from `advance_player_rules` where its edge is.
    pub(super) fn observe_xp(&mut self) {
        let slot = self.player_slot();
        if self.sim.xp.is_none() {
            return;
        }
        let standing = self.sim.world.ships[slot].standing;
        if standing.finish_tick.is_some() || oag_race::RaceState::thrust_gated(self.sim.world.tick)
        {
            return;
        }
        let place = self.sim.places()[slot];
        let field = usize::from(self.sim.world.ship_count);
        if let Some(xp) = self.sim.xp.as_mut() {
            xp.observe_kills(standing.kills);
            xp.observe_place(place, field);
        }
    }
}

#[cfg(test)]
mod tests;
