//! What Wipeout Pure ships.
//!
//! A title package in the sense of [ADR-0022], and the one that proves the seam
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
//! [ADR-0022]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0022-title-packages.md
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
    boot: frontend::BOOT_PROFILE,
};

pub mod frontend;

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
    /// **Eleven** ship directories carry one to Pulse's eight, and ten of the
    /// eleven author **five** speed classes to Pulse's four. The eleventh,
    /// `Zone_01`, authors no `<Class>` block at all.
    ///
    /// The roster is still not a constant here, deliberately: it is declared by
    /// `Data\Plugins\PI001\Definition.xml` on the disc itself, so a caller that
    /// reads it gets the list the player's own pressing ships rather than one
    /// this crate remembered. `crates/pure/tests/handling_schema_ground_truth.rs`
    /// does exactly that.
    #[must_use]
    pub fn handling_stats(team: &str) -> String {
        format!(r"Data\Ships\{team}\handlingstats.xml")
    }

    /// The boot movie the `IntroMovie1` widget on `Intro Screen` plays.
    ///
    /// The widget's `src` is `Data\Movies\IntroMovieP1`, `localised="true"` -
    /// `oag_game::screen::Movie::entry_name` appends `_US.PMF` for a localised
    /// source with no extension of its own, per its own doc comment. Hashing
    /// that gives `3d2c85f8`, a real entry in `pure-psp-usa.chd`'s `Data.wad` -
    /// but that is not independent confirmation, since a wrong guess can still
    /// land on a real entry by accident. **Checked properly**: extracted and
    /// decoded (`ffmpeg` on the cached `.ivf`), frames 150 and 230 read
    /// "SONY COMPUTER ENTERTAINMENT AMERICA" and "A STUDIO LIVERPOOL GAME" -
    /// this is `oag_pulse`'s own `DEVPUB_REEL` (`hash:b1ba72c3` is the EU cut,
    /// `hash:3d2c85f8` the American one both share), not new content. Pure's
    /// disc genuinely opens on this reel; Pulse ships the identical asset
    /// (`oag_pulse::DEVPUB_REEL`'s own doc comment already says the two discs
    /// carry it byte-identically) but no longer plays it at boot.
    ///
    /// **It plays on `Developer Publisher Screen`**, the step straight after the
    /// picker - inherited from the parent `Intro Screen` that declares the widget.
    /// Confirmed by matching captured frames against the decoded video: frame 144
    /// is the "PRESENTS" card and frame 231 the "A STUDIO LIVERPOOL GAME" card,
    /// each within resampling noise. See
    /// [`crate::frontend::states::DEVELOPER_PUBLISHER`].
    ///
    /// **This name is one of four, and picking it unconditionally is a known
    /// bug.** See [`INTRO_MOVIE_CUTS`]: `oag_game::screen::Movie::entry_name`
    /// appends `_US.PMF` to every `localised="true"` widget on every source, so a
    /// European pressing is currently shown the American card.
    pub const INTRO_MOVIE: &str = r"Data\Movies\IntroMovieP1_US.PMF";

    /// Every regional cut of the dev/pub reel, by the name that resolves it.
    ///
    /// **All four are on both Pure pressings** (the Korean one only on the EU
    /// disc), and all three of the cuts Pulse ships are here too - the reel is a
    /// shared asset across both titles, played by Pure and carried unused by
    /// Pulse. Each name was recovered by hashing the `_<REGION>` suffix pattern
    /// and confirming the entry exists, then cross-validated against the matching
    /// [`FMV_INTRO_MOVIE_CUTS`] set.
    ///
    /// This is what `localised="true"` selects between. The suffix the original
    /// picks for a given pressing has **not** been read out of any binary; that
    /// the EU disc carries an `_EU` cut whose frame 144 reads "EUROPE" is the
    /// evidence that it picks one at all.
    pub const INTRO_MOVIE_CUTS: &[(&str, &str)] = &[
        ("EU", r"Data\Movies\IntroMovieP1_EU.PMF"),
        ("US", r"Data\Movies\IntroMovieP1_US.PMF"),
        ("JAP", r"Data\Movies\IntroMovieP1_JAP.PMF"),
        ("KO", r"Data\Movies\IntroMovieP1_KO.PMF"),
    ];

    /// The same four cuts of the second boot movie, on the same evidence.
    pub const FMV_INTRO_MOVIE_CUTS: &[(&str, &str)] = &[
        ("EU", r"Data\Movies\WoFMVNew_EU.PMF"),
        ("US", r"Data\Movies\WoFMVNew_US.PMF"),
        ("JAP", r"Data\Movies\WoFMVNew_JAP.PMF"),
        ("KO", r"Data\Movies\WoFMVNew_KO.PMF"),
    ];

    /// The second boot movie, played by the `FMV Intro` screen state.
    ///
    /// The widget's `src` is `Data\Movies\WoFMVNew`, also `localised="true"`,
    /// declared as a sibling of `IntroMovie1` on `Intro Screen` rather than
    /// owned by `FMV Intro` itself - `FMV Intro` carries no widgets of its own,
    /// just a placeholder `Item` and a `Redirect` to `Title Screen`. Confirmed
    /// present the same way as [`INTRO_MOVIE`] (`oag-wad hash` -> `03fff874`).
    ///
    /// **Not wired to playback yet.** `oag_game::frontend` reaches this state
    /// and lets a button leave it, but does not yet decode or draw the movie -
    /// that needs its own decoded-frame slot the way the intro and the menu
    /// backdrop each have one (`crate::frontend::Video`, `Session::feed` /
    /// `Session::backdrop` in `oag_game::main`), which is real, separate work.
    pub const FMV_INTRO_MOVIE: &str = r"Data\Movies\WoFMVNew_US.PMF";
}

/// The bulk archive's candidates. Pure is PSP-only.
const DATA_CANDIDATES: &[(&str, Platform)] = &[(archives::DATA, Platform::Psp)];

/// The companion archive's candidates.
const FE_CANDIDATES: &[(&str, Platform)] = &[(archives::FE, Platform::Psp)];

/// Serials positively identified as a Studio Liverpool title other than Pure.
///
/// The mirror of `oag_pulse`'s list, and one-directional in the same way: it
/// rules a source *out*, never in. Pulse's verified serials are here because
/// Pulse ships `Data.wad` and `FE.wad` under the identical names Pure does,
/// so name matching alone would open one as if it were the other. `UCES-00465`
/// is Pulse's EU/Australia PSP pressing, the same one `oag_pulse::FOREIGN_SERIALS`
/// had to grow when Pure's own EU disc (`UCES-00001`) turned up missing from
/// *that* list - the two are added in the same change here so the pair does
/// not drift apart again.
const FOREIGN_SERIALS: &[ForeignSerial] = &[
    ForeignSerial {
        serial: "UCUS-98712",
        title: "Wipeout Pulse",
    },
    ForeignSerial {
        serial: "UCES-00465",
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
        assert_eq!(TITLE.foreign_title("UCES-00465"), Some("Wipeout Pulse"));
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
