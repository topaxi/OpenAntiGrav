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

/// The speed classes a fresh profile's `Class Selection` offers: Vector and
/// Venom (a PPSSPP capture on a fresh profile, 2026-09-10). The screen's menu
/// authors all five and the code hides the rest; which later class opens when
/// is the unread part (`docs/ghidra/functions/psp-pure-usa/campaign.md`).
pub const CLASSES_AT_START: &[&str] = &["Vector", "Venom"];

/// What the main menu's campaign row says on Pure: the original's top row is
/// `Single Player` (`Skin.xml`'s `Main Menu`), the chain this project's
/// `RACE CAMPAIGN` row replaces.
pub const MAIN_ROW: oag_title::MainRow = oag_title::MainRow {
    label: oag_title::Wording {
        id: "OAG_MENU_SINGLE_PLAYER",
        text: "SINGLE PLAYER",
    },
    subtitle: oag_title::Wording {
        id: "OAG_MENU_SINGLE_PLAYER_SUBTITLE",
        text: "Arcade, tournaments, time trials and Zone, class by class",
    },
};
