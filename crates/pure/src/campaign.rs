//! Where Wipeout Pure keeps the data its campaign is made of.
//!
//! One entry names the circuits, leagues and tournaments and the `<Unlock>`
//! rows that gate them; each circuit's medal figures sit in its own
//! `stats.xml` beside the `location` that entry gives. Read by
//! `oag_tables::pure_campaign`; the page is `docs/formats/pure-campaign.md`.
//! Identical on the EU and USA pressings.

/// The plugin definition: every `PI_Track`, `PI_League`, `PI_Tournament` and
/// `PI_Team`. The same entry `oag_pure::hashes` resolves the circuit list from.
pub const DEFINITION_ENTRY: &str = r"Data\Plugins\PI001\Definition.xml";

/// A circuit's medal figures, relative to its `location`.
pub const STATS_FILE: &str = "stats.xml";
