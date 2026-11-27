//! How a destroyed craft's sequence runs on a title: the timers, when the wreck
//! model replaces the hull, where the big blast is thrown, and whether the wreck
//! leaves the physics world.
//!
//! Pulse's sequence is `docs/ghidra/functions/psp-pulse-usa/ship-wreck-model.md`
//! and `wreck-collision.md`; HD/Fury's is
//! `docs/ghidra/functions/ps3-hdfury-eu/ship-destroy.md`. 2048 and Omega run
//! HD's `Ship_SetState` with the same cases and timers.

use crate::pre_race::Sourced;

/// Which state edge puts `shipwreck.vex` on the craft in place of its hull.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WreckFrom {
    /// State 5, the craft out of the race (Pulse).
    OutOfRace,
    /// State 4, the moment of the blast (HD, 2048, Omega).
    Explosion,
}

/// Where the big `WO_SHIP_EXPLOSION` is thrown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlastAt {
    /// As state 5 times out, 1.5 s after the craft went out of the race (Pulse).
    DwellEnd,
    /// On entering state 4, with the death plume at state 5 (HD, 2048, Omega).
    Explosion,
}

/// Which sound a craft's destruction makes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DestroyedSound {
    /// The player's own `~BLOWUP`, held through the explosion (Pulse, Pure); an opponent's
    /// destruction plays nothing.
    Blowup,
    /// Pure's: `~BLOWUP` held through the explosion as above, then `EXPLSMALL` at state 5 and
    /// `EXPLBIG` at the big blast on every craft, positional, with no `_PC` variant and no
    /// `SHIP_DEST` (`docs/ghidra/functions/psp-pure-usa/ship-destroy.md`, rows 2, 14, 18).
    BlowupAndBlasts,
    /// HD's `Ship_PlayDestroyExplosion`: `EXPLBIG_PC` (the local player) or `EXPLBIG` at the
    /// blast, the player's looped `~SHIP_DESTROYED`, and `~SHIP_ONFIRE` on every craft of an
    /// ordinary race (`ship-destroy.md`, rows 3, 12 and 15).
    Ship,
}

/// When a wreck that has come to rest is let go before its dwell is up.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Settle {
    /// Seconds in state 5 before the settled test is asked.
    pub after_secs: f32,
    /// The squared body speed under which the wreck counts as still.
    pub speed_squared: f32,
}

/// How a Zone race's craft ends on a title that never turns it into a wreck
/// (`docs/ghidra/functions/ps3-hdfury-eu/zone-destroy.md`): the hull stays, a white shell
/// animates over it, then hull, flare and shell hide and the debris pieces fly off.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ZoneEnd {
    /// How long the shell animates, `0` to `1` over this many seconds, before the hull, the
    /// flare and the shell hide and the debris is thrown (`Ship_SetState`'s state 5 timer,
    /// row Z7).
    pub shell_secs: f32,
    /// How long after the blast the results open: the race manager's `1.0` s then `4.5` s
    /// (rows Z1 and Z9).
    pub results_secs: f32,
    /// The Zone ship directory's death shell, relative to the hull's own directory.
    pub shell: &'static str,
    /// The Zone ship directory's debris model, relative to the hull's own directory.
    pub debris: &'static str,
    /// Whether the player's camera is the close orbit of `Camera_UpdateOrbitView` (mode 2) while
    /// the end plays (rows Z2a to Z2f in `destroy-camera.md`). The same law is read on 2048 and
    /// Omega (`destroy-camera.md`, "The orbit on 2048 and Omega").
    pub orbit_camera: bool,
    /// The title's `shipdestroy`, played at the blast (row Z5) after every other cue is stopped.
    /// `None` where the event is not resolved: the Zone end is then silent.
    pub cue: Option<ZoneEndCue>,
}

/// Where a title's `shipdestroy` lives and how its bank is read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ZoneEndCue {
    /// The race's environment bank in a Zone race (`env0_zone.bnk`).
    pub bank: &'static str,
    /// What the bank's delays count in.
    pub tick: crate::SequenceTick,
    /// Whether the bank's cue names are hashed (2048).
    pub hashed: bool,
}

impl ZoneEnd {
    /// HD/Fury's Zone end, read off `Ship_SetState` modes 6 (`zone-destroy.md`, rows Z1-Z12).
    pub const HD: Self = Self {
        shell_secs: 2.5,
        results_secs: 5.5,
        orbit_camera: true,
        shell: r"Zoneship_dead\zoneship_dead_whiteshell",
        debris: r"Zoneship_dead\zoneship_dead",
        cue: Some(ZoneEndCue {
            bank: r"Data\Sound\env0_zone.bnk",
            tick: crate::SequenceTick::Ps3,
            hashed: false,
        }),
    };

    /// 2048's Zone end: HD's, with `shipdestroy` out of its own hashed `env0_zone.bnk`
    /// (`zone-destroy.md`, row Z5: six waveforms in one timeline on the Vita tick).
    pub const VITA: Self = Self {
        cue: Some(ZoneEndCue {
            bank: r"Data\audio\sound\env0_zone.bnk",
            tick: crate::SequenceTick::Vita,
            hashed: true,
        }),
        ..Self::HD
    };

    /// HD's Zone end on a title whose `shipdestroy` event is not resolved: Omega's Wwise
    /// container is unread, so the cue stays off there.
    pub const HD_SILENT: Self = Self {
        cue: None,
        ..Self::HD
    };
}

/// The pieces an ordinary craft breaks into as its blast ends, thrown by
/// `DebrisManager_SpawnShipDebris` on `Ship_UpdateExploding`'s 4 -> 5 edge
/// (`docs/ghidra/functions/ps3-hdfury-eu/ship-destroy.md`, "The debris throw").
///
/// Every number is the executable's own (tuning `0x008c17d0`, `0x008c1810` and the code's
/// literals); what the original's physics world then does to a piece is not read, so
/// [`Self::gravity`] and the floor are this engine's.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShipDebris {
    /// The directory under the ships root holding the pieces, one `.vex` each.
    pub directory: &'static str,
    /// A piece's model: this stem followed by its number from `1`.
    pub model_stem: &'static str,
    /// How many pieces are thrown (`0x13`).
    pub pieces: usize,
    /// Pieces numbered after this carry `WO_DEBRIS_FIRE` (tuning word `0x10`, compared `<`
    /// against the zero-based index).
    pub fire_after: usize,
    /// Where a piece starts: this far along the craft's forward from its position (`0x00992d10`
    /// holds `(0, 0, -5)` in the body's own rows, and forward is `-Z`).
    pub spawn_ahead: f32,
    /// The scatter speed: `(A * right + B * up + forward) * spread`, `A` and `B` drawn in
    /// `[-1, 1)` (tuning `+0x24`, `25.0`).
    pub spread: f32,
    /// The draw `1 + jitter * C` scaling that scatter, `C` in `[-1, 1)` (tuning `+0x08`).
    pub jitter: f32,
    /// Seconds after the throw a piece is hidden (tuning `0x008c1810 + 0xc`).
    pub lifetime_secs: f32,
    /// The pull on a piece, units a second squared, along world down: the vector `(0, -9.81,
    /// 0) * 2` the throw's record carries.
    pub gravity: f32,
}

impl ShipDebris {
    /// HD/Fury's `DebrisManager_SpawnShipDebris` (`0x00104f18`).
    pub const HD: Self = Self {
        directory: "Ship_Damage",
        model_stem: "Part_",
        pieces: 19,
        fire_after: 16,
        spawn_ahead: 5.0,
        spread: 25.0,
        jitter: 0.95,
        lifetime_secs: 10.0,
        gravity: 19.62,
    };
}

/// How a title's destroy camera moves between cutting to a wreck and letting go of it
/// (`docs/ghidra/functions/ps3-hdfury-eu/destroy-camera.md`, rows D3 to D6, H1 and W8).
///
/// Pulse's mode 5 is one step per frame with fixed rates and never hands back; HD's mode 8 eases
/// in substeps with a random factor on each rate and hands back to the chase camera on the
/// camera's own 8 s clock; 2048 and Omega's mode 9 add the hold mode 8 after that hand-back.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DestroyCamera {
    /// Where the law comes from.
    pub origin: crate::effects::Origin,
    /// Whether the circuit's `Camera` nodes are read off `track.vex` for this camera even where
    /// the title's `circuit_effects` rule does not apply (`oag_title::effects`). Pulse's stations
    /// ride on that rule; HD's, 2048's and Omega's circuits author them beside an `.rcsmodel`.
    pub stations_without_circuit_effects: bool,
    /// `Some(k)`: each frame is `trunc(dt * k)` substeps of `rate / (dt * k)` (row D6). `None`:
    /// one step of `rate` per frame (Pulse, measured).
    pub substeps_per_second: Option<f32>,
    /// The factors drawn on the cut that scale the field rate and the focus rate (row D4),
    /// `None` where the rates are exact (Pulse).
    pub rate_factors: Option<RateFactors>,
    /// Seconds on the camera's own free-running clock after which the view hands back to the
    /// chase camera (row H1), `None` where it never does (Pulse).
    pub hands_back_after_secs: Option<f32>,
    /// Whether the chase camera holds still on a wrecked or resetting craft once it has handed
    /// back (row W8, 2048 and Omega).
    pub holds_the_wreck: bool,
}

/// The ranges the random factors on a destroy camera's ease rates are drawn from.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RateFactors {
    /// The factor on the field rate `0.06`: `Random_RangeFloat(0.5, 2.0)`.
    pub field: (f32, f32),
    /// The factor on the focus rate: `Random_RangeFloat(0.5, 1.0)`.
    pub focus: (f32, f32),
}

impl DestroyCamera {
    /// Pulse's mode 5, measured on a running PPSSPP; Pure keeps it.
    pub const PULSE: Self = Self {
        origin: crate::effects::Origin::Measured,
        stations_without_circuit_effects: false,
        substeps_per_second: None,
        rate_factors: None,
        hands_back_after_secs: None,
        holds_the_wreck: false,
    };

    /// HD's mode 8 (rows D1 to D8, H1), read statically.
    pub const HD: Self = Self {
        origin: crate::effects::Origin::Measured,
        stations_without_circuit_effects: true,
        substeps_per_second: Some(6000.0),
        rate_factors: Some(RateFactors {
            field: (0.5, 2.0),
            focus: (0.5, 1.0),
        }),
        hands_back_after_secs: Some(8.0),
        holds_the_wreck: false,
    };

    /// 2048's mode 9: HD's view with its own substep constant (`5999.9995`, row W4) and the hold
    /// mode 8 (row W8).
    pub const VITA: Self = Self {
        substeps_per_second: Some(5999.9995),
        holds_the_wreck: true,
        ..Self::HD
    };

    /// Omega's mode 9: 2048's, with HD's substep constant. **Chosen, not measured**: cases 9 to 11
    /// of Omega's view dispatcher `0x01383a10` are unread, so its substep constant is not.
    pub const OMEGA: Self = Self {
        substeps_per_second: Some(6000.0),
        origin: crate::effects::Origin::InheritedFrom("Wipeout HD"),
        ..Self::VITA
    };
}

/// How a Detonator race ends on a destroyed craft
/// (`docs/ghidra/functions/ps3-hdfury-eu/detonator-endrace.md`, "The wait before the results").
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DetonatorEnd {
    /// How long after the blast the results open, with the race rules stopped and the HUD gone
    /// meanwhile: the finish handler's `1.0` s then `4.5` s.
    pub results_secs: f32,
    /// Whether the camera is the close orbit of mode 2 until then, as a Zone end's
    /// ([`ZoneEnd::orbit_camera`]): `Ship_SetState`'s mode-2 arm takes the Zone family but
    /// modes 13 and 21, so Detonator's 14 with it (`zone-destroy.md`, row Z2).
    pub orbit_camera: bool,
}

impl DetonatorEnd {
    /// HD's: the finish handler's `1.0` s then `4.5` s, under the close orbit.
    pub const HD: Self = Self {
        results_secs: 5.5,
        orbit_camera: true,
    };
}

/// One title's destroyed-craft sequence.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DestroyedCraft {
    /// A Zone race's own end, `None` where the Zone craft becomes a wreck (Pulse, Pure).
    pub zone_end: Option<ZoneEnd>,
    /// A Detonator race's own end, `None` where the race is over when the explosion is. It carries
    /// its own provenance: Pulse's sequence is measured, the end its Detonator takes is HD's.
    pub detonator_end: Option<Sourced<DetonatorEnd>>,
    /// State 4's length: the explosion, before the craft is out of the race.
    pub explosion_secs: f32,
    /// State 5's length: the wreck is solid to other craft for this long, or until it
    /// settles ([`Self::settle`]).
    pub dwell_secs: f32,
    /// The early exit from state 5, `None` where only the timer ends it.
    pub settle: Option<Settle>,
    /// Which edge swaps the wreck model in.
    pub wreck_from: WreckFrom,
    /// Where the big blast is thrown.
    pub blast_at: BlastAt,
    /// Whether state 6 takes the body out of the physics world (it stops being
    /// integrated) rather than leaving it to land and roll.
    pub leaves_world: bool,
    /// The sound the destruction makes.
    pub sound: DestroyedSound,
    /// The camera the player is cut to when their craft is destroyed.
    pub camera: DestroyCamera,
    /// Whether the wreck stops being drawn at the big blast: its matrix is squashed to `1e-4`
    /// and the per-state update skips the live model's matrix in state 6 (Pulse
    /// `0x08841e50`, Pure `0x089233f0`, `ship-destroy.md` row 22). Not HD's, whose wreck lies
    /// in view until it is taken out of the world.
    pub wreck_hidden_at_blast: bool,
    /// The additive flash model thrown at the body on the 4 -> 5 edge and again at the big
    /// blast (`ShipGaseousFlash_Construct`, rows 8 and 21), as an archive path. `None` where
    /// the title throws no such object.
    pub gaseous_flash: Option<&'static str>,
    /// The pieces the craft breaks into at the 4 -> 5 edge, `None` where it throws none.
    pub ship_debris: Option<ShipDebris>,
}

impl DestroyedCraft {
    /// Pulse's sequence: `0.5` s explosion, the wreck swapped in at state 5 and
    /// solid for `1.5` s, the blast `1.5` s on, the wreck still integrated.
    pub const PULSE: Self = Self {
        zone_end: None,
        detonator_end: Some(Sourced::inherited(DetonatorEnd::HD, "Wipeout HD")),
        explosion_secs: 0.5,
        dwell_secs: 1.5,
        settle: None,
        wreck_from: WreckFrom::OutOfRace,
        blast_at: BlastAt::DwellEnd,
        leaves_world: false,
        sound: DestroyedSound::Blowup,
        camera: DestroyCamera::PULSE,
        wreck_hidden_at_blast: true,
        gaseous_flash: None,
        ship_debris: None,
    };

    /// Pure's `Ship_SetState` states 4 to 6 (`ship-destroy.md`): Pulse's timers and edges, the
    /// gaseous flash in place of the shockwave ring, its own cues. The destroy camera stays
    /// Pulse's mode with Pure's index.
    pub const PURE: Self = Self {
        sound: DestroyedSound::BlowupAndBlasts,
        gaseous_flash: Some(r"Data\Weapons\explosion_gaseous.vex"),
        // Pure races no Detonator.
        detonator_end: None,
        ..Self::PULSE
    };

    /// HD/Fury's `Ship_SetState` states 4 to 6 (`ship-destroy.md`, rows 1, 2, 9,
    /// 11, 12): `0.3` s explosion, the wreck in at state 4, the blast on entering it,
    /// state 5 ending when the wreck has lain still for `6.0` s (squared speed under
    /// `0.5`) or after `20.0` s, then out of the physics world.
    pub const HD: Self = Self {
        zone_end: Some(ZoneEnd::HD),
        detonator_end: Some(Sourced::measured(DetonatorEnd::HD)),
        explosion_secs: 0.3,
        dwell_secs: 20.0,
        settle: Some(Settle {
            after_secs: 6.0,
            speed_squared: 0.5,
        }),
        wreck_from: WreckFrom::Explosion,
        blast_at: BlastAt::Explosion,
        leaves_world: true,
        sound: DestroyedSound::Ship,
        camera: DestroyCamera::HD,
        wreck_hidden_at_blast: false,
        gaseous_flash: None,
        ship_debris: Some(ShipDebris::HD),
    };

    /// HD's sequence on a title whose destroy cues are not wired: it keeps `~BLOWUP`
    /// until its own cue names resolve in its banks.
    pub const HD_BLOWUP: Self = Self {
        zone_end: Some(ZoneEnd::HD_SILENT),
        sound: DestroyedSound::Blowup,
        camera: DestroyCamera::OMEGA,
        ..Self::HD
    };

    /// [`Self::HD_BLOWUP`] with 2048's Zone end, whose `shipdestroy` resolves in its own bank.
    pub const HD_VITA: Self = Self {
        zone_end: Some(ZoneEnd::VITA),
        camera: DestroyCamera::VITA,
        ..Self::HD_BLOWUP
    };
}

#[cfg(test)]
mod tests {
    use super::{DestroyedCraft, DetonatorEnd};
    use crate::pre_race::Sourced;

    /// The Detonator finish handler's two waits and its orbit are the same on HD, 2048 and Omega
    /// (`docs/ghidra/functions/ps3-hdfury-eu/detonator-endrace.md`, "The wait before the results").
    #[test]
    fn the_hd_family_waits_five_and_a_half_seconds_on_a_detonator_end() {
        let end = DetonatorEnd {
            results_secs: 5.5,
            orbit_camera: true,
        };
        for destroyed in [
            DestroyedCraft::HD,
            DestroyedCraft::HD_BLOWUP,
            DestroyedCraft::HD_VITA,
        ] {
            assert_eq!(destroyed.detonator_end, Some(Sourced::measured(end)));
        }
        // Pulse's Detonator takes HD's wait, labelled as inherited.
        assert_eq!(
            DestroyedCraft::PULSE.detonator_end,
            Some(Sourced::inherited(end, "Wipeout HD"))
        );
        assert_eq!(DestroyedCraft::PURE.detonator_end, None);
    }
}
