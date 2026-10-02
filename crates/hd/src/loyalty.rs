//! Wipeout HD / Fury's loyalty award, as `0x00023f98` computes it.
//!
//! Recovered 2026-10-02 from the executable and written up, with every address
//! and the second site that agrees with it, in
//! `docs/ghidra/functions/ps3-hdfury-eu/endrace-loyalty.md`. Static reading only:
//! no live run backs it, so the rates below carry that page's confidence (82),
//! not a measured one.
//!
//! **These are not Pulse's numbers.** HD's Eliminator pays 15/25 per lap rather
//! than 10/20, its zones pay 5/15 rather than 10/20, it has a Detonator row
//! that pays 150 per stage and has no kill term, and its difficulty multiplier
//! has no "x1" on a race with an AI craft in it. The shape (laps, perfect laps,
//! zones, perfect zones, kills, a multiplier, a `100000` ceiling on the banked
//! total) is Pulse's; the constants are this file's.
//!
//! What this leaves out on purpose: the executable has a byte at `0x009384e1`
//! that substitutes default rates when it is set (`docs/ghidra/functions/
//! ps3-hdfury-eu/camera.md` records it as "meaning not read"), and nothing in
//! this project sets it.

#[cfg(test)]
mod tests;

/// The ceiling on a team's banked total - `0x00023f98` clamps the record to it.
pub const CAP: u32 = 100_000;

/// `g_GameState+0xe0` values, the ones the law branches on. The names are
/// `docs/ghidra/functions/ps3-hdfury-eu/mode-manager.md`'s: `3` is `SPArcade`
/// (a live RPCS3 run of a campaign cell logged `GetMode()==3`), `4`
/// `SPTournament`, `5`/`10` `SPTimeTrial`, `8` `SPElimination`. `6` (Zone) and
/// `14` (Detonator) are readings of the unnamed ids, 75.
pub mod mode {
    /// `SPArcade`: a single race.
    pub const SINGLE_RACE: u32 = 3;
    /// `SPTournament`.
    pub const TOURNAMENT: u32 = 4;
    /// `SPTimeTrial`.
    pub const TIME_TRIAL: u32 = 5;
    /// A `ModeManager`-less id the loading screen and the medal evaluator both
    /// treat as Zone.
    pub const ZONE: u32 = 6;
    /// `SPElimination`.
    pub const ELIMINATION: u32 = 8;
    /// `SPTimeTrial`'s second id.
    pub const SPEED_LAP: u32 = 10;
    /// A `ModeManager`-less id the camera and the texture loader treat as
    /// Detonator.
    pub const DETONATOR: u32 = 14;
}

/// The campaign difficulty rung, `g_GameState+0xdc`: `0` easy, `1` medium, `2`
/// hard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tier {
    Easy,
    Medium,
    Hard,
}

/// What the race tallied - the fields of the stats block at `stats+0x1330`,
/// `+0x1334`, `+0x1308`, `+0x130c` and `+0x1344`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Tally {
    /// Laps completed (`+0x1330`).
    pub laps: u32,
    /// Perfect laps (`+0x1334`).
    pub perfect_laps: u32,
    /// Zones cleared (`+0x1308`).
    pub zones: u32,
    /// Perfect zones (`+0x130c`).
    pub perfect_zones: u32,
    /// Kills (`+0x1344`).
    pub kills: u32,
}

/// Everything `0x00023f98` reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Inputs {
    /// `g_GameState+0xe0`, one of [`mode`]'s ids.
    pub mode: u32,
    pub tally: Tally,
    /// `g_GameState+0xdc`.
    pub tier: Tier,
    /// `byte(g_GameState+0xe4) != *g_GameState`: not every craft on the grid is
    /// a local player. Read as "an AI craft is racing", 70.
    pub ai_present: bool,
}

/// Points per lap and per perfect lap. The four classes are the bitmasks
/// `0x332318` (modes 3 4 8 9 13 16 17 20 21), `0x420` (5 10), `0x4000` (14) and
/// everything else.
const fn lap_rates(mode: u32) -> (u32, u32) {
    match mode {
        3 | 4 | 8 | 9 | 13 | 16 | 17 | 20 | 21 => (15, 25),
        5 | 10 => (30, 50),
        14 => (150, 20),
        _ => (10, 20),
    }
}

/// `Race_ComputeLoyaltyAward`, minus the banking: the points this race earns.
#[must_use]
pub fn award(inputs: Inputs) -> u32 {
    let Inputs {
        mode,
        tally,
        tier,
        ai_present,
    } = inputs;
    let (lap_rate, perfect_rate) = lap_rates(mode);
    let mut sum = tally
        .laps
        .saturating_mul(lap_rate)
        .saturating_add(tally.perfect_laps.saturating_mul(perfect_rate))
        .saturating_add(tally.zones.saturating_mul(5))
        .saturating_add(tally.perfect_zones.saturating_mul(15));
    // Detonator has no kill term at all.
    if mode != mode::DETONATOR {
        let kill_rate = if mode == mode::ELIMINATION || mode == 0x14 {
            30
        } else {
            15
        };
        sum = sum.saturating_add(tally.kills.saturating_mul(kill_rate));
    }
    let mut multiplier = 1;
    if matches!(mode, 3 | 4 | 8 | 9 | 13) && ai_present {
        multiplier = match tier {
            Tier::Easy => 2,
            Tier::Medium => 3,
            Tier::Hard => 4,
        };
    }
    // Online modes pay x3 whatever the rung.
    if mode > 0xf {
        multiplier = 3;
    }
    sum.saturating_mul(multiplier)
}

/// A team's running total after `award` is folded in: `record+0x6c += award`,
/// clamped to [`CAP`].
#[must_use]
pub fn bank(total: u32, award: u32) -> u32 {
    total.saturating_add(award).min(CAP)
}
