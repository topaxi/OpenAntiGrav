//! Detonator's pulse cannon: the clip, the rounds, what they hit, and the score.
//!
//! The law is `docs/ghidra/functions/ps3-hdfury-eu/detonator-cannon.md` (HD static, 55-85 per
//! piece). The craft's fixed cannon fires a volley of two rounds every `rate` seconds while the
//! trigger is held, out of a clip of `rounds` volleys; a round skims the track, bounces once off a
//! wall and is gone after a second. A round that meets a mine or a bomb scores
//! (`DetonatorScoring.xml`) and counts toward the stage percentage.
//!
//! [`step_cannon`] is one tick of the whole thing. It is generic over [`Craft`] and over the
//! collision query: a caller passes a closure that sweeps a segment through the track.
//!
//! # What is ours
//!
//! - **Muzzles** are the hull's `cannon_flash_left/right` locators
//!   ([`Config::muzzles`], carried by [`crate::projectile::plasma::hull_point`]), as the
//!   original fires them. A hull with none falls back to
//!   [`crate::projectile::cannon::launch`]'s offsets from the craft's dimensions (**chosen, not
//!   measured**). A round leaves along the craft's forward, whatever the locator's own axes.
//! - **The order inside a tick** is unread in the original (`detonator-cannon.md`, "Open"). Here:
//!   the trigger, the manual reload, the empty reload, the lockout, the streak reset, the volley,
//!   every round (update, then hit test, then retire), the streak scan. **Chosen.**
//! - **The clip starts full and ready**; what the original's slot holds at race build is unread
//!   (hypothesis, 50).
//! - **The reload button** is a press edge (the original's edge-or-level is unread). **Chosen.**
//! - **A shot mine leaves at once**; the original flags it and `BaseMine_Update` explodes it the
//!   next tick, which no test of the cannon can see.
//! - **The round pool is the original's thirty**, found by a free flag instead of a compact
//!   count; with a 1 s life and a 0.1 s volley at most twenty are ever alive.

use oag_core::math::Vec3;
use oag_tables::detonator_layout::{GamePlayValues, Scoring};

use super::{Event, EventKind, Field, Survey};
use crate::Craft;

/// Rounds the pool holds: fifteen volleys of two.
pub const POOL: usize = 30;

/// The longest clip the cannon holds: its hit slots, streak awards and HUD icons are one volley of
/// two rounds each over [`POOL`]. A table naming more (Pulse's Cannon, 30) is held to it
/// (`docs/gameplay/detonator-pulse.md`, "The cannon").
pub const CLIP_VOLLEYS: u8 = (POOL / 2) as u8;

/// Round speed, units/s, from the first update on; the launch tick adds the craft's speed.
const SPEED: f32 = 500.0;

/// How far down the surface normal a round probes for the track.
const PROBE: f32 = 9.0;

/// How far above the track it rides.
const RIDE: f32 = 3.5;

/// How far off a wall a rebounded round is put.
const REBOUND_OFFSET: f32 = 3.0;

/// A round's life, seconds (`CannonManager_Update`'s retire constant, `0x008c1834`).
const LIFE: f32 = 1.0;

/// The squared distance a destroyed bomb sets mines off within (`0x008c14d0`, the executable's
/// static; the file's `BombChainReactionDistance` is parsed and dropped).
const CHAIN_DISTANCE_SQUARED: f32 = 5000.0;

/// What a streak of five, ten and fifteen volleys with a hit pays.
pub const STREAK_FIVE: i32 = 1000;
/// Ten.
pub const STREAK_TEN: i32 = 3000;
/// Fifteen.
pub const STREAK_FIFTEEN: i32 = 5000;

/// What a round's segment met.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Sweep {
    /// Nothing.
    None,
    /// A floor the round rides on.
    Floor {
        /// Where the segment met it.
        point: Vec3,
        /// The surface's unit normal.
        normal: Vec3,
    },
    /// A wall.
    Wall {
        /// Where the segment met it.
        point: Vec3,
        /// The wall's unit normal.
        normal: Vec3,
    },
}

/// One round in flight.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Round {
    /// Whether the slot holds a round.
    pub live: bool,
    /// This tick's position.
    pub pos: Vec3,
    /// Last tick's: the hit test sweeps `prev` to `pos`.
    pub prev: Vec3,
    /// Velocity.
    pub vel: Vec3,
    /// The surface normal the track probe follows. It is also the round's up: the heading is kept
    /// perpendicular to it, so a round turns with the surface it rides
    /// (`docs/ghidra/functions/ps3-hdfury-eu/detonator-cannon.md`, "The round, polled live").
    pub normal: Vec3,
    /// The unit heading the round is drawn along: the velocity flattened against `normal` before this
    /// tick's wall test, so on a bounce tick it is still the incoming heading and the reflected one
    /// shows from the next tick.
    pub heading: Vec3,
    /// Seconds since launch.
    pub age: f32,
    /// Wall reflections left.
    pub rebounds: u8,
    /// Which [`Cannon::hits`] entry this round counts into.
    pub slot: u8,
}

impl Round {
    const NONE: Self = Self {
        live: false,
        pos: Vec3::ZERO,
        prev: Vec3::ZERO,
        vel: Vec3::ZERO,
        normal: Vec3::ZERO,
        heading: Vec3::ZERO,
        age: 0.0,
        rebounds: 0,
        slot: 0,
    };
}

/// Which streak awards the clip has paid (`clip+0xbc..+0xc0`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Awards {
    /// The first five-volley streak.
    pub first_five: bool,
    /// The second five-volley streak.
    pub second_five: bool,
    /// Ten.
    pub ten: bool,
    /// Fifteen.
    pub fifteen: bool,
    /// The volley the first five was paid at.
    pub first_five_at: u8,
}

/// The cannon's state, and the score it earns.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cannon {
    /// Whether the race has a cannon at all; a field that never had one stays empty.
    pub armed: bool,
    /// Volleys left in the clip.
    pub clip: u8,
    /// The volley countdown while the trigger is held (`s+0x168`).
    pub trigger: f32,
    /// Whether the cannon may fire (`s+0x175`): false inside a reload lockout.
    pub ready: bool,
    /// Seconds of lockout left (`s+0x178`).
    pub lockout: f32,
    /// The lockout the reload set (`s+0x198`).
    pub lockout_total: f32,
    /// A reload asked for the clip's streak to be cleared at the next ready tick.
    pub reset_pending: bool,
    /// The next volley's first [`Self::hits`] entry; stays even and is not reset by a reload.
    pub ring: u8,
    /// Per round fired: `1` once fired, more once it hit something.
    pub hits: [u8; POOL],
    /// The streak awards paid this clip.
    pub awards: Awards,
    /// The rounds.
    pub rounds: [Round; POOL],
    /// The score (`rm+0x2df0`).
    pub score: i32,
    /// The live-mine multiplier, reset at a reload (`g_DetonatorScoring+0x28`).
    pub m_mine: f32,
    /// The chain-reaction multiplier, reset at each bomb the cannon destroys (`+0x34`).
    pub m_chain: f32,
    /// Bombs the cannon has destroyed (`+0x38`); the next one pays `n * bomb_increment` more.
    pub bombs_shot: i32,
    /// Mines the cannon hit directly (`rm+0x6de3c`); chain-reaction mines are not counted.
    pub destroyed_mines: u32,
    /// Bombs the cannon destroyed (`rm+0x6de40`).
    pub destroyed_bombs: u32,
    /// Ticks the trigger was held with a loaded, ready cannon (`rm+0x6de44`).
    pub held_ticks: u32,
    /// Five-volley streak awards paid over the race (`rm+0x2de8`); a clip's second five counts
    /// again. [`Awards`] are per clip and reset at a reload, these are not.
    pub fives: u16,
    /// Ten-volley streak awards paid over the race (`rm+0x2dea`).
    pub tens: u16,
    /// Fifteen-volley streak awards paid over the race (`rm+0x2dec`).
    pub fifteens: u16,
}

impl Cannon {
    /// A cannon the race has not armed.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            armed: false,
            clip: 0,
            trigger: 0.0,
            ready: false,
            lockout: 0.0,
            lockout_total: 0.0,
            reset_pending: false,
            ring: 0,
            hits: [0; POOL],
            awards: Awards {
                first_five: false,
                second_five: false,
                ten: false,
                fifteen: false,
                first_five_at: 0,
            },
            rounds: [Round::NONE; POOL],
            score: 0,
            m_mine: 1.0,
            m_chain: 1.0,
            bombs_shot: 0,
            destroyed_mines: 0,
            destroyed_bombs: 0,
            held_ticks: 0,
            fives: 0,
            tens: 0,
            fifteens: 0,
        }
    }

    /// How many rounds are in flight.
    #[must_use]
    pub fn rounds_in_flight(&self) -> usize {
        self.rounds.iter().filter(|r| r.live).count()
    }
}

impl Default for Cannon {
    fn default() -> Self {
        Self::new()
    }
}

/// What the cannon reads of the title's tables.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Config {
    /// `rounds`: volleys in a clip, at most [`CLIP_VOLLEYS`].
    pub rounds: u8,
    /// Seconds between volleys (the table's `rate`, already the reciprocal).
    pub interval: f32,
    /// `num_rebounds`.
    pub rebounds: u8,
    /// The Bomb's `number_of_shots_to_destroy`.
    pub shots_to_destroy: u8,
    /// The Mine's `trigger_radius`: the capsule a round hits it with.
    pub mine_radius: f32,
    /// The Bomb's.
    pub bomb_radius: f32,
    /// `CannonLockoutTimeFromManualReload`.
    pub lockout_manual: f32,
    /// `CannonLockoutTimeRunOutBullets`.
    pub lockout_empty: f32,
    /// `DistanceOffsetBetweenMines`, to place a laid object.
    pub lane_width: f32,
    /// `DetonatorScoring.xml`.
    pub scoring: Scoring,
    /// The hull's `cannon_flash_left` and `cannon_flash_right` locators in its model space,
    /// `[left, right]`: where a volley's two rounds leave. A side the hull authors none for
    /// leaves from [`crate::projectile::cannon::launch`]'s muzzle instead.
    pub muzzles: [Option<Vec3>; 2],
}

impl Config {
    /// Reads a Detonator weapon table and the layout files, or `None` where the table authors
    /// no Cannon, Mine or Bomb.
    #[must_use]
    pub fn from_tables(
        stats: &oag_tables::weapons::WeaponStats,
        values: &GamePlayValues,
        scoring: Scoring,
        muzzles: [Option<Vec3>; 2],
    ) -> Option<Self> {
        let (cannon, mine, bomb) = (stats.cannon()?, stats.mine()?, stats.bomb()?);
        let detonator = stats.detonator()?;
        let count = |v: f32| v.max(0.0).round().min(255.0) as u8;
        Some(Self {
            rounds: count(cannon.rounds).min(CLIP_VOLLEYS),
            interval: cannon.rate,
            rebounds: count(detonator.cannon.num_rebounds?),
            shots_to_destroy: count(detonator.bomb.number_of_shots_to_destroy?),
            mine_radius: mine.trigger_radius,
            bomb_radius: bomb.trigger_radius,
            lockout_manual: values.cannon_lockout_time_from_manual_reload,
            lockout_empty: values.cannon_lockout_time_run_out_bullets,
            lane_width: values.distance_offset_between_mines,
            scoring,
            muzzles,
        })
    }
}

/// What the player's pad says this tick.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Input {
    /// The fire button is held.
    pub fire: bool,
    /// The reload (absorb) button went down.
    pub reload: bool,
}

/// `mine_count_stage` summed: what the percentage is a share of (`1860` on the disc).
#[must_use]
pub fn mines_in_the_race(values: &GamePlayValues) -> f32 {
    values.mine_count_stage.iter().map(|&n| n as f32).sum()
}

/// `destroyed / sum(MineCountStage1..14) * 100`, in `f32`, as `Detonator_UpdateRace` computes it.
#[must_use]
pub fn percentage(destroyed: u32, values: &GamePlayValues) -> f32 {
    let total = mines_in_the_race(values);
    if total > 0.0 {
        destroyed as f32 / total * 100.0
    } else {
        0.0
    }
}

fn refill(cannon: &mut Cannon, config: &Config, lockout: f32) {
    cannon.ready = false;
    cannon.lockout = lockout;
    cannon.lockout_total = lockout;
    cannon.clip = config.rounds;
    cannon.reset_pending = true;
}

/// One tick of the cannon: the trigger and reloads, the volley, every round, and the streaks.
///
/// `ship` is the craft that owns the cannon. `sweep(from, to)` answers what the segment meets in
/// the track. `survey` places the laid objects. Returns what the tick did that a view shows or
/// plays: a volley fired, a mine or bomb shot, a round dying on a wall.
pub fn step_cannon<S: Craft>(
    field: &mut Field,
    survey: &Survey,
    config: &Config,
    input: Input,
    ship: &S,
    dt: f32,
    sweep: &impl Fn(Vec3, Vec3) -> Sweep,
) -> Vec<Event> {
    let mut events = Vec::new();
    if !field.cannon.armed {
        field.cannon.armed = true;
        field.cannon.ready = true;
        field.cannon.clip = config.rounds;
    }

    let cannon = &mut field.cannon;
    let mut volley = false;
    if !input.fire {
        cannon.trigger = 0.0;
    } else if cannon.clip > 0 && cannon.ready {
        cannon.held_ticks = cannon.held_ticks.saturating_add(1);
        cannon.trigger -= dt;
        if cannon.trigger <= 0.0 {
            cannon.clip -= 1;
            cannon.trigger += config.interval;
            volley = true;
        }
    }
    if input.reload && cannon.ready {
        refill(cannon, config, config.lockout_manual);
    }
    if cannon.clip == 0 && !input.fire {
        refill(cannon, config, config.lockout_empty);
    }

    if cannon.lockout > 0.0 {
        cannon.ready = false;
        cannon.lockout -= dt;
    } else {
        cannon.lockout = 0.0;
        cannon.ready = true;
    }
    if cannon.reset_pending && cannon.ready {
        cannon.reset_pending = false;
        cannon.hits = [0; POOL];
        cannon.awards = Awards::default();
        cannon.m_mine = 1.0;
    }

    if volley && fire_volley(cannon, config, ship) {
        events.push(Event {
            kind: EventKind::Volley,
            at: ship.physics().body.position,
        });
    }

    for index in 0..POOL {
        if !field.cannon.rounds[index].live {
            continue;
        }
        let wall = update_round(&mut field.cannon.rounds[index], dt, sweep);
        if let Some((at, normal)) = wall {
            let round = &field.cannon.rounds[index];
            events.push(Event {
                kind: EventKind::RoundWall {
                    normal,
                    spawn: at - round.normal * RIDE,
                    rebound: (round.vel - normal * (2.0 * round.vel.dot(normal)))
                        .normalize_or_zero(),
                },
                at,
            });
        }
        let hit = test_hits(field, survey, config, index, &mut events);
        let round = &mut field.cannon.rounds[index];
        if wall.is_some() || hit || round.age > LIFE {
            round.live = false;
        }
    }

    award_streaks(&mut field.cannon);
    events
}

/// Launches a volley's two rounds. Returns whether the pool had room for both.
fn fire_volley<S: Craft>(cannon: &mut Cannon, config: &Config, ship: &S) -> bool {
    let free: Vec<usize> = (0..POOL)
        .filter(|&i| !cannon.rounds[i].live)
        .take(2)
        .collect();
    let [a, b] = free[..] else {
        return false;
    };
    let physics = ship.physics();
    let dimensions = ship.dimensions();
    let speed =
        SPEED + crate::projectile::KMH_PER_UNIT_PER_SECOND * physics.body.linear_velocity.length();
    let forward = physics.body.forward();
    let up = ship.launch_normal();
    let first = usize::from(cannon.ring) % POOL;
    for (index, (slot, left)) in [(a, true), (b, false)].into_iter().enumerate() {
        let entry = (first + index) % POOL;
        let muzzle = match config.muzzles[usize::from(!left)] {
            Some(local) => crate::projectile::plasma::hull_point(&physics.body, local),
            None => crate::projectile::cannon::launch(physics, &dimensions, left).0,
        };
        cannon.hits[entry] = cannon.hits[entry].saturating_add(1);
        cannon.rounds[slot] = Round {
            live: true,
            pos: muzzle,
            prev: muzzle,
            vel: forward * speed,
            normal: up,
            heading: forward,
            age: 0.0,
            rebounds: config.rebounds,
            slot: entry as u8,
        };
    }
    cannon.ring = ((first + 2) % POOL) as u8;
    true
}

/// `PulseBullet_Update`. Returns where and against which normal the round died on a wall.
fn update_round(
    round: &mut Round,
    dt: f32,
    sweep: &impl Fn(Vec3, Vec3) -> Sweep,
) -> Option<(Vec3, Vec3)> {
    round.age += dt;
    round.prev = round.pos;
    let mut next = round.prev + round.vel * dt;
    if let Sweep::Floor { point, normal } = sweep(next, next - round.normal * PROBE) {
        round.normal = normal;
        next = point + normal * RIDE;
    }
    round.pos = next;

    let heading = round.vel.normalize_or_zero();
    let flat = (heading - round.normal * heading.dot(round.normal)).normalize_or_zero();
    round.heading = flat;
    round.vel = flat * SPEED;

    if let Sweep::Wall { point, normal } = sweep(round.prev, round.pos) {
        if round.rebounds == 0 {
            return Some((point, normal));
        }
        round.rebounds -= 1;
        round.vel -= normal * (2.0 * round.vel.dot(normal));
        round.pos = point + normal * REBOUND_OFFSET;
    }
    None
}

/// Whether the capsule round the segment `prev..pos` holds `target`
/// (`CannonManager_TestMineBombHits`).
fn capsule_hits(round: &Round, target: Vec3, radius: f32) -> bool {
    let (a, b) = (round.pos, round.prev);
    let r2 = radius * radius;
    if (target - a).length_squared() <= r2 {
        return true;
    }
    let along = a - b;
    if along == Vec3::ZERO {
        return false;
    }
    let u = along.normalize();
    if u.dot(target - a) < 0.0 && u.dot(target - b) > 0.0 {
        let to = target - b;
        let across = to - u * u.dot(to);
        return across.length() < radius;
    }
    false
}

/// `CannonManager_TestMineBombHits` for one round: every untriggered mine, then every bomb;
/// neither loop stops at a hit. Returns whether the round hit anything.
fn test_hits(
    field: &mut Field,
    survey: &Survey,
    config: &Config,
    index: usize,
    events: &mut Vec<Event>,
) -> bool {
    let round = field.cannon.rounds[index];
    let mut hit = false;

    let mut i = 0;
    while i < usize::from(field.live_mines) {
        let placed = field.mines[i];
        let Some((at, _)) = survey.mine_point(&placed, config.lane_width) else {
            i += 1;
            continue;
        };
        if !capsule_hits(&round, at, config.mine_radius) {
            i += 1;
            continue;
        }
        hit = true;
        let cannon = &mut field.cannon;
        cannon.destroyed_mines = cannon.destroyed_mines.saturating_add(1);
        let slot = usize::from(round.slot);
        cannon.hits[slot] = cannon.hits[slot].saturating_add(1);
        let points = if placed.live {
            let points = (config.scoring.new_mine_score as f32 * cannon.m_mine) as i32;
            cannon.m_mine += config.scoring.multiplier;
            points
        } else {
            config.scoring.old_mine_score
        };
        cannon.score = cannon.score.saturating_add(points);
        events.push(Event {
            kind: EventKind::MineShot { live: placed.live },
            at,
        });
        field.remove_mine(i);
    }

    let mut i = 0;
    while i < usize::from(field.live_bombs) {
        let Some((at, _)) = survey.point(&field.bombs[i], config.lane_width) else {
            i += 1;
            continue;
        };
        if !capsule_hits(&round, at, config.bomb_radius) {
            i += 1;
            continue;
        }
        hit = true;
        let slot = usize::from(round.slot);
        if field.bomb_hits[i] < config.shots_to_destroy {
            field.bomb_hits[i] += 1;
            field.cannon.hits[slot] = field.cannon.hits[slot].saturating_add(1);
        }
        if field.bomb_hits[i] < config.shots_to_destroy {
            i += 1;
            continue;
        }
        let cannon = &mut field.cannon;
        cannon.destroyed_bombs = cannon.destroyed_bombs.saturating_add(1);
        cannon.hits[slot] = cannon.hits[slot].saturating_add(1);
        let points = config.scoring.bomb_score.saturating_add(
            cannon
                .bombs_shot
                .saturating_mul(config.scoring.bomb_increment),
        );
        cannon.score = cannon.score.saturating_add(points);
        cannon.bombs_shot = cannon.bombs_shot.saturating_add(1);
        cannon.m_chain = 1.0;
        events.push(Event {
            kind: EventKind::BombShot,
            at,
        });
        field.remove_bomb(i);
        chain_reaction(field, survey, config, at, events);
    }
    hit
}

/// The mines a destroyed bomb sets off: scored, not counted, no hit for the clip.
fn chain_reaction(
    field: &mut Field,
    survey: &Survey,
    config: &Config,
    bomb_at: Vec3,
    events: &mut Vec<Event>,
) {
    let mut i = 0;
    while i < usize::from(field.live_mines) {
        let placed = field.mines[i];
        let Some((at, _)) = survey.mine_point(&placed, config.lane_width) else {
            i += 1;
            continue;
        };
        if (at - bomb_at).length_squared() >= CHAIN_DISTANCE_SQUARED {
            i += 1;
            continue;
        }
        let cannon = &mut field.cannon;
        let points = if placed.live {
            let points = (config.scoring.new_mine_score as f32 * cannon.m_chain) as i32;
            cannon.m_chain += config.scoring.multiplier;
            points
        } else {
            config.scoring.old_mine_score
        };
        cannon.score = cannon.score.saturating_add(points);
        events.push(Event {
            kind: EventKind::MineChained { live: placed.live },
            at,
        });
        field.remove_mine(i);
    }
}

/// `PulseCannon_AwardStreakBonuses`: consecutive volleys of the clip with a hit in either round.
fn award_streaks(cannon: &mut Cannon) {
    let mut streak = 0;
    for volley in 0..POOL / 2 {
        if cannon.hits[2 * volley] <= 1 && cannon.hits[2 * volley + 1] <= 1 {
            streak = 0;
            continue;
        }
        streak += 1;
        let at = volley as u8;
        let awards = &mut cannon.awards;
        let pay = if streak >= 15 && !awards.fifteen {
            awards.fifteen = true;
            cannon.fifteens = cannon.fifteens.saturating_add(1);
            STREAK_FIFTEEN
        } else if streak >= 10 && !awards.ten {
            awards.ten = true;
            cannon.tens = cannon.tens.saturating_add(1);
            STREAK_TEN
        } else if (5..=9).contains(&streak) {
            if !awards.first_five {
                awards.first_five = true;
                awards.first_five_at = at;
                cannon.fives = cannon.fives.saturating_add(1);
                STREAK_FIVE
            } else if !awards.second_five && i32::from(at) - i32::from(awards.first_five_at) > 5 {
                awards.second_five = true;
                cannon.fives = cannon.fives.saturating_add(1);
                STREAK_FIVE
            } else {
                0
            }
        } else {
            0
        };
        cannon.score = cannon.score.saturating_add(pay);
    }
}

#[cfg(test)]
mod tests;
