//! What Wipeout Pure ships.
//!
//! A title package in the sense of [ADR-0021], and the one that proves the seam
//! is real rather than decorative: if `oag-assets` can open Pure's disc through
//! the same mechanism it opens Pulse's, with nothing but a different table, then
//! title is a data axis.
//!
//! # Deliberately thin
//!
//! This carries only what [`pure-status.md`] measured. In particular it does
//! **not** restate Pure's `.vex` class ids: those are keyed off each file's own
//! version word inside `oag-formats`, because a class-id table is a property of
//! a format version rather than of a release - which the Pulse disc's own
//! version-4 `Data\Defaults\Skycube.vex` settles.
//!
//! Nothing about Pure's collision classes, its sound banks or its HUD is here,
//! because none of it is recovered. An empty module is the honest record of
//! that; a plausible guess would not be.
//!
//! [ADR-0021]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0021-title-packages.md
//! [`pure-status.md`]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/formats/pure-status.md

use oag_assets::{Archives, Result};
use oag_title::{ArchiveCandidates, ForeignSerial, Platform, Title};

/// Wipeout Pure, as the asset layer needs to know it.
pub const TITLE: &Title = &Title {
    name: "Wipeout Pure",
    archives: ArchiveCandidates {
        data: DATA_CANDIDATES,
        fe: FE_CANDIDATES,
    },
    foreign_serials: FOREIGN_SERIALS,
};

/// The archives a PSP Pure disc ships, relative to the image root.
///
/// **Three, not four.** `Data.wad`, `FE.wad` and `FEData.wad` are all present
/// under `PSP_GAME/USRDIR/`; Pure has no `BEData.wad`. All 1,229 entries across
/// the three decode to their declared size, so the container itself is
/// unchanged from Pulse's - it is the payloads' class numbering that differs.
pub mod archives {
    /// Front-end fonts and shared images.
    pub const FE: &str = "PSP_GAME/USRDIR/FE.wad";
    /// Per-ship and per-track front-end screens.
    pub const FEDATA: &str = "PSP_GAME/USRDIR/FEData.wad";
    /// Everything else: tracks, ships, handling, plugins.
    pub const DATA: &str = "PSP_GAME/USRDIR/Data.wad";
}

/// Entry names inside `Data.wad`.
///
/// The WAD name hash carries over from Pulse with no salt or seed change, so a
/// Pulse-shaped path hits Pure's directory when Pure really has that entry.
/// These are the ones confirmed present.
pub mod names {
    /// The game plugin's own definition.
    pub const GAME_PLUGIN_DEFINITION: &str = r"Data\Plugins\PI001\Definition.xml";

    /// One team's `handlingstats.xml`.
    ///
    /// Pure has **nine** teams to Pulse's eight, and each file carries **five**
    /// speed classes to Pulse's four. Which nine is not recorded here: the team
    /// list has not been read off the disc, and inventing one would be worse
    /// than the caller passing a name it got from somewhere real.
    #[must_use]
    pub fn handling_stats(team: &str) -> String {
        format!(r"Data\Ships\{team}\handlingstats.xml")
    }
}

/// The bulk archive's candidates. Pure is PSP-only.
const DATA_CANDIDATES: &[(&str, Platform)] = &[(archives::DATA, Platform::Psp)];

/// The companion archive's candidates.
const FE_CANDIDATES: &[(&str, Platform)] = &[(archives::FE, Platform::Psp)];

/// Serials positively identified as a Studio Liverpool title other than Pure.
///
/// The mirror of `oag_pulse`'s list, and one-directional in the same way: it
/// rules a source *out*, never in. Pulse's two verified serials are here
/// because Pulse ships `Data.wad` and `FE.wad` under the identical names Pure
/// does, so name matching alone would open one as if it were the other.
const FOREIGN_SERIALS: &[ForeignSerial] = &[
    ForeignSerial {
        serial: "UCUS-98712",
        title: "Wipeout Pulse",
    },
    ForeignSerial {
        serial: "SCES-54748",
        title: "Wipeout Pulse",
    },
];

/// Opens whichever archives `source` carries, as Wipeout Pure.
///
/// # Errors
///
/// Propagates [`Archives::open`].
pub fn open(source: &str) -> Result<Archives> {
    Archives::open(source, TITLE)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The deny-list rules out, never in - the same asymmetry `oag_pulse`'s has.
    #[test]
    fn pulses_serials_are_ruled_out_and_pures_own_get_no_verdict() {
        assert_eq!(TITLE.foreign_title("UCUS-98712"), Some("Wipeout Pulse"));
        assert_eq!(TITLE.foreign_title("SCES-54748"), Some("Wipeout Pulse"));
        assert_eq!(TITLE.foreign_title("UCUS-98612"), None);
        assert_eq!(TITLE.foreign_title("UCES-00001"), None);
    }

    /// Pure and Pulse rule *each other* out, which is what stops name matching
    /// opening one as the other.
    #[test]
    fn the_two_titles_are_mutually_exclusive() {
        assert!(TITLE.foreign_title("UCUS-98712").is_some());
        assert!(oag_pulse::TITLE.foreign_title("UCUS-98612").is_some());
    }
}
