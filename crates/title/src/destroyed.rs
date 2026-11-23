//! How a destroyed craft's sequence runs on a title: the timers, when the wreck
//! model replaces the hull, where the big blast is thrown, and whether the wreck
//! leaves the physics world.
//!
//! Pulse's sequence is `docs/ghidra/functions/psp-pulse-usa/ship-wreck-model.md`
//! and `wreck-collision.md`; HD/Fury's is
//! `docs/ghidra/functions/ps3-hdfury-eu/ship-destroy.md`. 2048 and Omega run
//! HD's `Ship_SetState` with the same cases and timers.

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
    /// Whether the title plays `shipdestroy` from its environment bank at the blast
    /// (row Z5), after every other cue is stopped.
    pub cue: bool,
}

impl ZoneEnd {
    /// HD/Fury's Zone end, read off `Ship_SetState` modes 6 (`zone-destroy.md`, rows Z1-Z12).
    pub const HD: Self = Self {
        shell_secs: 2.5,
        results_secs: 5.5,
        shell: r"Zoneship_dead\zoneship_dead_whiteshell",
        debris: r"Zoneship_dead\zoneship_dead",
        cue: true,
    };

    /// HD's Zone end on a title whose `shipdestroy` event is not resolved: 2048's bank
    /// resolution and Omega's Wwise container are unread, so the cue stays off there.
    pub const HD_SILENT: Self = Self {
        cue: false,
        ..Self::HD
    };
}

/// One title's destroyed-craft sequence.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DestroyedCraft {
    /// A Zone race's own end, `None` where the Zone craft becomes a wreck (Pulse, Pure).
    pub zone_end: Option<ZoneEnd>,
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
    /// Where the destroy camera comes from. HD's own mode 8 is unread, so it keeps the
    /// existing destroy camera, which is Pulse's.
    pub camera: crate::effects::Origin,
}

impl DestroyedCraft {
    /// Pulse's sequence: `0.5` s explosion, the wreck swapped in at state 5 and
    /// solid for `1.5` s, the blast `1.5` s on, the wreck still integrated.
    pub const PULSE: Self = Self {
        zone_end: None,
        explosion_secs: 0.5,
        dwell_secs: 1.5,
        settle: None,
        wreck_from: WreckFrom::OutOfRace,
        blast_at: BlastAt::DwellEnd,
        leaves_world: false,
        sound: DestroyedSound::Blowup,
        camera: crate::effects::Origin::Measured,
    };

    /// HD/Fury's `Ship_SetState` states 4 to 6 (`ship-destroy.md`, rows 1, 2, 9,
    /// 11, 12): `0.3` s explosion, the wreck in at state 4, the blast on entering it,
    /// state 5 ending when the wreck has lain still for `6.0` s (squared speed under
    /// `0.5`) or after `20.0` s, then out of the physics world.
    pub const HD: Self = Self {
        zone_end: Some(ZoneEnd::HD),
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
        camera: crate::effects::Origin::InheritedFrom("Wipeout Pulse"),
    };

    /// HD's sequence on a title whose destroy cues are not wired: it keeps `~BLOWUP`
    /// until its own cue names resolve in its banks.
    pub const HD_BLOWUP: Self = Self {
        zone_end: Some(ZoneEnd::HD_SILENT),
        sound: DestroyedSound::Blowup,
        ..Self::HD
    };
}
