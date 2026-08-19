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
    zone: oag_title::ZoneCircuit::Separate(DEFAULT_ZONE_TRACK),
};

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
