//! Wipeout Pure's opponent: the earlier sibling Pulse's driver grew out of
//! (`docs/ghidra/functions/psp-pure-usa/ai-controls.md`).
//!
//! Pulse's steering, lookahead, lane bumps and rail gates, around four things
//! of its own: the cruise throttle ([`super::cruise_thrust`]), **one** planned
//! lane moved by a bang-bang instead of a ring ([`SingleLane`]), a rail that
//! moves the position alone ([`super::Laws::rail_turns_velocity`]), and a
//! pitch that fades with distance from the track ([`super::Laws::pitch_fade`]).
//! Every opponent runs every part every frame (measured: the snapshot flags
//! every racer).
//!
//! **Two of the planner's scorers are absent because the original's are dead**:
//! `Ai_ScoreOtherCraft` (`0x08918570`) hands `Ai_AddLaneBump` a weight of
//! `-2 * w`, `w >= 0`, and `Ai_AddLaneBump` returns at once for a weight of
//! `0.001` or less (`0x08918ee4`); `FUN_08918d30`'s track objects are the scene
//! nodes tagged `Pad_Importer`'s base type (`0x22bdd8`), which both concrete pad
//! importers overwrite, and the list read `0` entries live on Vineta K. 209 of
//! 209 live planner calls held only the two fixed bumps.
//!
//! **Chosen, not measured**: the `driver+0x80` flag (`x 0.7` thrust and a lane
//! wander, set by a craft state at `craft+0xd0 -> +0x134` this port has not
//! read) is never set; `driver+0x1d8`, the speed scale, starts at `0`
//! (`Ai_Construct` not read for it).

use super::field::{Field, Output, Standings, Throttle, Tick};
use super::lane::{add_bump, local_best, rand_unit};
use super::steer::{self, SteerInput};
use super::track::TrackPoint;
use super::{CruiseDraws, cruise_thrust, finish_thrust, gap_along_track, zone_multiplier};
use oag_core::Rng;
use oag_core::math::Vec3;
use oag_physics::ShipControls;
use oag_tables::pure_ai_stats::PureClassRace;

/// `Ai_PlanLane`'s state on a Pure driver (`driver+0x1c4..+0x228`).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SingleLane {
    /// The first call has located the craft (`driver+0x3c` cleared).
    located: bool,
    /// `+0x1c4`: the planned lane, which the controls and the rail steer to
    /// (`driver+0x44`).
    pub lane: f32,
    /// `+0x1c8`: the chosen candidate.
    pub target: f32,
    /// `+0x1cc`: the lane's velocity, lane units a frame.
    pub velocity: f32,
    /// `+0x1d8`: the smoothed speed scale on the bang-bang's step.
    pub speed_scale: f32,
    /// `+0x1dc`: the preferred lane.
    preferred: f32,
    /// `+0x204`: the nine smoothed candidate scores.
    scores: [f32; 9],
}

/// The bang-bang's acceleration, lane units a frame squared (`0x3a51b717`).
const ACCEL: f32 = 0.0008;
/// Its position term, `ACCEL / 2` (`0x39d1b717`).
const HALF: f32 = 0.0004;

impl SingleLane {
    /// One call of `Ai_PlanLane` (`0x08919b6c`) for an opponent: `own_lane`
    /// the craft's lane coordinate on its own sample, `velocity` and
    /// `forward` the craft's, `dt60` the frame's `dt * 60`. Returns the lane
    /// the controls and the rail read. Reproduced live on 209 of 209 calls
    /// (`scripts/psp-pure-ai-lane.py`).
    pub fn plan(
        &mut self,
        own_lane: f32,
        velocity: Vec3,
        forward: Vec3,
        dt60: f32,
        rng: &mut Rng,
    ) -> f32 {
        if !self.located {
            self.located = true;
            self.lane = own_lane.clamp(-3.0, 3.0);
            self.target = self.lane.clamp(-1.0, 1.0);
            self.velocity = 0.0;
            self.preferred = 0.0;
        }
        let mut fresh = [0.0_f32; 9];
        add_bump(&mut fresh, 0.0, 1.0, 0.05);
        add_bump(&mut fresh, self.preferred, 0.15, 0.5);
        for (smoothed, new) in self.scores.iter_mut().zip(fresh) {
            *smoothed = *smoothed * 0.9 + new * 0.1;
        }
        // The global pass the original runs first is overwritten by this one.
        self.target = local_best(&self.scores, own_lane) as f32 * 0.25 - 1.0;
        if rng.below(400) == 125 {
            let u = rand_unit(rng) * 2.0 - 1.0;
            self.preferred = if u < 0.0 { u * u - 1.0 } else { 1.0 - u * u };
        }
        let e = self.target - self.lane;
        let v = self.velocity;
        let (a, b) = if 0.0 < e {
            if v < 0.0 {
                (ACCEL, HALF)
            } else {
                let k = v / ACCEL + 1.0;
                if e < v * k + k * HALF * k {
                    (-ACCEL, -HALF)
                } else {
                    (ACCEL, HALF)
                }
            }
        } else if 0.0 < v {
            (-ACCEL, -HALF)
        } else {
            let k = v / -ACCEL + 1.0;
            if v * k + k * -HALF * k < e {
                (ACCEL, HALF)
            } else {
                (-ACCEL, -HALF)
            }
        };
        let along = (velocity.x * forward.x + velocity.z * forward.z).abs();
        let raw = ((along - 10.0) * 0.02).clamp(0.0, 1.0);
        self.speed_scale = self.speed_scale * 0.9 + raw * 0.1;
        let k = dt60 * self.speed_scale;
        self.velocity = v + a * k;
        self.lane = v * k + b * k * k + self.lane;
        self.lane
    }
}

impl Field {
    /// A Pure field: the class's legacy block, the track's speed-tweak range
    /// (`<<Class>Tweak Min Max>` in its `stats.xml`), and the two per-race
    /// draws `AiStats_LoadAll` makes, taken from the field's own generator
    /// (chosen, not measured: the original's `rand()` stream is not
    /// reproduced). `grid` as [`Field::new`]'s.
    #[must_use]
    pub fn cruising(
        table: PureClassRace,
        tweak: (f32, f32),
        controller: super::Controller,
        laws: super::Laws,
        grid: [u8; super::SLOTS],
        seed: u64,
    ) -> Self {
        Self::assemble(
            |rng| Throttle::Cruise {
                draws: CruiseDraws::draw(&table, tweak, rng),
                table,
                zone: 1.0,
                launch_phase: true,
            },
            controller,
            laws,
            0.0,
            0,
            grid,
            seed,
        )
    }

    /// The race manager's two field-wide writes, once a tick after the
    /// opponents: `RaceManager_UpdateZoneSpeed` (`0x08810b30`, the zone
    /// multiplier from the nearest opponent's gap to the player) and
    /// `RaceManager_UpdateLaunchPhase` (`0x08810ddc`, the launch flag cleared
    /// once the field's spread passes `GridSpread * tweak`). The next tick's
    /// drivers read them, as the original's read the previous frame's
    /// (measured). `places` and `racers` as [`Standings`]'; `out` marks an
    /// eliminated racer. Nothing on a Pulse-shaped field.
    pub fn end_tick(&mut self, standings: &Standings<'_>) {
        let Throttle::Cruise {
            table,
            draws,
            zone,
            launch_phase,
        } = &mut self.throttle
        else {
            return;
        };
        let player = standings.player;
        let Some(&(player_distance, player_place)) = standings.racers.get(player) else {
            return;
        };
        let others = standings
            .racers
            .iter()
            .enumerate()
            .filter(|&(k, _)| k != player)
            .map(|(_, r)| r.0);
        *zone = zone_multiplier(
            table,
            player_distance,
            player_place == 1,
            others,
            standings.lap_length,
        );
        if *launch_phase {
            let (lo, hi) = standings
                .racers
                .iter()
                .fold((f32::INFINITY, f32::NEG_INFINITY), |(lo, hi), r| {
                    (lo.min(r.0), hi.max(r.0))
                });
            if table.grid_spread * draws.tweak < hi - lo {
                *launch_phase = false;
            }
        }
    }

    /// Pure's `Ai_Update` (`0x08919394`) for the opponent in `slot`, every
    /// frame: the smoothed speed, the planner, the launch and fatigue, the
    /// steering and the cruise thrust, the speed floors, the rail, the pitch.
    pub(super) fn drive_sibling(
        &mut self,
        slot: usize,
        tick: &Tick,
        standings: &Standings<'_>,
        dt: f32,
        mut ahead: impl FnMut(f32) -> Option<TrackPoint>,
    ) -> Output {
        let Throttle::Cruise {
            table,
            draws,
            zone,
            launch_phase,
        } = self.throttle
        else {
            unreachable!("drive_sibling runs on a cruising field");
        };
        let controller = self.controller;
        let pose = &tick.pose;
        let speed = pose.velocity.length();
        let driver = &mut self.drivers[slot];
        driver.smoothed_speed = driver.smoothed_speed * 0.95 + speed * 0.05;
        if driver.authority < 1.0 {
            driver.authority += 0.025;
        }
        if 1.0 < driver.authority {
            driver.authority = 1.0;
        }
        let lane = driver.single_lane.plan(
            tick.own.lane_of(pose.position),
            pose.velocity,
            pose.forward,
            dt * 60.0,
            &mut self.rng,
        );
        let player = standings.racers.get(standings.player).map_or(0.0, |r| r.0);
        let own = standings.racers.get(slot).map_or(player, |r| r.0);
        let zone_fatigue = driver.cruise.update(
            &table.fatigue,
            launch_phase,
            gap_along_track(player, own, standings.lap_length),
            standings.race_time,
            zone,
        );
        let air = steer::airborne(pose, &tick.own);
        let distance =
            steer::lookahead_distance(driver.smoothed_speed, controller.look_ahead_secs + air);
        let steering = ahead(distance).map(|point| {
            steer::steer(
                pose,
                &SteerInput {
                    own: &tick.own,
                    own_lane: lane,
                    ahead: &point,
                    ahead_lane: lane,
                    air,
                },
                &controller,
                &mut driver.last_error,
            )
        });
        // The output record keeps its default `100` when the lookahead fails.
        let thrust = if steering.is_some() {
            cruise_thrust(
                &table,
                &draws,
                driver.grid,
                zone_fatigue,
                driver.cruise.launch,
            )
        } else {
            100.0
        };
        let below_floor = tick.speed_floor.filter(|&x| x < 35).is_some_and(|x| {
            let x = f32::from(x);
            speed < -0.002_715 * x * x - 0.869_027 * x + 95.115_88
        });
        driver.held_thrust = finish_thrust(thrust, false, below_floor);
        let steering = steering.unwrap_or_default();
        let a = driver.authority;
        let rail = driver.rail.snap(
            pose,
            &tick.own,
            lane,
            tick.touching,
            tick.finished,
            self.laws.rail_turns_velocity,
            dt,
        );
        let fade = self
            .laws
            .pitch_scale((pose.position - tick.own.position).length());
        let pitch = steer::pitch(
            pose,
            &tick.own,
            tick.grounded,
            fade,
            &mut driver.last_forward,
        );
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
