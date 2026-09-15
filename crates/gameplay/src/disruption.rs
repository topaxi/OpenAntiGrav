//! What a Disruptor hit does to the craft it lands on - Pure's one weapon that
//! hurts nobody.
//!
//! A bolt from [`crate::projectile::disruptor`] that sweeps through a craft
//! hands it one of eight control effects for an authored number of seconds,
//! and this module is that state and the law that spends it. The whole
//! reading is on `docs/ghidra/functions/psp-pure-usa/weapons.md`, "Hit: one
//! flag, one timer, one switch on the craft, another on the ship"; the
//! addresses below are all on that page.
//!
//! # Five of the eight are built, three land and do nothing, and the split is
//! the point
//!
//! `Ship_UpdateWeapons` (`0x0892935c`) spends the eight kinds as bits on the
//! body's flag word, and five of those bits are read by code that does
//! nothing but rewrite the *inputs* the force law is about to see - no
//! thrust, no airbrakes, a negated or blended yaw, an autopilot with a
//! literal thrust scale. Those five are [`Disruption::filter`] and
//! [`Disruption::autopilot_thrust_scale`], and a composition root applies
//! them to the `ShipControls` it was about to hand `oag_physics::step`, on
//! the same footing `Ship::autopilot_timer` already decides *which* controls
//! a craft is handed.
//!
//! The other three are not input filters, and they are **not approximated
//! here**:
//!
//! - **Drunk** adds a random yaw offset `((rand() & 0xff) - 128) * 0.01 *
//!   amount` to the steering input inside `Ship_ApplySteeringTorque`
//!   (`0x0892edfc`). The law is read; the *scale of the input it is added to*
//!   is not - whether that input is `-1..1` or a wider range decides whether
//!   the authored `amount` is a wobble or a spin, and guessing it would put
//!   an invented number on an authored one.
//! - **Rubber Ship** scales a hover damping term by `0.2` inside
//!   `FUN_0892dea8`, which is the suspension law in `oag-physics`, a crate
//!   this module does not reach into.
//! - **Drunk Camera** moves the camera and touches the simulation not at all.
//!
//! A craft hit by one of those three is still *disrupted* - the kind and the
//! timer are set, the hash moves, a second bolt bounces off it for the
//! duration - it just drives as if nothing happened. That is the honest
//! shape: the state the original keeps, none of the force it applies.

use oag_physics::ShipControls;
use oag_tables::weapons::{DisruptorEffectKind, DisruptorStats};

use crate::world::Ship;

/// One craft's current disruption, or none.
///
/// The original's `craft+0x134` (the flag), `+0x138` (the kind) and `+0x13c`
/// (the seconds left), folded into one `Option` and one `f32`: a `kind` of
/// `None` is the flag clear, and the timer is then meaningless and held at
/// zero. Hashed by [`crate::hash`], because a craft that cannot thrust is a
/// different race from one that can.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Disruption {
    /// Which effect is running, or `None` for a craft nothing has hit.
    pub kind: Option<DisruptorEffectKind>,
    /// Seconds left before the effect is torn down.
    ///
    /// `Ship_UpdateWeapons` subtracts `dt` after applying the tick's flags and
    /// tears everything down at or below zero, so an effect authored at `t`
    /// seconds is applied for `ceil(t / dt)` ticks. [`Self::advance`] is that
    /// order.
    pub timer: f32,
}

/// The thrust scale `Autopilot Slow` drives at.
///
/// **Recovered, confidence 82, and a code literal rather than the authored
/// `speed_percent`.** `Ship_UpdateWeapons`'s kind-3 arm writes `0.7` into
/// `body+0x258` and `Ship_UpdateEngine` (`0x0892f1f0`) multiplies thrust by
/// it while bit `4` is set. The table's `speed_percent` is copied to
/// `craft+0x144` and read by nothing found - see
/// `oag_tables::weapons::DisruptorEffect::speed_percent`.
pub const AUTOPILOT_SLOW_THRUST: f32 = 0.7;

/// The thrust scale `Autopilot Fast` drives at - the same literal pair's
/// other half, `1.3`, confidence 82.
pub const AUTOPILOT_FAST_THRUST: f32 = 1.3;

/// How long before the end `Mirror Left Right` starts blending the yaw back.
///
/// **Recovered, confidence 80.** `Ship_ApplySteeringTorque` negates the yaw
/// outright while the timer at `body+0x2a0` is above `1.0` and scales it by
/// `1.0 - 2.0 * timer` below that, so the last second walks from `-1` to
/// `+1` and the hand-back is a ramp rather than a snap. See
/// [`mirror_scale`].
pub const MIRROR_BLEND_SECONDS: f32 = 1.0;

impl Disruption {
    /// Whether an effect is running.
    #[must_use]
    pub fn active(&self) -> bool {
        self.kind.is_some()
    }

    /// The controls this craft actually gets, from the ones its pilot asked
    /// for.
    ///
    /// `ai_driven` is whether an AI is producing `controls` - an opponent, or
    /// the player under either autopilot. **Stall reads it and nothing else
    /// does**: `Ship_UpdateEngine`'s no-thrust exit is `flags & 0x10 && !ai`,
    /// so a stalled opponent keeps driving, which is the original's own
    /// asymmetry and is kept rather than evened out. Mirror and No Airbrakes
    /// sit on the torque and airbrake laws that every craft runs.
    ///
    /// Autopilot Slow/Fast filter nothing here - they change *who* drives,
    /// which is the caller's decision; see [`Self::autopilot_thrust_scale`].
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

    /// `Some(scale)` while an Autopilot effect has this craft, `None`
    /// otherwise.
    ///
    /// A caller that gets `Some` hands the craft to its own driver and
    /// multiplies the thrust the driver asks for by the scale - the original's
    /// `FUN_0892c620(body, 1)` and `body+0x258`. The player's craft therefore
    /// drives itself for the duration, slower or faster than the player
    /// would; an opponent, already driven, only changes speed.
    #[must_use]
    pub fn autopilot_thrust_scale(&self) -> Option<f32> {
        match self.kind? {
            DisruptorEffectKind::AutopilotSlow => Some(AUTOPILOT_SLOW_THRUST),
            DisruptorEffectKind::AutopilotFast => Some(AUTOPILOT_FAST_THRUST),
            _ => None,
        }
    }

    /// Counts the effect down and tears it down at zero.
    ///
    /// After the tick's controls were filtered, not before - the order the
    /// original runs `Ship_UpdateWeapons` in, so a `time` of exactly one tick
    /// is one tick of effect and not none.
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

/// The yaw scale `Mirror Left Right` applies with `timer` seconds left.
///
/// `-1.0` above [`MIRROR_BLEND_SECONDS`], then `1.0 - 2.0 * timer` - so it
/// passes through zero with half a second left and reaches `+1.0`, plain
/// steering, exactly as the effect ends. The arithmetic is the original's.
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
/// disrupted or is under a fired Shield, in that order, and this does both -
/// the second through `oag_physics::ShipState::shield_pickup_timer`, this
/// crate's reading of `craft+0x178`. It also refuses an effect the table
/// does not author, which the original cannot meet: its zero-initialised
/// table would apply a `0.0`-second effect there, and a `None` here is the
/// same nothing without the one-tick flicker.
///
/// Returns whether the effect took, so a caller can play the ship-hit sound
/// on a landing and not on a refusal.
pub fn land(ship: &mut Ship, kind: DisruptorEffectKind, stats: &DisruptorStats) -> bool {
    if ship.disruption.active() || ship.physics.shield_pickup_timer > 0.0 {
        return false;
    }
    let Some(effect) = stats.effect(kind) else {
        return false;
    };
    ship.disruption = Disruption {
        kind: Some(kind),
        timer: effect.time,
    };
    true
}

/// Counts every craft's disruption down by one tick.
///
/// One call over the whole field, for the reason [`crate::slowdown::drain`]
/// gives about itself: the countdown must not depend on whether a craft is
/// stepped by slot 0's branch or the field's.
pub fn advance(world: &mut crate::World, dt: f32) {
    let count = world.ship_count as usize;
    for ship in &mut world.ships[..count] {
        ship.disruption.advance(dt);
    }
}

#[cfg(test)]
mod tests;
