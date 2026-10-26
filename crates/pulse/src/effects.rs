//! What Wipeout Pulse throws and draws in a race, as the `oag_title` tables the
//! raceplay loader reads instead of comparing a title's name (ADR-0058).
//!
//! Every entry is **Measured** on the PSP executable and cites its page. The PS2
//! port's own paths are unread, and the PSP is the reference this engine
//! implements, so where an entry applies to a PS2 disc too it is the PSP's law
//! carried over, which the [`LOOKS`] platform lists make explicit.

use oag_title::{
    Burst, EffectSpec, Effects, Looks, Origin, Platform, Platforms, Rule, ShieldPalette,
    ShieldPalettes, Trigger, engine_effects,
};

/// `Ship_PlayAbsorbFeedback` (`0x08840640`): the effect once per `Ship
/// Collision Fx` node, up to ten, node `i` delayed `i * 0.1` s
/// (`DAT_08abf564`). Pulse, PSP and PS2 alike - the PS2's own absorb path is
/// unread.
pub const ABSORB_BURST: Burst = Burst::Sequential {
    cap: 10,
    stagger: 0.1,
};

/// Pulse's tables: the engine's own names, and the four triggers a title
/// answers for itself. Measured; pages cited per entry.
pub const EFFECTS: &Effects = &Effects::engine(Origin::Measured)
    // Names the shared engine table carries from HD and 2048 that no Pulse
    // archive holds and no Pulse executable requests - the by-design absence is
    // proven in `docs/formats/pulse-absent-effects.md`.
    .without(Trigger::PlasmaLightningExpand)
    .without(Trigger::PlasmaLightningCollapse)
    .without(Trigger::TrailHitship)
    .without(Trigger::TrailHitshipRed)
    .without(Trigger::LeachbeamAbsorb)
    .without(Trigger::MagstripSparks)
    .without(Trigger::MagstripZone)
    // The PS2 disc ships `WO_SHIP_ENGINEFLARE.POB` and nothing plays it: the
    // executable names every `Data\Psys` effect it loads in one path list and this
    // one is not in it (no name, no `0x0afe819a` hash), and GS dumps of the player's
    // nozzle at rest and after four seconds of thrust hold one 128x64 flare quad per
    // craft and no particle batch. The flare both pressings draw is the procedural
    // quad `oag_fx::exhaust` builds from the locator; `engine-trail.md`, "The engine
    // flare is one quad".
    .without(Trigger::EngineFlare)
    // `Ship_Damage` (`0x088439ac`) weapon branch; `docs/ghidra/functions/psp-pulse-usa/shield.md`.
    // The names are `WO_SHIP_COLL_SPARK_DAMAGE` and the LeachBeam variant.
    .with(
        Trigger::HitSpark,
        EffectSpec::new(engine_effects::COLLISION_SPARK_EFFECT, Origin::Measured),
    )
    .with(
        Trigger::LeachHitSpark,
        EffectSpec::new(engine_effects::LEACHBEAM_HIT_SPARK_EFFECT, Origin::Measured),
    )
    // `docs/ghidra/functions/psp-pulse-usa/shield.md`.
    .with(
        Trigger::ShieldAbsorb,
        EffectSpec::new(engine_effects::ABSORB_EFFECT, Origin::Measured).with_burst(ABSORB_BURST),
    )
    // `Ship_SetState` case 5, read live on PPSSPP 2026-10-01;
    // `docs/ghidra/functions/psp-pulse-usa/screen-flash-callers.md`.
    .with(
        Trigger::WreckNode,
        EffectSpec::new(engine_effects::FXNODE_EXPLO_EFFECT, Origin::Measured),
    )
    .with(
        Trigger::WreckSparks,
        EffectSpec::new(engine_effects::DEATH_SPARKS_EFFECT, Origin::Measured),
    )
    .with(
        Trigger::WreckExplosion,
        EffectSpec::new(engine_effects::EXPLOSION_EFFECT, Origin::Measured),
    );

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
    // `Ship_SetState` case 5's `shipwreck.vex` swap, measured on the PSP; the
    // PS2 disc ships the same file per team and its port is unread, so the
    // PSP's law is carried over (PS2 skins it from the preceding entry).
    hull_wreck: Rule {
        on: Platforms::Any,
        origin: Origin::Measured,
    },
    absorb_shell: Rule::UNREAD,
    // `Mine_PoseNode` and `Bomb_Init`; `docs/ghidra/functions/psp-pulse-usa/mine.md`.
    laid_pose: Rule {
        on: Platforms::Any,
        origin: Origin::Measured,
    },
    // Pulse lays at the body itself (measured 2026-10-01), so the rear anchor does not apply.
    laid_from_rear: Rule::UNREAD,
    laid_pose_scaled: Rule {
        on: Platforms::Any,
        origin: Origin::Measured,
    },
    // The circuit's chrome-map pass, `oag_render::shine::build_track`, each batch's ordinary
    // pass weighted by its own texture's alpha when it carries the weight bit: a recorded
    // Pulse PSP GE list draws those ordinary passes with BLENDMODE `0x2a`, as Pure's
    // (`docs/ghidra/functions/psp-pulse-usa/circuit-shine.md`).
    track_shine: Rule {
        on: PSP,
        origin: Origin::Measured,
    },
    track_shine_weighted: Rule {
        on: PSP,
        origin: Origin::Measured,
    },
    // GE lights, glow mask, quake: `scene-light.md`, `docs/rendering/glow-mask.md`.
    measured_draws: Rule {
        on: PSP,
        origin: Origin::Measured,
    },
    // The stencil-stamped mask and the bloom over it: `docs/rendering/glow-mask.md`,
    // `docs/ghidra/functions/psp-pulse-usa/bloom.md`.
    psp_glow_mask: Rule {
        on: PSP,
        origin: Origin::Measured,
    },
    // Pulse's composite sits at queue key `0x70`, after every HUD widget
    // (`bloom.md`, "The bloom draws over the HUD").
    bloom_under_hud: Rule::UNREAD,
    // `ParticleSystem_EmitLine`/`EmitSphere`, `DrawStreak` and the frame advance, read
    // on the PSP. The PS2 ships the same `.pob` files (`docs/formats/pob.md`), so the
    // PSP's law is carried over, unmeasured on the PS2.
    particle_laws: Rule {
        on: Platforms::Only(&[Platform::Psp, Platform::Ps2]),
        origin: Origin::Measured,
    },
    // Measured off GS dumps of the original on PCSX2: `docs/rendering/ps2-bloom.md`.
    ps2_glow_mask: Rule {
        on: Platforms::Only(&[Platform::Ps2]),
        origin: Origin::Measured,
    },
    // The PS2 trail draw `0x001f0a08`, read statically and on a GS dump of the original on
    // PCSX2: `docs/ghidra/functions/ps2-pulse-eu/engine-trail.md`.
    ps2_trail: Rule {
        on: Platforms::Only(&[Platform::Ps2]),
        origin: Origin::Measured,
    },
    // `PsysNode_Init`, `Weather_Update` and the destroy camera, read on the PSP
    // (`placed-particle-systems.md`, `weather.md`); the PS2 circuits author the
    // same nodes, so the PSP's law is carried over.
    circuit_effects: Rule {
        on: Platforms::Only(&[Platform::Psp, Platform::Ps2]),
        origin: Origin::Measured,
    },
    // `Quake_UpdateSpan`, measured on PPSSPP (`cannon-quake-leachbeam.md`). The PS2
    // circuits author the same table (parameters in a pool, `oag_vex::quake`), so
    // the PSP's law is carried over, unmeasured on the PS2.
    road_ripple: Rule {
        on: Platforms::Only(&[Platform::Psp, Platform::Ps2]),
        origin: Origin::Measured,
    },
    // `SceneLight_BuildLightingList` and `Craft_ApplyHullLightScale`, read and
    // matched live on the PSP (`scene-light.md`). The PS2 circuits author the
    // same light nodes and the PS2 builds the same `wopoint` vector, but its hull
    // vertices carry a colour, so the rig is inert there today
    // (`ps2-pulse-eu/hull-lights.md`).
    hull_lights: Rule {
        on: Platforms::Only(&[Platform::Psp, Platform::Ps2]),
        origin: Origin::Measured,
    },
    // Pulse has no sun flare: `lensflare` (`0x3de`) is in its name table, zero nodes
    // and zero strings (`docs/rendering/lens-flare.md`, "Cross-title").
    lens_flare: Rule::UNREAD,
    // `docs/ghidra/functions/psp-pulse-usa/shield-pickup.md`; the PS2 tint is
    // measured on PCSX2 (`oag_render::shield::PS2_PULSE_PALETTE`).
    shield_palette: ShieldPalettes {
        ps2: ShieldPalette::Ps2Pulse,
        elsewhere: ShieldPalette::Pulse,
        origin: Origin::Measured,
    },
    // Pulse shows its picker; this build waits at it too.
    skips_language_picker: Rule::UNREAD,
    // Pulse lights on the GE; no fragment-program output scale exists there.
    lit_output_halved: Rule::UNREAD,
    // The GE selects the level by its slope law (`oag_mesh::mesh_render::PSP_TEXLOD_SLOPE`), not a bias.
    material_lod_bias: Rule::UNREAD,
    material_render_state: Rule::UNREAD,
    blends_encoded: Rule::UNREAD,
    // Pulse ends its frame in its own bloom (`oag_post::bloom`), not the `wo_*` chain.
    wo_composite: Rule::UNREAD,
};
