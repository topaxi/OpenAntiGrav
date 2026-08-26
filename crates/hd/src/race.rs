//! What an HD race loads before it has opened anything: the default circuit and
//! team.
//!
//! The counterpart of `oag_pulse::race` and `oag_pure::race`, and the shortest
//! of the three, because the interesting result is how little needed saying.
//!
//! # Pulse's entry-name spellings resolve here unchanged
//!
//! A PSARC stores real paths where a WAD stores a name hash, so the two look
//! incompatible. They are not, because
//! [`oag_assets::psarc::Archive`](oag_assets::psarc) folds separators, case and
//! a leading `/` before it looks a path up. So
//! `oag_formats::handling::entry_name("assegai")` produces
//! `Data\Ships\assegai\handlingstats.xml`, which normalises to
//! `data/ships/assegai/handlingstats.xml`, which is what the manifest stores -
//! and `handling::GLOBAL_ENTRY`'s `Data\XML\HandlingStats.xml` reaches
//! `/data/xml/handlingstats.xml` the same way.
//!
//! That is why there is no `handling_stats` helper here and no HD-specific
//! spelling of one. Adding it would be a second copy of a string that already
//! works.
//!
//! **The circuit is the one thing that could not carry over**, and not because
//! of the container: HD calls Talon's Junction `talons_junction` where Pulse
//! calls the same circuit `16_Track`. A name, not a path shape.

/// The two things a race needs before it has opened anything.
pub const DEFAULTS: &oag_title::RaceDefaults = &oag_title::RaceDefaults {
    track: DEFAULT_TRACK,
    team: DEFAULT_TEAM,
    // The directory three of the four titles spell identically; Wipeout 2048 is
    // the one that does not. See `oag_title::RaceDefaults::ship_dir`.
    ship_dir: oag_title::race::SHIP_DIR,
    // The same directory: this title keeps a team's tuning beside its models.
    handling_dir: oag_title::race::SHIP_DIR,
    zone: oag_title::ZoneCircuit::Separate(DEFAULT_ZONE_TRACK),
    zone_craft: oag_title::ZoneCraft::OwnShip(ZONE_SHIP),
    sounds: SOUND_BANKS,
};

/// Where each race cue lives, and the one title where two of them move.
///
/// Measured off the disc rather than assumed from Pulse: the seven PSARCs hold
/// 18 distinct `.bnk` names and **`hud.bnk` is not among them**. `SPEEDUPPAD`
/// is in `weapons.bnk` here, and the ship bank is `shiphd.bnk`.
///
/// Spelled `Data\Sound\...` even though a PSARC stores `/data/sound/...`,
/// because [`oag_assets::psarc`] folds case and separators - so one spelling
/// reaches every container in the lineage and no caller branches on the disc.
///
/// **`ship_zone` is `shiphd.bnk` because HD ships no separate Zone ship bank.**
/// Its Zone content is `env0_zone.bnk` and `speech_zone.bnk`, which are a
/// circuit and a voice-over; there is no `shiphd_zone.bnk` to point at, and
/// "the same bank" is the honest answer rather than a fallback.
///
/// **HD has no `~ENGINE`.** Its ship audio is a per-event set - `c_CShipWall`,
/// `c_CShipShip`, `c_GShipShip`, `c_ElecArcA`..`D`, `c_CrackLoopL/C/R` - which
/// is a different design and not a renamed cue, so no field here can supply it
/// and the loader reports the miss. See `docs/formats/psp-audio.md`.
pub const SOUND_BANKS: &oag_title::SoundBanks = &oag_title::SoundBanks {
    hud: r"Data\Sound\weapons.bnk",
    ship: r"Data\Sound\shiphd.bnk",
    ship_zone: r"Data\Sound\shiphd.bnk",
    weapons: r"Data\Sound\weapons.bnk",
    speech: r"Data\Sound\speech.bnk",
};

/// The ship directory a Zone race flies out of.
///
/// **Shaped like Pure's and not like Pulse's**, which is what gave
/// [`oag_title::ZoneCraft`] its third measurement and so its licence to exist:
/// Zone has a ship directory of its own, and the player's team choice does not
/// reach the hull.
///
/// This name was already on the disc's roster before anything wanted it -
/// `zone` is one of the four [`crate::names::MODE_SHIPS`], read off the manifest
/// beside `detonator`, `zone battle` and `test`, and that listing already
/// recorded the shape of the finding: **the mode ships author no `<Class>` block
/// at all** where every team file authors four.
/// `crates/assets/tests/hd_psarc_ground_truth.rs` asserts exactly that and names
/// `zone` as one of the two classless ones.
///
/// `/data/ships/zone/ship.vex` resolves and decodes - 29 meshes, 29,678
/// triangles through its `.rcsmodel` sibling, probed 2026-08-19. Confidence
/// **94** for the name; **nothing about which mode selects it has been read in
/// this executable**, so the identification rests on the directory's name, its
/// classless handling file and its company in `MODE_SHIPS`, exactly as Pure's
/// does.
///
/// Spelled without a leading slash and capitalised the way Pulse spells a team,
/// because it goes through `oag_pulse::race::ships::entry_name` like every other
/// hull and a PSARC folds case and separators - see this module's own docs.
/// `Data\Ships\zone\Ship.vex` normalises onto `data/ships/zone/ship.vex`, which
/// is what the manifest stores.
pub const ZONE_SHIP: &str = "zone";

/// The circuit a Zone race loads when the caller names none.
///
/// **HD is shaped like Pure here, not like Pulse**: Zone runs on circuits of its
/// own rather than on a second `.vex` inside a race circuit's directory. Four of
/// them, and this build already knew their names without knowing what they were
/// for - `zone_1` through `zone_4` are four of the sixteen entries in
/// [`crate::names::ENVIRONMENTS`], which `crates/hd/tests/hd_title_ground_truth.rs`
/// re-derives from the manifest rather than trusting. Each holds a plain
/// `track.vex`, the same as every other environment on the disc.
///
/// `zone_1` for the reason [`DEFAULT_TRACK`] gives for Talon's Junction inverted:
/// there is no capture to match here, so the disc's own ordering answers.
///
/// **What is *not* claimed** is which of the four a player reaches first, or how
/// they are unlocked. Pure declares that in its plugin definition and HD's
/// track-selection XML has not been read - see [`crate::names::ENVIRONMENTS`],
/// which says the same about the sixteen. This is a name that resolves, nothing
/// more.
pub const DEFAULT_ZONE_TRACK: &str = "/data/environments/zone_1/track.vex";

/// The circuit a race loads when the caller names none.
///
/// **Talon's Junction, and the reason is that it is the same circuit as Pulse's
/// `16_Track` in the same world coordinates** - checked in
/// `crates/assets/tests/hd_psarc_ground_truth.rs`, which loads both and compares
/// their bounds and start positions. So an HD run and an existing Pulse capture
/// are directly comparable, which is worth more here than picking the disc's own
/// opening circuit would be.
///
/// It is also the circuit this project has read end to end on HD: 826 nodes, an
/// 862-point spline, 400 collision objects, 18 speedup and 9 weapon pads.
///
/// Spelled as the archive stores it, leading slash and all. Any spelling would
/// resolve - see this module's docs - and the stored one is the one a reader can
/// grep the manifest for.
pub const DEFAULT_TRACK: &str = "/data/environments/talons_junction/track.vex";

/// The team whose `handlingstats.xml` a race uses by default.
///
/// **Assegai, as on both PSP titles, and lowercase, as HD spells it.** Sharing
/// the default across all three titles is deliberate: every PPSSPP capture under
/// `data/traces/` was taken with Assegai, so a default that matched would leave
/// the three `--race` runs differing in one thing rather than two.
///
/// The claim about HD specifically is only that the disc carries the team:
/// `/data/ships/assegai/handlingstats.xml` is in `DATA02`, and it parses with
/// Pulse's schema. Nothing about Assegai is HD's own preference.
pub const DEFAULT_TEAM: &str = "assegai";
