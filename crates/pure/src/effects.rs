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
        .without(Trigger::LeachbeamAbsorb);
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
