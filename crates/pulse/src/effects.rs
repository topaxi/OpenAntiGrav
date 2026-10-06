//! What Wipeout Pulse throws and draws in a race, as the `oag_title` tables the
//! raceplay loader reads instead of comparing a title's name (ADR-0058).
//!
//! Every entry is **Measured** on the PSP executable and cites its page. The PS2
//! port's own paths are unread, and the PSP is the reference this engine
//! implements, so where an entry applies to a PS2 disc too it is the PSP's law
//! carried over, which the [`LOOKS`] platform lists make explicit.

use oag_title::{
    Burst, EffectSpec, Effects, Looks, Origin, Platform, Platforms, Rule, ShieldPalette,
    ShieldPalettes,
};

/// `Ship_PlayAbsorbFeedback` (`0x08840640`): the effect once per `Ship
/// Collision Fx` node, up to ten, node `i` delayed `i * 0.1` s
/// (`DAT_08abf564`). Pulse, PSP and PS2 alike - the PS2's own absorb path is
/// unread.
pub const ABSORB_BURST: Burst = Burst::Sequential {
    cap: 10,
    stagger: 0.1,
};

/// Pulse's tables. Measured; pages cited per entry.
pub const EFFECTS: &Effects = &Effects {
    // `Ship_Damage` (`0x088439ac`) weapon branch; `docs/ghidra/functions/psp-pulse-usa/shield.md`.
    // The names are `WO_SHIP_COLL_SPARK_DAMAGE` and the LeachBeam variant.
    hit_spark: Some(EffectSpec {
        effects: &[
            "WO_SHIP_COLL_SPARK_DAMAGE",
            "WO_SHIP_SPARK_DAMAGE_LEACHBEAM",
        ],
        burst: None,
        origin: Origin::Measured,
    }),
    // HD's mechanism; Pulse throws it from the craft's own hull instead.
    weapon_spark: None,
    // `docs/ghidra/functions/psp-pulse-usa/shield.md`.
    shield_absorb: Some(EffectSpec {
        effects: &["WO_WEAPON_ABSORB"],
        burst: Some(ABSORB_BURST),
        origin: Origin::Measured,
    }),
    // `Ship_SetState` case 5, read live on PPSSPP 2026-10-01;
    // `docs/ghidra/functions/psp-pulse-usa/screen-flash-callers.md`.
    wreck: Some(EffectSpec {
        effects: &[
            "WO_SHIP_FXNODE_EXPLO",
            "WO_SHIP_DEATH_SPARKS",
            "WO_SHIP_EXPLOSION",
        ],
        burst: None,
        origin: Origin::Measured,
    }),
};

const PSP: Platforms = Platforms::Only(&[Platform::Psp]);

/// Pulse's race looks.
pub const LOOKS: &Looks = &Looks {
    // `HullOverlay_DrawAbsorb`; `cannon-quake-leachbeam.md`, "the hull overlay pair".
    hull_overlay: Rule {
        on: Platforms::Any,
        origin: Origin::Measured,
    },
    // The hull's extra pass; `docs/ghidra/functions/psp-pulse-usa/mesh-draw.md`.
    hull_shine: Rule {
        on: Platforms::Any,
        origin: Origin::Measured,
    },
    // `Ship_SetState` case 5's `shipwreck.vex` swap, measured on the PSP only.
    hull_wreck: Rule {
        on: PSP,
        origin: Origin::Measured,
    },
    absorb_shell: Rule::UNREAD,
    // `Mine_PoseNode` and `Bomb_Init`; `docs/ghidra/functions/psp-pulse-usa/mine.md`.
    laid_pose: Rule {
        on: Platforms::Any,
        origin: Origin::Measured,
    },
    // GE lights, glow mask, quake: `scene-light.md`, `docs/rendering/glow-mask.md`.
    measured_draws: Rule {
        on: PSP,
        origin: Origin::Measured,
    },
    // Measured off GS dumps of the original on PCSX2: `docs/rendering/ps2-bloom.md`.
    ps2_glow_mask: Rule {
        on: Platforms::Only(&[Platform::Ps2]),
        origin: Origin::Measured,
    },
    // `docs/ghidra/functions/psp-pulse-usa/shield-pickup.md`; the PS2 tint is
    // measured on PCSX2 (`oag_render::shield::PS2_PULSE_PALETTE`).
    shield_palette: ShieldPalettes {
        ps2: ShieldPalette::Ps2Pulse,
        elsewhere: ShieldPalette::Pulse,
        origin: Origin::Measured,
    },
};
