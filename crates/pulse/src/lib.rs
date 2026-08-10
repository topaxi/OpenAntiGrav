//! What Wipeout Pulse ships: which archives its releases carry, what the
//! entries inside them are called, and the hashes of the ones nobody has named.
//!
//! This is a title package in the sense of [ADR-0022]: tables, not decoding.
//! Every constant here was resolved by hashing a candidate name and matching it
//! against a real archive directory, not guessed. See
//! `docs/architecture/frontend-boot.md` for how each one was found and what it
//! contains.
//!
//! # What is deliberately *not* here
//!
//! Nothing about how a Pulse file decodes. `.vex` class IDs, the `WO Track`
//! header shape and the handling-stats schema all live in `oag-formats`,
//! selected from each file's own version word, because those are properties of a
//! format version rather than of a release - Pulse and Pure differ there, but a
//! decoder learns which it is holding by reading the file, never by being told
//! which disc it came off.
//!
//! [ADR-0022]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0022-title-packages.md

pub mod frontend;
pub mod hud;
pub mod loading;
pub mod movies;
pub mod race;
pub mod textures;

use oag_assets::{Archives, Result};
use oag_title::{ArchiveCandidates, ForeignSerial, Platform, Title};

/// Wipeout Pulse, as the asset layer needs to know it.
pub const TITLE: &Title = &Title {
    name: "Wipeout Pulse",
    archives: ArchiveCandidates {
        data: DATA_CANDIDATES,
        fe: FE_CANDIDATES,
    },
    foreign_serials: FOREIGN_SERIALS,
};

/// The bulk archive's candidates, in the order they are tried.
const DATA_CANDIDATES: &[(&str, Platform)] = &[
    (archives::DATA, Platform::Psp),
    (archives::ps2::DATA, Platform::Ps2),
];

/// The companion archive's candidates, in the order they are tried.
const FE_CANDIDATES: &[(&str, Platform)] = &[
    (archives::FE, Platform::Psp),
    (archives::ps2::FE, Platform::Ps2),
];

/// Serials positively identified as a Studio Liverpool title other than
/// Wipeout Pulse, each backed by an owned disc recorded in
/// `docs/reverse-engineering/source-images.md`.
///
/// **Not the inverse of a Pulse allow-list.** The two Pulse serials this
/// project has actually verified (`UCUS-98712`, `SCES-54748`) are not the
/// full universe of legitimate Pulse pressings - an unconfirmed EU PSP
/// release (`UCES-00465`) is already implied by evidence recorded on
/// [`hashes::DEVPUB_REEL_SCEE`]'s doc comment. Allow-listing would hard-reject
/// a real player's own legitimately-owned disc, which is worse than not
/// checking at all. So this table only rules a source *out*: a serial absent
/// from it gets no verdict and still has to find its own archive by name,
/// same as before this table existed.
const FOREIGN_SERIALS: &[ForeignSerial] = &[ForeignSerial {
    serial: "UCUS-98612",
    title: "Wipeout Pure",
}];

/// Opens whichever archives `source` carries, as Wipeout Pulse.
///
/// A one-line convenience over [`Archives::open`] so a caller that already knows
/// which title it wants does not repeat [`TITLE`] at every call site.
///
/// # Errors
///
/// Propagates [`Archives::open`].
pub fn open(source: &str) -> Result<Archives> {
    Archives::open(source, TITLE)
}

/// [`open`], with downloadable content mounted behind the source.
///
/// The packs are Pulse's, so the title they mount behind is Pulse's too - see
/// [`Archives::open_with_packs`] for the search order and why the disc wins a
/// collision.
///
/// # Errors
///
/// Propagates [`Archives::open_with_packs`].
pub fn open_with_packs(source: &str, packs: Vec<oag_assets::dlc::Pack>) -> Result<Archives> {
    Archives::open_with_packs(source, TITLE, packs)
}

/// Reads a front-end or HUD image the XML named, from wherever this source
/// keeps it.
///
/// The declared name is tried first, so the PSP path is exactly
/// [`Archives::read_name`] and needs no console test. Only when that misses does
/// the PS2 substitution in [`PS2_IMAGES`] get a look, and an image that is in
/// neither still fails with the original name in the message -
/// `Data\FE\Images\gameshare_backdrop.mip` really is absent from the PS2 disc
/// and should keep saying so.
///
/// This lives here rather than on [`Archives`] because the substitution table is
/// a fact about Pulse's PS2 pressing, not about archives in general.
///
/// # Errors
///
/// [`oag_assets::Error::NoSuchEntry`] naming the entry the caller asked for.
pub fn read_image(archives: &mut Archives, name: &str) -> Result<Vec<u8>> {
    match archives.read_name(name) {
        Ok(blob) => Ok(blob),
        Err(original) => {
            let Some(hash) = ps2_image_hash(name) else {
                return Err(original);
            };
            archives.read_hash(hash).map_err(|_| original)
        }
    }
}

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

    /// The four archives a PS2 Pulse disc ships.
    ///
    /// **File names only, and deliberately so.** The directory holding them is
    /// derived from the disc's own serial - `54748/` on SCES-54748 - so a path
    /// constant would be right for one pressing and wrong for the next.
    /// [`oag_assets::Layout::resolve`] finds them by name instead, which needs no
    /// assumption about how a serial becomes a directory.
    ///
    /// See `docs/ps2/pulse-disc-layout.md`.
    pub mod ps2 {
        /// Tracks, ships and handling: the [`super::DATA`] analogue, 7,200
        /// entries in the same WAD container the PSP uses.
        pub const DATA: &str = "WADS2.WAD";
        /// The 193-entry companion archive. Also carries models, which is why
        /// it is searched rather than assumed redundant.
        pub const FE: &str = "WADSP.WAD";
        /// Music, in a container with a different header. Not parsed.
        pub const MUSIC: &str = "PS2MUSIC.WAD";
        /// 85 MiB at entropy 0.084, header unread. Not parsed.
        pub const PRERACE: &str = "PRERACE.WAD";
    }
}

/// Entry names inside `Data.wad`.
pub mod names {
    /// The front-end root: every boot screen, the `FEGlobals` variables, and
    /// the `LoadXML` list that pulls in the rest of the menus.
    pub const FRONTEND_ROOT: &str = r"Data\Plugins\PI001\GUI\Skin.xml";

    /// The game plugin's own definition: which circuits, teams, ship models and
    /// music tracks this release carries.
    ///
    /// `PI001` is the game plugin, where `PI008`-`PI012` are the language ones.
    /// The circuits are the part this project reads today - see
    /// `oag_game::catalogue` - and they are **entries rather than directories**:
    /// two of them can name one environment and differ only by `Reversed`.
    pub const GAME_PLUGIN_DEFINITION: &str = r"Data\Plugins\PI001\Definition.xml";

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
    /// The dev/pub reel `Intro Screen->IntroMovie1` plays, in its three
    /// regional cuts.
    ///
    /// **Not a boot movie.** The disc's own boot opens `Data\Movies\Intro.PMF`
    /// and `Data\Movies\Backdrop.PMF` and nothing else, and never enters the
    /// state whose counters these fit; where they *are* played is unestablished.
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
    /// This one is what `oag-game --reel` defaults to, because the executable on the image
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

    /// The PS2 `WADS2.WAD` entry holding the in-race HUD atlas, the one the
    /// five layouts call `Data\HUD\Textures\PulseHUD.mip`.
    ///
    /// **That name hashes to nothing on the PS2 disc**, and neither does any
    /// spelling of it: 60 path, case and extension variants were tried against
    /// all 7,393 hashes in both archives and none hit. The entry was found by
    /// its picture instead - see [`PS2_IMAGES`] for the method and the
    /// evidence.
    pub const PS2_HUD_ATLAS: u32 = 0xbeaf_613c;
    /// A second, byte-identical copy of [`PS2_HUD_ATLAS`], at entry 3583 where
    /// the first is at 3518.
    ///
    /// Not an ambiguity to resolve: the PSP ships this atlas twice too, in
    /// `FE.wad` and `Data.wad`, byte-identical at 66,576 bytes. Both PS2 copies
    /// are 13,446 stored / 66,829 unpacked and both score 0.9999 against the
    /// PSP silhouette. Recorded so a reader who finds the second one knows it
    /// is the same picture.
    pub const PS2_HUD_ATLAS_DUPLICATE: u32 = 0xf012_b5af;
    /// The PS2 entry holding `Data\FE\Images\pulse_logo.mip`, entry 3421.
    ///
    /// This is the missing half of the `Show Logo` screen recorded in
    /// `HANDOVER.md`.
    pub const PS2_PULSE_LOGO: u32 = 0x1e6c_873e;
    /// The PS2 entry holding `Data\FE\Images\pulse_assets.mip`, entry 3420 -
    /// immediately before [`PS2_PULSE_LOGO`].
    pub const PS2_PULSE_ASSETS: u32 = 0x0d31_af1b;
}

/// What a PS2 archive calls the front-end and HUD images the XML asks for by
/// their PSP `.mip` names.
///
/// # Why a table and not a rule
///
/// The PS2 release keeps its raster images as ordinary PS2 textures (see
/// `oag_formats::ps2_texture`) in `WADS2.WAD`, but **not under the
/// names its own XML uses**. `Data\HUD\Textures\PulseHUD.mip` hashes to
/// `57d37d8c`, which is in neither archive, and no variation on the spelling
/// helps: 60 candidates across path shape, case and extension were hashed
/// against all 7,393 entry hashes on the disc and every one missed. The
/// directory-position rules that solve the PS2's models and fonts do not apply
/// either - the five `*_HUD.xml` entries sit in a run of XML with no texture
/// anywhere near them. **How the game itself performs this lookup is not
/// known**; a repeated 3,656-byte blob next to the HUD assets looked like a
/// name table and was checked and ruled out (none of its 914 words is an entry
/// hash).
///
/// # How the entries were found, and how sure it is
///
/// By the picture. Each PSP `.mip` was decoded, reduced to a silhouette - one
/// bit per pixel, "is this texel's palette entry transparent" - and compared
/// against every same-shaped PS2 texture on the disc. The measure is
/// independent of palette order, which differs between the two builds.
///
/// | PSP name | PS2 entry | Silhouette agreement |
/// | --- | ---: | ---: |
/// | `Data\HUD\Textures\PulseHUD.mip` | 3518, 3583 | 0.9999, over 614 same-shape candidates |
/// | `Data\FE\Images\pulse_logo.mip` | 3421 | 0.9946 |
/// | `Data\FE\Images\pulse_assets.mip` | 3420 | 0.9999 |
///
/// Each is separated from the runner-up by a wide margin (0.71, 0.51, 0.51),
/// and all three were then decoded and looked at: 3518 is the speed and shield
/// bars with the weapon icons, 3421 is the Wipeout Pulse wordmark. For the HUD
/// atlas there is a fourth, independent agreement: the `U`/`V`/`TxtrWidth`/
/// `TxtrHeight` boxes in the PS2 layouts are **identical** to the PSP's, so the
/// two discs index the same 256x256 arrangement.
///
/// # One image really is absent
///
/// `Data\FE\Images\gameshare_backdrop.mip` is **not on the PS2 disc**, which is
/// a finding rather than a gap - Game Sharing is a PSP ad-hoc feature. Note the
/// silhouette test cannot say so: that image is 93.75 % opaque, so every
/// candidate scores 0.9375 trivially. Correlating the picture instead settles
/// it, at 0.02 across all 28 same-shaped candidates. Do not re-run the weaker
/// test on it.
pub const PS2_IMAGES: &[(&str, u32)] = &[
    (r"Data\HUD\Textures\PulseHUD.mip", hashes::PS2_HUD_ATLAS),
    (r"Data\FE\Images\pulse_logo.mip", hashes::PS2_PULSE_LOGO),
    (r"Data\FE\Images\pulse_assets.mip", hashes::PS2_PULSE_ASSETS),
];

/// The PS2 entry hash for an image the XML names the PSP way, if there is one.
///
/// See [`PS2_IMAGES`] for how the mapping was established.
#[must_use]
pub fn ps2_image_hash(name: &str) -> Option<u32> {
    PS2_IMAGES
        .iter()
        .find(|(psp, _)| psp.eq_ignore_ascii_case(name))
        .map(|(_, hash)| *hash)
}

/// The plugins that carry a language, in the order the disc lists them.
///
/// The plugin id is the only stable handle: the language's own name is inside
/// the plugin, not in its path. A PAL disc has a different set, so this is read
/// back from the archive rather than trusted.
pub const LANGUAGE_PLUGINS: &[&str] = &["PI008", "PI009", "PI010", "PI011", "PI012"];

#[cfg(test)]
mod tests {
    use super::*;

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

    /// The deny-list rules a source *out* and never in: an unknown serial gets
    /// no verdict. See [`FOREIGN_SERIALS`].
    #[test]
    fn only_a_known_foreign_serial_gets_a_verdict() {
        assert_eq!(TITLE.foreign_title("UCUS-98612"), Some("Wipeout Pure"));
        assert_eq!(TITLE.foreign_title("UCUS-98712"), None);
        assert_eq!(TITLE.foreign_title("SCES-54748"), None);
        assert_eq!(TITLE.foreign_title("UCES-00465"), None);
    }

    /// Both consoles' archives are offered, bulk before companion, so a "nothing
    /// here" error names everything that was looked for.
    #[test]
    fn every_archive_name_is_reported_bulk_first() {
        assert_eq!(
            TITLE.archive_names(),
            vec![
                archives::DATA,
                archives::ps2::DATA,
                archives::FE,
                archives::ps2::FE,
            ]
        );
    }
}
