//! Where Wipeout Pulse keeps the things the front end needs.
//!
//! Every constant here was resolved by hashing a candidate name and matching it
//! against a real archive directory, not guessed. See
//! `docs/architecture/frontend-boot.md` for how each one was found and what it
//! contains.

use oag_disc::DiscImage;

/// Re-exported because [`Layout::platform`] is one, and a caller that reads that
/// field should not have to depend on `oag-disc` to name what it read.
pub use oag_disc::Platform;

use crate::{Archive, Error, Result};

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
    /// [`super::Layout::resolve`] finds them by name instead, which needs no
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
}

/// The plugins that carry a language, in the order the disc lists them.
///
/// The plugin id is the only stable handle: the language's own name is inside
/// the plugin, not in its path. A PAL disc has a different set, so this is read
/// back from the archive rather than trusted.
pub const LANGUAGE_PLUGINS: &[&str] = &["PI008", "PI009", "PI010", "PI011", "PI012"];

/// Where one source keeps its archives.
///
/// The two releases name their archives differently and put them in different
/// places, so nothing above this layer should spell either layout out. A caller
/// asks for the bulk archive and gets whichever of `Data.wad` and `WADS2.WAD`
/// the source actually carries.
///
/// **Found by name, not derived from the platform.** The candidates are tried in
/// order against the source's own file list, so a disc that identifies as
/// neither console still opens if it holds an archive one of them would
/// recognise, and a PS2 pressing whose serial directory is not `54748/` needs no
/// change here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Layout {
    /// What the source says it is, or what its archives imply when it says
    /// nothing. Carried for reports and error messages; nothing branches on it,
    /// because every decode this project has is chosen by the *data* rather than
    /// by the disc it came off.
    pub platform: Platform,
    /// The bulk archive: tracks, ships, handling, and on the PSP the movies,
    /// screens and string tables too.
    pub data: String,
    /// The companion archive, when the source has one.
    pub fe: Option<String>,
}

/// The bulk archive's candidates, in the order they are tried.
const DATA_CANDIDATES: [(&str, Platform); 2] = [
    (archives::DATA, Platform::Psp),
    (archives::ps2::DATA, Platform::Ps2),
];

/// The companion archive's candidates, in the order they are tried.
const FE_CANDIDATES: [(&str, Platform); 2] = [
    (archives::FE, Platform::Psp),
    (archives::ps2::FE, Platform::Ps2),
];

impl Layout {
    /// Works out which archives `source` carries and how to open them.
    ///
    /// `source` is either a disc image or a directory previously extracted with
    /// `oag-unpack`, which is what ADR-0004 means by "load from the user's own
    /// originals".
    ///
    /// # Errors
    ///
    /// [`Error::NoArchive`] when nothing in the source matches any candidate,
    /// naming every candidate looked for. Reading the source itself propagates.
    pub fn resolve(source: &str) -> Result<Self> {
        let (mut platform, files) = survey(source)?;

        let data = pick(source, &files, &DATA_CANDIDATES).ok_or_else(|| Error::NoArchive {
            looked_in: source.to_string(),
            platform: platform.to_string(),
            looked_for: DATA_CANDIDATES
                .iter()
                .map(|(name, _)| (*name).to_string())
                .collect(),
        })?;
        let fe = pick(source, &files, &FE_CANDIDATES);

        // An extracted directory that holds only `USRDIR` has no `UMD_DATA.BIN`
        // and no `SYSTEM.CNF` to identify it, so the archive that matched is the
        // only thing left that knows. Never the other way round: a disc that
        // says what it is is believed.
        if platform == Platform::Unknown {
            platform = data.1;
        }

        Ok(Self {
            platform,
            data: data.0,
            fe: fe.map(|(spec, _)| spec),
        })
    }

    /// One line for a load report: what was found and what it was found on.
    #[must_use]
    pub fn describe(&self) -> String {
        match &self.fe {
            Some(fe) => format!("{} source: {} and {}", self.platform, self.data, fe),
            None => format!(
                "{} source: {}, with no companion archive",
                self.platform, self.data
            ),
        }
    }
}

/// The archives one source carries, opened.
///
/// Each holds its own handle on the image. That costs a little memory and
/// nothing else, and it keeps every archive independently seekable.
#[derive(Debug)]
pub struct Archives {
    /// Which archives these are and where they came from.
    pub layout: Layout,
    /// The bulk archive: `Data.wad` on the PSP, `WADS2.WAD` on the PS2.
    pub data: Archive,
    /// The companion archive: `FE.wad` on the PSP, `WADSP.WAD` on the PS2.
    pub fe: Option<Archive>,
}

impl Archives {
    /// Resolves the layout of `source` and opens what it names.
    ///
    /// # Errors
    ///
    /// Propagates [`Layout::resolve`], and a directory or blob that will not
    /// open or parse.
    pub fn open(source: &str) -> Result<Self> {
        let layout = Layout::resolve(source)?;
        let data = Archive::open(&layout.data)?;
        let fe = layout.fe.as_deref().map(Archive::open).transpose()?;
        Ok(Self { layout, data, fe })
    }

    /// The specifier of the archive holding `name`, searching the bulk archive
    /// first.
    ///
    /// **Both are searched, and they have to be.** On the PSP `pulse_logo.mip`
    /// is in `FE.wad` *and* `Data.wad` at the same size, which makes one look
    /// sufficient; `gameshare_backdrop.mip` is in `Data.wad` only, which proves
    /// it is not. On the PS2 both `WADS2.WAD` and `WADSP.WAD` carry models.
    #[must_use]
    pub fn locate(&self, name: &str) -> Option<&str> {
        let hash = oag_formats::wad::hash_name(name);
        if self.data.index_of_hash(hash).is_some() {
            return Some(self.data.label());
        }
        self.fe
            .as_ref()
            .filter(|fe| fe.index_of_hash(hash).is_some())
            .map(Archive::label)
    }

    /// Reads and decompresses `name` out of whichever archive holds it.
    ///
    /// # Errors
    ///
    /// [`Error::NoSuchEntry`] against the bulk archive when neither has it, so
    /// the message names the archive a reader would look in first.
    pub fn read_name(&mut self, name: &str) -> Result<Vec<u8>> {
        let hash = oag_formats::wad::hash_name(name);
        if let Some(fe) = self.fe.as_mut()
            && self.data.index_of_hash(hash).is_none()
            && fe.index_of_hash(hash).is_some()
        {
            return fe.read_name(name);
        }
        self.data.read_name(name)
    }

    /// Reads the entry immediately before `name`'s own entry, in whichever
    /// archive holds `name`.
    ///
    /// This is where a PS2 model's texture set sits: not addressed by any
    /// name or hash a model declares, but by directory position. Checked
    /// against every recovered ship team independently, not just one: each
    /// team's `Ship.vex` has the archive entry directly before it decode as a
    /// texture set with exactly as many entries as the model has `Texture`
    /// nodes. See `docs/formats/ps2-texture.md`.
    ///
    /// # Errors
    ///
    /// [`Error::NoSuchEntry`] when `name` itself is not found, or its entry is
    /// first in the directory and nothing precedes it.
    pub fn read_preceding(&mut self, name: &str) -> Result<Vec<u8>> {
        let hash = oag_formats::wad::hash_name(name);
        let in_fe = self.data.index_of_hash(hash).is_none()
            && self
                .fe
                .as_ref()
                .is_some_and(|fe| fe.index_of_hash(hash).is_some());

        let archive = if in_fe {
            self.fe.as_mut().expect("checked above")
        } else {
            &mut self.data
        };
        let index = archive
            .index_of_hash(hash)
            .ok_or_else(|| Error::NoSuchEntry {
                archive: archive.label().to_string(),
                hash,
                name: Some(name.to_string()),
            })?;
        let preceding = index.checked_sub(1).ok_or_else(|| Error::NoSuchEntry {
            archive: archive.label().to_string(),
            hash,
            name: Some(format!("{name} (first in its directory)")),
        })?;
        archive.read(preceding)
    }
}

/// Reads a file that sits loose on `source`'s own filesystem - not inside any
/// WAD - trying each of `candidates` in order and matching the same way
/// [`Layout::resolve`] matches an archive: by trailing path components,
/// case-insensitively, so a disc's serial-named directory does not need to be
/// spelled out.
///
/// This is for containers no archive addresses by hash: the PS2's movies are
/// plain files under `DATA/MOVIES/`, unlike the PSP's, which are `Data.wad`
/// entries. See `docs/ps2/pulse-disc-layout.md`.
///
/// `Ok(None)` when nothing in the source matches any candidate. Not an error:
/// a caller answering a default treats "not present" as "this source has
/// none," the same distinction [`Archives`]'s own callers make for a WAD
/// entry. A name the caller actually asked for should still be an error, at
/// the call site rather than here.
///
/// # Errors
///
/// Propagates a source that will not open at all.
pub fn read_loose_file(source: &str, candidates: &[&str]) -> Result<Option<(String, Vec<u8>)>> {
    let path = std::path::Path::new(source);

    if path.is_dir() {
        let mut files = Vec::new();
        collect(path, "", &mut files);
        let Some(file) = files
            .iter()
            .find(|f| candidates.iter().any(|c| names(f, c)))
        else {
            return Ok(None);
        };
        let data = std::fs::read(path.join(file))?;
        return Ok(Some((file.clone(), data)));
    }

    let mut disc = DiscImage::open(source)?;
    let found = disc
        .entries()?
        .iter()
        .find(|entry| !entry.is_directory && candidates.iter().any(|c| names(&entry.path, c)))
        .cloned();
    let Some(entry) = found else {
        return Ok(None);
    };
    let data = disc.read_entry(&entry)?;
    Ok(Some((entry.path, data)))
}

/// Every file in `source`, and what `source` says it is.
///
/// A disc is asked directly; a directory is walked, and identified from the
/// files it turned out to hold.
fn survey(source: &str) -> Result<(Platform, Vec<String>)> {
    let path = std::path::Path::new(source);
    if path.is_dir() {
        let mut files = Vec::new();
        collect(path, "", &mut files);
        // Sorted so that two candidates matching the same role resolve the same
        // way on every filesystem, rather than in readdir order.
        files.sort();
        let platform = platform_of(&files);
        return Ok((platform, files));
    }

    let mut disc = DiscImage::open(source)?;
    let platform = disc.identify()?.platform;
    let files = disc
        .entries()?
        .iter()
        .filter(|entry| !entry.is_directory)
        .map(|entry| entry.path.clone())
        .collect();
    Ok((platform, files))
}

/// Every file under `dir`, as paths relative to the walk's root.
///
/// Errors are swallowed rather than propagated: an unreadable subdirectory of an
/// extract is a reason to not find an archive there, and the caller's
/// "nothing matched, here is what I looked for" says more than an `io::Error`
/// about one directory would.
fn collect(dir: &std::path::Path, prefix: &str, into: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let relative = if prefix.is_empty() {
            name
        } else {
            format!("{prefix}/{name}")
        };
        match entry.file_type() {
            Ok(kind) if kind.is_dir() => collect(&entry.path(), &relative, into),
            Ok(_) => into.push(relative),
            Err(_) => {}
        }
    }
}

/// Which console a file list belongs to, by the file each one identifies itself
/// with. The same two files [`oag_disc::platform`] reads.
fn platform_of(files: &[String]) -> Platform {
    if files.iter().any(|f| names(f, "UMD_DATA.BIN")) {
        Platform::Psp
    } else if files.iter().any(|f| names(f, "SYSTEM.CNF")) {
        Platform::Ps2
    } else {
        Platform::Unknown
    }
}

/// The first candidate present in `files`, as an openable specifier.
fn pick(
    source: &str,
    files: &[String],
    candidates: &[(&str, Platform)],
) -> Option<(String, Platform)> {
    candidates.iter().find_map(|(candidate, platform)| {
        files
            .iter()
            .find(|file| names(file, candidate))
            .map(|file| (archive_spec(source, file), *platform))
    })
}

/// Whether `path` names `candidate`: the same trailing path components,
/// case-insensitively.
///
/// Matching a tail rather than a whole path is what lets one candidate list hold
/// both `PSP_GAME/USRDIR/Data.wad`, which is a fixed location, and `WADS2.WAD`,
/// which sits in a directory named after the disc's serial.
fn names(path: &str, candidate: &str) -> bool {
    let path = path.replace('\\', "/");
    let candidate = candidate.replace('\\', "/");
    let (path, candidate) = (path.as_bytes(), candidate.as_bytes());

    let Some(start) = path.len().checked_sub(candidate.len()) else {
        return false;
    };
    if !path[start..].eq_ignore_ascii_case(candidate) {
        return false;
    }
    start == 0 || path[start - 1] == b'/'
}

/// Builds the specifier for one archive inside `source`.
///
/// A directory is joined with a separator; anything else is treated as a disc
/// image and joined with a colon, which is the form [`Archive::open`] takes.
fn archive_spec(source: &str, relative: &str) -> String {
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

    /// A directory shaped like an extract, under a name of its own so two tests
    /// never collide.
    fn extract(name: &str, files: &[&str]) -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!("oag-assets-layout-{name}"));
        std::fs::remove_dir_all(&root).ok();
        for file in files {
            let path = root.join(file);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            // Never opened: resolving a layout is a question about paths, so
            // these do not have to be WADs to answer it.
            std::fs::write(&path, b"").unwrap();
        }
        root
    }

    #[test]
    fn a_psp_extract_resolves_to_the_psp_archives() {
        let root = extract(
            "psp",
            &[
                "UMD_DATA.BIN",
                "PSP_GAME/USRDIR/Data.wad",
                "PSP_GAME/USRDIR/FE.wad",
                "PSP_GAME/USRDIR/BEData.wad",
            ],
        );
        let source = root.to_str().unwrap();

        let layout = Layout::resolve(source).unwrap();
        assert_eq!(layout.platform, Platform::Psp);
        assert_eq!(layout.data, format!("{source}/PSP_GAME/USRDIR/Data.wad"));
        assert_eq!(
            layout.fe.as_deref(),
            Some(format!("{source}/PSP_GAME/USRDIR/FE.wad").as_str())
        );

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn a_ps2_extract_resolves_through_its_serial_directory() {
        // The directory is named after the disc's serial, so nothing may depend
        // on `54748` in particular: the archive is found by its own name.
        let root = extract(
            "ps2",
            &[
                "SYSTEM.CNF",
                "12345/WADS2.WAD",
                "12345/WADSP.WAD",
                "12345/PS2MUSIC.WAD",
            ],
        );
        let source = root.to_str().unwrap();

        let layout = Layout::resolve(source).unwrap();
        assert_eq!(layout.platform, Platform::Ps2);
        assert_eq!(layout.data, format!("{source}/12345/WADS2.WAD"));
        assert_eq!(
            layout.fe.as_deref(),
            Some(format!("{source}/12345/WADSP.WAD").as_str())
        );

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn an_archive_alone_identifies_a_partial_extract() {
        // `oag-unpack` can be pointed at one directory, so the file that names
        // the console is often absent. The archive that matched is then the only
        // thing that knows which one it is.
        let root = extract("partial", &["WADS2.WAD"]);
        let source = root.to_str().unwrap();

        let layout = Layout::resolve(source).unwrap();
        assert_eq!(layout.platform, Platform::Ps2);
        assert_eq!(layout.data, format!("{source}/WADS2.WAD"));
        assert_eq!(layout.fe, None);

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn a_source_with_no_archive_says_what_it_looked_for() {
        let root = extract("empty", &["SYSTEM.CNF", "IOP/LIBSD.IRX"]);
        let source = root.to_str().unwrap();

        let error = Layout::resolve(source).unwrap_err().to_string();
        assert!(error.contains("PS2"), "{error}");
        assert!(error.contains(archives::DATA), "{error}");
        assert!(error.contains(archives::ps2::DATA), "{error}");

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn an_archive_name_is_matched_on_a_component_boundary() {
        assert!(names("54748/WADS2.WAD", archives::ps2::DATA));
        assert!(names("54748/wads2.wad", archives::ps2::DATA));
        assert!(names("WADS2.WAD", archives::ps2::DATA));
        assert!(names(r"54748\WADS2.WAD", archives::ps2::DATA));
        assert!(names("PSP_GAME/USRDIR/Data.wad", archives::DATA));

        // A name that merely ends with the candidate's characters is not that
        // candidate, which is the whole reason this is not `ends_with`.
        assert!(!names("54748/NOTWADS2.WAD", archives::ps2::DATA));
        assert!(!names("WADS2.WADX", archives::ps2::DATA));
        // The PSP candidate is a path, so a bare file of the same name in some
        // other directory does not answer for it.
        assert!(!names("elsewhere/Data.wad", archives::DATA));
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
