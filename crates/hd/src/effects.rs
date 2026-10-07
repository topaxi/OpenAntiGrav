//! What Wipeout HD / Fury throws and draws in a race: the `oag_title` tables the
//! raceplay loader reads instead of comparing a title's name (ADR-0058).
//!
//! HD's own mechanisms, none of them Pulse's: the Cannon throws its spark from
//! the weapon's side, the absorb burst fires off `absorb` locators in mirrored
//! pairs, and the shell is a separate authored model per team. Hit sparks and
//! the wreck are unread, so `None`.

use oag_title::{
    Burst, EffectSpec, Effects, Looks, Origin, Platforms, Rule, ShieldPalette, ShieldPalettes,
    Trigger, engine_effects,
};

/// `FUN_000d9398`: six `absorb` locators, three mirrored pairs at `0.0`, `0.2`
/// and `0.4` s (the settings block's `+0x54`, `0.2` in `.data`) -
/// `docs/ghidra/functions/ps3-hdfury-eu/absorb-feedback.md`.
pub const ABSORB_BURST: Burst = Burst::MirroredPairs { stagger: 0.2 };

/// The scenery effects HD's own disc authors: `wo_blue_welder.pob` and
/// `wo_modesto_steam_a.pob` in `DATA02`. Pulse's list also names `WO_RAIN`,
/// `WO_RAIN_LENS` and `WO_SNOW`, which no archive of HD's disc carries
/// (`docs/formats/pob.md`, "Which names HD does not author").
const SCENERY: &[&str] = &[
    engine_effects::BLUE_WELDER_EFFECT,
    engine_effects::MODESTO_STEAM_EFFECT,
];

/// HD's tables: the engine's own names by inheritance from Pulse, and HD's own
/// weapon spark and absorb burst.
///
/// **Two inherited triggers are taken back.** `WO_MAGSTRIP_SPARKS` and
/// `WO_MAGSTRIP_ZONE` are 2048-lineage files (Omega's `Data/particles2048`):
/// no PSARC of HD's disc carries either, and HD builds the arc wake instead
/// (`Title::magstrip_pob` is `false` here) - `docs/formats/pob.md`, "Which
/// names HD does not author". The scenery list drops `WO_RAIN`, `WO_RAIN_LENS`
/// and `WO_SNOW` for the same reason; see [`SCENERY`].
pub const EFFECTS: &Effects = &{
    let mut effects = Effects::engine(Origin::InheritedFrom("Wipeout Pulse"))
        // `Cannon_ApplyCraftHit`; `docs/ghidra/functions/ps3-hdfury-eu/ship-collision-fx.md`.
        .with(
            Trigger::WeaponSpark,
            EffectSpec::new(engine_effects::WEAPON_SPARK_EFFECT, Origin::Measured),
        )
        // `ShipDamageFx_Update_q`; `particle-triggers.md`, "The damage smoke".
        .with(
            Trigger::DamageMild,
            EffectSpec::new(engine_effects::DAMAGE_MILD_EFFECT, Origin::Measured),
        )
        .with(
            Trigger::DamageModerate,
            EffectSpec::new(engine_effects::DAMAGE_MODERATE_EFFECT, Origin::Measured),
        )
        .with(
            Trigger::DamageCritical,
            EffectSpec::new(engine_effects::DAMAGE_CRITICAL_EFFECT, Origin::Measured),
        )
        // `0x000e7760` sets `+0x260` on a LeachBeam hit and `0x002a06e0` spawns
        // it attached; `particle-triggers.md`, "The LeachBeam hit spark".
        .with(
            Trigger::LeachHitSpark,
            EffectSpec::new(engine_effects::LEACHBEAM_HIT_SPARK_EFFECT, Origin::Measured),
        )
        .with(
            Trigger::ShieldAbsorb,
            EffectSpec::new(engine_effects::ABSORB_EFFECT, Origin::Measured)
                .with_burst(ABSORB_BURST),
        )
        // `NormalBombBlast_Update` (`0x001503d8`) spawns `WO_BOMB_RAYS` once,
        // the first tick the blast is older than half a second;
        // `weapons.md`, 2026-10-07.
        .with(
            Trigger::BombRays,
            EffectSpec::new(engine_effects::BOMB_RAYS_EFFECT, Origin::Measured),
        )
        .without(Trigger::MagstripSparks)
        .without(Trigger::MagstripZone);
    effects.scenery = SCENERY;
    effects
};

/// Two of the palette's three colours are read off HD's executable
/// (`docs/ghidra/functions/ps3-hdfury-eu/shield.md`); the settled target is
/// chosen, and `oag_render::shield::HD_PALETTE` says so.
const SHIELD: ShieldPalettes = ShieldPalettes {
    ps2: ShieldPalette::Hd,
    elsewhere: ShieldPalette::Hd,
    origin: Origin::Measured,
};

/// HD's race looks.
pub const LOOKS: &Looks = &Looks {
    // `ShipAbsorbShell_Load` and its step; `absorb-feedback.md`, "The absorb shell".
    absorb_shell: Rule {
        on: Platforms::Any,
        origin: Origin::Measured,
    },
    // `Language Selection` left within 2-296 ms on four RPCS3 boots through
    // the screen's own `LanguageAutoRedirect`;
    // `docs/formats/hd-frontend.md#is-the-language-picker-ever-shown`.
    skips_language_picker: Rule {
        on: Platforms::Any,
        origin: Origin::Measured,
    },
    ..Looks::unread(SHIELD)
};

#[cfg(test)]
mod tests {
    use super::*;

    /// The five names no archive of HD's disc carries
    /// (`docs/formats/pob.md`, "Which names HD does not author"); the
    /// disc-backed half is `psys_inventory_ground_truth.rs`.
    #[test]
    fn hd_does_not_ask_for_the_effects_its_disc_does_not_author() {
        let names = EFFECTS.names();
        for absent in [
            engine_effects::RAIN_EFFECT,
            engine_effects::RAIN_LENS_EFFECT,
            engine_effects::SNOW_EFFECT,
            engine_effects::MAGSTRIP_SPARKS_EFFECT,
            engine_effects::MAGSTRIP_ZONE_EFFECT,
        ] {
            assert!(!names.contains(&absent), "{absent}");
        }
        assert!(names.contains(&engine_effects::MODESTO_STEAM_EFFECT));
        assert!(names.contains(&engine_effects::ROCKET_FLARE_EFFECT));
    }
}
