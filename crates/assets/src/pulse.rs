//! Where Wipeout Pulse keeps the things the front end needs.
//!
//! Every constant here was resolved by hashing a candidate name and matching it
//! against a real archive directory, not guessed. See
//! `docs/architecture/frontend-boot.md` for how each one was found and what it
//! contains.

use crate::{Archive, Result};

/// The four archives a PSP Pulse disc ships, relative to the image root.
pub mod archives {
    /// Front-end fonts and shared images.
    pub const FE: &str = "PSP_GAME/USRDIR/FE.wad";
    /// Per-ship and per-track front-end screens.
    pub const FEDATA: &str = "PSP_GAME/USRDIR/FEData.wad";
    /// Everything else: tracks, ships, movies, plugins, string tables.
    pub const DATA: &str = "PSP_GAME/USRDIR/Data.wad";
    /// Back-end data.
    pub const BEDATA: &str = "PSP_GAME/USRDIR/BEData.wad";
}

/// Entry names inside `Data.wad`.
pub mod names {
    /// The front-end root: every boot screen, the `FEGlobals` variables, and
    /// the `LoadXML` list that pulls in the rest of the menus.
    pub const FRONTEND_ROOT: &str = r"Data\Plugins\PI001\GUI\Skin.xml";

    /// The intro movie played by the `LogoFMV` and `Play Intro` screens.
    ///
    /// The name is not in the executable: the `Movie` widget builds it from the
    /// `src` attribute in [`FRONTEND_ROOT`] plus `.PMF`.
    pub const INTRO_MOVIE: &str = r"Data\Movies\Intro.PMF";

    /// The looping backdrop behind the main menu.
    pub const BACKDROP_MOVIE: &str = r"Data\Movies\Backdrop.PMF";

    /// The five `.fnt` bitmap fonts, in `FE.wad`.
    ///
    /// Named from the `<Font Src="...">` attributes in the language plugins and
    /// confirmed by hashing: each of these hashes to an entry that really is in
    /// the archive. See `docs/formats/fnt.md`.
    pub mod fonts {
        /// `Default`, 13-pixel line height.
        pub const TEXT: &str = r"Data\FE\Fonts\pulse_text.fnt";
        /// `Small`, `Title`, `InGame` and `Stats`, 17-pixel line height.
        pub const PULSE_14: &str = r"Data\FE\Fonts\Pulse_14.fnt";
        /// `Menu`, 22-pixel line height.
        pub const PULSE_20: &str = r"Data\FE\Fonts\Pulse_20.fnt";
        /// `HUD`, 25-pixel line height, the only 512-wide atlas.
        pub const HUD: &str = r"Data\FE\Fonts\PulseHud.fnt";
        /// `HUDSmall`, 10-pixel line height.
        pub const SMALL: &str = r"Data\FE\Fonts\small.fnt";
    }

    /// The font the front end draws body text with.
    pub const DEFAULT_FONT: &str = fonts::TEXT;

    /// A language plugin's font table, native language name and string table
    /// pointer.
    #[must_use]
    pub fn language_definition(plugin: &str) -> String {
        format!(r"Data\Plugins\{plugin}\Definition.xml")
    }

    /// A language plugin's string table, the file its `Definition.xml` calls
    /// the "Dynamic Entry File Source".
    #[must_use]
    pub fn language_entries(plugin: &str) -> String {
        format!(r"Data\Plugins\{plugin}\entries.xml")
    }
}

/// Name hashes for `Data.wad` entries whose names are not recovered.
///
/// A WAD directory stores only the hash of each name, so an entry nobody has
/// named is still perfectly addressable. Recording the hash is what keeps such
/// an entry usable without inventing a name for it - which the naming rules in
/// `CLAUDE.md` forbid below 50 confidence, and a movie filename that no string
/// search, no XML and no runtime trace has produced is well below that.
pub mod hashes {
    /// The dev/pub reel the intro state plays, in its three regional cuts.
    ///
    /// Each is 480x272, 260 frames, 8.68 s, `PSMF0012` - a different container
    /// version from the `PSMF0014` of [`names::INTRO_MOVIE`], which is a second
    /// sign the two came off different pipelines - with ATRAC3+ audio. All three
    /// are static across frames 144 and 231, the two the intro state holds for
    /// two seconds, and moving on either side. Decoded, all three show the same
    /// pair of cards, and only the publisher line differs:
    ///
    /// | Constant | Frame 144 | Frame 231 |
    /// | --- | --- | --- |
    /// | [`DEVPUB_REEL_SCEE`] | Sony Computer Entertainment *Europe* presents | A Studio Liverpool game |
    /// | [`DEVPUB_REEL_SCEI`] | Sony Computer Entertainment *Inc.* presents | A Studio Liverpool game |
    /// | [`DEVPUB_REEL_SCEA`] | Sony Computer Entertainment *America* presents | A Studio Liverpool game |
    ///
    /// See `docs/architecture/frontend-boot.md`.
    /// All three ship on every disc regardless of that disc's own region -
    /// *Pure*'s USA disc carries these same three hashes at the same three
    /// sizes - so the set is region-invariant content and the cut must be picked
    /// at runtime. Which mechanism picks it has not been read out of the binary.
    ///
    /// This one is `oag-game`'s default, because the executable on the image
    /// this project reads is the EU build throughout despite its `UCUS-98712`
    /// serial: 18 `UCES00465` strings in `BOOT.BIN` and no `UCUS` string at all,
    /// an ISO volume id and publisher of `SCEE`, and a whole
    /// `PSP_GAME/USRDIR/UCES00465/` tree on the disc.
    pub const DEVPUB_REEL_SCEE: u32 = 0xb1ba_72c3;
    /// The Japanese cut. See [`DEVPUB_REEL_SCEE`].
    pub const DEVPUB_REEL_SCEI: u32 = 0x41fb_d22f;
    /// The American cut, which the disc's `UCUS-98712` serial argues for and
    /// nothing else does. See [`DEVPUB_REEL_SCEE`].
    pub const DEVPUB_REEL_SCEA: u32 = 0x3d2c_85f8;
}

/// The plugins that carry a language, in the order the disc lists them.
///
/// The plugin id is the only stable handle: the language's own name is inside
/// the plugin, not in its path. A PAL disc has a different set, so this is read
/// back from the archive rather than trusted.
pub const LANGUAGE_PLUGINS: &[&str] = &["PI008", "PI009", "PI010", "PI011", "PI012"];

/// The archives the front end reads from, all backed by one disc image.
///
/// Each holds its own handle on the image. That costs a little memory and
/// nothing else, and it keeps every archive independently seekable.
#[derive(Debug)]
pub struct Frontend {
    /// `Data.wad`: movies, screens, string tables.
    pub data: Archive,
    /// `FE.wad`: fonts and shared front-end images.
    pub fe: Archive,
}

impl Frontend {
    /// Opens the archives the boot sequence needs.
    ///
    /// `source` is either a disc image or a directory previously extracted with
    /// `oag-unpack`, which is what ADR-0004 means by "load from the user's own
    /// originals".
    pub fn open(source: &str) -> Result<Self> {
        Ok(Self {
            data: Archive::open(&archive_spec(source, archives::DATA))?,
            fe: Archive::open(&archive_spec(source, archives::FE))?,
        })
    }
}

/// Builds the specifier for one archive inside `source`.
///
/// A directory is joined with a separator; anything else is treated as a disc
/// image and joined with a colon, which is the form [`Archive::open`] takes.
#[must_use]
pub fn archive_spec(source: &str, relative: &str) -> String {
    if std::path::Path::new(source).is_dir() {
        let trimmed = source.trim_end_matches(['/', '\\']);
        format!("{trimmed}/{relative}")
    } else {
        format!("{source}:{relative}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_disc_image_is_joined_with_a_colon() {
        assert_eq!(
            archive_spec("data/images/pulse-psp-usa.chd", archives::DATA),
            "data/images/pulse-psp-usa.chd:PSP_GAME/USRDIR/Data.wad"
        );
    }

    #[test]
    fn an_extracted_directory_is_joined_with_a_separator() {
        // The current directory always exists, so it stands in for an extract.
        let spec = archive_spec(".", archives::DATA);
        assert_eq!(spec, "./PSP_GAME/USRDIR/Data.wad");
    }

    #[test]
    fn the_language_plugins_are_distinct() {
        let mut sorted = LANGUAGE_PLUGINS.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), LANGUAGE_PLUGINS.len());
    }

    #[test]
    fn plugin_entry_names_are_built_the_way_the_game_builds_them() {
        assert_eq!(
            names::language_definition("PI008"),
            r"Data\Plugins\PI008\Definition.xml"
        );
        assert_eq!(
            names::language_entries("PI012"),
            r"Data\Plugins\PI012\entries.xml"
        );
    }
}
