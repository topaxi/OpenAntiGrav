//! What a Wipeout 2048 race loads: the default circuit and team, where its
//! ships live, and which bank each cue is in.
//!
//! Names and defaults in the [ADR-0022] sense, the counterpart to
//! [`oag_hd::race`]. Every entry name here was probed against
//! `PCSF00007/base/PSP2/data.psarc` by asking the manifest for it, the same
//! bar `oag_pure::race` sets: **confidence 90** for a name that resolves to an
//! entry, and no more than that, because nothing here has been watched
//! running.
//!
//! [ADR-0022]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0022-title-packages.md
//! [`oag_hd::race`]: https://github.com/topaxi/OpenAntiGrav/blob/main/crates/hd/src/race.rs

/// What a race needs before it has opened anything, as
/// [`oag_title::Title::race`] carries it.
pub const DEFAULTS: &oag_title::RaceDefaults = &oag_title::RaceDefaults {
    track: DEFAULT_TRACK,
    team: DEFAULT_TEAM,
    ship_dir: SHIP_DIR,
    zone: oag_title::ZoneCircuit::Separate(DEFAULT_ZONE_TRACK),
    zone_craft: oag_title::ZoneCraft::OwnShip(ZONE_SHIP),
    sounds: SOUND_BANKS,
};

/// Where this title keeps its roster - **the one axis 2048 forced into
/// existence**.
///
/// `Data\Ships\<Team>\` is what Pulse, Pure and Wipeout HD all spell, and
/// [`oag_title::race::SHIP_DIR`]'s own docs used to argue it was shared
/// vocabulary rather than a title fact. 2048 disagrees: its fourteen
/// HD-derived teams are at `Data\art\published\hdships\<Team>\`, carrying
/// `ship.vex`, `handlingstats.xml`, `shipshield.vex`, `Engineflare.vex`,
/// `Locators.vex` and four LODs each - the same file set under the same names,
/// one directory up and three deep.
///
/// **This is not the whole roster**, and the part it leaves out is recorded
/// rather than guessed at: 2048's own five teams (`ag_systems2048`,
/// `auricom2048`, `feisar2048`, `piranha2048`, `qirex2048`) live under
/// `Data\HandlingStats\<team>\<1..4>\handlingstats.xml`, a *two*-level shape
/// this single-string axis cannot express, and what the numbered levels select
/// is unread. So this build races 2048's HD-derived craft and not its native
/// ones. See `docs/formats/2048-status.md`.
pub const SHIP_DIR: &str = r"Data\art\published\hdships";

/// The circuit a race loads when the caller names none.
///
/// **Altima, the circuit 2048's own campaign opens on**, and the one whose
/// `WO Track` payload the version `0x107` control-point layout was recovered
/// against - 6 paths, 4 junctions, 2,029 control points, `encoded_len()`
/// landing on the payload length to the byte. See `docs/formats/track.md`.
///
/// Its drivable geometry is another matter: `track.rcsmodel` is 17.4 MiB of a
/// container this build cannot decode, so a race here comes up as the derived
/// ribbon rather than the authored surface, exactly as a Wipeout HD race does.
pub const DEFAULT_TRACK: &str = r"Data\art\published\environments\altima\track.vex";

/// The team whose `handlingstats.xml` a race uses by default.
///
/// **Assegai, as on all three other titles**, and spelled with the capital
/// 2048 spells it. Sharing the default across the lineage is deliberate: every
/// capture under `data/traces/` was taken with Assegai, so a default that
/// matched leaves the comparison available. See [`oag_hd::race::DEFAULT_TEAM`].
///
/// [`oag_hd::race::DEFAULT_TEAM`]: https://github.com/topaxi/OpenAntiGrav/blob/main/crates/hd/src/race.rs
pub const DEFAULT_TEAM: &str = "Assegai";

/// The hull directory a Zone race flies out of, under [`SHIP_DIR`].
///
/// `Data\art\published\hdships\Zone\Ship.vex` resolves, which is HD's own
/// shape - a ship directory of its own, and the player's team not reaching the
/// hull at all.
pub const ZONE_SHIP: &str = "Zone";

/// The circuit a Zone race loads when the caller names none.
///
/// **In `dlc2.psarc`, not in the base package**, so this resolves only on a
/// source that carries the downloadable content - see
/// [`crate::EXTRA_CANDIDATES`].
///
/// **What is recorded rather than resolved**: every circuit in the base
/// package ships a `ZoneMode2048.effectSettings` beside its `track.vex`, which
/// says 2048's base game has a Zone mode running on its *ordinary* circuits -
/// a third shape that is neither [`oag_title::ZoneCircuit::Prefixed`] (there is
/// no second `.vex` to prefix to) nor [`oag_title::ZoneCircuit::Separate`].
/// Nothing here has read how the original selects it, so this names the four
/// Zone circuits that do exist as files and leaves that question open.
pub const DEFAULT_ZONE_TRACK: &str = r"Data\art\published\DLC1\environments\zone_1\track.vex";

/// Where each race cue lives.
///
/// **Measured off the manifest rather than inherited**, the way
/// [`oag_hd::race::SOUND_BANKS`] was: the archive holds 29 distinct `.bnk`
/// names and, like HD, **no `hud.bnk`** - so `SPEEDUPPAD` is looked for in
/// `weapons.bnk` here too. The ship bank is `shipHD.bnk`, HD's spelling
/// exactly, and there is a `Ship_NGP.bnk` and a `Ship_NGP_Zone.bnk` beside it
/// that are presumably 2048's own ("NGP" being the Vita's development name).
///
/// **Which of the two pairs the original loads is unread**, and this points at
/// HD's, because HD's is the one whose cue strings this build has actually
/// decoded. That is a choice made for a reason, not a measurement; a race
/// reports every cue it cannot find.
///
/// [`oag_hd::race::SOUND_BANKS`]: https://github.com/topaxi/OpenAntiGrav/blob/main/crates/hd/src/race.rs
pub const SOUND_BANKS: &oag_title::SoundBanks = &oag_title::SoundBanks {
    hud: r"Data\audio\sound\weapons.bnk",
    ship: r"Data\audio\sound\shipHD.bnk",
    ship_zone: r"Data\audio\sound\shipHD.bnk",
    weapons: r"Data\audio\sound\weapons.bnk",
    speech: r"Data\audio\sound\speech.bnk",
};
