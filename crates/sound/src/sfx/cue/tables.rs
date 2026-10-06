//! [`Cue`]'s lookup tables: every cue, its bank, bus, name, whether it is held
//! or repeats, its radius and where it is heard from.
//!
//! Split out of [`super`] under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move with no behaviour change. The doc
//! comment on each variant, in the parent, is where the evidence is.

use oag_audio::Bus;

use super::Cue;
use crate::sfx::{BankName, Placement};

impl Cue {
    /// The two start-of-race cues, loaded on their own because their bank is
    /// the mode's and not the title's. See [`Self::Ready`].
    pub const COUNTDOWN: [Self; 3] = [Self::Ready, Self::Go, Self::ContElim];

    /// Every cue this port fires, which is every one it knows how to load.
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
        }
    }

    /// Which mix bus this cue's voice belongs on.
    ///
    /// **Read straight off [`Self::bank`], because the bank is where the
    /// original draws the line.** `shieldactive` sits in `speech.bnk` and not
    /// in `weapons.bnk` beside the `~SHIELD` loop it fires with, and that is
    /// already this module's own reason for calling it a voice line rather
    /// than an effect - see [`Self::ShieldActive`]. `Disengaging` agrees from
    /// the other direction: `Autopilot_Update` plays it through the dry,
    /// full-volume path instead of the craft's emitter, which is a line in the
    /// player's ear rather than a thing happening in the world.
    ///
    /// So a cue moves bus by moving bank, and nothing here holds a second list
    /// that can disagree with `bank()`. See
    /// [ADR-0027](../../../../../docs/architecture/adr/0027-three-mix-buses.md).
    #[must_use]
    pub fn bus(self) -> Bus {
        match self.bank() {
            BankName::Speech => Bus::Speech,
            BankName::Hud | BankName::Ship | BankName::Weapons => Bus::Sfx,
        }
    }

    /// The string to look up in that bank's name table.
    ///
    /// # The dot in `.COLLISIONS` is the executable's own, not a lookup fix-up
    ///
    /// **Correction, 2026-09-04**: this section used to read the executable as
    /// passing bare `"COLLISIONS"` and claimed the leading dot was added here
    /// to bridge a SCREAM sound/child-sound naming split, at confidence 70.
    /// That was a misread of the call site - `ShipCollisionFx_Trigger`'s
    /// string argument is not built in place the way its three spark names
    /// are; it is loaded indirectly out of a small pointer table
    /// (`0x08924854: lw a3,0x46ec(a2)`), and the decompiler's inline literal
    /// showed the table slot's own apparent text, not the string the pointer
    /// it holds actually points at. Reading that pointer's target directly
    /// (`0x08a886e0`) gives `.COLLISIONS\0` - the dot is already in the
    /// executable's own data. See the correction paragraph in
    /// `docs/ghidra/functions/psp-pulse-usa/contact-response.md`.
    ///
    /// So this is a plain, exact match: no bridging, no SCREAM child-sound
    /// theory needed, just the same name the game itself passes. It happens
    /// to be the only cue named `.COLLISIONS` on any disc, and Wipeout HD's
    /// bank independently confirms the dot is a plain naming convention, not
    /// a parent/child marker: `.COLLISIONS` is the **parent**, and the cues
    /// it plays are the plainly named `c_CShipShip` and `c_CShipWall`. See
    /// `oag_formats::sblk::child`.
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
        }
    }

    /// Whether this port keeps a handle to the voice rather than firing and
    /// forgetting it.
    ///
    /// Both of these are cues the original opens with `Sound_PlayLooping` and
    /// an out-parameter, which is what the `~` prefix marks - see
    /// `docs/architecture/adr/0018-audio-mixer-architecture.md`. The `~` is
    /// **not** the test: `~SPARKS` and `~FLYBY_DIST` also carry it and are
    /// one-shots the caller can cancel, so a held cue is one whose *holder* is
    /// written here.
    ///
    /// [`Self::PlasmaTravel`], [`Self::RocketTravel`], [`Self::MissileTravel`]
    /// and [`Self::ShurikenTravel`] all join this list held **per projectile
    /// slot** rather than per grid slot or as a single race-wide handle - see
    /// [`super::travel::TravelVoices`]. [`Self::LeachAttach`] is held once for
    /// the whole race, on [`super::SfxVoices::leach_attach`], since the
    /// LeachBeam is a world-wide single instance rather than a projectile
    /// slot. [`Self::Autopilot`] joins them the same way `Blowup` already
    /// does: a level-driven handle (`ships[0].autopilot_timer > 0.0`), not a
    /// per-projectile one. None of the six is ever pushed through the cue
    /// queue, so this only guards against a stray push being mis-played.
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

    /// Whether this cue's list repeats while it is held, so it plays as the
    /// list runs rather than as one timeline laid down or one waveform picked.
    ///
    /// `~BLOWUP` (`[key-on loop, 0x15, key-on, 0x1a, 0x16]`, the second
    /// waveform re-keyed every 43 master ticks) and `~ROCKLOCK` (a guard on
    /// cue parameter 0 in front of each of two key-ons, inside a `0x15`/`0x16`
    /// loop). See `oag_formats::sblk::runner`.
    #[must_use]
    pub fn repeats(self) -> bool {
        matches!(self, Self::Blowup | Self::LockOn)
    }

    /// How far this cue's voice can be heard, for a cue that carries its own
    /// emitter rather than riding a craft's.
    ///
    /// Only read by [`super::place`]'s [`Placement::Point`] arm and by the
    /// bespoke held-voice trackers ([`super::travel::TravelVoices`],
    /// [`super::SfxVoices::leach_attach`]) - every [`Placement::Craft`] cue
    /// keeps [`oag_audio::Emitter::CRAFT_RADIUS`] unconditionally, by
    /// construction, so this only matters for the handful of cues with a
    /// bolt- or beam-owned emitter.
    ///
    /// Defaults to [`oag_audio::Emitter::CRAFT_RADIUS`] - `SoundEmitter_Init`'s
    /// own default, which is what every cue here keeps unless its own call
    /// site is read overriding it. **Measured at `600.0` for the Rocket**
    /// (`Rocket_Init` writes `+0x38` directly) and **for the LeachBeam's own
    /// body** (`LeachBeam_InitLocked`'s `+0x4c` emitter). **Chosen at `600.0`
    /// for the Missile**, reusing the Rocket's measured figure rather than
    /// [`oag_audio::Emitter::CRAFT_RADIUS`] - see [`Self::MissileTravel`]'s
    /// own doc comment for why. The Cannon's own hit cues keep the default:
    /// no override is stated for them. **Measured at `300.0` for
    /// [`Self::LeachEnergy`]** - half the beam body's own `600.0`, its own
    /// separate call site.
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
            // Measured: `Shuriken_Init` and `Shuriken_Bounce` each write
            // `0x43960000` to the blade's own emitter's `+0x38`.
            Self::ShurikenTravel | Self::ShurikenHit => 300.0,
            // `Repulser_HitCraft` writes `0x43960000` to the Repulser's own
            // emitter's `+0x38` before it plays.
            Self::RepulsorHit => 300.0,
            // Measured, not reused from `LeachAttach`'s `600.0` -
            // `LeachBeam_Advance`'s pulse block writes its own emitter at a
            // `300.0` falloff, half the beam body's own; see
            // `docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md`,
            // "Assets and cues - checked, not assumed".
            Self::LeachEnergy => 300.0,
            _ => oag_audio::Emitter::CRAFT_RADIUS,
        }
    }

    /// Which emitter the original plays this cue on.
    ///
    /// Read off each call site's first argument to `Sound_Play`, which is the
    /// emitter record and nothing else - see
    /// [`positional-audio.md`](../../../../../docs/ghidra/functions/psp-pulse-usa/positional-audio.md).
    /// A cue whose call site has not been read stays [`Placement::Unplaced`]
    /// rather than being placed somewhere plausible.
    #[must_use]
    pub fn placement(self) -> Placement {
        match self {
            // `lw a0, 0x50(a0)` at `0x08924844`, inside
            // `ShipCollisionFx_Trigger` - the craft's own emitter, with no
            // branch, so the player's hull is positional too.
            Self::Collision => Placement::Craft,
            // `FUN_08840640` is the absorb effect and sits on the same craft;
            // its emitter argument was not disassembled, so this rides
            // `Collision`'s reading of the contact path rather than its own.
            Self::Absorb => Placement::Craft,
            // `Shield_Activate`'s own doc comment above: `Sound_PlayLooping(1.0,
            // entity->0x50, ...)`, the same `+0x50` the collision path uses.
            Self::Shield => Placement::Craft,
            // Two branches, and which is taken splits on `racer+0x368` -
            // `ExhaustFlare_OnSpeedupPad` (`0x08904f74`) plays positionally off
            // `craft+0x50` on one side and dry at volume `0x400` through
            // `FUN_0883e9b0` on the other. See [`Placement::CraftUnlessPlayer`].
            Self::SpeedupPad => Placement::CraftUnlessPlayer,
            // `ExhaustFlare_OnPerfectStart` (`0x08904fd4`) is the same
            // two-branch dispatcher on `racer+0x368`, with `"TURBO"`.
            Self::Turbo => Placement::CraftUnlessPlayer,
            // `ExhaustFlare_Init` gives the note its own emitter at a quarter
            // of the craft radius, and [`Engine`] holds the voice, so this
            // never reaches the one-shot path.
            Self::Engine => Placement::Engine,
            // Recorded as `Sound_Play(entity, ..., "shieldactive", 0x400, 0)` -
            // full volume and pan zero, which is the shape of the *non*-emitter
            // path. Left dry, which is also what this port has always done.
            //
            // `Disengaging` joins it on a stronger footing: its call site is
            // read, and it is `FUN_0883e9b0` -> `FUN_0893a768`, the path that
            // takes no emitter at all.
            Self::ShieldActive | Self::Disengaging => Placement::Unplaced,
            // `RaceMode_SetState` and `RaceMode_UpdateCountdown` both call
            // `Sound_PlayNamedInSlot` with no emitter and `0x400`: a voice in
            // the player's ear, the same shape as `Disengaging` above.
            Self::Ready | Self::Go | Self::ContElim => Placement::Unplaced,
            // Read, not assumed: case 4 hands it to the path that takes no
            // emitter and a volume of `0x400`.
            Self::Blowup | Self::Message => Placement::Unplaced,
            // Both halves of `Ship_FireHeldWeapon`'s case 6 go through
            // `FUN_0883e9b0` too - the same dry, no-emitter path `Blowup` and
            // `Disengaging` take - see each variant's own doc comment.
            Self::Autopilot | Self::Engaging => Placement::Unplaced,
            // `HudSight_UpdateTone` opens it with a volume of `0x400` and no
            // emitter argument, which is the same dry, full-volume shape - and
            // it is a HUD sound about the player's own reticle rather than a
            // thing happening somewhere in the world.
            Self::LockOn => Placement::Unplaced,
            // Traces to the firing craft's own `+0x50` - see this variant's
            // own doc comment - so it rides the craft, not the mine.
            Self::MineLaunch => Placement::Craft,
            // `Plasma_Init` plays this off the argument it was handed
            // directly - the firing craft's own emitter - before it ever
            // constructs the bolt's own. See this variant's own doc comment.
            Self::Plasma => Placement::Craft,
            // All three ride the bolt's own emitter, which `Plasma_Init`
            // constructs pointed at the bolt's own matrix rather than any
            // craft's - see each variant's own doc comment. None is routed
            // through this function's `craft` slice: [`Self::PlasmaTravel`]
            // is a bespoke per-tick tracker keyed by projectile slot, and
            // [`Self::PlasmaHitWall`]/[`Self::PlasmaHitShip`] each carry their
            // own point on the [`CueEvent`] rather than a grid slot.
            Self::PlasmaTravel | Self::PlasmaHitWall | Self::PlasmaHitShip => Placement::Point,
            // Same shape as the Plasma's three, on the Rocket's own
            // `600.0`-radius emitter - see each variant's own doc comment and
            // [`Self::radius`].
            Self::RocketTravel | Self::RocketHitWall | Self::RocketHitShip => Placement::Point,
            // `Ship_FireHeldWeapon`'s own `Sound_Play(1.0, *(param_1+0x50),
            // weapons.bnk, 0, "ROCKET", 0)` - a plain positional call taking
            // an explicit emitter argument, the same shape [`Self::Shield`]
            // reads above, not the dry path - see this variant's own doc
            // comment.
            Self::Rocket => Placement::Craft,
            // Chosen, not measured - see each variant's own doc comment.
            Self::Missile | Self::Cannon => Placement::Craft,
            Self::MissileTravel
            | Self::MissileHitWall
            | Self::MissileHitShip
            | Self::MissileExpire => Placement::Point,
            Self::CannonHitWall | Self::CannonHitShip => Placement::Point,
            // Same `Ship_FireHeldWeapon` switch, same positional call shape
            // as `Rocket` - see that variant's own doc comment.
            Self::QuakeLaunch => Placement::Craft,
            // Settled by cross-reading the surrounding block's shield gate
            // and pending-damage fields, all the struck craft's own - see
            // this variant's own doc comment.
            Self::QuakeHit => Placement::Craft,
            // The wave's own road midpoint, tracked per tick by
            // [`super::super::travel::TravelVoices`] - see this variant's own
            // doc comment.
            Self::QuakeTravel => Placement::Point,
            // `LeachBeam_InitLocked` plays `LEACH` on the shooter's own
            // emitter - see this variant's own doc comment.
            Self::Leach | Self::LeachFail => Placement::Craft,
            // A bespoke per-tick tracker on the beam's own `600.0`-radius
            // emitter, the same shape [`Self::PlasmaTravel`] takes for a
            // projectile slot - see [`super::SfxVoices::leach_attach`] and
            // [`Self::radius`].
            Self::LeachAttach => Placement::Point,
            // Pushed with the ribbon's own `energy_point` as a
            // [`super::CueEvent::at_point`] - the pulse block's own re-spawn
            // target, not a craft's emitter - see this variant's own doc
            // comment and [`Self::radius`].
            Self::LeachEnergy => Placement::Point,
            // `SHURIKEN` rides the emitter `Shuriken_Init` is handed - the
            // firing craft's - and the blade's own two ride the emitter it
            // allocates for itself; see each variant's own doc comment.
            Self::ShurikenLaunch => Placement::Craft,
            Self::ShurikenHit | Self::ShurikenTravel => Placement::Point,
            // `Repulser_Init` plays `REPULSOR` on the firer's emitter, and
            // `Repulser_HitCraft` points the Repulser's own at the struck
            // craft's node - so both sit on a craft, the firer and the victim.
            Self::Repulsor | Self::RepulsorHit => Placement::Craft,
            // The group is anchored to `ship+0x70c8 + 0x10`, the craft's own
            // transform; see this variant's doc comment.
            Self::Magstrip | Self::MagstripStop => Placement::Craft,
        }
    }
}
