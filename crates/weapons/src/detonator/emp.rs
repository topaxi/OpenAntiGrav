//! Detonator's EMP: the pads charge it, the gauge fills, the EMP button sends a wave down the
//! track that clears every mine and bomb in the sections it sweeps.
//!
//! The law is `docs/ghidra/functions/ps3-hdfury-eu/detonator-emp.md` (HD static plus two RPCS3
//! runs, 60-92 per piece). A pad raises the craft's charge flag; [`step_charge`] spends it as
//! `Detonator_UpdateRace` does (one charge, the gauge, the two announcer edges); [`fire`] is the
//! EMP arm of `Ship_UpdateCannonTrigger`; [`step_wave`] is the wave's kill.
//!
//! # The wave
//!
//! `EMP_UpdateWave` (`0x0013c320`) adds the frame's `dt` to the wave's age, steps its AI-track
//! cursor `speed` points forward (the table's `speed`, `1` on the disc) and ends once the age
//! reaches `extent = minDistance + (maxDistance - minDistance) * gauge`: `extent` is a lifetime
//! in seconds, `0.25` to `2.0` (`detonator-emp.md`, "The wave: a Repulser wave that clears
//! mines"). At HD's 60 frames a second a full EMP sweeps about 120 points. Here a point is a
//! survey entry, one per control point, which is what the original's placement records key on.
//!
//! # What is ours
//!
//! - **Which entries a tick clears**: the entry the front stands on and those it steps over, the
//!   start entry on the launch tick. The original kills the lanes of the point its cursor reaches
//!   (`EMP_SweepSections`); with `speed = 1` the two agree on every entry but, perhaps, the first.
//!   **Chosen** for that first entry.
//! - **The gauge's divide is `f32`**; the original divides in double and rounds to `f32`. Both
//!   operands are small integers, exact in `f32`, so the two agree on every `n` the HUD can
//!   reach (the page's live table, `0..39`).
//! - A bomb the wave takes sets nothing off: `BombManager_DestroyAtLane`'s chain is unread.
//! - **The forks**: `EMP_SweepSections` is the Repulser's wave stepper cloned, so a front that
//!   steps onto a split spawns a second wave on the alternate path (its `REP2` branch), one at a
//!   time. Ours follows `Repulser_AdvanceWave`'s shape ([`Fork`]), sharing the wave's age and
//!   lifetime. A front stepping off a merge releases it. `EMP_ForkAtJunction` adds the
//!   launch-time one ([`launch_fork`]): a craft within [`SIBLING_REACH`] of the other route at a
//!   split gets a second front there at once, and one born on the far half of that route never
//!   moves (`detonator-emp.md`, "The launch-time fork").
//! - **The other route's record**: a front on a point with no usable placement record destroys
//!   the lanes at the other route's nearest point instead (`sweep_entry`).

use oag_tables::detonator_layout::Scoring;

use super::{Event, EventKind, Field, Survey};

/// The `1 / 1.3` the auto-speed ramp climbs by per second (`0x3f44ec4f`,
/// `docs/ghidra/functions/ps3-hdfury-eu/detonator-speed.md`, "The ramp and the rescue").
const RAMP_RATE: f32 = 0.769_230_8;
/// The ramp's dead start.
const RAMP_DEAD: f32 = 0.1;
/// `1 / 0.9` (`0x3f8e38e4`).
const RAMP_SCALE: f32 = 1.111_111_2;

/// How near the other route's point must be to the firer's for `EMP_ForkAtJunction` to start a
/// second front on it: `dist^2 < 100.0` (`0x00784c80`).
pub const SIBLING_REACH: f32 = 10.0;

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
    /// `speed`: survey entries the wave steps a tick (`slot+0x1f8`, `EMPManager_Fire`; the
    /// sweep's step count, `EMP_UpdateWave`).
    pub speed: u32,
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
            speed: emp.speed.map_or(1, count),
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
    /// The [`Survey::entry`] index it started from: the ring's, or a branch's.
    pub from: u16,
    /// Entries the front has stepped so far; it stands that many on from [`Self::from`].
    pub front: u16,
    /// Seconds it lives, the launch's `extent` (`emp+0x54`).
    pub lifetime: f32,
    /// Seconds since launch (`emp+0x158`).
    pub age: f32,
    /// Whether a second front is out (`emp+0x188`): one at a time, until a front reaches a
    /// merge.
    pub forked: bool,
    /// The second front, meaningful while [`Self::forked`].
    pub fork: Fork,
}

/// The wave's second front on an alternate path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Fork {
    /// Which [`Survey::branches`] entry it was born on.
    pub branch: u8,
    /// Whether it has rejoined the ring: [`Self::at`] is then a ring entry.
    pub on_ring: bool,
    /// The entry it stands on: an offset into its branch, or a ring entry.
    pub at: u16,
}

impl Fork {
    /// The [`Self::branch`] of a launch fork born on the far half of the other route: the
    /// original only advances a second front travelling its own way, so it walks nothing
    /// (`0x0013bb54`) until the main front's merge releases it. The original leaves its effect
    /// and light at the world origin meanwhile; ours draws neither, a departure.
    pub const INERT: Self = Self {
        branch: u8::MAX,
        on_ring: false,
        at: 0,
    };

    /// The [`Survey::entry`] index it stands on, or `None` for one that walks nothing.
    fn standing(&self, survey: &Survey) -> Option<usize> {
        if self.branch == Self::INERT.branch {
            return None;
        }
        if self.on_ring {
            return Some(usize::from(self.at));
        }
        survey.branch_entry(usize::from(self.branch), usize::from(self.at))
    }

    /// Steps once and returns the [`Survey::entry`] index it lands on, or `None` for a branch
    /// the survey does not hold.
    fn step(&mut self, survey: &Survey) -> Option<usize> {
        let total = survey.entries.len();
        let branch = survey.branches.get(usize::from(self.branch))?;
        if total == 0 {
            return None;
        }
        if self.on_ring {
            self.at = u16::try_from((usize::from(self.at) + 1) % total).ok()?;
            return Some(usize::from(self.at));
        }
        let next = usize::from(self.at) + 1;
        if next < branch.entries.len() {
            self.at = u16::try_from(next).ok()?;
            return survey.branch_entry(usize::from(self.branch), next);
        }
        self.on_ring = true;
        self.at = branch.merge % u16::try_from(total).ok()?;
        Some(usize::from(self.at))
    }
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
                front: 0,
                lifetime: 0.0,
                age: 0.0,
                forked: false,
                fork: Fork {
                    branch: 0,
                    on_ring: false,
                    at: 0,
                },
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
            points: None,
        });
    }
    if g >= 1.0 && emp.full_armed {
        emp.full_armed = false;
        events.push(Event {
            kind: EventKind::EmpFull,
            at,
            points: None,
        });
    }
    events
}

/// The entry `offset` on from the launch entry `from`: round the ring, or along a branch and on
/// round the ring from its merge.
fn route_entry(survey: &Survey, from: usize, offset: usize) -> Option<usize> {
    let total = survey.entries.len();
    if from < total {
        return Some((from + offset) % total);
    }
    let (branch, at) = survey.branch_of(from)?;
    let mut cursor = Fork {
        branch: u8::try_from(branch).ok()?,
        on_ring: false,
        at: u16::try_from(at).ok()?,
    };
    let mut entry = from;
    for _ in 0..offset {
        entry = cursor.step(survey)?;
    }
    Some(entry)
}

/// Whether `entry` is the last of a route that ends in a merge: a branch's last, or the ring's
/// just short of a branch's rejoin. A front stepping off it releases the second front
/// (`0x0013c014`-`0x0013c040`, `detonator-emp.md`, "The launch-time fork").
fn ends_at_merge(survey: &Survey, entry: usize) -> bool {
    if let Some((branch, at)) = survey.branch_of(entry) {
        return at + 1 == survey.branches[branch].entries.len();
    }
    let total = survey.entries.len();
    survey
        .branches
        .iter()
        .any(|b| total > 0 && (entry + 1) % total == usize::from(b.merge))
}

/// Drops the second front, as the original releases its effect and clears its fork flag.
fn release_fork(wave: &mut Wave) {
    wave.forked = false;
    wave.fork = Fork::default();
}

/// Steps the second front `steps` entries, clearing each, and releases it once it has stepped
/// off a merge: it clears the first entry past and goes.
fn advance_fork(
    field: &mut Field,
    survey: &Survey,
    config: &Config,
    steps: usize,
    events: &mut Vec<Event>,
) {
    for _ in 0..steps {
        let leaves = field
            .emp
            .wave
            .fork
            .standing(survey)
            .is_some_and(|e| ends_at_merge(survey, e));
        let Some(entry) = field.emp.wave.fork.step(survey) else {
            break;
        };
        sweep_entry(field, survey, config, entry, events);
        if leaves {
            release_fork(&mut field.emp.wave);
            break;
        }
    }
}

/// `EMP_ForkAtJunction` (`0x0013b5e8`), the launch call's fork: a firer on one route of a split,
/// within [`SIBLING_REACH`] of the other, starts a second front on that other route's nearest
/// point. On the far half of it (`point > count / 2`) the front travels backward, which the EMP's
/// forward-only update never advances.
#[must_use]
pub fn launch_fork(survey: &Survey, from: usize) -> Option<Fork> {
    let total = survey.entries.len();
    let born = |offset: usize, count: usize, fork: Fork| {
        Some(if offset > count / 2 {
            Fork::INERT
        } else {
            fork
        })
    };
    if let Some((branch, at)) = survey.branch_of(from) {
        let b = &survey.branches[branch];
        let partner = (*b.to_primary.get(at)?).filter(|p| p.near)?;
        let offset = (usize::from(partner.at) + total - usize::from(b.split)) % total;
        let fork = Fork {
            branch: u8::try_from(branch).ok()?,
            on_ring: true,
            at: partner.at,
        };
        return born(offset, b.from_primary.len(), fork);
    }
    survey.branches.iter().enumerate().find_map(|(branch, b)| {
        let offset = (from + total).checked_sub(usize::from(b.split))? % total.max(1);
        let partner = (*b.from_primary.get(offset)?).filter(|p| p.near)?;
        let fork = Fork {
            branch: u8::try_from(branch).ok()?,
            on_ring: false,
            at: partner.at,
        };
        born(usize::from(partner.at), b.entries.len(), fork)
    })
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
        let fork = launch_fork(survey, usize::from(from));
        emp.wave = Wave {
            live: true,
            from,
            front: 0,
            lifetime: extent,
            age: 0.0,
            forked: fork.is_some(),
            fork: fork.unwrap_or_default(),
        };
    }
    emp.charge = 0;
    emp.gauge = 0.0;
    let at = survey
        .entry(usize::from(from))
        .map_or(oag_core::math::Vec3::ZERO, |e| e.centre);
    vec![Event {
        kind: EventKind::EmpLaunch,
        at,
        points: None,
    }]
}

/// One tick of the wave (`EMP_UpdateWave`, `0x0013c320`): the age grows by `dt`, the front clears
/// the entry it stands on and steps `speed` entries on, and the wave ends once the age has
/// reached its lifetime. Every mine and bomb on a cleared entry goes, scored with `<Emp>` (a mine
/// laid this stage `new_mine_score`, a dark one `old_mine_score`, a bomb `bomb_score`); the mines
/// count into the destroyed count. A front that steps onto a branch's split entry forks (see
/// [`Fork`]), and the fork clears what it walks too.
pub fn step_wave(field: &mut Field, survey: &Survey, config: &Config, dt: f32) -> Vec<Event> {
    let mut events = Vec::new();
    let total = survey.entries.len();
    let wave = &mut field.emp.wave;
    if !wave.live || total == 0 {
        return events;
    }
    wave.age += dt;
    let from = usize::from(wave.from);
    let first = usize::from(wave.front);
    let steps = config.speed.max(1) as usize;
    wave.front = wave.front.saturating_add(steps as u16);
    if wave.age >= wave.lifetime {
        wave.live = false;
    }
    let last = first + steps;
    // The main front moves first in the original, so a merge it steps off takes the second front
    // before that one moves this tick.
    let main_merges = (first.max(1)..last)
        .any(|step| route_entry(survey, from, step - 1).is_some_and(|e| ends_at_merge(survey, e)));
    if main_merges {
        release_fork(&mut field.emp.wave);
    }
    if field.emp.wave.forked {
        if first == 0 {
            // A launch fork stands on its point at birth, cleared as the front's start is.
            if let Some(entry) = field.emp.wave.fork.standing(survey) {
                sweep_entry(field, survey, config, entry, &mut events);
            }
        } else {
            advance_fork(field, survey, config, steps, &mut events);
        }
    }
    for step in first..last.min(total) {
        let Some(entry) = route_entry(survey, from, step) else {
            break;
        };
        sweep_entry(field, survey, config, entry, &mut events);
        let wave = &mut field.emp.wave;
        if wave.forked || step == 0 {
            continue;
        }
        let Some(branch) = survey
            .branches
            .iter()
            .position(|b| usize::from(b.split) == entry && !b.entries.is_empty())
        else {
            continue;
        };
        let Ok(branch_id) = u8::try_from(branch) else {
            continue;
        };
        wave.forked = true;
        wave.fork = Fork {
            branch: branch_id,
            on_ring: false,
            at: 0,
        };
        if let Some(first_entry) = survey.branch_entry(branch, 0) {
            sweep_entry(field, survey, config, first_entry, &mut events);
        }
        advance_fork(field, survey, config, last - step - 1, &mut events);
    }
    events
}

/// The [`Survey::entry`] index the wave's second front stands on, or `None` before it forks,
/// once the wave has ended and for one that walks nothing.
#[must_use]
pub fn fork_entry(wave: &Wave, survey: &Survey) -> Option<usize> {
    if !wave.live || !wave.forked {
        return None;
    }
    wave.fork.standing(survey)
}

/// The survey entry the wave's front stands on, or `None` with no wave in flight.
#[must_use]
pub fn front_entry(wave: &Wave, survey: &Survey) -> Option<usize> {
    if !wave.live || survey.entries.is_empty() {
        return None;
    }
    route_entry(survey, usize::from(wave.from), usize::from(wave.front))
}

/// What a front does on the point it reaches (`EMP_SweepSections`): destroys the lanes of the
/// point's own placement record, or, with none, those of the record at the other route's nearest
/// point (`0x0013baa4`-`0x0013bb2c`, `detonator-emp.md`, "The other route's record").
fn sweep_entry(
    field: &mut Field,
    survey: &Survey,
    config: &Config,
    entry: usize,
    events: &mut Vec<Event>,
) {
    let target = if survey.has_record(entry) {
        entry
    } else {
        match survey.sibling(entry).filter(|&s| survey.has_record(s)) {
            Some(sibling) => sibling,
            None => return,
        }
    };
    clear_entry(field, survey, config, target, events);
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
            .mine_point(&placed, config.lane_width)
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
            points: Some(points),
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
        let cannon = &mut field.cannon;
        cannon.score = cannon.score.saturating_add(config.scoring.emp_bomb_score);
        cannon.destroyed_bombs = cannon.destroyed_bombs.saturating_add(1);
        events.push(Event {
            kind: EventKind::BombEmped,
            at,
            points: Some(config.scoring.emp_bomb_score),
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
