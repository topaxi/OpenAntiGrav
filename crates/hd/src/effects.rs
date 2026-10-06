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

/// HD's tables: the engine's own names by inheritance from Pulse, and HD's own
/// weapon spark and absorb burst.
pub const EFFECTS: &Effects = &Effects::engine(Origin::InheritedFrom("Wipeout Pulse"))
    // `Cannon_ApplyCraftHit`; `docs/ghidra/functions/ps3-hdfury-eu/ship-collision-fx.md`.
    .with(
        Trigger::WeaponSpark,
        EffectSpec::new(engine_effects::WEAPON_SPARK_EFFECT, Origin::Measured),
    )
    .with(
        Trigger::ShieldAbsorb,
        EffectSpec::new(engine_effects::ABSORB_EFFECT, Origin::Measured).with_burst(ABSORB_BURST),
    );

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
