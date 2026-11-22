//! Detonator's EMP: the pads charge it, the gauge fills, the EMP button sends a wave down the
//! track that clears every mine and bomb in the sections it sweeps.
//!
//! The law is `docs/ghidra/functions/ps3-hdfury-eu/detonator-emp.md` (HD static plus two RPCS3
//! runs, 60-92 per piece). A pad raises the craft's charge flag; [`step_charge`] spends it as
//! `Detonator_UpdateRace` does (one charge, the gauge, the two announcer edges); [`fire`] is the
//! EMP arm of `Ship_UpdateCannonTrigger`; [`step_wave`] is the wave's kill.
//!
//! # What is ours
//!
//! - **The wave's reach and speed** are unread (`detonator-emp.md`, "The wave's kill": how
//!   `EMP_SweepSections`' caller turns `emp+0x54` into a section count). Here the wave runs
//!   forward from the craft's survey entry over [`SECTIONS_PER_EXTENT`] `* extent` entries,
//!   linearly over the table's `wave_time`. **Chosen, not measured**; recipe on the page (an
//!   RPCS3 breakpoint on `MineManager_DestroyAtLane`, `0x0011a590`, after a fire, logging the
//!   section against `ship+0x6284`). Live the original cleared 12 mines over 1.8 s ahead of the
//!   craft at gauge `1`.
//! - **The gauge's divide is `f32`**; the original divides in double and rounds to `f32`. Both
//!   operands are small integers, exact in `f32`, so the two agree on every `n` the HUD can
//!   reach (the page's live table, `0..39`).
//! - A bomb the wave takes sets nothing off: `BombManager_DestroyAtLane`'s chain is unread.

use oag_tables::detonator_layout::Scoring;

use super::{Event, EventKind, Field, Survey};

/// Survey entries the wave sweeps per unit of `extent` (`0.25 .. 2.0`). **Chosen, not
/// measured** (see the module): at gauge `1` the wave reaches eighty sections ahead, a few
/// hundred metres on an HD circuit, which is the order of the live run's 1.8 s of kills.
pub const SECTIONS_PER_EXTENT: f32 = 40.0;

/// The `1 / 1.3` the auto-speed ramp climbs by per second (`0x3f44ec4f`,
/// `docs/ghidra/functions/ps3-hdfury-eu/detonator-speed.md`, "The ramp and the rescue").
const RAMP_RATE: f32 = 0.769_230_8;
/// The ramp's dead start.
const RAMP_DEAD: f32 = 0.1;
/// `1 / 0.9` (`0x3f8e38e4`).
const RAMP_SCALE: f32 = 1.111_111_2;

/// What the EMP reads of the title's tables.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Config {
    /// `minNumPadsToCollectEMP`: charges before the EMP is ready (`table+0x1bc`).
    pub lo: u32,
    /// `maxNumPadsToCollectEMP` (`table+0x1c0`).
    pub hi: u32,
    /// `minDistance`: the wave's extent at gauge `0`.
    pub min_distance: f32,
    /// `maxDistance`: at gauge `1`.
    pub max_distance: f32,
    /// `wave_time`: seconds the wave takes to run its reach (chosen use, see the module).
    pub wave_time: f32,
    /// `DistanceOffsetBetweenMines`, to place a laid object.
    pub lane_width: f32,
    /// `DetonatorScoring.xml`'s `<Emp>` values.
    pub scoring: Scoring,
}

impl Config {
    /// Reads the Detonator weapon table's `EMP` block, or `None` where it authors none or
    /// lacks the two pad counts.
    #[must_use]
    pub fn from_tables(
        stats: &oag_tables::weapons::WeaponStats,
        lane_width: f32,
        scoring: Scoring,
    ) -> Option<Self> {
        let emp = stats.detonator()?.emp?;
        let count = |v: f32| v.max(0.0) as u32;
        Some(Self {
            lo: count(emp.min_num_pads_to_collect?),
            hi: count(emp.max_num_pads_to_collect?),
            min_distance: emp.min_distance.unwrap_or(0.0),
            max_distance: emp.max_distance.unwrap_or(0.0),
            wave_time: emp.wave_time.unwrap_or(1.0),
            lane_width,
            scoring,
        })
    }
}

/// A wave in flight.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Wave {
    /// Whether a wave is running (`EMPManager`'s one at a time).
    pub live: bool,
    /// The survey entry it started from.
    pub from: u16,
    /// Survey entries it sweeps in all.
    pub reach: f32,
    /// Entries swept so far.
    pub swept: f32,
    /// Seconds since launch.
    pub age: f32,
}

/// The EMP's state: the charge, the gauge, the cue latches and the wave.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Emp {
    /// Whether the race runs an EMP; a field that never had one stays empty.
    pub armed: bool,
    /// A pad raised the charge flag this tick (`ship+0x6a64 & 0x8000000`).
    pub pending: bool,
    /// Charges collected since the last fire (`rm+0x2dd8`).
    pub charge: u32,
    /// The gauge, `0..=1` (`craft+0x7da0`).
    pub gauge: f32,
    /// `EMPREADY` may still sound (`rm+0x2e1d`).
    pub ready_armed: bool,
    /// `EMPFULL` may still sound (`rm+0x2e1e`).
    pub full_armed: bool,
    /// The weapon pad that last charged (`ship+0x7960`), cleared on each start-line crossing.
    pub weapon_pad: Option<u16>,
    /// The wave.
    pub wave: Wave,
}

impl Emp {
    /// An EMP the race has not armed.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            armed: false,
            pending: false,
            charge: 0,
            gauge: 0.0,
            ready_armed: true,
            full_armed: true,
            weapon_pad: None,
            wave: Wave {
                live: false,
                from: 0,
                reach: 0.0,
                swept: 0.0,
                age: 0.0,
            },
        }
    }

    /// Whether the charge has reached the threshold the EMP fires at.
    #[must_use]
    pub fn ready(&self, config: &Config) -> bool {
        self.charge >= config.lo
    }
}

impl Default for Emp {
    fn default() -> Self {
        Self::new()
    }
}

/// The gauge at `n` charges: `0` below `lo`, else `min((n - (hi - lo)) / (hi - lo), 1)`.
///
/// Literal on purpose: it equals `(n - lo) / (hi - lo)` only because the disc's `hi - lo` is
/// `lo` (`detonator-emp.md`, "The charge and the gauge").
#[must_use]
pub fn gauge(n: u32, lo: u32, hi: u32) -> f32 {
    if n < lo {
        return 0.0;
    }
    let span = i64::from(hi) - i64::from(lo);
    if span <= 0 {
        return 1.0;
    }
    let g = (i64::from(n) - span) as f32 / span as f32;
    g.min(1.0)
}

/// Spends a pending charge: `n += 1`, the gauge, and the `EMPREADY`/`EMPFULL` edges
/// (`Detonator_UpdateRace`, `0x00067bb0`-`0x00067d64`). Returns the cues it raised.
pub fn step_charge(field: &mut Field, config: &Config) -> Vec<Event> {
    let mut events = Vec::new();
    let emp = &mut field.emp;
    emp.armed = true;
    if !emp.pending {
        return events;
    }
    emp.pending = false;
    emp.charge = emp.charge.saturating_add(1);
    if emp.charge < config.lo {
        emp.ready_armed = true;
        emp.full_armed = true;
        emp.gauge = 0.0;
        return events;
    }
    let g = gauge(emp.charge, config.lo, config.hi);
    emp.gauge = g;
    let at = oag_core::math::Vec3::ZERO;
    if g >= 0.0 && emp.ready_armed {
        emp.ready_armed = false;
        events.push(Event {
            kind: EventKind::EmpReady,
            at,
        });
    }
    if g >= 1.0 && emp.full_armed {
        emp.full_armed = false;
        events.push(Event {
            kind: EventKind::EmpFull,
            at,
        });
    }
    events
}

/// The EMP button, ready: launch a wave from survey entry `from` with
/// `extent = minDistance + (maxDistance - minDistance) * gauge`, then `n = 0` and gauge `0`.
/// A wave already in flight keeps running and the charge is spent anyway, as
/// `EMPManager_Fire` launches nothing while one is live and its caller resets the charge
/// regardless. The cannon's clip is untouched. Returns the launch, or nothing when not ready.
pub fn fire(field: &mut Field, survey: &Survey, config: &Config, from: u16) -> Vec<Event> {
    let emp = &mut field.emp;
    if !emp.ready(config) {
        return Vec::new();
    }
    let extent = config.min_distance + (config.max_distance - config.min_distance) * emp.gauge;
    if !emp.wave.live {
        emp.wave = Wave {
            live: true,
            from,
            reach: extent * SECTIONS_PER_EXTENT,
            swept: 0.0,
            age: 0.0,
        };
    }
    emp.charge = 0;
    emp.gauge = 0.0;
    let at = survey
        .entries
        .get(usize::from(from))
        .map_or(oag_core::math::Vec3::ZERO, |e| e.centre);
    vec![Event {
        kind: EventKind::EmpLaunch,
        at,
    }]
}

/// One tick of the wave: every mine and bomb on an entry the front passes this tick goes,
/// scored with `<Emp>` (a mine laid this stage `new_mine_score`, a dark one `old_mine_score`, a
/// bomb `bomb_score`); the mines count into the destroyed count.
pub fn step_wave(field: &mut Field, survey: &Survey, config: &Config, dt: f32) -> Vec<Event> {
    let mut events = Vec::new();
    let total = survey.entries.len();
    let wave = &mut field.emp.wave;
    if !wave.live || total == 0 {
        return events;
    }
    wave.age += dt;
    let share = if config.wave_time > 0.0 {
        (wave.age / config.wave_time).min(1.0)
    } else {
        1.0
    };
    let front = wave.reach * share;
    let (first, last) = (wave.swept as usize, front as usize);
    let from = usize::from(wave.from);
    wave.swept = front;
    if share >= 1.0 {
        wave.live = false;
    }
    for step in first..last.min(first + total) {
        let entry = (from + step) % total;
        clear_entry(field, survey, config, entry, &mut events);
    }
    events
}

fn clear_entry(
    field: &mut Field,
    survey: &Survey,
    config: &Config,
    entry: usize,
    events: &mut Vec<Event>,
) {
    let mut i = 0;
    while i < usize::from(field.live_mines) {
        let placed = field.mines[i];
        if usize::from(placed.entry) != entry {
            i += 1;
            continue;
        }
        let at = survey
            .point(&placed, config.lane_width)
            .map_or(oag_core::math::Vec3::ZERO, |(at, _)| at);
        let cannon = &mut field.cannon;
        let points = if placed.live {
            config.scoring.emp_new_mine_score
        } else {
            config.scoring.emp_old_mine_score
        };
        cannon.score = cannon.score.saturating_add(points);
        cannon.destroyed_mines = cannon.destroyed_mines.saturating_add(1);
        events.push(Event {
            kind: EventKind::MineEmped { live: placed.live },
            at,
        });
        field.remove_mine(i);
    }
    let mut i = 0;
    while i < usize::from(field.live_bombs) {
        let placed = field.bombs[i];
        if usize::from(placed.entry) != entry {
            i += 1;
            continue;
        }
        let at = survey
            .point(&placed, config.lane_width)
            .map_or(oag_core::math::Vec3::ZERO, |(at, _)| at);
        field.cannon.score = field
            .cannon
            .score
            .saturating_add(config.scoring.emp_bomb_score);
        events.push(Event {
            kind: EventKind::BombEmped,
            at,
        });
        field.remove_bomb(i);
    }
}

/// One tick of Detonator's auto-speed ramp (`craft+0x2d4`): `r += dt / 1.3` while under `1`,
/// returning `max(r - 0.1, 0) / 0.9`, and `1` once it has reached `1`. The original fuses
/// `r + dt * k` (`fmadds`); here it is a multiply then an add.
pub fn step_ramp(ramp: &mut f32, dt: f32) -> f32 {
    if *ramp >= 1.0 {
        return 1.0;
    }
    let next = *ramp + dt * RAMP_RATE;
    *ramp = if next > 1.0 { 1.0 } else { next };
    ramp_of(next)
}

/// The ramp's factor at the stored value `r` below `1`: `max(r - 0.1, 0) / 0.9`.
#[must_use]
pub fn ramp_of(r: f32) -> f32 {
    (r - RAMP_DEAD).max(0.0) * RAMP_SCALE
}

#[cfg(test)]
mod tests;
