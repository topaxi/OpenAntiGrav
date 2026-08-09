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

    /// The PS2 `WADS2.WAD` entry holding the in-race HUD atlas, the one the
    /// five layouts call `Data\HUD\Textures\PulseHUD.mip`.
    ///
    /// **That name hashes to nothing on the PS2 disc**, and neither does any
    /// spelling of it: 60 path, case and extension variants were tried against
    /// all 7,393 hashes in both archives and none hit. The entry was found by
    /// its picture instead - see [`super::PS2_IMAGES`] for the method and the
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
/// The PS2 release keeps its raster images as ordinary [PS2
/// textures](oag_formats::ps2_texture) in `WADS2.WAD`, but **not under the
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
const OTHER_TITLES: &[(&str, &str)] = &[("UCUS-98612", "Wipeout Pure")];

/// `Err(Error::WrongTitle)` when `serial` is positively known to belong to a
/// title other than Wipeout Pulse.
///
/// Called from [`survey`], before archive-name matching runs, so a Pure disc
/// never reaches [`Layout::resolve`]'s `Data.wad`/`FE.wad` search - it ships
/// those under the identical names Pulse does, so name matching alone would
/// open it exactly as if it were Pulse.
fn reject_other_title(looked_in: &str, serial: &str) -> Result<()> {
    if let Some((_, title)) = OTHER_TITLES.iter().find(|(known, _)| *known == serial) {
        return Err(Error::WrongTitle {
            looked_in: looked_in.to_string(),
            serial: serial.to_string(),
            title: (*title).to_string(),
        });
    }
    Ok(())
}

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
/// change here. This is unrelated to [`reject_other_title`]: that check runs on
/// the disc's *serial* in [`survey`], before this layer's name matching starts.
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
    /// Mounted [downloadable content](crate::dlc), searched after the source's
    /// own archives so the disc always wins a collision.
    ///
    /// Empty unless the caller passed packs to [`Archives::open_with_packs`].
    pub packs: Vec<Archive>,
    /// What those packs declare: the `PI_Team` and `PI_Track` fragments out of
    /// [`crate::dlc::Pack::manifests`], concatenated in mount order.
    ///
    /// Carried here rather than handed back separately because every caller
    /// that mounts packs also needs to know what they added, and threading two
    /// values through the same six call sites is two chances to thread one.
    pub manifests: Vec<String>,
}

impl Archives {
    /// Resolves the layout of `source` and opens what it names.
    ///
    /// # Errors
    ///
    /// Propagates [`Layout::resolve`], and a directory or blob that will not
    /// open or parse.
    pub fn open(source: &str) -> Result<Self> {
        Self::open_with_packs(source, Vec::new())
    }

    /// As [`Archives::open`], with downloadable content mounted behind the
    /// source's own archives.
    ///
    /// # Ordering, and why the disc wins
    ///
    /// A pack is searched only after `data` and `fe` have both missed. Nothing
    /// on the shipped discs collides with pack content - the four teams and
    /// four circuits a pack adds are absent from the base archives, which is
    /// what makes them downloadable content - so the order is not load-bearing
    /// for any real pack. It is chosen for the case that is not real: a
    /// modified pack that shadowed a disc entry would change the base game
    /// silently, and this makes that impossible.
    ///
    /// Between packs, first mounted wins. The shipped packs overlap only where
    /// they carry byte-identical copies of the same entry, so that rule cannot
    /// pick between differing content; see
    /// [`docs/formats/dlc-pack.md`](../../../docs/formats/dlc-pack.md).
    ///
    /// # Errors
    ///
    /// As [`Archives::open`]. The packs are already open, so mounting them
    /// cannot fail.
    pub fn open_with_packs(source: &str, packs: Vec<crate::dlc::Pack>) -> Result<Self> {
        let layout = Layout::resolve(source)?;
        let data = Archive::open(&layout.data)?;
        let fe = layout.fe.as_deref().map(Archive::open).transpose()?;

        let mut manifests = Vec::new();
        let mut mounted = Vec::new();
        for pack in packs {
            manifests.extend(pack.manifests);
            mounted.extend(pack.archives);
        }

        Ok(Self {
            layout,
            data,
            fe,
            packs: mounted,
            manifests,
        })
    }

    /// Which mounted archive holds this hash, if any.
    ///
    /// The one place the search order lives. Returning a position rather than a
    /// reference keeps it usable from `&self` and from `&mut self` alike, which
    /// is what lets [`Self::locate`] stay immutable while the readers below
    /// share the same rule.
    fn holder_of(&self, hash: u32) -> Option<Held> {
        if self.data.index_of_hash(hash).is_some() {
            return Some(Held::Data);
        }
        if self
            .fe
            .as_ref()
            .is_some_and(|fe| fe.index_of_hash(hash).is_some())
        {
            return Some(Held::Fe);
        }
        self.packs
            .iter()
            .position(|pack| pack.index_of_hash(hash).is_some())
            .map(Held::Pack)
    }

    fn held(&mut self, at: Held) -> &mut Archive {
        match at {
            Held::Data => &mut self.data,
            Held::Fe => self.fe.as_mut().expect("holder_of found it here"),
            Held::Pack(index) => &mut self.packs[index],
        }
    }

    /// The specifier of the archive holding `name`, searching the bulk archive
    /// first.
    ///
    /// **Both are searched, and they have to be.** On the PSP `pulse_logo.mip`
    /// is in `FE.wad` *and* `Data.wad` at the same size, which makes one look
    /// sufficient; `gameshare_backdrop.mip` is in `Data.wad` only, which proves
    /// it is not. On the PS2 both `WADS2.WAD` and `WADSP.WAD` carry models.
    /// Mounted packs come last; see [`Self::open_with_packs`].
    #[must_use]
    pub fn locate(&self, name: &str) -> Option<&str> {
        let hash = oag_formats::wad::hash_name(name);
        Some(match self.holder_of(hash)? {
            Held::Data => self.data.label(),
            Held::Fe => self.fe.as_ref().expect("holder_of found it here").label(),
            Held::Pack(index) => self.packs[index].label(),
        })
    }

    /// Reads and decompresses `name` out of whichever archive holds it.
    ///
    /// # Errors
    ///
    /// [`Error::NoSuchEntry`] against the bulk archive when nothing has it, so
    /// the message names the archive a reader would look in first.
    pub fn read_name(&mut self, name: &str) -> Result<Vec<u8>> {
        let hash = oag_formats::wad::hash_name(name);
        match self.holder_of(hash) {
            Some(at) => self.held(at).read_name(name),
            None => self.data.read_name(name),
        }
    }

    /// Reads a front-end or HUD image the XML named, from wherever this source
    /// keeps it.
    ///
    /// The declared name is tried first, so the PSP path is exactly
    /// [`Self::read_name`] and needs no platform test. Only when that misses
    /// does the PS2 substitution in [`PS2_IMAGES`] get a look, and an image
    /// that is in neither still fails with the original name in the message -
    /// `Data\FE\Images\gameshare_backdrop.mip` really is absent from the PS2
    /// disc and should keep saying so.
    ///
    /// # Errors
    ///
    /// [`Error::NoSuchEntry`] naming the entry the caller asked for.
    pub fn read_image(&mut self, name: &str) -> Result<Vec<u8>> {
        match self.read_name(name) {
            Ok(blob) => Ok(blob),
            Err(original) => {
                let Some(hash) = ps2_image_hash(name) else {
                    return Err(original);
                };
                self.read_hash(hash).map_err(|_| original)
            }
        }
    }

    /// Reads the entry with this name hash out of whichever archive holds it.
    ///
    /// The hash-addressed counterpart of [`Self::read_name`], for the entries
    /// whose names are not recovered - see [`hashes`].
    ///
    /// # Errors
    ///
    /// [`Error::NoSuchEntry`] against the bulk archive when nothing has it.
    pub fn read_hash(&mut self, hash: u32) -> Result<Vec<u8>> {
        match self.holder_of(hash) {
            Some(at) => self.held(at).read_hash(hash),
            None => self.data.read_hash(hash),
        }
    }

    /// Reads a `.fnt` and returns it with its glyph atlas attached, wherever
    /// that atlas lives on this source.
    ///
    /// The two builds differ and the file itself says which is which: a PSP
    /// `.fnt` carries its atlas inside it, a PS2 `.fnt` stops where the atlas
    /// would begin and keeps the glyph sheet in the **next archive entry**, as
    /// a [`PSMT4`](oag_formats::ps2_texture::Layout::Psmt4) texture. So the
    /// embedded atlas is tried first and the following entry is only read when
    /// there is none, which means a PSP source never touches the fallback and
    /// this needs no platform test.
    ///
    /// # Errors
    ///
    /// [`Error::NoSuchEntry`] when `name` is not on this source, and
    /// [`Error::Font`] when the metrics, or the atlas the following entry is
    /// supposed to hold, do not decode.
    pub fn read_font(&mut self, name: &str) -> Result<oag_formats::fnt::Font> {
        use oag_formats::{fnt, ps2_texture};

        let blob = self.read_name(name)?;
        let metrics = fnt::Metrics::parse(&blob).map_err(|source| Error::Font {
            name: name.to_string(),
            source,
        })?;
        if metrics.has_embedded_atlas(&blob) {
            return fnt::Font::parse(&blob).map_err(|source| Error::Font {
                name: name.to_string(),
                source,
            });
        }

        let atlas = self.read_following(name)?;
        let texture = ps2_texture::parse(&atlas).map_err(|source| Error::FontAtlas {
            name: name.to_string(),
            source,
        })?;
        // The GS calls 128 fully opaque; `alpha_at` is read as 0-255 coverage.
        let palette = texture
            .palette
            .iter()
            .map(|&[r, g, b, a]| [r, g, b, a.saturating_mul(2)])
            .collect();
        fnt::Font::with_atlas(
            metrics,
            texture.width,
            texture.height,
            palette,
            texture.indices,
        )
        .map_err(|source| Error::Font {
            name: name.to_string(),
            source,
        })
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
        self.read_neighbour(name, Neighbour::Before)
    }

    /// Reads the entry immediately after `name`'s own entry, in whichever
    /// archive holds `name`.
    ///
    /// The other half of the PS2's directory-position addressing: a model's
    /// texture set sits *before* it ([`Self::read_preceding`]), and a font's
    /// glyph atlas sits *after* it. All five `.fnt` entries in `WADS2.WAD` are
    /// followed by a `PSMT4` texture whose dimensions cover that font's own
    /// glyph boxes; see `docs/formats/fnt.md`.
    ///
    /// # Errors
    ///
    /// [`Error::NoSuchEntry`] when `name` itself is not found, or its entry is
    /// last in the directory and nothing follows it.
    pub fn read_following(&mut self, name: &str) -> Result<Vec<u8>> {
        self.read_neighbour(name, Neighbour::After)
    }

    /// Directory position is only meaningful **within one archive**, so the
    /// neighbour is always taken from whichever archive held `name` - never
    /// from a flattened view across all of them, which would hand a model the
    /// last entry of the previous archive and decode it as a texture set.
    fn read_neighbour(&mut self, name: &str, which: Neighbour) -> Result<Vec<u8>> {
        let hash = oag_formats::wad::hash_name(name);
        let archive = match self.holder_of(hash) {
            Some(at) => self.held(at),
            None => &mut self.data,
        };
        let index = archive
            .index_of_hash(hash)
            .ok_or_else(|| Error::NoSuchEntry {
                archive: archive.label().to_string(),
                hash,
                name: Some(name.to_string()),
            })?;
        let (neighbour, edge) = match which {
            Neighbour::Before => (index.checked_sub(1), "first"),
            Neighbour::After => (
                Some(index + 1).filter(|&next| next < archive.directory().entries.len()),
                "last",
            ),
        };
        let neighbour = neighbour.ok_or_else(|| Error::NoSuchEntry {
            archive: archive.label().to_string(),
            hash,
            name: Some(format!("{name} ({edge} in its directory)")),
        })?;
        archive.read(neighbour)
    }
}

/// Which mounted archive an entry was found in.
///
/// A position rather than a reference so [`Archives::holder_of`] can be
/// immutable; see it for the search order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Held {
    Data,
    Fe,
    Pack(usize),
}

/// Which side of an entry [`Archives::read_neighbour`] wants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Neighbour {
    Before,
    After,
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
/// A disc is asked directly, and rejected with [`Error::WrongTitle`] if its
/// serial positively identifies it as a title other than Pulse. A directory
/// is walked and identified from the files it turned out to hold - it carries
/// no disc serial to check, which is intentional: ADR-0004's "load from your
/// own already-unpacked originals" workflow stays permitted unconditionally.
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
    let info = disc.identify()?;
    if let Some(serial) = &info.serial {
        reject_other_title(source, serial)?;
    }
    let files = disc
        .entries()?
        .iter()
        .filter(|entry| !entry.is_directory)
        .map(|entry| entry.path.clone())
        .collect();
    Ok((info.platform, files))
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
    use crate::testing;

    /// Builds `Archives` over hand-authored files, so the search order can be
    /// tested without a disc. `Layout::resolve` is bypassed on purpose: what is
    /// under test is which archive answers, not how a source is recognised.
    fn mounted(
        dir: &std::path::Path,
        data: &[(&str, &[u8])],
        packs: Vec<crate::dlc::Pack>,
    ) -> Archives {
        let spec = testing::write_wad(dir, "Data.wad", data);
        let mut manifests = Vec::new();
        let mut mounted = Vec::new();
        for pack in packs {
            manifests.extend(pack.manifests);
            mounted.extend(pack.archives);
        }
        Archives {
            layout: Layout {
                platform: Platform::Psp,
                data: spec.clone(),
                fe: None,
            },
            data: Archive::open(&spec).expect("the test archive"),
            fe: None,
            packs: mounted,
            manifests,
        }
    }

    fn pack(dir: &std::path::Path, name: &str, entries: &[(&str, &[u8])]) -> crate::dlc::Pack {
        let sub = dir.join(name);
        std::fs::create_dir_all(&sub).expect("a test pack directory");
        let _ = testing::write_wad(&sub, "PACK.edat", entries);
        crate::dlc::open_dir(&sub).expect("a pack")
    }

    /// The rule from [`Archives::open_with_packs`]: a pack cannot shadow the
    /// disc, whatever it carries.
    #[test]
    fn the_source_wins_a_collision_with_a_pack() {
        let dir = testing::temp_dir("mount-disc-wins");
        let mut archives = mounted(
            &dir,
            &[("shared.bin", b"from the disc")],
            vec![pack(&dir, "one", &[("shared.bin", b"from the pack")])],
        );

        assert_eq!(archives.read_name("shared.bin").unwrap(), b"from the disc");
    }

    #[test]
    fn a_pack_answers_for_a_name_the_source_does_not_have() {
        let dir = testing::temp_dir("mount-pack-only");
        let mut archives = mounted(
            &dir,
            &[("base.bin", b"disc")],
            vec![pack(&dir, "one", &[("extra.bin", b"pack")])],
        );

        assert_eq!(archives.read_name("extra.bin").unwrap(), b"pack");
        assert_eq!(
            archives.locate("extra.bin"),
            Some(archives.packs[0].label()),
            "and it reports which archive answered"
        );
    }

    /// First mounted wins. The shipped packs only ever overlap on
    /// byte-identical entries, so this rule never has to choose between two
    /// different pictures - see `docs/formats/dlc-pack.md`.
    #[test]
    fn the_first_mounted_pack_wins_between_packs() {
        let dir = testing::temp_dir("mount-first-wins");
        let mut archives = mounted(
            &dir,
            &[("base.bin", b"disc")],
            vec![
                pack(&dir, "first", &[("shared.bin", b"first")]),
                pack(&dir, "second", &[("shared.bin", b"second")]),
            ],
        );

        assert_eq!(archives.read_name("shared.bin").unwrap(), b"first");
    }

    /// Directory position is per archive. Flattening the mounted archives into
    /// one sequence would hand a PS2 model the tail of the previous archive and
    /// decode it as a texture set, so the neighbour must come from the archive
    /// that held the entry.
    #[test]
    fn a_neighbour_comes_from_the_archive_that_held_the_entry() {
        let dir = testing::temp_dir("mount-neighbour");
        let mut archives = mounted(
            &dir,
            &[("base.bin", b"disc last entry")],
            vec![pack(
                &dir,
                "one",
                &[("before.bin", b"the neighbour"), ("model.bin", b"model")],
            )],
        );

        assert_eq!(
            archives.read_preceding("model.bin").unwrap(),
            b"the neighbour",
            "not the disc's last entry"
        );
    }

    /// An entry in nothing at all still fails against the bulk archive, so the
    /// message names the file a reader would open first rather than whichever
    /// pack happened to be mounted last.
    #[test]
    fn a_miss_is_reported_against_the_source() {
        let dir = testing::temp_dir("mount-miss");
        let mut archives = mounted(
            &dir,
            &[("base.bin", b"disc")],
            vec![pack(&dir, "one", &[("extra.bin", b"pack")])],
        );

        let error = archives.read_name("absent.bin").unwrap_err();
        let Error::NoSuchEntry { archive, .. } = error else {
            panic!("expected a missing entry, got {error:?}");
        };
        assert!(archive.ends_with("Data.wad"), "{archive}");
    }

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
    fn wipeout_pure_s_serial_is_rejected_by_name() {
        let error = reject_other_title("pure-psp-usa.chd", "UCUS-98612").unwrap_err();
        let message = error.to_string();
        assert!(message.contains("UCUS-98612"), "{message}");
        assert!(message.contains("Wipeout Pure"), "{message}");
        assert!(message.contains("M8"), "{message}");
    }

    #[test]
    fn an_unrecognised_serial_is_not_rejected() {
        // The deny-list only rules a source *out*; an uncatalogued serial is
        // not assumed to be Pulse either, so it gets no verdict here and is
        // left to archive-name matching, same as before this check existed.
        assert!(reject_other_title("some.chd", "ULUS-99999").is_ok());
    }

    #[test]
    fn pulse_s_own_serials_are_not_rejected() {
        // Regression guard: this table must never grow a Pulse serial by
        // mistake, or a legitimately-owned disc would hard-reject.
        for serial in ["UCUS-98712", "SCES-54748"] {
            assert!(reject_other_title("pulse.chd", serial).is_ok(), "{serial}");
        }
    }

    #[test]
    fn an_extracted_directory_is_never_checked_against_a_title() {
        // An extract carries no disc header, so it has no serial to reject -
        // ADR-0004's "load from your own already-unpacked originals" stays
        // permitted unconditionally, even for a title `OTHER_TITLES` names.
        let root = extract(
            "no-title-check",
            &[
                "UMD_DATA.BIN",
                "PSP_GAME/USRDIR/Data.wad",
                "PSP_GAME/USRDIR/FE.wad",
            ],
        );
        let source = root.to_str().unwrap();

        assert!(Layout::resolve(source).is_ok());

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
