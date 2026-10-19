//! The player's own original driver: the autopilot.
//!
//! The original hands the player's craft to an AI driver of its own,
//! registered as `"autopilot input"` by `Ai_Construct` and consumed by
//! `Ship_UpdateCraft` through `craft+0x40` while the autopilot blend is set
//! (`docs/ghidra/functions/psp-pulse-usa/ai-controls.md`, "Who drives whom").
//! It runs the opponents' `Ai_Update` with four differences, each a branch on
//! "this is the player's craft":
//!
//! - **Lane `0`.** `Ai_PlanLane` zeroes the chosen lane and the target after
//!   the contact push (`0x0885566c..0x08855674`): the racing line, the
//!   authored band aside ([`super::PlanInput::player`]).
//! - **Thrust `100`.** `Ai_ComputeControls` calls `AI_ComputeOpponentThrust`
//!   only for a driver that is not the player's or a finished racer
//!   (`lbu 0x12(racer+0x70)`, read at instruction level), so the output's
//!   default `100` stands: full throttle, no formation and no rubber band.
//!   The per-point speed floor is skipped for the player too (it never lowers
//!   thrust anyway). HD: `0x00100d7c..0x00100d9c`; Pure keeps the default
//!   `100` on its cruise path.
//! - **The rail through contact.** `Ai_SnapToLane` drops the rail on a
//!   contact for a craft that is not the player's; the player's keeps it.
//! - **Its own generator** (chosen, not measured): the player's draws cannot
//!   change its output (the choice they feed is zeroed), and taking them from
//!   the field's would reshuffle the opponents whenever an autopilot engaged.
//!
//! A finished player's driver runs the thrust law in the original; this
//! engine caps the finished craft with that law's figure instead
//! (`oag_raceplay`'s `finished_thrust`), so [`Autopilot::drive`] stays at `100`.
//!
//! **Warm on engage, chosen, not measured.** The original runs the player's
//! `Ai_Update` every tick whether or not its controls are consumed, so when an
//! autopilot takes the craft the driver's authority is already `1` (it ramps
//! toward `1` for a racing player), its smoothed speed is current, its ring
//! holds lane `0`, and its heading error is current. This engine drives it
//! only while it flies, and on the first tick of a stint puts it in that
//! state: authority `1`, smoothed speed the craft's speed, the planner
//! [`super::Planner::warm_on_line`], and one steering call whose derivative is
//! discarded. The rail's blend starts at `0` either way: the original's is
//! `0` while the controls are not consumed.
//!
//! What sets the original's blend `craft+0x1d4`, and so how the handover from
//! the pad is shaped, is not read; this engine hands over in one tick.

use super::field::{OriginalDriver, Output, Tick};
use super::lane::PlanInput;
use super::laws::Laws;
use super::steer::{self, Controller, SteerInput};
use super::track::TrackPoint;
use oag_core::Rng;
use oag_physics::ShipControls;

/// The throttle the player's driver always holds: the output record's default.
pub const AUTOPILOT_THRUST: f32 = 100.0;

/// One player's autopilot driver.
#[derive(Debug, Clone, PartialEq)]
pub struct Autopilot {
    controller: Controller,
    laws: Laws,
    /// Pure's ringless planner ([`super::SingleLane`]) rather than the ring.
    ringless: bool,
    rng: Rng,
    /// The driver's state.
    pub driver: OriginalDriver,
    /// The next call starts a stint and warms the driver first.
    cold: bool,
}

impl Autopilot {
    /// An autopilot on the class's controller row and the title's laws.
    /// `ringless` is Pure's planner. `seed` seeds its own generator.
    #[must_use]
    pub fn new(controller: Controller, laws: Laws, ringless: bool, seed: u64) -> Self {
        Self {
            controller,
            laws,
            ringless,
            rng: Rng::new(seed ^ 0x6175_746f_7069_6c6f),
            driver: OriginalDriver::new(8),
            cold: true,
        }
    }

    /// The craft went back to its pilot: the next [`Self::drive`] starts a
    /// new stint.
    pub fn release(&mut self) {
        self.cold = true;
    }

    /// One tick of the player's `Ai_Update`. `ahead` samples the track a
    /// distance further along the craft's route, as for an opponent.
    pub fn drive(
        &mut self,
        tick: &Tick,
        dt: f32,
        mut ahead: impl FnMut(f32) -> Option<TrackPoint>,
    ) -> Output {
        let pose = &tick.pose;
        let speed = pose.velocity.length();
        let controller = self.controller;
        let laws = self.laws;
        let cold = std::mem::replace(&mut self.cold, false);
        let driver = &mut self.driver;
        if cold {
            driver.authority = 1.0;
            driver.smoothed_speed = speed;
            driver.last_forward = pose.forward;
            if self.ringless {
                driver.single_lane = super::SingleLane::default();
            } else {
                driver.planner.warm_on_line();
                driver.planner.ring.advance(tick.own.progress);
            }
        }
        driver.smoothed_speed = driver.smoothed_speed * 0.95 + speed * 0.05;
        driver.authority = (driver.authority + 0.025).min(1.0);

        let (own_lane, lane_at) = if self.ringless {
            let lane = driver
                .single_lane
                .plan_player(pose.velocity, pose.forward, dt * 60.0);
            (lane, None)
        } else {
            driver.planner.ring.advance(tick.own.progress);
            driver.planner.plan(
                &PlanInput {
                    own_lane: tick.own.lane_of(pose.position),
                    lane_tick: false,
                    contact_sides: tick.contact_sides,
                    band: tick.band,
                    player: true,
                },
                &mut self.rng,
            );
            (
                driver.planner.ring.at(tick.own.progress),
                Some(driver.planner.ring),
            )
        };

        let air = steer::airborne(pose, &tick.own);
        let mut look = controller.look_ahead_secs + air;
        if laws.halves_lookahead(tick.lap_distance) {
            look *= 0.5;
        }
        let distance = steer::lookahead_distance(driver.smoothed_speed, look);
        let steering = ahead(distance).map(|point| {
            let input = SteerInput {
                own: &tick.own,
                own_lane,
                ahead: &point,
                ahead_lane: lane_at.map_or(own_lane, |ring| ring.at(point.progress)),
                air,
            };
            if cold {
                // Discarded: it only brings the heading error up to date.
                steer::steer(pose, &input, &controller, &mut driver.last_error);
            }
            steer::steer(pose, &input, &controller, &mut driver.last_error)
        });
        driver.held_thrust = AUTOPILOT_THRUST;
        let steering = steering.unwrap_or_default();
        let a = driver.authority;
        // The player's rail is not dropped on a contact.
        let rail = driver.rail.snap(
            pose,
            &tick.own,
            own_lane,
            false,
            tick.finished,
            laws.rail_turns_velocity,
            dt,
        );
        let fade = laws.pitch_scale((pose.position - tick.own.position).length());
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
