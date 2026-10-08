//! [`Cue`]'s lookup tables: every cue, its bank, bus, name, whether it is held
//! or repeats, its radius and where it is heard from. Split out of [`super`]
//! under the 1,000-line rule; the evidence is on each variant's doc, in the parent.

use oag_audio::Bus;

use super::Cue;
use crate::sfx::{BankName, Placement};

impl Cue {
    /// The two start-of-race cues, loaded on their own because their bank is the
    /// mode's, not the title's. See [`Self::Ready`].
    pub const COUNTDOWN: [Self; 3] = [Self::Ready, Self::Go, Self::ContElim];

    /// Every cue this port fires, which is every one it knows how to load.
    /// The navigation sounds the front end plays, loaded by
    /// [`crate::sfx::MenuSfx`] from the title's front-end bank and nothing else.
    ///
    /// Not in [`Self::ALL`]: a race does not read that bank. [`Self::MenuTeletype`]
    /// is left out because its trigger's extra argument is unread.
    pub const FRONT_END: [Self; 8] = [
        Self::MenuUp,
        Self::MenuDown,
        Self::MenuLeft,
        Self::MenuRight,
        Self::MenuStepLeft,
        Self::MenuStepRight,
        Self::MenuAccept,
        Self::MenuDecline,
    ];

    /// The cue name this front-end role plays in the title's bank, in the
    /// style the front end is in, or [`None`] for a cue that is not one.
    #[must_use]
    pub fn front_end_name(self, cues: &oag_title::MenuCues, fury: bool) -> Option<&'static str> {
        Some(match self {
            Self::MenuUp => cues.up,
            Self::MenuDown => cues.down,
            Self::MenuLeft => cues.left,
            Self::MenuRight => cues.right,
            Self::MenuStepLeft => cues.step_left,
            Self::MenuStepRight => cues.step_right,
            Self::MenuAccept => cues.accept.name(fury),
            Self::MenuDecline => cues.decline.name(fury),
            _ => return None,
        })
    }

    pub const ALL: [Self; 43] = [
        Self::SpeedupPad,
        Self::Message,
        Self::Turbo,
        Self::Collision,
        Self::Absorb,
        Self::Engine,
        Self::Shield,
        Self::ShieldActive,
        Self::Autopilot,
        Self::Engaging,
        Self::Disengaging,
        Self::Blowup,
        Self::LockOn,
        Self::MineLaunch,
        Self::Plasma,
        Self::PlasmaTravel,
        Self::PlasmaHitWall,
        Self::PlasmaHitShip,
        Self::Rocket,
        Self::RocketTravel,
        Self::RocketHitWall,
        Self::RocketHitShip,
        Self::Missile,
        Self::MissileTravel,
        Self::MissileHitWall,
        Self::MissileHitShip,
        Self::MissileExpire,
        Self::Cannon,
        Self::CannonHitWall,
        Self::CannonHitShip,
        Self::QuakeLaunch,
        Self::QuakeHit,
        Self::QuakeTravel,
        Self::Leach,
        Self::LeachFail,
        Self::LeachAttach,
        Self::LeachEnergy,
        Self::ShurikenLaunch,
        Self::ShurikenHit,
        Self::ShurikenTravel,
        Self::Repulsor,
        Self::RepulsorHit,
        Self::Magstrip,
    ];

    /// The bank the cue is looked up in.
    #[must_use]
    pub fn bank(self) -> BankName {
        match self {
            Self::SpeedupPad | Self::Message | Self::Blowup | Self::LockOn | Self::Autopilot => {
                BankName::Hud
            }
            Self::Collision | Self::Engine | Self::Magstrip | Self::MagstripStop => BankName::Ship,
            Self::Absorb
            | Self::Shield
            | Self::MineLaunch
            | Self::Plasma
            | Self::PlasmaTravel
            | Self::PlasmaHitWall
            | Self::PlasmaHitShip
            | Self::Rocket
            | Self::RocketTravel
            | Self::RocketHitWall
            | Self::RocketHitShip
            | Self::Missile
            | Self::MissileTravel
            | Self::MissileHitWall
            | Self::MissileHitShip
            | Self::MissileExpire
            | Self::Cannon
            | Self::CannonHitWall
            | Self::CannonHitShip
            | Self::QuakeLaunch
            | Self::QuakeHit
            | Self::QuakeTravel
            | Self::Leach
            | Self::LeachFail
            | Self::LeachAttach
            | Self::LeachEnergy
            | Self::ShurikenLaunch
            | Self::ShurikenHit
            | Self::ShurikenTravel
            | Self::Repulsor
            | Self::RepulsorHit
            | Self::Turbo => BankName::Weapons,
            Self::ShieldActive
            | Self::Engaging
            | Self::Disengaging
            | Self::Ready
            | Self::Go
            | Self::ContElim => BankName::Speech,
            Self::MenuUp
            | Self::MenuDown
            | Self::MenuLeft
            | Self::MenuRight
            | Self::MenuStepLeft
            | Self::MenuStepRight
            | Self::MenuAccept
            | Self::MenuDecline
            | Self::MenuTeletype => BankName::Frontend,
        }
    }

    /// Which mix bus this cue's voice belongs on.
    ///
    /// Read straight off [`Self::bank`], where the original draws the line:
    /// `shieldactive` sits in `speech.bnk`, not `weapons.bnk` beside `~SHIELD`,
    /// making it a voice line ([`Self::ShieldActive`]); `Disengaging` agrees,
    /// `Autopilot_Update` playing it through the dry, full-volume path. A cue
    /// moves bus by moving bank, with no second list to disagree. See
    /// [ADR-0027](../../../../../docs/architecture/adr/0027-three-mix-buses.md).
    #[must_use]
    pub fn bus(self) -> Bus {
        match self.bank() {
            BankName::Speech => Bus::Speech,
            BankName::Hud | BankName::Ship | BankName::Weapons | BankName::Frontend => Bus::Sfx,
        }
    }

    /// The string to look up in that bank's name table.
    ///
    /// # The dot in `.COLLISIONS` is the executable's own, not a lookup fix-up
    ///
    /// Correction, 2026-09-04: this once read the executable as passing bare
    /// `"COLLISIONS"` and claimed the dot was added here to bridge a SCREAM
    /// sound/child-sound naming split (confidence 70). That misread the call
    /// site: `ShipCollisionFx_Trigger`'s string argument is loaded from a pointer
    /// table (`0x08924854: lw a3,0x46ec(a2)`), and the decompiler's inline
    /// literal showed the slot's apparent text, not the pointee. The pointer's
    /// target (`0x08a886e0`) is `.COLLISIONS\0`, dot included. See
    /// `docs/ghidra/functions/psp-pulse-usa/contact-response.md`.
    ///
    /// So this is a plain exact match, the only cue named `.COLLISIONS` on any
    /// disc. Wipeout HD's bank confirms the dot is a naming convention, not a
    /// parent/child marker: `.COLLISIONS` is the parent, playing the plainly
    /// named `c_CShipShip` and `c_CShipWall` (`oag_formats::sblk::child`).
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::SpeedupPad => "SPEEDUPPAD",
            Self::Message => "MESSAGE",
            Self::Turbo => "TURBO",
            Self::Collision => ".COLLISIONS",
            Self::Absorb => "ABSORB",
            Self::Engine => "~ENGINE",
            Self::Shield => "~SHIELD",
            Self::ShieldActive => "shieldactive",
            Self::Autopilot => "~AUTOPILOT",
            Self::Engaging => "autopilot_eng",
            Self::Disengaging => "disengaging",
            Self::Ready => "ready",
            Self::Go => "go",
            Self::ContElim => "cont_elim",
            Self::Blowup => "~BLOWUP",
            Self::LockOn => "~ROCKLOCK",
            Self::MineLaunch => "MINELAUNCH",
            Self::Plasma => "PLASMA",
            Self::PlasmaTravel => "~PLASMATVL",
            Self::PlasmaHitWall => "PLASMAHITWALL",
            Self::PlasmaHitShip => "PLASMAHITSHIP",
            Self::Rocket => "ROCKET",
            Self::RocketTravel => "~ROCKETTVL",
            Self::RocketHitWall => "ROCKEXPLWALL",
            Self::RocketHitShip => "ROCKEXPLSHIP",
            Self::Missile => "MISSILE",
            Self::MissileTravel => "~MISSILETVL",
            Self::MissileHitWall => "MISSILEEXPWALL",
            Self::MissileHitShip => "MISSILEEXPSHIP",
            Self::MissileExpire => "SHURIKENEXPL",
            Self::Cannon => "CANNON",
            Self::CannonHitWall => "CANNONEXPLWALL",
            Self::CannonHitShip => "CANNONEXPLSHIP",
            Self::QuakeLaunch => "QUAKELAUNCH",
            Self::QuakeHit => "QUAKEHIT",
            Self::QuakeTravel => "~QUAKETRAVEL",
            Self::Leach => "LEACH",
            Self::LeachFail => "LEACHFAIL",
            Self::LeachAttach => "~LEACHATTACH",
            Self::LeachEnergy => "LEACHENERGY",
            Self::ShurikenLaunch => "SHURIKEN",
            Self::ShurikenHit => "SHURIKENHIT",
            Self::ShurikenTravel => "~SHURIKENTRAVEL",
            Self::Repulsor => "REPULSOR",
            Self::RepulsorHit => "REPULSORHIT",
            Self::Magstrip | Self::MagstripStop => "~magstrip01",
            Self::MenuUp | Self::MenuDown | Self::MenuLeft | Self::MenuRight => "UPDOWN",
            Self::MenuStepLeft | Self::MenuStepRight => "LEFTRIGHT",
            Self::MenuAccept => "ACCEPT",
            Self::MenuDecline => "DECLINE",
            Self::MenuTeletype => "TELETYPE",
        }
    }

    /// Whether this port keeps a handle to the voice rather than firing and
    /// forgetting it.
    ///
    /// Both are cues the original opens with `Sound_PlayLooping` and an
    /// out-parameter (the `~` prefix, see
    /// `docs/architecture/adr/0018-audio-mixer-architecture.md`). The `~` is not
    /// the test: `~SPARKS` and `~FLYBY_DIST` carry it and are cancellable
    /// one-shots, so a held cue is one whose holder is written here.
    ///
    /// [`Self::PlasmaTravel`], [`Self::RocketTravel`], [`Self::MissileTravel`]
    /// and [`Self::ShurikenTravel`] are held per projectile slot
    /// ([`super::travel::TravelVoices`]); [`Self::LeachAttach`] once per race on
    /// [`super::SfxVoices::leach_attach`] (a world-wide single instance);
    /// [`Self::Autopilot`] as `Blowup` is, a level-driven handle
    /// (`ships[0].autopilot_timer > 0.0`). None of the six is pushed through the
    /// cue queue; this guards against a stray push being mis-played.
    #[must_use]
    pub fn held(self) -> bool {
        matches!(
            self,
            Self::Engine
                | Self::Shield
                | Self::Blowup
                | Self::LockOn
                | Self::Autopilot
                | Self::PlasmaTravel
                | Self::RocketTravel
                | Self::MissileTravel
                | Self::QuakeTravel
                | Self::LeachAttach
                | Self::ShurikenTravel
                | Self::Magstrip
                | Self::MagstripStop
        )
    }

    /// Whether this cue's list repeats while held, so it plays as the list runs
    /// rather than as one timeline or one picked waveform: `~BLOWUP`
    /// (`[key-on loop, 0x15, key-on, 0x1a, 0x16]`, the second waveform re-keyed
    /// every 43 master ticks) and `~ROCKLOCK` (a guard on cue parameter 0 before
    /// each of two key-ons, in a `0x15`/`0x16` loop). See `oag_formats::sblk::runner`.
    #[must_use]
    pub fn repeats(self) -> bool {
        matches!(self, Self::Blowup | Self::LockOn)
    }

    /// How far this cue's voice can be heard, for a cue carrying its own emitter
    /// rather than riding a craft's.
    ///
    /// Read only by [`super::place`]'s [`Placement::Point`] arm and the bespoke
    /// held-voice trackers ([`super::travel::TravelVoices`],
    /// [`super::SfxVoices::leach_attach`]); every [`Placement::Craft`] cue keeps
    /// [`oag_audio::Emitter::CRAFT_RADIUS`], `SoundEmitter_Init`'s default, unless
    /// its call site is read overriding it. Measured `600.0` for the Rocket
    /// (`Rocket_Init` writes `+0x38`) and the LeachBeam's body
    /// (`LeachBeam_InitLocked`'s `+0x4c` emitter); chosen `600.0` for the Missile,
    /// reusing the Rocket's figure ([`Self::MissileTravel`]); the Cannon's hit
    /// cues keep the default; measured `300.0` for [`Self::LeachEnergy`], half the
    /// body's, from its own call site.
    #[must_use]
    pub fn radius(self) -> f32 {
        match self {
            Self::RocketTravel
            | Self::RocketHitWall
            | Self::RocketHitShip
            | Self::MissileTravel
            | Self::MissileHitWall
            | Self::MissileHitShip
            | Self::MissileExpire
            | Self::QuakeTravel
            | Self::LeachAttach => 600.0,
            // Measured: `Shuriken_Init` and `Shuriken_Bounce` write `0x43960000` to
            // the blade's emitter `+0x38`.
            Self::ShurikenTravel | Self::ShurikenHit => 300.0,
            // `Repulser_HitCraft` writes `0x43960000` to the Repulser's emitter `+0x38`.
            Self::RepulsorHit => 300.0,
            // Measured, not reused from `LeachAttach`'s `600.0`: `LeachBeam_Advance`'s
            // pulse block writes its own emitter at `300.0`
            // (`docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md`,
            // "Assets and cues - checked, not assumed").
            Self::LeachEnergy => 300.0,
            _ => oag_audio::Emitter::CRAFT_RADIUS,
        }
    }

    /// Which emitter the original plays this cue on: each call site's first
    /// argument to `Sound_Play`, the emitter record
    /// ([`positional-audio.md`](../../../../../docs/ghidra/functions/psp-pulse-usa/positional-audio.md)).
    /// An unread call site stays [`Placement::Unplaced`], not somewhere plausible.
    #[must_use]
    pub fn placement(self) -> Placement {
        match self {
            // `lw a0, 0x50(a0)` at `0x08924844` in `ShipCollisionFx_Trigger`: the
            // craft's emitter, unbranched, so the player's hull is positional too.
            Self::Collision => Placement::Craft,
            // `FUN_08840640`'s emitter argument was not disassembled, so this
            // rides `Collision`'s reading of the contact path.
            Self::Absorb => Placement::Craft,
            // `Sound_PlayLooping(1.0, entity->0x50, ...)`, the `+0x50` the
            // collision path uses.
            Self::Shield => Placement::Craft,
            // Two branches split on `racer+0x368`: `ExhaustFlare_OnSpeedupPad`
            // (`0x08904f74`) plays positionally off `craft+0x50` or dry at `0x400`
            // through `FUN_0883e9b0`. See [`Placement::CraftUnlessPlayer`].
            Self::SpeedupPad => Placement::CraftUnlessPlayer,
            // `ExhaustFlare_OnPerfectStart` (`0x08904fd4`): the same two-branch
            // dispatcher on `racer+0x368`, with `"TURBO"`.
            Self::Turbo => Placement::CraftUnlessPlayer,
            // `ExhaustFlare_Init` gives the note an emitter at a quarter of the
            // craft radius and [`Engine`] holds the voice, so never one-shot.
            Self::Engine => Placement::Engine,
            // `Sound_Play(entity, ..., "shieldactive", 0x400, 0)`: full volume,
            // pan zero, the non-emitter path. Left dry, as always. `Disengaging`
            // joins it on a stronger footing: `FUN_0883e9b0` -> `FUN_0893a768`,
            // which takes no emitter.
            Self::ShieldActive | Self::Disengaging => Placement::Unplaced,
            // `RaceMode_SetState` and `RaceMode_UpdateCountdown` call
            // `Sound_PlayNamedInSlot` with no emitter and `0x400`, as `Disengaging`.
            Self::Ready | Self::Go | Self::ContElim => Placement::Unplaced,
            // Read: case 4 hands it to the path with no emitter at `0x400`.
            Self::Blowup | Self::Message => Placement::Unplaced,
            // Both halves of `Ship_FireHeldWeapon`'s case 6 use `FUN_0883e9b0`, the
            // dry, no-emitter path of `Blowup` and `Disengaging`.
            Self::Autopilot | Self::Engaging => Placement::Unplaced,
            // `HudSight_UpdateTone` opens it at `0x400` with no emitter: dry, a HUD
            // sound about the player's own reticle.
            Self::LockOn => Placement::Unplaced,
            // Traces to the firing craft's `+0x50`: rides the craft, not the mine.
            Self::MineLaunch => Placement::Craft,
            // `Plasma_Init` plays this off the firing craft's emitter, before it
            // constructs the bolt's own.
            Self::Plasma => Placement::Craft,
            // All three ride the bolt's emitter, which `Plasma_Init` points at the
            // bolt's matrix. None goes through this function's `craft` slice:
            // [`Self::PlasmaTravel`] is a per-tick tracker by projectile slot and
            // [`Self::PlasmaHitWall`]/[`Self::PlasmaHitShip`] carry their own point
            // on the [`CueEvent`].
            Self::PlasmaTravel | Self::PlasmaHitWall | Self::PlasmaHitShip => Placement::Point,
            // As the Plasma's three, on the Rocket's `600.0`-radius emitter
            // ([`Self::radius`]).
            Self::RocketTravel | Self::RocketHitWall | Self::RocketHitShip => Placement::Point,
            // `Sound_Play(1.0, *(param_1+0x50), weapons.bnk, 0, "ROCKET", 0)`: a
            // positional call with an explicit emitter, as [`Self::Shield`], not
            // the dry path.
            Self::Rocket => Placement::Craft,
            // Chosen, not measured; see each variant's doc.
            Self::Missile | Self::Cannon => Placement::Craft,
            Self::MissileTravel
            | Self::MissileHitWall
            | Self::MissileHitShip
            | Self::MissileExpire => Placement::Point,
            Self::CannonHitWall | Self::CannonHitShip => Placement::Point,
            // Same `Ship_FireHeldWeapon` switch and positional shape as `Rocket`.
            Self::QuakeLaunch => Placement::Craft,
            // Settled by the surrounding block's shield gate and pending-damage
            // fields, all the struck craft's.
            Self::QuakeHit => Placement::Craft,
            // The wave's road midpoint, tracked per tick by
            // [`super::super::travel::TravelVoices`].
            Self::QuakeTravel => Placement::Point,
            // `LeachBeam_InitLocked` plays `LEACH` on the shooter's emitter.
            Self::Leach | Self::LeachFail => Placement::Craft,
            // A per-tick tracker on the beam's `600.0`-radius emitter, as
            // [`Self::PlasmaTravel`] ([`super::SfxVoices::leach_attach`],
            // [`Self::radius`]).
            Self::LeachAttach => Placement::Point,
            // Pushed with the ribbon's `energy_point` as a
            // [`super::CueEvent::at_point`]: the pulse block's re-spawn target
            // ([`Self::radius`]).
            Self::LeachEnergy => Placement::Point,
            // `SHURIKEN` rides the emitter `Shuriken_Init` is handed (the firing
            // craft's); the blade's two ride the emitter it allocates.
            Self::ShurikenLaunch => Placement::Craft,
            Self::ShurikenHit | Self::ShurikenTravel => Placement::Point,
            // `Repulser_Init` plays `REPULSOR` on the firer's emitter and
            // `Repulser_HitCraft` points the Repulser's at the struck craft's node.
            Self::Repulsor | Self::RepulsorHit => Placement::Craft,
            // Anchored to `ship+0x70c8 + 0x10`, the craft's transform.
            Self::Magstrip | Self::MagstripStop => Placement::Craft,
            // `Sound_PlayNamedInSlot` at `0x400` with no emitter: dry.
            Self::MenuUp
            | Self::MenuDown
            | Self::MenuLeft
            | Self::MenuRight
            | Self::MenuStepLeft
            | Self::MenuStepRight
            | Self::MenuAccept
            | Self::MenuDecline
            | Self::MenuTeletype => Placement::Unplaced,
        }
    }
}
