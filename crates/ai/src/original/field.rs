//! `AiManager_Update` (`0x08834b14`) and `Ai_Update` (`0x08853a80`): the field
//! of original drivers, one round-robin driver a tick, each driver's tick in
//! the original's order. `docs/ghidra/functions/psp-pulse-usa/ai-controls.md`.
//!
//! **Chosen, not measured**: the racer-table order is the grid's slot order,
//! the round-robin counter starts at slot `0` on the first tick the field is
//! driven (`1` under [`Laws::round_robin_pre_increment`], HD's order), and the field draws from one generator of its own, seeded from the
//! race seed, in a fixed order (the shuffle once, then per driver per tick the
//! preferred-lane draw before the wander's). The original's `rand()` stream is
//! not reproduced; its draws feed nothing a live sample can check.

use super::lane::{PlanInput, Planner};
use super::laws::Laws;
use super::pack::{Wander, shuffle};
use super::rail::{Rail, RailCorrection};
use super::steer::{self, Controller, SteerInput};
use super::thrust::{Racer, ThrustInput, opponent_thrust};
use super::track::{Pose, TrackPoint};
use oag_core::Rng;
use oag_core::math::Vec3;
use oag_physics::ShipControls;
use oag_tables::ai_race_stats::ClassRace;

/// The racer table's size, and the round-robin period: the counter wraps at
/// eight whatever the field size (measured, 92).
pub const SLOTS: usize = 8;

/// One driver's state: everything `Ai_Construct` allocates that the recovered
/// functions read.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OriginalDriver {
    /// `+0x64`: steering authority, ramped in at `0.025` a tick.
    pub authority: f32,
    /// `+0x170`: smoothed speed. Starts at `0` (chosen; `Ai_Construct` not read for it).
    pub smoothed_speed: f32,
    /// `+0x174[0]`: last heading error.
    pub last_error: f32,
    /// The lane planner and its ring.
    pub planner: Planner,
    /// The rail.
    pub rail: Rail,
    /// The formation slot and its wander.
    pub wander: Wander,
    /// `+0x74`: the formation slot as the thrust law last left it.
    pub spread: f32,
    /// The thrust record (`+0x0c`), held between round-robin ticks.
    pub held_thrust: f32,
    /// `+0x190`: the forward axis the pitch controller last saw.
    pub last_forward: Vec3,
}

impl OriginalDriver {
    /// A driver for grid position `position`, `1` the front.
    #[must_use]
    pub fn new(position: u8) -> Self {
        let wander = Wander::constructed(position);
        Self {
            authority: 0.0,
            smoothed_speed: 0.0,
            last_error: 0.0,
            planner: Planner::default(),
            rail: Rail::default(),
            spread: wander.start,
            wander,
            held_thrust: 0.0,
            last_forward: Vec3::ZERO,
        }
    }
}

/// What a craft's tick hands its driver.
#[derive(Debug, Clone, Copy)]
pub struct Tick {
    /// The craft.
    pub pose: Pose,
    /// Its own projected sample.
    pub own: TrackPoint,
    /// The current point's `unk_0x5c`/`unk_0x5d`.
    pub band: Option<(u8, u8)>,
    /// The current point's `unk_0x66`.
    pub speed_floor: Option<u8>,
    /// The craft's ground fraction.
    pub grounded: f32,
    /// Signed count of contacts by side, `+` on the craft's left.
    pub contact_sides: i32,
    /// The body has any contact at all.
    pub touching: bool,
    /// This craft has finished (`racer+0x82`, ship state `2`).
    pub finished: bool,
    /// [`super::race_fraction`] for this craft.
    pub race_fraction: f32,
    /// How far into its current lap the craft is, in lap-length units
    /// (HD's `racer+0x90`, measured): what [`Laws::halved_lookahead`] reads.
    pub lap_distance: f32,
}

/// What the race reads of the whole field this tick.
#[derive(Debug, Clone, Copy)]
pub struct Standings<'a> {
    /// Every racer's progress (lap-length units) and place, in slot order;
    /// the spread is the field's own.
    pub racers: &'a [(f32, u8)],
    /// The player's slot.
    pub player: usize,
    /// The player's craft is in ship state `4..=6`.
    pub player_rubber_band_off: bool,
    /// Race time in seconds.
    pub race_time: f32,
    /// The race is an Eliminator, which [`Laws::eliminator_drops_when_behind`] reads.
    pub eliminator: bool,
}

/// One driver's output.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Output {
    /// The control record, divided down to [`ShipControls`]' scale. Thrust may
    /// exceed `1.0`: above `100` the original's throttle lifts the engine's cap.
    pub controls: ShipControls,
    /// The rail's body write, when it fires.
    pub rail: Option<RailCorrection>,
}

/// The field: `AiManager_Update`'s state and every driver.
#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    table: ClassRace,
    controller: Controller,
    laws: Laws,
    skill: f32,
    level: usize,
    rng: Rng,
    counter: usize,
    shuffled: bool,
    /// The drivers, by slot. The player's slot holds one too, never driven:
    /// its `spread` is what the thrust law reads for the player's racer.
    pub drivers: [OriginalDriver; SLOTS],
}

impl Field {
    /// A field for one race. `skill` is `AI_ResolveSkillScale`'s value,
    /// `level` the integer difficulty the shuffle reads (`0..=2`), `grid`
    /// each slot's grid position, `1` the front (the live capture's racer
    /// table puts the player last, at `8`, with a formation slot of `0`).
    /// `laws` is the title's variant of the driver, [`Laws::PULSE`] for Pulse.
    #[must_use]
    pub fn new(
        table: ClassRace,
        controller: Controller,
        laws: Laws,
        skill: f32,
        level: usize,
        grid: [u8; SLOTS],
        seed: u64,
    ) -> Self {
        let mut rng = Rng::new(seed ^ 0x6f72_6967_696e_616c);
        let drivers = std::array::from_fn(|slot| {
            let mut driver = OriginalDriver::new(grid[slot]);
            // 2048's `Ai_Construct` draws the coin once (`rand() % 2`,
            // `driver+0xc8`); the order the slots draw in is chosen.
            if laws.coin_ordered_search {
                driver.planner.coin = Some(rng.below(2) == 1);
            }
            driver.planner.preferred_follows_choice = laws.preferred_follows_choice;
            driver
        });
        Self {
            table,
            controller,
            laws,
            skill,
            level: level.min(2),
            rng,
            counter: usize::from(laws.round_robin_pre_increment),
            shuffled: false,
            drivers,
        }
    }

    /// A 2048 event's two AI skills (`m_aiPadTrackingSkill`,
    /// `m_aiWeaponAvoidanceSkill`): under [`Laws::event_ring_rate`] they set
    /// every driver's ring rate ([`super::event_ring_rate`]); otherwise this
    /// does nothing. What else they drive, the pad tracking and the weapon
    /// avoidance, is not carried.
    pub fn set_event_ai_skills(&mut self, pad_tracking: f32, weapon_avoidance: f32) {
        if !self.laws.event_ring_rate {
            return;
        }
        let rate = super::event_ring_rate(pad_tracking, weapon_avoidance);
        for driver in &mut self.drivers {
            driver.planner.rate = rate;
        }
    }

    /// The title's variant of the driver this field flies.
    #[must_use]
    pub fn laws(&self) -> &Laws {
        &self.laws
    }

    /// `AiManager_Update`'s head, once a tick before any driver: the shuffle
    /// on the first call, then which slot runs its law this tick.
    pub fn begin_tick(&mut self, opponents: &[usize]) -> usize {
        if !self.shuffled {
            self.shuffled = true;
            let mut wanders: Vec<Wander> =
                opponents.iter().map(|&s| self.drivers[s].wander).collect();
            shuffle(
                &mut wanders,
                &self.table.pack_swapping[self.level],
                &mut self.rng,
            );
            for (&slot, wander) in opponents.iter().zip(wanders) {
                self.drivers[slot].wander = wander;
            }
        }
        let lane_slot = self.counter;
        self.counter = (self.counter + 1) % SLOTS;
        lane_slot
    }

    /// `Ai_Update` for the opponent in `slot`. `ahead` samples the track a
    /// distance further along the craft's own route and sets that sample's
    /// progress.
    pub fn drive(
        &mut self,
        slot: usize,
        lane_tick: bool,
        tick: &Tick,
        standings: &Standings<'_>,
        dt: f32,
        mut ahead: impl FnMut(f32) -> Option<TrackPoint>,
    ) -> Output {
        // 2048 flags every racer every tick (`Laws::every_tick`).
        let lane_tick = lane_tick || self.laws.every_tick;
        let controller = self.controller;
        let pose = &tick.pose;
        let speed = pose.velocity.length();
        let spreads: [f32; SLOTS] = std::array::from_fn(|k| self.drivers[k].spread);
        let driver = &mut self.drivers[slot];
        driver.planner.ring.advance(tick.own.progress);
        driver.smoothed_speed = driver.smoothed_speed * 0.95 + speed * 0.05;

        if driver.authority < 1.0 {
            driver.authority += 0.025;
        }
        if 1.0 < driver.authority {
            driver.authority = 1.0;
        }
        driver.planner.plan(
            &PlanInput {
                own_lane: tick.own.lane_of(pose.position),
                lane_tick,
                contact_sides: tick.contact_sides,
                band: tick.band,
            },
            &mut self.rng,
        );

        let own_lane = driver.planner.ring.at(tick.own.progress);
        let air = steer::airborne(pose, &tick.own);
        let mut look = controller.look_ahead_secs + air;
        if self.laws.halves_lookahead(tick.lap_distance) {
            look *= 0.5;
        }
        let distance = steer::lookahead_distance(driver.smoothed_speed, look);
        let steering = ahead(distance).map(|point| {
            steer::steer(
                pose,
                &SteerInput {
                    own: &tick.own,
                    own_lane,
                    ahead: &point,
                    ahead_lane: driver.planner.ring.at(point.progress),
                    air,
                },
                &controller,
                &mut driver.last_error,
            )
        });
        let mut thrust = 100.0;
        if lane_tick && steering.is_some() {
            driver.spread = if self.laws.finished_wander_to_place && tick.finished {
                let place = standings.racers.get(slot).map_or(SLOTS as u8, |r| r.1);
                driver.wander.finish(place)
            } else {
                driver.wander.update(tick.race_fraction, &mut self.rng)
            };
            let count = standings.racers.len().min(SLOTS);
            let racers: [Racer; SLOTS] = std::array::from_fn(|k| {
                let (progress, place) = standings.racers.get(k).copied().unwrap_or((0.0, 0));
                Racer {
                    progress,
                    place,
                    spread: if k == slot { driver.spread } else { spreads[k] },
                }
            });
            thrust = opponent_thrust(
                &self.table,
                &ThrustInput {
                    racers: &racers[..count],
                    own: slot,
                    player: standings.player,
                    finished: tick.finished,
                    finished_spread: self.laws.finished_spread,
                    drop_when_behind: standings.eliminator
                        && self.laws.eliminator_drops_when_behind,
                    rubber_band_off: standings.player_rubber_band_off,
                    race_time: standings.race_time,
                    skill: self.skill,
                },
            )
            .throttle;
        }
        let driver = &mut self.drivers[slot];
        let steering = steering.unwrap_or_default();
        let a = driver.authority;
        if lane_tick {
            driver.held_thrust = thrust;
            if let Some(x) = tick.speed_floor.filter(|&x| x < 35) {
                let x = f32::from(x);
                if speed < -0.002_715 * x * x - 0.869_027 * x + 95.115_88 {
                    driver.held_thrust = 100.0;
                }
            }
        }
        let rail = driver
            .rail
            .snap(pose, &tick.own, own_lane, tick.touching, tick.finished, dt);
        let pitch = steer::pitch(pose, &tick.own, tick.grounded, &mut driver.last_forward);
        Output {
            controls: ShipControls {
                steer_x: steering.steer * a / 100.0,
                steer_y: pitch / 100.0,
                thrust: driver.held_thrust / 100.0,
                airbrake_left: steering.left * a / 100.0,
                airbrake_right: steering.right * a / 100.0,
                ..ShipControls::default()
            },
            rail,
        }
    }
}

#[cfg(test)]
mod tests;
