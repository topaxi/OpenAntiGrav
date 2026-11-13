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

use oag_gameplay::MAX_SHIPS;
use oag_tables::weapons::Weapon;
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
/// A lap with no damage.
const PERFECT_LAP: u32 = 5;
/// A weapon pickup from a pad.
const WEAPON_PAD: u32 = 8;
/// The perfect launch.
const BOOST_START: u32 = 11;
/// `Cross Start/Finish Line with full health`.
const FULL_HEALTH: u32 = 44;
/// `Taken 1st!` is award 21, `2nd` 22, `3rd` 23.
const TAKEN_BASE: u32 = 20;
/// `Hit with Rocket` is award 12; the weapons follow in [`weapon_offset`] order.
const HIT_WITH_BASE: u32 = 12;
/// `Shielded against Rocket` is award 32, in the same order.
const SHIELDED_BASE: u32 = 32;
/// The best place before the first evaluation.
const NO_PLACE: u8 = 99;

/// Where `weapon` sits in the original's weapon-type order (rocket 0, missile 1, plasma 2,
/// cannon 3, mine 4, quake 5, bomb 6, leach beam 7), which orders both the `Hit with` and the
/// `Shielded against` awards. Light Barrier (type 10, the ninth row of each) is not a weapon
/// this engine has; anything else pays nothing
/// (`docs/ghidra/functions/vita-2048-eu-v104/xp-award.md`, "Weapon hits").
fn weapon_offset(weapon: Weapon) -> Option<u32> {
    Some(match weapon {
        Weapon::Rocket => 0,
        Weapon::Missile => 1,
        Weapon::Plasma => 2,
        Weapon::Cannon => 3,
        Weapon::Mine => 4,
        Weapon::Quake => 5,
        Weapon::Bomb => 6,
        Weapon::LeachBeam => 7,
        _ => return None,
    })
}

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
    best_place: u8,
    last_shield: Option<f32>,
    damaged_lap: bool,
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
            best_place: NO_PLACE,
            last_shield: None,
            damaged_lap: false,
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
        if zone > 0 && zone <= 100 && zone.is_multiple_of(10) {
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

    /// The player's best place so far, `Taken 1st/2nd/3rd!` once each: the first time the place
    /// reaches `1`, `2` or `3` with more craft than that in the race (`Race_SortPlaces`). The
    /// best place is kept for every place, so a craft that first shows up fourth and climbs to
    /// third is paid for third. Call only once the player has crossed the line.
    pub fn observe_taken(&mut self, place: u8, field: usize) {
        if place >= self.best_place {
            return;
        }
        self.best_place = place;
        if (1..=3).contains(&place) && field > usize::from(place) {
            self.award(TAKEN_BASE + u32::from(place));
        }
    }

    /// The player's health this tick. A drop is damage: the original sets the lap's damage flag
    /// on any damage above nothing (`Pilot_TakeDamage`), and a hit a shield absorbs never
    /// reaches it, so it never lowers the health either.
    pub fn observe_health(&mut self, shield: f32) {
        if self.last_shield.is_some_and(|before| shield < before) {
            self.damaged_lap = true;
        }
        self.last_shield = Some(shield);
    }

    /// The player completed a lap with `shield` of at most `max` health: `Perfect lap` when no
    /// damage came in the lap, then `Cross Start/Finish Line with full health` at the maximum,
    /// then the lap's damage is forgotten (`Pilot_UpdateLapProgress`).
    pub fn lap_completed(&mut self, shield: f32, max: f32) {
        self.observe_health(shield);
        if !self.damaged_lap {
            self.award(PERFECT_LAP);
        }
        if shield >= max {
            self.award(FULL_HEALTH);
        }
        self.damaged_lap = false;
    }

    /// A weapon pickup was handed over by a pad.
    pub fn weapon_pad_taken(&mut self) {
        self.award(WEAPON_PAD);
    }

    /// The perfect launch (`Race_UpdateLaunchGrade`'s grade 2).
    pub fn perfect_start(&mut self) {
        self.award(BOOST_START);
    }

    /// The player's weapon damaged another craft: `Hit with <weapon>`.
    pub fn weapon_hit(&mut self, weapon: Weapon) {
        if let Some(offset) = weapon_offset(weapon) {
            self.award(HIT_WITH_BASE + offset);
        }
    }

    /// The player's shield swallowed a hit: `Shielded against <weapon>`.
    pub fn shielded_against(&mut self, weapon: Weapon) {
        if let Some(offset) = weapon_offset(weapon) {
            self.award(SHIELDED_BASE + offset);
        }
    }

    /// The race-end awards: `Event complete` as `completion` says and `Finished Nth`.
    pub fn finish(&mut self, completion: Completion, place: Option<u8>) {
        if completion.pass {
            self.award(award::EVENT_PASS);
        }
        if completion.elite {
            self.award(award::EVENT_ELITE);
        }
        if let Some(p) = place.filter(|p| (1..=8).contains(p)) {
            self.award(award::FINISHED_1ST + u32::from(p) - 1);
        }
    }

    /// What the completion awards paid at their authored value, chain bonuses left out: the
    /// end-race page's `PassBonusXP`.
    #[must_use]
    pub fn completion_xp(&self) -> u32 {
        [award::EVENT_PASS, award::EVENT_ELITE]
            .into_iter()
            .map(|index| self.awards.xp(index, self.column) * self.times_paid(index))
            .sum()
    }
}

/// Which `Event complete` awards a finish pays.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Completion {
    /// `Event complete- PASS`.
    pub pass: bool,
    /// `Event complete- ELITE`.
    pub elite: bool,
}

impl Completion {
    /// The awards a result pays given the best before it, each `None` for no pass, `Some(false)`
    /// for a pass and `Some(true)` for an elite. The completion awards pay on improvement only:
    /// the end-race page shows `PassBonusXP` when the result state differs from the best before
    /// (`docs/ghidra/functions/vita-2048-eu-v104/xp-award.md`, "Completion pays on improvement").
    /// An elite on a first pass pays both, an elite over a pass the Elite alone.
    #[must_use]
    pub fn on_improvement(before: Option<bool>, now: Option<bool>) -> Self {
        let rank = |tier: Option<bool>| tier.map_or(0, |elite| 1 + u8::from(elite));
        if rank(now) <= rank(before) {
            return Self::default();
        }
        Self {
            pass: before.is_none(),
            elite: now == Some(true),
        }
    }
}

impl crate::Race {
    /// Starts counting this race's in-race XP into `tally`.
    pub fn arm_xp(&mut self, tally: XpTally) {
        self.sim.xp = Some(Box::new(tally));
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
        self.sim.xp.as_deref()
    }

    /// The race's XP tally, to bank it at the finish.
    pub fn xp_tally_mut(&mut self) -> Option<&mut XpTally> {
        self.sim.xp.as_deref_mut()
    }

    /// Whether the player's in-race awards count this tick: the tally is armed and the player is
    /// racing (`FUN_8100d2a6` is false on the grid and after the finish). The boost start and the
    /// N-seconds-ahead awards skip this gate in the original and do not ask it.
    fn xp_racing(&self) -> bool {
        self.sim.xp.is_some() && {
            let standing = &self.sim.world.ships[self.player_slot()].standing;
            standing.finish_tick.is_none()
                && !oag_race::RaceState::thrust_gated(self.sim.world.tick)
        }
    }

    /// Feeds the tally what the player did this tick: kills, place, health. The zone step and the
    /// lap are fed from `advance_player_rules` where their edges are.
    pub(super) fn observe_xp(&mut self) {
        if !self.xp_racing() {
            return;
        }
        let slot = self.player_slot();
        let ship = &self.sim.world.ships[slot];
        let (kills, shield) = (ship.standing.kills, ship.physics.shield);
        let crossed = ship.standing.lap_start_tick.is_some();
        let place = self.sim.places()[slot];
        let field = usize::from(self.sim.world.ship_count);
        if let Some(xp) = self.sim.xp.as_mut() {
            xp.observe_kills(kills);
            xp.observe_place(place, field);
            xp.observe_health(shield);
            if crossed {
                xp.observe_taken(place, field);
            }
        }
    }

    /// The player completed a lap this tick.
    pub(super) fn xp_lap_completed(&mut self) {
        let ship = &self.sim.world.ships[self.player_slot()];
        let (shield, max) = (ship.physics.shield, ship.handling.dimensions.shield);
        if let Some(xp) = self.sim.xp.as_mut() {
            xp.lap_completed(shield, max);
        }
    }

    /// A pad handed `slot` a weapon.
    pub(super) fn xp_weapon_pad(&mut self, slot: usize) {
        if slot == self.player_slot()
            && self.xp_racing()
            && let Some(xp) = self.sim.xp.as_mut()
        {
            xp.weapon_pad_taken();
        }
    }

    /// `slot` made the perfect launch. No gate.
    pub(super) fn xp_perfect_start(&mut self, slot: usize) {
        if slot == self.player_slot()
            && let Some(xp) = self.sim.xp.as_mut()
        {
            xp.perfect_start();
        }
    }

    /// This tick's weapon hits, from the player's side: a hit the player's shield swallowed, and
    /// a hit the player's weapon landed on another craft. `by[slot]` names who hit `slot` and
    /// with what, where the caller can tell (a weapon with no owner on its hit report stays
    /// unpaid: Leach Beam, whose drain is per tick, and Light Barrier, which this engine has not).
    pub(super) fn xp_weapon_hits(
        &mut self,
        hits: &[oag_weapons::projectile::WeaponHit; MAX_SHIPS],
        by: &[Option<(u8, Weapon)>; MAX_SHIPS],
    ) {
        if !self.xp_racing() {
            return;
        }
        let me = self.player_slot();
        let Some(xp) = self.sim.xp.as_mut() else {
            return;
        };
        if hits[me].absorbed
            && let Some(weapon) = hits[me].weapon
        {
            xp.shielded_against(weapon);
        }
        for (slot, hit) in hits.iter().enumerate() {
            if slot != me
                && hit.landed
                && let Some((owner, weapon)) = by[slot]
                && usize::from(owner) == me
            {
                xp.weapon_hit(weapon);
            }
        }
    }
}

#[cfg(test)]
mod tests;
