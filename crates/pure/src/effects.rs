//! What Wipeout Pure throws and draws in a race: the `oag_title` tables the
//! raceplay loader reads instead of comparing a title's name (ADR-0058).
//!
//! Deliberately thin, like the rest of this crate: Pure's absorb burst
//! (`FUN_08925e20`) and its Quake wave (`Quake_Update`, `0x08879d54`) are the
//! two effect laws read. The engine's own names are Pulse's by inheritance
//! (Pure's loader tries each; the LeachBeam's are removed, Pure having none). A title package does not depend on another outside
//! `[dev-dependencies]`, so the one Pulse rule Pure takes, the shield tint, is
//! restated here and labelled as inherited.

use oag_title::{
    Burst, EffectSpec, Effects, Looks, Origin, Platforms, QuakeLook, QuakeUp, Rule, ShieldPalette,
    ShieldPalettes, Trigger, engine_effects,
};

/// Pure's `FUN_08925e20`: the same loop over the same `Ship Collision Fx` class
/// as Pulse's, eight nodes instead of ten, read off its own disassembly -
/// `docs/ghidra/functions/psp-pure-usa/rocket-and-collision-fx.md`.
pub const ABSORB_BURST: Burst = Burst::Sequential {
    cap: 8,
    stagger: 0.1,
};

/// Pure's tables. Hit sparks and the wreck are unread (`Ship_Damage` is not),
/// and Pure's Cannon throws no weapon spark of its own that has been read. The
/// engine's own names are Pulse's, taken by inheritance: Pure's loader tries
/// each and a disc without it costs one report line.
pub const EFFECTS: &Effects = &{
    let mut effects = Effects::engine(Origin::InheritedFrom("Wipeout Pulse"))
        .with(
            Trigger::ShieldAbsorb,
            EffectSpec::new(engine_effects::ABSORB_EFFECT, Origin::Measured)
                .with_burst(ABSORB_BURST),
        )
        // Pure ships no LeachBeam: its `weaponstats.xml` authors none, neither
        // pressing's executable names one and none of Pulse's six LeachBeam
        // entries is in its `Data.wad`
        // (`docs/ghidra/functions/psp-pure-usa/weapons.md`).
        .without(Trigger::LeachbeamEnergy)
        .without(Trigger::LeachbeamCharging)
        .without(Trigger::LeachbeamAbsorb)
        // Names the engine table carries from Pulse, HD and 2048 that Pure's
        // executable never requests: its whole `WO_*` list is 24 effect names
        // (`0x08a47f50`..`0x08a7a49c`, `search_strings` over
        // `/pure/BOOT-psp-pure-usa.BIN`; `docs/formats/pure-status.md`) and none
        // of these is among them, nor in its `Data.wad`, so each load reported
        // them absent.
        .without(Trigger::PlasmaLightningExpand)
        .without(Trigger::PlasmaLightningCollapse)
        .without(Trigger::ShurikenFlare)
        .without(Trigger::ShurikenTrail)
        .without(Trigger::ShurikenBounce)
        .without(Trigger::ShurikenExpire)
        .without(Trigger::TrackBlast)
        .without(Trigger::CannonSparks)
        .without(Trigger::EngineFlare)
        .without(Trigger::TrailHitship)
        .without(Trigger::TrailHitshipRed)
        .without(Trigger::RepulserBlast)
        .without(Trigger::Repulser)
        .without(Trigger::MagstripSparks)
        .without(Trigger::MagstripZone);
    effects.extents = EXTENTS;
    effects.quake_look = QUAKE_LOOK;
    effects
};

/// The effects whose emitters spread by the extent law on Pure: the Quake's
/// wave. `Quake_Update` (`0x08879d54`) writes `edge distance / 50` into slot 1
/// of the seven-word block `0x0893be60` reads out of the instance and
/// `0x0893bf08` writes back, the same instance `+0x2c` Pulse's `0x088f443c` and
/// `0x088f44d8` do, and Pure's own `WO_QUAKE.POB` authors three shape-1 line
/// emitters of extent `50`
/// (`docs/ghidra/functions/psp-pure-usa/quake.md`).
const EXTENTS: &[&str] = &[engine_effects::QUAKE_EFFECT];

/// Pure's Quake: world up, the anchor `10` units above the road, and a wash.
/// `Quake_Update` loads `(0, 1, 0)` for the frame's `Y` instead of looking up
/// the track, and in the ordinary (non-VR) build adds
/// `10 * ((across x Y) x across)` to the midpoint; Pulse does neither. Every
/// frame it also calls `ScreenFlash_Start` (`0x0882e648`) with kind 3 at the
/// span's first edge point, whose case (`0x0882eac4`) writes the numbers of
/// Pulse's kind 4 - `0.4` s, near `0`, far `300`, additive, `(1, 0.2, 0, 0.5)`
/// fading to alpha `0` - so `oag_fx::flash::QUAKE` is its table. The
/// consumer that turns those numbers into a tint is unread on Pure: its law is
/// Pulse's, inherited, and the capture shows the tint present
/// (`docs/ghidra/functions/psp-pure-usa/quake.md`, confidence 85 for the
/// frame and the kind's numbers).
const QUAKE_LOOK: QuakeLook = QuakeLook {
    up: QuakeUp::World,
    lift: 10.0,
    flash: true,
    origin: Origin::Measured,
};

/// Pure's race looks: none read. The shield tint is Pulse's, as it has always
/// been for a title with no palette of its own - the project's rule that an
/// unmeasured title runs Pulse's (`docs/formats/pure-status.md` measures none
/// of the shell).
pub const LOOKS: &Looks = &Looks {
    // The Mine's pose is Pulse's code: `Mine_Construct` (`0x0885bab4`) writes
    // the same `0.6` scale, `Mine_Init` (`0x0885bd0c`) rolls the same random
    // axis, and its pose node (`0x0885bf4c`) turns it by `4 x` the fuse.
    // Read off the decompile, not yet pinned to live matrices.
    // Pure lays both charges at the rear anchor, not at the body: see
    // `oag_raceplay::weapons::REAR_ANCHOR_BACK`.
    laid_from_rear: Rule {
        on: Platforms::Any,
        origin: Origin::Measured,
    },
    laid_pose_scaled: Rule {
        on: Platforms::Any,
        origin: Origin::InheritedFrom("Wipeout Pulse"),
    },
    // The hull's GE light list is the circuit's own `AmbientLight` and
    // `DirectionalLight` nodes, as on Pulse: Pure's builder `FUN_08841648` is
    // Pulse's `SceneLight_BuildLightingList` read instruction for instruction
    // (the context's light list at `+0x6d0` where Pulse's is at `+0x6cc`), and
    // every Pure circuit authors the nodes under its own class ids (`0x125`,
    // `0x12a`). Turned on, the four compare pairs' teal body moves with the
    // circuit as the original's does
    // (`docs/ghidra/functions/psp-pure-usa/scene-light.md`).
    hull_lights: Rule {
        on: Platforms::Only(&[oag_title::Platform::Psp]),
        origin: Origin::Measured,
    },
    // The sun's lens flare, read off `FxLensflare` and measured live on PPSSPP
    // (`docs/rendering/lens-flare.md`): drawn where the circuit authors a node.
    lens_flare: Rule {
        on: Platforms::Only(&[oag_title::Platform::Psp]),
        origin: Origin::Measured,
    },
    // The hull's `0x2000` extra pass, as on Pulse: Feisar's `feisarShape` mesh
    // word and batches read `0x3001`, its materials name `envtest4.tga` as the
    // second texture, and Pure's draw (`FUN_0886f9ec`) brackets the pass in
    // `FUN_0886e144`, which programs what Pulse's `Mesh_BeginTransparentPass`
    // does call for call (`Gu_TexMapMode(2, 0, 1)` included). Its fixed light
    // pair is built from the same `-1.0`/`0.3` radian literals
    // (`0x08a8e8ec`) in `FUN_08864018`
    // (`docs/ghidra/functions/psp-pure-usa/hull-shine.md`).
    hull_shine: Rule {
        on: Platforms::Only(&[oag_title::Platform::Psp]),
        origin: Origin::Measured,
    },
    // The circuits' chrome-map pass: the four recorded GE lists carry 57, 36, 31 and 28
    // `TEXMAPMODE` 2 PRIMs off the craft, each batch's ordinary pass weighted by its own
    // texture's alpha like the hull's
    // (`docs/ghidra/functions/psp-pure-usa/hull-shine.md`, "The circuits").
    track_shine: Rule {
        on: Platforms::Only(&[oag_title::Platform::Psp]),
        origin: Origin::Measured,
    },
    track_shine_weighted: Rule {
        on: Platforms::Only(&[oag_title::Platform::Psp]),
        origin: Origin::Measured,
    },
    // Pure runs Pulse's bloom class over Pulse's stencil mask: its Modesto GE list draws the
    // bright pass, the 1,320-vertex blur pair (weights 20/30/40/50/64/64/64/50/40/30/20) and the
    // `FIXA 0xaf` composite, and stamps ref 4 on ordinary batches and the texture's glow byte
    // on the `_GLOW` ones (`docs/ghidra/functions/psp-pure-usa/bloom.md`). Its composite is
    // drawn before the first HUD sprite, so the HUD takes none of the haze.
    psp_glow_mask: Rule {
        on: Platforms::Only(&[oag_title::Platform::Psp]),
        origin: Origin::Measured,
    },
    bloom_under_hud: Rule {
        on: Platforms::Only(&[oag_title::Platform::Psp]),
        origin: Origin::Measured,
    },
    ..Looks::unread(ShieldPalettes {
        ps2: ShieldPalette::Ps2Pulse,
        elsewhere: ShieldPalette::Pulse,
        origin: Origin::InheritedFrom("Wipeout Pulse"),
    })
};

#[cfg(test)]
mod tests {
    use super::*;

    /// Pure has no LeachBeam, so its table names none of the beam's effects:
    /// dropping a `.without` above brings an inherited Pulse name back, and a
    /// load report line for an entry no Pure disc carries.
    #[test]
    fn pure_names_no_leachbeam_effect() {
        for trigger in [
            Trigger::LeachbeamEnergy,
            Trigger::LeachbeamCharging,
            Trigger::LeachbeamAbsorb,
        ] {
            assert!(EFFECTS.on(trigger).is_none(), "{trigger:?}");
        }
    }

    /// None of these names is in Pure's executable's `WO_*` list: dropping a
    /// `.without` above brings an inherited engine name back, and with it a
    /// load report line for an effect no Pure disc carries.
    #[test]
    fn pure_names_no_effect_its_executable_does_not() {
        for trigger in [
            Trigger::PlasmaLightningExpand,
            Trigger::PlasmaLightningCollapse,
            Trigger::ShurikenFlare,
            Trigger::ShurikenTrail,
            Trigger::ShurikenBounce,
            Trigger::ShurikenExpire,
            Trigger::TrackBlast,
            Trigger::CannonSparks,
            Trigger::EngineFlare,
            Trigger::TrailHitship,
            Trigger::TrailHitshipRed,
            Trigger::RepulserBlast,
            Trigger::Repulser,
            Trigger::MagstripSparks,
            Trigger::MagstripZone,
        ] {
            assert!(EFFECTS.on(trigger).is_none(), "{trigger:?}");
        }
    }

    /// Pure's Quake: the extent law is on for `WO_QUAKE` and for nothing else,
    /// the frame takes world up with the anchor lifted `10` units, and the
    /// wave washes the screen. Each is read off `Quake_Update` (`0x08879d54`);
    /// dropping one is a different picture (a point blob at the midpoint, a
    /// track-up frame, no tint).
    #[test]
    fn pures_quake_spreads_by_the_extent_law_world_up_and_washes() {
        assert_eq!(EFFECTS.extents, &[engine_effects::QUAKE_EFFECT]);
        let look = EFFECTS.quake_look;
        assert_eq!(look.up, QuakeUp::World);
        assert_eq!(look.lift, 10.0);
        assert!(look.flash);
        assert_eq!(look.origin, Origin::Measured);
    }
}
