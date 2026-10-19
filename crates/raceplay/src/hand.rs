//! A stand-in for a person's hand on an analog pad, for measuring what continuous
//! input costs a predicting network client (ADR-0069, condition 3).
//!
//! **Chosen, not measured.** No recording of an analog stick flying a race exists; the
//! only human input recorded is three-valued keyboard. This turns the driver's wants
//! (what the player's own autopilot would ask, [`Race::autopilot_wants`]) into what a
//! thumb would send, so the stream is continuous like a hand's but drives the circuit
//! as well as the driver does. It is a verification aid: nothing in a race uses it.
//!
//! The model, each part argued rather than fitted:
//!
//! - **No pure reaction delay, and that is measured.** A person steers toward a bend
//!   they have already seen, so their reaction time is absorbed by looking ahead; the
//!   driver is the look-ahead. Delaying its wants inside the loop was tried (8, 6, 4, 3
//!   and 2 ticks, 2 to 5 Hz sticks): this driver, which has no delay in its design,
//!   then limit-cycles against the stops, 18 to 50 percent of ticks in the top tenth of
//!   the stick's range against 3 percent undelayed, which is a worse driver and not a
//!   more human one.
//! - **A second-order stick** ([`STICK_HZ`], [`STICK_DAMPING`]): a thumb has mass and
//!   a spring, so the stick lags its target and, at a damping of 0.6, overshoots it by
//!   about 10 percent before settling. This halves the tick-to-tick change of the
//!   stick (mean 0.034 against the driver's 0.08).
//! - **Tremor** ([`TREMOR`]): a seeded 10 Hz sine plus white noise of a few hundredths
//!   of the range, the physiological tremor and the pad's sensor noise.
//! - **A deadzone** ([`DEADZONE`], 0.1 with the rest rescaled, the pad layer's figure)
//!   and **8-bit axes** (the stick reports 1/127 steps).
//! - **Airbrakes are analog triggers**, a first-order lag ([`TRIGGER_TICKS`]) behind
//!   the delayed want, as a trigger's travel is; thrust is the Cross button, held
//!   through the countdown the way a player holds it.
//!
//! Deterministic: `f32` only, no `mul_add`, noise from [`oag_core::Rng`] seeded by the
//! caller. Both the network test and the prediction sweep build their stream from it,
//! so there is one definition.

use oag_core::Rng;
use oag_gameplay::InputSnapshot;
use oag_gameplay::input::{Button, Input};
use oag_physics::ShipControls;

/// The stick's natural frequency in hertz. **Chosen.**
pub const STICK_HZ: f32 = 4.0;

/// The stick's damping ratio. **Chosen**: 0.6 overshoots by about 10 percent.
pub const STICK_DAMPING: f32 = 0.6;

/// Tremor amplitude, as a fraction of full deflection (sine and noise each). **Chosen.**
pub const TREMOR: f32 = 0.012;

/// Radial deadzone, rescaled linearly outside it. **Chosen**, the pad layer's figure.
pub const DEADZONE: f32 = 0.1;

/// Ticks for a trigger to close 63 percent of the way to its target. **Chosen.**
pub const TRIGGER_TICKS: f32 = 3.0;

const DT: f32 = 1.0 / 60.0;

/// One sprung axis.
#[derive(Debug, Clone, Copy, Default)]
struct Axis {
    position: f32,
    velocity: f32,
}

impl Axis {
    /// One tick toward `target`; stops at the stick's limits.
    fn step(&mut self, target: f32) {
        let wn = std::f32::consts::TAU * STICK_HZ;
        let acceleration =
            wn * wn * (target - self.position) - 2.0 * STICK_DAMPING * wn * self.velocity;
        self.velocity += acceleration * DT;
        self.position += self.velocity * DT;
        if self.position.abs() > 1.0 {
            self.position = self.position.signum();
            self.velocity = 0.0;
        }
    }
}

/// One hand: feed it the driver's wants every tick.
#[derive(Debug, Clone)]
pub struct Hand {
    x: Axis,
    y: Axis,
    left: f32,
    right: f32,
    tremor_phase: f32,
    noise: Rng,
    thrust: Input,
}

impl Hand {
    /// A hand at rest. `seed` makes its tremor its own.
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self {
            x: Axis::default(),
            y: Axis::default(),
            left: 0.0,
            right: 0.0,
            tremor_phase: (seed % 628) as f32 * 0.01,
            noise: Rng::new(seed),
            thrust: Input::new(),
        }
    }

    /// The pad's report for this tick, from the driver's `wants`. `gated` is whether the
    /// countdown still holds thrust back (Cross is held through it).
    pub fn apply(&mut self, wants: ShipControls, gated: bool) -> InputSnapshot {
        let target = wants;
        self.x.step(target.steer_x);
        self.y.step(-target.steer_y);
        let close = 1.0 - (-1.0 / TRIGGER_TICKS).exp();
        self.left += (target.airbrake_left - self.left) * close;
        self.right += (target.airbrake_right - self.right) * close;
        self.tremor_phase += std::f32::consts::TAU * 10.0 * DT;
        let shake = |hand: &mut Self, offset: f32| {
            let sine = (hand.tremor_phase + offset).sin() * TREMOR;
            sine + (hand.noise.next_f32() * 2.0 - 1.0) * TREMOR
        };
        let sx = shake(self, 0.0);
        let sy = shake(self, 1.7);
        let thrust = gated || target.thrust > 0.5;
        self.thrust
            .begin_frame(if thrust { Button::Cross.bit() } else { 0 });
        InputSnapshot {
            buttons: self.thrust,
            stick_x: axis_out(self.x.position + sx),
            stick_y: axis_out(self.y.position + sy),
            airbrake_left: axis_out(self.left).max(0.0),
            airbrake_right: axis_out(self.right).max(0.0),
        }
        .sanitised()
    }
}

/// Deadzone, rescale and 8-bit quantisation of one axis.
fn axis_out(raw: f32) -> f32 {
    let raw = raw.clamp(-1.0, 1.0);
    let scaled = if raw.abs() <= DEADZONE {
        0.0
    } else {
        raw.signum() * (raw.abs() - DEADZONE) / (1.0 - DEADZONE)
    };
    (scaled * 127.0).round() / 127.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wants(steer_x: f32) -> ShipControls {
        ShipControls {
            steer_x,
            thrust: 1.0,
            ..ShipControls::default()
        }
    }

    #[test]
    fn a_step_in_the_wants_lags_overshoots_and_settles() {
        let mut hand = Hand::new(1);
        let outs: Vec<f32> = (0..90)
            .map(|_| hand.apply(wants(0.6), false).stick_x)
            .collect();
        assert!(outs[0] < 0.2, "the stick does not jump: {}", outs[0]);
        let peak = outs.iter().copied().fold(0.0, f32::max);
        assert!(peak > 0.56 && peak < 0.7, "overshoot but no runaway: {peak}");
        assert!((outs[89] - 0.56).abs() < 0.06, "settles on the rescaled target");
    }

    #[test]
    fn it_is_continuous_where_a_driver_holds_and_deterministic_for_a_seed() {
        let run = |seed| {
            let mut hand = Hand::new(seed);
            (0..300)
                .map(|t| hand.apply(wants(if t % 120 < 60 { 0.4 } else { -0.4 }), false))
                .map(|s| s.stick_x.to_bits())
                .collect::<Vec<_>>()
        };
        assert_eq!(run(7), run(7));
        assert_ne!(run(7), run(8));
        let mut distinct = run(7);
        distinct.sort_unstable();
        distinct.dedup();
        assert!(distinct.len() > 20, "a hand is not three-valued: {}", distinct.len());
    }

    #[test]
    fn cross_is_held_through_the_countdown() {
        let mut hand = Hand::new(1);
        let out = hand.apply(ShipControls::default(), true);
        assert_ne!(out.buttons.masks()[0] & Button::Cross.bit(), 0);
    }
}
