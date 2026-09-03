//! Collision sparks: which `.pob` the hull plays, and how hard.
//!
//! Everything about how the burst *looks* - four emitters, their schedules,
//! speeds, lifetimes, colour tables, drag, blend classes and streak
//! classes - is read out of `Data\Psys\WO_SHIP_COLL_SPARK_DAMAGE.POB` at
//! load and played by [`crate::psys`]. This module is what is left over
//! once that is true: the two decisions the *executable* makes rather than
//! the asset.
//!
//! **This is a deliberate reduction.** Until 2026-08-12 this module carried
//! a hand-transcribed `EMITTERS: [EmitterSpec; 4]` constant, read off the
//! file by hand and correct - the parser reproduces its values - but not
//! re-derivable, and no use at all for the other 34 effects on the disc.
//! Deleting it in favour of [`crate::psys::Effect::parse`] is what makes
//! the rocket, missile and ship-death effects reachable without another
//! transcription pass.
//!
//! # 1. Which file
//!
//! `ShipCollisionFx_Trigger` (`0x089246b4`,
//! `docs/ghidra/functions/psp-pulse-usa/contact-response.md`) spawns
//! [`DAMAGE_EFFECT`] when the contact dealt damage and [`NO_DAMAGE_EFFECT`]
//! when it did not - a two-emitter tree that is the damage file's bright
//! fountain plus `bits`, same bytes for every shared field, no smoke and no
//! embers. A live time-trial crash spawns the damage tree (its `_TRAIL`
//! instances were caught emitting), so a caller with no damage flag to hand
//! should play the damage set.
//!
//! # 2. How hard
//!
//! [`severity`] is the instance field the trigger derives from the contact
//! and hands to the particle system, where it multiplies every emitter's
//! ejection speed and every particle's drawn size - **never the particle
//! count**. A harder impact is faster and bigger, not busier.
//!
//! # 3. How often
//!
//! [`COLLISION_COOLDOWN`] is the trigger's own re-arm. The original spawns
//! a fresh instance tree per trigger and lets old ones finish; a caller
//! reusing one [`crate::psys::System`] restarts its emitters instead, which
//! can cut short a few trailing embers of the previous burst - accepted
//! rather than modelling overlapping instance trees.
//!
//! # The pipeline lives here for compatibility
//!
//! [`Pipeline`], [`BLEND`], [`BLEND_ALPHA_OVER`], [`MAX_VERTICES`] and
//! [`effect_path`] are re-exports of [`crate::psys`]'s, which is where they
//! moved when they stopped being spark-specific.

pub use crate::psys::{BLEND, BLEND_ALPHA_OVER, MAX_VERTICES, Pipeline, effect_path};

/// The effect `ShipCollisionFx_Trigger` spawns when the contact dealt
/// damage: a four-emitter tree - orange smoke puffs, a bright spark
/// fountain, white `bits` debris, and lingering `_TRAIL` embers.
pub const DAMAGE_EFFECT: &str = "WO_SHIP_COLL_SPARK_DAMAGE";

/// The effect it spawns when the contact dealt none: the same fountain and
/// `bits`, without the smoke or the embers.
pub const NO_DAMAGE_EFFECT: &str = "WO_SHIP_COLL_SPARK_NODAMAGE";

/// Seconds a wall contact must persist before another burst is allowed.
///
/// **Recovered.** `ShipCollisionFx_Trigger` re-arms exactly this long after
/// every collision-variant spawn: `instance + 100 = now + 0.8`, checked on
/// entry and skipped while still armed.
pub const COLLISION_COOLDOWN: f32 = 0.8;

/// Converts a contact's impact speed into `[0, 1]` intensity.
///
/// **Recovered - this is the exact literal `FUN_088418e0` uses**:
/// `min(|impulse| * 0.0125, 1.0)`, the value `Ship_DispatchCollisionFx` and
/// then `ShipCollisionFx_Trigger` receive as `intensity`. Confirmed by a
/// live capture reading the real value at a real wall hit. This module's
/// own input is a *speed*, not the original's impulse magnitude, so the
/// scale is still borrowed across a unit difference; the coefficient and
/// the clamp are not.
pub const SEVERITY_SCALE: f32 = 0.0125;

/// The multiplier `ShipCollisionFx_Trigger` applies to [`SEVERITY_SCALE`]'s
/// clamped intensity. **Recovered**, from the same `intensity * 2.0 + 0.4`
/// literal.
pub const SEVERITY_SLOPE: f32 = 2.0;

/// The floor it adds on top - a hit is never zero severity, only ever `0.4`
/// at its gentlest. **Recovered.**
pub const SEVERITY_FLOOR: f32 = 0.4;

/// The raw `min(|impulse| * `[`SEVERITY_SCALE`]`, 1.0)` clamp itself, before
/// [`severity`]'s slope-and-floor reshaping.
///
/// This is the value `Ship_DispatchCollisionFx` passes on unmodified to its
/// *second* call, `Camera_ArmShake` - see
/// `docs/ghidra/functions/ps2-pulse-eu/collision-shake.md`. A caller arming
/// [`crate::camera::shake::Shake`] from the same contact wants this, not
/// [`severity`]'s own output: the two reactions share one clamp and diverge
/// only in what each does with it afterwards.
#[must_use]
pub fn clamped_intensity(impact_speed: f32) -> f32 {
    (impact_speed * SEVERITY_SCALE).clamp(0.0, 1.0)
}

/// The scale factor to pass [`crate::psys::System::ignite`] for a contact
/// at `impact_speed` world units per second.
///
/// `intensity * 2.0 + 0.4` over the clamped intensity - see
/// [`SEVERITY_SCALE`].
#[must_use]
pub fn severity(impact_speed: f32) -> f32 {
    clamped_intensity(impact_speed) * SEVERITY_SLOPE + SEVERITY_FLOOR
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn severity_is_floored_and_saturates() {
        assert_eq!(severity(0.0), SEVERITY_FLOOR);
        assert_eq!(severity(-10.0), SEVERITY_FLOOR);
        assert_eq!(
            severity(1.0 / SEVERITY_SCALE),
            SEVERITY_SLOPE + SEVERITY_FLOOR
        );
        assert_eq!(severity(1e6), SEVERITY_SLOPE + SEVERITY_FLOOR);
    }

    #[test]
    fn the_effect_path_is_the_one_the_original_builds() {
        assert_eq!(
            effect_path(DAMAGE_EFFECT),
            r"Data\Psys\WO_SHIP_COLL_SPARK_DAMAGE.POB"
        );
    }
}
