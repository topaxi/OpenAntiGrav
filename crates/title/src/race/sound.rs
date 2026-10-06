//! Which sound banks a title keeps each race cue in.
//!
//! Split out of [`super`] under this crate's 1,000-line ceiling: a move, with no behaviour change.

/// Which sound bank a title keeps each race cue in.
///
/// # Why this is an axis rather than a constant
///
/// The banks and the cue strings inside them are **shared across the lineage** -
/// Pulse, Pure and Wipeout HD all carry `SPEEDUPPAD`, `.COLLISIONS`, `ABSORB`,
/// `~SHIELD` and `shieldactive`, spelled identically. What moves is which file
/// they live in, and only on HD:
///
/// | Cue | Pulse and Pure | Wipeout HD |
/// | --- | --- | --- |
/// | `SPEEDUPPAD` | `hud.bnk` | **`weapons.bnk`** - HD has no `hud.bnk` |
/// | `.COLLISIONS` | `ship.bnk` | **`shiphd.bnk`** |
/// | `ABSORB`, `~SHIELD` | `weapons.bnk` | `weapons.bnk` |
/// | `shieldactive` | `speech.bnk` | `speech.bnk` |
///
/// So this is a five-field path table, measured per title, and not a
/// re-derivation of anything. `oag_sound::sfx::Cue` maps a cue to a field
/// here; nothing else knows a filename.
///
/// # A missing cue is an ordinary state
///
/// HD has **no `~ENGINE` at all** - its ship audio is a per-event `c_*` set
/// (`c_CShipWall`, `c_ElecArcA`..`D`) rather than one held loop, which is a
/// different design and not a renamed cue. There is no field for "the bank a
/// cue this title has not got lives in": the loader reports the miss and the
/// engine voice never opens. See `docs/formats/psp-audio.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SoundBanks {
    /// Where `SPEEDUPPAD` is. `hud.bnk` everywhere but HD.
    pub hud: &'static str,
    /// Where `~ENGINE` and `.COLLISIONS` are.
    pub ship: &'static str,
    /// The Zone counterpart of [`Self::ship`].
    ///
    /// Equal to `ship` on a title that ships no separate Zone bank, which is
    /// what HD does - the field is not an `Option` because "the same bank" is
    /// the honest answer there rather than "none".
    pub ship_zone: &'static str,
    /// Where `ABSORB` and `~SHIELD` are.
    pub weapons: &'static str,
    /// Where `shieldactive` is: the announcer, not an effect.
    pub speech: &'static str,
    /// Where a circuit's authored emitters find their banks: the shared ones
    /// and the directory the circuit's own bank sits in.
    ///
    /// The odd one out in this struct, because the shared entries are the
    /// only ones no *cue* names. See [`TrackBanks`]. A circuit's `.vex` authors its own emitters and each of them
    /// spells the bank it wants by that bank's own seven-character label - and
    /// on Pulse, 568 of the 1,164 `sound` nodes spell `gentrak`, which is
    /// `generaltrack.bnk`'s label rather than any part of its path
    /// (`docs/formats/psp-audio.md`, "the field is an abbreviation, not a
    /// truncation"). The circuit-specific bank beside it is named by the
    /// circuit's own `trackstartup.xml` and so needs no field here; this one is
    /// named by nothing on the disc, only by the executable.
    ///
    /// See `oag_sound::sfx::TrackEmitters` in `oag-game`.
    pub track: TrackBanks,
    /// Where this title's crossfaded engine tables address their sounds, or
    /// [`None`] to read them from [`Self::ship`] / [`Self::ship_zone`] under
    /// the plain `xfship_<team>.xfx` names, which is what HD does.
    pub crossfade: Option<Crossfade>,
}

/// Which banks a circuit's authored `sound` nodes can resolve against.
///
/// A node spells its bank by the bank's own label, so the loader opens every
/// bank listed here plus the circuit's own and matches on the label each
/// reports (`oag_sound::sfx::TrackEmitters::load`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrackBanks {
    /// Banks every circuit's nodes may name besides its own, as archive
    /// entries, in the order they are opened. Empty where the title has none.
    ///
    /// Pulse and Pure: `generaltrack.bnk`. 2048: `crowd_NGP.bnk` (label
    /// `crowd`) and `generaltrack.bnk` (`gentrak`).
    pub shared: &'static [&'static str],
    /// The archive directory a circuit's own bank is read from when it is not
    /// beside the track (tried first, named by the `trackstartup.xml`
    /// `<LoadSoundBank>` filename), or [`None`] where it always is: Pulse, Pure
    /// and HD.
    ///
    /// 2048's base circuits keep their bank in `Data\audio\sound\` instead:
    /// `env_altima.bnk` reads there and
    /// `Data\art\published\environments\altima\env_altima.bnk` does not
    /// exist. Its downloadable circuits do sit beside their track
    /// (`DLC1\environments\Metropia\env2_metropia.bnk`).
    pub circuit_directory: Option<&'static str>,
    /// Where this came from: ADR-0058's per-entry provenance.
    pub origin: crate::Origin,
}

/// A title whose engine tables address a bank other than its `ship` bank, and
/// keep a second set of tables for Zone.
///
/// Wipeout 2048 is the one: its five `<team>2048` tables name their sounds by
/// **cue index**, and those indices bind looping waveforms only in
/// `Ship_NGP.bnk` (Zone's three tables in `Ship_NGP_Zone.bnk`), where
/// `shipHD.bnk` binds one-shots under the same numbers. The measurement is
/// `crates/formats/tests/xfx_2048_ground_truth.rs`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Crossfade {
    /// The bank a race's tables address.
    pub ship: &'static str,
    /// The bank a Zone race's tables address.
    pub ship_zone: &'static str,
    /// The infix a Zone race's table names carry (`xfship_ZONE_<team>.xfx`),
    /// or [`None`] where Zone reads the same tables as any race.
    pub zone_infix: Option<&'static str>,
}
