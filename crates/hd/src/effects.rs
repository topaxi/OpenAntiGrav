//! What Wipeout HD / Fury throws and draws in a race: the `oag_title` tables the
//! raceplay loader reads instead of comparing a title's name (ADR-0058).
//!
//! HD's own mechanisms, none of them Pulse's: the Cannon throws its spark from
//! the weapon's side, the absorb burst fires off `absorb` locators in mirrored
//! pairs, and the shell is a separate authored model per team. Hit sparks and
//! the wreck are unread, so `None`.

use oag_title::{
    Burst, EffectSpec, Effects, Looks, NearLaw, Origin, Platforms, Rule, ShieldPalette,
    ShieldPalettes, Trigger, engine_effects,
};

/// `FUN_000d9398`: six `absorb` locators, three mirrored pairs at `0.0`, `0.2`
/// and `0.4` s (the settings block's `+0x54`, `0.2` in `.data`) -
/// `docs/ghidra/functions/ps3-hdfury-eu/absorb-feedback.md`.
pub const ABSORB_BURST: Burst = Burst::HD;

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
        // `Ship_PlayDestroyExplosion` (`0x000d96c8`), state 4's entry, and
        // `ShipDamageFx_Update_q`'s death branch, state 5's;
        // `docs/ghidra/functions/ps3-hdfury-eu/ship-destroy.md`, rows 2 and 10. HD names
        // neither of Pulse's two per-locator wreck effects, so those stay unset.
        .with(
            Trigger::WreckExplosion,
            EffectSpec::new(engine_effects::EXPLOSION_EFFECT, Origin::Measured),
        )
        .with(
            Trigger::WreckPlume,
            EffectSpec::new(engine_effects::DEATH_PLUME_EFFECT, Origin::Measured),
        )
        // `Debris_Launch` (`0x0010c958`) attaches it to a piece whose record byte `+0x58` is set;
        // `ship-destroy.md`, "The debris throw".
        .with(
            Trigger::DebrisFire,
            EffectSpec::new(engine_effects::DEBRIS_FIRE_EFFECT, Origin::Measured),
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
        // `Plasma_Init` and `Plasma_Launch`; `plasma.md`, "The effects and their triggers".
        .with(
            Trigger::PlasmaCharging,
            EffectSpec::new(engine_effects::PLASMA_CHARGING_EFFECT, Origin::Measured),
        )
        .with(
            Trigger::PlasmaLaunch,
            EffectSpec::new(engine_effects::PLASMA_LAUNCH_EFFECT, Origin::Measured),
        )
        // Detonator's four, off `DetonatorMine_Explode`, `DetonatorBombBlast_Start` and
        // `PulseBullet_Update`; `docs/formats/pob.md`, "Detonator's explosions".
        .with(
            Trigger::DetonatorMineExploDead,
            EffectSpec::new(
                engine_effects::DETONATOR_MINE_EXPLO_DEAD_EFFECT,
                Origin::Measured,
            ),
        )
        .with(
            Trigger::DetonatorMineExplo,
            EffectSpec::new(
                engine_effects::DETONATOR_MINE_EXPLO_EFFECT,
                Origin::Measured,
            ),
        )
        .with(
            Trigger::DetonatorBombExplo,
            EffectSpec::new(
                engine_effects::DETONATOR_BOMB_EXPLO_EFFECT,
                Origin::Measured,
            ),
        )
        .with(
            Trigger::DetonatorCannonSparks,
            EffectSpec::new(
                engine_effects::DETONATOR_CANNON_SPARKS_EFFECT,
                Origin::Measured,
            ),
        )
        // The EMP's wave, `EMP_SpawnLaunchEffect`; `detonator-emp.md`, "What HD draws".
        .with(
            Trigger::DetonatorEmp,
            EffectSpec::new(engine_effects::DETONATOR_EMP_EFFECT, Origin::Measured),
        )
        // The disc ships `wo_plasma_flash.pob` and the executable never names it: the
        // detonation is `WO_PLASMA_LIGHTNING_EXPAND` (same section).
        .without(Trigger::PlasmaBlast)
        .without(Trigger::EngineFlare)
        .without(Trigger::MagstripSparks)
        .without(Trigger::MagstripZone);
    effects.scenery = SCENERY;
    effects.colour_walk = oag_title::ColourWalk::LightBlend;
    effects.sprites = SPRITES;
    effects.extents = EXTENTS;
    effects.far_bars = FAR_BARS;
    effects.templates = TEMPLATES;
    effects.turbulence = TURBULENCE;
    // The `PartRender` SPU job, image `0x0080bb80`;
    // `docs/ghidra/functions/ps3-hdfury-eu/particle-render.md`.
    // The template cap is measured on the Rocket's craft blast, where it is read is not:
    // `docs/ghidra/functions/ps3-hdfury-eu/rocket-craft-blast.md`, "The screen wash".
    effects.near_law =
        Some(NearLaw::one_to_four(Origin::Measured).with_template_cap(2.0, Origin::Measured));
    // `Rocket_SpawnCraftBlast` (`0x0014a7d8`), `0x0014a970`-`0x0014a98c`;
    // `docs/ghidra/functions/ps3-hdfury-eu/rocket-craft-blast.md`.
    effects.craft_blast_carry = Some(Origin::Measured);
    // `Psys_UpdateParticles` (`0x0028cc10`), `0x0028d0f0`-`0x0028d1bc`;
    // `docs/ghidra/functions/ps3-hdfury-eu/particle-render.md`, "The atlas frame".
    effects.flipbooks = Some(Origin::Measured);
    // `Psys_InitParticle` (`0x00289890`), `0x00289ee8`-`0x00289f10`;
    // `docs/ghidra/functions/ps3-hdfury-eu/particle-render.md`, "The per-particle size
    // multiplier".
    effects.size_ramps = Some(Origin::Measured);
    effects
};

/// The effects whose particles feel their noise force: the Detonator EMP's wave, whose
/// `SPARKS` author no speed and move by it alone
/// (`docs/ghidra/functions/ps3-hdfury-eu/detonator-emp.md`, "The grey rows were sparks that
/// never moved").
const TURBULENCE: &[&str] = &[engine_effects::DETONATOR_EMP_EFFECT];

/// The effects whose class-6 particles draw as HD's view-space bar: the Detonator cannon's
/// wall spark's three streaks
/// (`docs/ghidra/functions/ps3-hdfury-eu/detonator-cannon.md`, "The spark's matrix, read
/// live"), the engine's two trail hits, the Detonator EMP's sparks (`docs/ghidra/functions/ps3-hdfury-eu/detonator-emp.md`,
/// "The grey rows were sparks that never moved"), and the Plasma's five, whose lightning is that class. Paired against the
/// original phase by phase: with the bar the launch's and the blast's white blobs are
/// lightning (`docs/ghidra/functions/ps3-hdfury-eu/plasma.md`, "The pairs").
const FAR_BARS: &[&str] = &[
    engine_effects::DETONATOR_CANNON_SPARKS_EFFECT,
    engine_effects::TRAIL_HITSHIP_EFFECT,
    engine_effects::TRAIL_HITSHIP_RED_EFFECT,
    engine_effects::DETONATOR_EMP_EFFECT,
    engine_effects::PLASMA_FLARE_EFFECT,
    engine_effects::PLASMA_CHARGING_EFFECT,
    engine_effects::PLASMA_LAUNCH_EFFECT,
    engine_effects::PLASMA_LIGHTNING_EXPAND_EFFECT,
    engine_effects::PLASMA_LIGHTNING_COLLAPSE_EFFECT,
];

/// The effects whose sprite templates play: the Plasma head's ring and glow and the
/// wind-up's glow. The release's `FLASH` and the blast's `START_FLASH` stay muted: with
/// them on the frame whites out for their few ticks where the original's does not, and
/// why is unread (same page). The Rocket's craft blast: its root's `glow_longer` is the
/// yellow screen wash and `FIRE_TRAILS_ONEQUAD`'s `Glow` the white flash of the hit, both
/// removed from the original's frame by zeroing their gate
/// (`docs/ghidra/functions/ps3-hdfury-eu/rocket-craft-blast.md`, "The screen wash").
const TEMPLATES: &[&str] = &[
    engine_effects::PLASMA_FLARE_EFFECT,
    engine_effects::PLASMA_CHARGING_EFFECT,
    engine_effects::CRAFT_BLAST_EFFECT,
];

/// The effects whose `.gtf` sprites HD plays, from `/data/psys/tex/`: every effect this
/// table loads with a camera-facing emitter or a read bar. The disc names each sprite and
/// every one decodes (`docs/rendering/particle-fill.md`, "Which effects draw a sprite").
///
/// **Not listed: the effects whose every emitter is a streak that [`FAR_BARS`] does not
/// name.** A procedural strip samples nothing, so listing one changes no pixel: `WO_BOMB_RAYS`,
/// `WO_REPULSER`, the three LeachBeam effects and its hit spark, the weapon spark, the
/// no-damage collision spark, `WO_DAMAGE_CRITICAL` and `WO_BLUE_WELDER`. The streak
/// emitters of a listed effect stay procedural for the same reason.
///
/// Paired against the original: the Detonator cannon's wall spark
/// (`docs/ghidra/functions/ps3-hdfury-eu/detonator-cannon.md`, "The spark's matrix, read
/// live"), the Rocket's two blasts (`particle-triggers.md`, "The Rocket's smoke";
/// `particle-fill.md`, "Against the original at a near blast"), the Detonator EMP's wave
/// (`detonator-emp.md`, "What HD draws"), the Missile's head (`weapons.md`, "The Missile's red
/// rings"), the ship explosion (`ship-destroy.md`, "`colour_mode` 3 is a blend toward the
/// scene light"), the two trail hits (`engine-trail.md`, "The red variant's colour is in its
/// sprites") and the Plasma's five (`plasma.md`, "The pairs"). The Detonator Mine and Bomb
/// explosions' sheets were seen walking in the original (`particle-render.md`, "The atlas
/// frame"), their picture not paired. **Every other entry is unpaired**: it draws the sprite
/// its own emitter names, and no capture of the original has judged the result.
const SPRITES: &[&str] = &[
    engine_effects::DETONATOR_CANNON_SPARKS_EFFECT,
    engine_effects::TRACK_BLAST_EFFECT,
    engine_effects::CRAFT_BLAST_EFFECT,
    engine_effects::EXPLOSION_EFFECT,
    engine_effects::QUAKE_EFFECT,
    engine_effects::DETONATOR_EMP_EFFECT,
    engine_effects::MISSILE_FLARE_EFFECT,
    engine_effects::TRAIL_HITSHIP_EFFECT,
    engine_effects::TRAIL_HITSHIP_RED_EFFECT,
    engine_effects::PLASMA_FLARE_EFFECT,
    engine_effects::PLASMA_CHARGING_EFFECT,
    engine_effects::PLASMA_LAUNCH_EFFECT,
    engine_effects::PLASMA_LIGHTNING_EXPAND_EFFECT,
    engine_effects::PLASMA_LIGHTNING_COLLAPSE_EFFECT,
    engine_effects::MINE_EXPLO_EFFECT,
    engine_effects::BOMB_SMOKERING_EFFECT,
    engine_effects::MISSILE_EXPLO_EFFECT,
    engine_effects::MISSILE_BOUNCE_EFFECT,
    engine_effects::DETONATOR_MINE_EXPLO_DEAD_EFFECT,
    engine_effects::DETONATOR_MINE_EXPLO_EFFECT,
    engine_effects::DETONATOR_BOMB_EXPLO_EFFECT,
    engine_effects::ROCKET_FLARE_EFFECT,
    engine_effects::CANNON_SPARKS_EFFECT,
    engine_effects::SHURIKEN_FLARE_EFFECT,
    engine_effects::SHURIKEN_TRAIL_EFFECT,
    engine_effects::SHURIKEN_BOUNCE_EFFECT,
    engine_effects::SHURIKEN_EXPIRE_EFFECT,
    engine_effects::REPULSER_BLAST_EFFECT,
    engine_effects::ABSORB_EFFECT,
    engine_effects::COLLISION_SPARK_EFFECT,
    engine_effects::DEATH_PLUME_EFFECT,
    engine_effects::DEBRIS_FIRE_EFFECT,
    engine_effects::DAMAGE_MILD_EFFECT,
    engine_effects::DAMAGE_MODERATE_EFFECT,
    engine_effects::MODESTO_STEAM_EFFECT,
];

/// The effects whose emitters spread by the extent law on HD: the Quake's wave,
/// which the original draws as a band of fire across the whole road
/// (`docs/ghidra/functions/ps3-hdfury-eu/quake.md`), the Detonator EMP's, a band of blue smoke
/// across it (`detonator-emp.md`), and the Plasma blast's two,
/// whose lightning fills the blast instead of bunching at its centre (`plasma.md`,
/// "The pairs"), and the Rocket's craft blast, whose `FIRE_TRAILS_ONEQUAD` heads start on
/// their 8.8-unit hemisphere, so their smoke trails fan out around the blast instead of
/// piling up in front of the camera (`docs/rendering/particle-fill.md`, "The trails start on
/// the blast's hemisphere"). Every other HD effect keeps its emitters at their anchors until a
/// matched capture says otherwise.
const EXTENTS: &[&str] = &[
    engine_effects::QUAKE_EFFECT,
    engine_effects::DETONATOR_EMP_EFFECT,
    engine_effects::PLASMA_LIGHTNING_EXPAND_EFFECT,
    engine_effects::PLASMA_LIGHTNING_COLLAPSE_EFFECT,
    engine_effects::CRAFT_BLAST_EFFECT,
];

/// The activation and hit colours are read off HD's executable; the settled
/// target is each team's own authored `ShieldColour`
/// (`docs/ghidra/functions/ps3-hdfury-eu/shield.md`).
const SHIELD: ShieldPalettes = ShieldPalettes {
    ps2: ShieldPalette::Hd,
    elsewhere: ShieldPalette::Hd,
    origin: Origin::Measured,
};

/// HD's race looks.
pub const LOOKS: &Looks = &Looks {
    // `Ship_SelectWreckModel` (`0x000db400`) in `Ship_SetState` case 4;
    // `docs/ghidra/functions/ps3-hdfury-eu/ship-destroy.md`, row 1.
    hull_wreck: Rule {
        on: Platforms::Any,
        origin: Origin::Measured,
    },
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
    // The scale field of the final instruction of every lit program drawn
    // into the scene target (draws 87-522 of 640 on Talon's Junction) reads
    // `/2`; the sky's six draws and the late additive draws carry none. The
    // road program evaluated on the live textures lands on the live scene
    // target at 1.03 with it and 2.0 without.
    // `docs/ghidra/functions/ps3-hdfury-eu/renderer.md`, "Every lit program
    // ends on a /2 output scale".
    lit_output_halved: Rule {
        on: Platforms::Any,
        origin: Origin::Measured,
    },
    // The RSX texture filter word `0x02063e80` on 2,619 + 3,935 binds of Anulpha
    // Pass and The Amphiseum is trilinear with a `-1.5` bias: the request is read
    // off the original. Applied everywhere on the maintainer's decision
    // (2026-10-27, after comparing full frames at -0.75 and -1.5): the game's own
    // value, not the -0.75 fit to RPCS3, which closed the near fine-band gap to
    // 1.00 where -1.5 reads 1.11. Held off before this because the stored RPCS3
    // frames' mean grey rose; that is added detail (`docs/rendering/hd-track-tone.md`,
    // "Texture crispness"). The number is the filter word's, the choice of it over
    // the RPCS3 fit is the maintainer's.
    material_lod_bias: Rule {
        on: Platforms::Any,
        origin: Origin::Measured,
    },
    // State bit 7 is the alpha-to-coverage path (blend and alpha test off,
    // depth written): all 31 `cf_tree` draws (`0xbd`) of a live Moa Therma race
    // frame are issued that way. Bit 2 on a blended state writes depth: the
    // same frame's `0x2d` and `0x3d` draws blend with the depth mask on. `docs/ghidra/functions/ps3-hdfury-eu/material-state.md`,
    // "The race pass sets pass bits 3, 5 and 6". How this renderer stands in for
    // coverage on one sample is chosen (`oag_mesh::mesh::render_state`).
    material_render_state: Rule {
        on: Platforms::Any,
        origin: Origin::Measured,
    },
    // `SET_SHADER_PACKER` is 1 on every scene draw of three live race frames
    // (Talon's Junction, a hull frame, a gantry frame), so the packer encodes a
    // draw before the blend and blends combine encoded values; an additive
    // grill's lift on the cold `hd/talons` pair is grey and uncorrelated with
    // what is behind it. `docs/rendering/hd-track-tone.md`, "HD's blends combine
    // encoded values", confidence 85.
    blends_encoded: Rule {
        on: Platforms::Any,
        origin: Origin::Measured,
    },
    // The last full-screen quad of a frame runs fragment program `0x01113c41` with unit 0
    // filter `0x02022000` and TEXCOORD0 at `(0.5 / width, 0.5 / height)` of the scaled
    // source: 23 of 23 1080p frames, 8 of them races at widths 1300, 1320 and 1660.
    // `docs/ghidra/functions/ps3-hdfury-eu/menu-backdrop.md`, "A race's final composite
    // is the same quad", confidence 90.
    final_composite_half_texel: Rule {
        on: Platforms::Any,
        origin: Origin::Measured,
    },
    // The behind-the-glass target (640 x 360, Vineta K) and the planar sea reflection
    // target (960 x 540, Moa Therma, Ubermall) are surfaces the race draws ahead of the main
    // view, read live on RPCS3: `docs/rendering/water.md`, "The planar reflection", and
    // `docs/formats/rcsmaterial.md`, "The behind-the-glass target is drawn".
    screen_space_target: Rule {
        on: Platforms::Any,
        origin: Origin::Measured,
    },
    // The launch boost: HD authors `<StartBoost>`; the grading law is Pulse PSP's, unmeasured on
    // HD (`docs/physics/launch-boost.md`).
    start_boost: Rule {
        on: Platforms::Any,
        origin: Origin::InheritedFrom("Wipeout Pulse (PSP)"),
    },
    ..Looks::unread(SHIELD)
};

#[cfg(test)]
mod tests {
    use super::*;

    /// HD's lit programs end on a `/2` output scale and no other title's
    /// race looks claim it; Pulse's rig is the GE's and stays at one.
    #[test]
    fn hd_halves_its_lit_output_and_pulse_does_not() {
        assert!(LOOKS.lit_output_halved.applies_everywhere());
        assert_eq!(LOOKS.lit_output_halved.origin, Origin::Measured);
        assert!(!Looks::unread(SHIELD).lit_output_halved.applies_everywhere());
    }

    /// HD honours its circuit materials' render-state word (bit 7 draws
    /// among the cutouts, measured off a live race frame); a title with no
    /// reading of its own does not.
    #[test]
    fn hd_honours_the_material_render_state_and_an_unread_title_does_not() {
        assert!(LOOKS.material_render_state.applies_everywhere());
        assert_eq!(LOOKS.material_render_state.origin, Origin::Measured);
        assert!(
            !Looks::unread(SHIELD)
                .material_render_state
                .applies_everywhere()
        );
    }

    /// HD's final composite reads half a source texel off; an unread title's does not.
    #[test]
    fn hd_reads_its_final_composite_half_a_texel_off_and_an_unread_title_does_not() {
        assert!(LOOKS.final_composite_half_texel.applies_everywhere());
        assert_eq!(LOOKS.final_composite_half_texel.origin, Origin::Measured);
        assert!(
            !Looks::unread(SHIELD)
                .final_composite_half_texel
                .applies_everywhere()
        );
    }

    /// HD's scene stores encoded values, measured live; an unread title's
    /// keeps the value as computed.
    #[test]
    fn hd_blends_encoded_and_an_unread_title_does_not() {
        assert!(LOOKS.blends_encoded.applies_everywhere());
        assert_eq!(LOOKS.blends_encoded.origin, Origin::Measured);
        assert!(!Looks::unread(SHIELD).blends_encoded.applies_everywhere());
    }

    /// HD samples its circuit materials with the filter word's `-1.5` bias
    /// (the maintainer chose it over the -0.75 RPCS3 fit); a title with no
    /// reading of its own gets the sampler's own selection.
    #[test]
    fn hd_applies_the_material_lod_bias_and_an_unread_title_does_not() {
        assert!(LOOKS.material_lod_bias.applies_everywhere());
        assert_eq!(LOOKS.material_lod_levels(), -1.5);
        assert_eq!(LOOKS.material_lod_levels(), oag_title::MATERIAL_LOD_BIAS);
        assert_eq!(oag_title::MATERIAL_LOD_BIAS, -384.0 / 256.0);
        assert_eq!(Looks::unread(SHIELD).material_lod_levels(), 0.0);
    }

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

    /// The disc ships `wo_ship_engineflare.pob`, but no string or call of
    /// `EBOOT.elf` plays it, and the original's nozzle at rest shows none of
    /// its glare (three RPCS3 frames, `docs/ghidra/functions/ps3-hdfury-eu/engine-trail.md`,
    /// "The engine flare is not the Pulse particle effect"). HD's flare is the
    /// `engineflare.rcsmodel` shapes and the sprite, so the trigger the PS2
    /// port's engine table carries is not asked for.
    #[test]
    fn hd_does_not_play_the_ps2_engine_flare_particle() {
        assert!(EFFECTS.on(Trigger::EngineFlare).is_none());
        assert!(
            !EFFECTS
                .names()
                .contains(&engine_effects::ENGINE_FLARE_EFFECT)
        );
    }
}
