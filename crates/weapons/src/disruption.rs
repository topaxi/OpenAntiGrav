//! What a Disruptor hit does to the craft it lands on: Pure's one weapon that
//! hurts nobody.
//!
//! A bolt from [`crate::projectile::disruptor`] hands a craft one of eight control
//! effects for an authored number of seconds; this is that state and its law.
//! Evidence, with every address below, on `docs/ghidra/functions/psp-pure-usa/weapons.md`,
//! "Hit: one flag, one timer, one switch on the craft, another on the ship".
//!
//! # Five of eight are built, three land and do nothing
//!
//! `Ship_UpdateWeapons` (`0x0892935c`) spends the eight kinds as bits on the
//! body's flag word. Five are read by code that only rewrites the inputs the force
//! law sees (no thrust, no airbrakes, a negated or blended yaw, an autopilot with
//! a literal thrust scale): [`Disruption::filter`] and
//! [`Disruption::autopilot_thrust_scale`], applied by the composition root to the
//! `ShipControls` before `oag_physics::step`, as `Ship::autopilot_timer` decides
//! which controls a craft gets.
//!
//! The other three are **not approximated**:
//!
//! - **Drunk** adds a random yaw `((rand() & 0xff) - 128) * 0.01 * amount` inside
//!   `Ship_ApplySteeringTorque` (`0x0892edfc`). The scale of the input it is added
//!   to is unread (`-1..1` or wider decides wobble versus spin), and guessing
//!   would put an invented number on an authored one.
//! - **Rubber Ship** scales a hover damping term by `0.2` in `FUN_0892dea8`, the
//!   suspension law in `oag-physics`.
//! - **Drunk Camera** touches the simulation not at all.
//!
//! A craft hit by one is still disrupted (kind and timer set, hash moved, a second
//! bolt bounces off) and drives normally: the original's state, none of its force.

use oag_physics::ShipControls;
use oag_tables::weapons::{DisruptorEffectKind, DisruptorStats};

use crate::Craft;

/// One craft's current disruption, or none.
///
/// The original's `craft+0x134` (flag), `+0x138` (kind) and `+0x13c` (seconds
/// left) as one `Option` and one `f32`; `None` means the flag is clear and the
/// timer is held at zero. Hashed by `oag_gameplay::hash`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Disruption {
    /// Which effect is running, or `None`.
    pub kind: Option<DisruptorEffectKind>,
    /// Seconds left. `Ship_UpdateWeapons` subtracts `dt` after applying the tick's
    /// flags and tears down at or below zero, so an effect of `t` seconds applies
    /// for `ceil(t / dt)` ticks; [`Self::advance`] is that order.
    pub timer: f32,
}

/// The thrust scale `Autopilot Slow` drives at.
///
/// **Recovered, confidence 82, a code literal** and not the authored
/// `speed_percent`: `Ship_UpdateWeapons`'s kind-3 arm writes `0.7` to `body+0x258`
/// and `Ship_UpdateEngine` (`0x0892f1f0`) multiplies thrust by it while bit `4` is
/// set. `speed_percent` is copied to `craft+0x144` and read by nothing found; see
/// `oag_tables::weapons::DisruptorEffect::speed_percent`.
pub const AUTOPILOT_SLOW_THRUST: f32 = 0.7;

/// The thrust scale `Autopilot Fast` drives at: the other half, `1.3`, confidence 82.
pub const AUTOPILOT_FAST_THRUST: f32 = 1.3;

/// How long before the end `Mirror Left Right` starts blending the yaw back.
///
/// **Recovered, confidence 80.** `Ship_ApplySteeringTorque` negates the yaw while
/// the timer at `body+0x2a0` is above `1.0` and scales it by `1.0 - 2.0 * timer`
/// below, so the hand-back is a ramp. See [`mirror_scale`].
pub const MIRROR_BLEND_SECONDS: f32 = 1.0;

impl Disruption {
    /// Whether an effect is running.
    #[must_use]
    pub fn active(&self) -> bool {
        self.kind.is_some()
    }

    /// The controls this craft actually gets, from the ones its pilot asked for.
    ///
    /// `ai_driven` is whether an AI produces `controls`. Only Stall reads it:
    /// `Ship_UpdateEngine`'s no-thrust exit is `flags & 0x10 && !ai`, so a stalled
    /// opponent keeps driving (the original's asymmetry, kept). Autopilot
    /// Slow/Fast change who drives, the caller's decision; see
    /// [`Self::autopilot_thrust_scale`].
    #[must_use]
    pub fn filter(&self, mut controls: ShipControls, ai_driven: bool) -> ShipControls {
        match self.kind {
            Some(DisruptorEffectKind::Stall) if !ai_driven => controls.thrust = 0.0,
            Some(DisruptorEffectKind::NoAirbrakes) => {
                controls.airbrake_left = 0.0;
                controls.airbrake_right = 0.0;
            }
            Some(DisruptorEffectKind::MirrorLeftRight) => {
                controls.steer_x *= mirror_scale(self.timer);
            }
            _ => {}
        }
        controls
    }

    /// `Some(scale)` while an Autopilot effect has this craft. The caller hands the
    /// craft to its own driver and multiplies the driver's thrust by the scale
    /// (the original's `FUN_0892c620(body, 1)` and `body+0x258`); an already
    /// driven opponent only changes speed.
    #[must_use]
    pub fn autopilot_thrust_scale(&self) -> Option<f32> {
        match self.kind? {
            DisruptorEffectKind::AutopilotSlow => Some(AUTOPILOT_SLOW_THRUST),
            DisruptorEffectKind::AutopilotFast => Some(AUTOPILOT_FAST_THRUST),
            _ => None,
        }
    }

    /// Counts the effect down and tears it down at zero, after the tick's controls
    /// were filtered (`Ship_UpdateWeapons`' order), so a `time` of one tick is one
    /// tick of effect.
    pub fn advance(&mut self, dt: f32) {
        if self.kind.is_none() {
            return;
        }
        self.timer -= dt;
        if self.timer <= 0.0 {
            *self = Self::default();
        }
    }
}

/// The yaw scale `Mirror Left Right` applies with `timer` seconds left: `-1.0`
/// above [`MIRROR_BLEND_SECONDS`], then `1.0 - 2.0 * timer`, reaching `+1.0` as
/// the effect ends. The arithmetic is the original's.
#[must_use]
pub fn mirror_scale(timer: f32) -> f32 {
    if timer > MIRROR_BLEND_SECONDS {
        -1.0
    } else {
        1.0 - 2.0 * timer
    }
}

/// Lands one bolt's effect on a craft, or refuses.
///
/// `Disruptor_ApplyEffect` (`0x08850ec8`) refuses when the victim is already
/// disrupted or under a fired Shield (`oag_physics::ShipState::shield_pickup_timer`,
/// read as `craft+0x178`), in that order. It also refuses an effect the table does
/// not author; the original would apply a `0.0`-second effect there. Returns
/// whether it took, so a caller plays the ship-hit sound on a landing only.
pub fn land<S: Craft>(ship: &mut S, kind: DisruptorEffectKind, stats: &DisruptorStats) -> bool {
    if ship.disruption().active() || ship.physics().shield_pickup_timer > 0.0 {
        return false;
    }
    let Some(effect) = stats.effect(kind) else {
        return false;
    };
    *ship.disruption_mut() = Disruption {
        kind: Some(kind),
        timer: effect.time,
    };
    true
}

/// Counts every craft's disruption down by one tick, over the whole field for
/// [`crate::slowdown::drain`]'s reason. `ships` is the occupied part of the field.
pub fn advance<S: Craft>(ships: &mut [S], dt: f32) {
    for ship in ships {
        ship.disruption_mut().advance(dt);
    }
}

#[cfg(test)]
mod tests;
