//! Finding and opening the archives one source carries, whichever title it is.
//!
//! This is the *mechanism* half of [ADR-0022]. It knows how to walk a disc image
//! or an extracted directory, match archive names by trailing path component,
//! and read entries by name, hash or directory position. It knows no archive
//! name, no entry name and no title: those come in as an [`oag_title::Title`],
//! from `oag-pulse` or `oag-pure`.
//!
//! **Found by name, not derived from the console.** Candidates are tried in
//! order against the source's own file list, so a disc that identifies as
//! neither console still opens if it holds an archive one of them would
//! recognise, and a PS2 pressing whose serial directory is not `54748/` needs no
//! change here.
//!
//! [ADR-0022]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0022-title-packages.md

use oag_disc::DiscImage;
use oag_title::Title;

/// Re-exported because [`Layout::platform`] is one, and a caller that reads that
/// field should not have to depend on `oag-disc` to name what it read.
pub use oag_disc::Platform;

use crate::{Archive, Error, Result};

/// `Err(Error::WrongTitle)` when `serial` is positively known to belong to some
/// title other than the one being opened.
///
/// Called from [`survey`], before archive-name matching runs, so a Pure disc
/// never reaches a Pulse [`Layout::resolve`]'s `Data.wad`/`FE.wad` search - the
/// two ship those under identical names, so name matching alone would open one
/// exactly as if it were the other.
///
/// The list this consults belongs to the title, not to this crate, and it rules
/// a source *out* only; see [`oag_title::ForeignSerial`] for why an allow-list
/// would be worse than no check at all.
fn reject_foreign_title(looked_in: &str, serial: &str, title: &Title) -> Result<()> {
    if let Some(other) = title.foreign_title(serial) {
        return Err(Error::WrongTitle {
            looked_in: looked_in.to_string(),
            serial: serial.to_string(),
            title: other.to_string(),
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
/// change here. This is unrelated to [`reject_foreign_title`]: that check runs on
/// the disc's *serial* in [`survey`], before this layer's name matching starts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Layout {
    /// What the source says it is, or what its archives imply when it says
    /// nothing. Carried for reports and error messages, and every *decode* is
    /// still chosen by the **data** rather than by the disc it came off.
    ///
    /// **One thing branches on it, and only because it cannot be read out of
    /// the data**: `oag_render::psys::ColourScale`, the byte value a
    /// particle effect's colour table treats as fully bright. The PSP
    /// authors `0..=255` and the PS2 `0..=127.5` - the same effect, exported
    /// twice at scales differing by exactly two - and no `.pob` header word
    /// distinguishes them. See that type for the corpus evidence and why a
    /// per-file heuristic is unsafe. A second such case would be worth
    /// looking at hard before adding it.
    pub platform: Platform,
    /// The bulk archive: tracks, ships, handling, and on the PSP the movies,
    /// screens and string tables too.
    pub data: String,
    /// The companion archive, when the source has one.
    pub fe: Option<String>,
}

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
    pub fn resolve(source: &str, title: &Title) -> Result<Self> {
        let (mut platform, files) = survey(source, title)?;

        let data = pick(source, &files, title.archives.data).ok_or_else(|| Error::NoArchive {
            looked_in: source.to_string(),
            platform: platform.to_string(),
            looked_for: title.archive_names(),
        })?;
        let fe = pick(source, &files, title.archives.fe);

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
    pub fn open(source: &str, title: &Title) -> Result<Self> {
        Self::open_with_packs(source, title, Vec::new())
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
    ///
    /// # Why `title` is here too
    ///
    /// Packs are mounted *behind* a source, and which archives that source has
    /// is the title's question ([ADR-0022]) - so this needs the same axis
    /// [`Archives::open`] does. A pack does not change which title it is.
    ///
    /// [ADR-0022]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0022-title-packages.md
    pub fn open_with_packs(
        source: &str,
        title: &Title,
        packs: Vec<crate::dlc::Pack>,
    ) -> Result<Self> {
        let layout = Layout::resolve(source, title)?;
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
fn survey(source: &str, title: &Title) -> Result<(Platform, Vec<String>)> {
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
        reject_foreign_title(source, serial, title)?;
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
    use oag_title::{ArchiveCandidates, ForeignSerial};

    /// A stand-in for what a title crate supplies.
    ///
    /// These are Pulse's real archive names, because the matching rules being
    /// tested here - a serial-named parent directory, a case-insensitive tail,
    /// a component boundary - were all derived from real pressings and a
    /// synthetic name would test the rules against nothing. They are a
    /// *fixture* rather than this crate's knowledge: `oag-assets` itself names
    /// no archive, and swapping this table for Pure's would leave every
    /// assertion below meaningful.
    const TITLE: &Title = &Title {
        name: "Wipeout Pulse",
        archives: ArchiveCandidates {
            data: &[
                ("PSP_GAME/USRDIR/Data.wad", Platform::Psp),
                ("WADS2.WAD", Platform::Ps2),
            ],
            fe: &[
                ("PSP_GAME/USRDIR/FE.wad", Platform::Psp),
                ("WADSP.WAD", Platform::Ps2),
            ],
        },
        foreign_serials: &[ForeignSerial {
            serial: "UCUS-98612",
            title: "Wipeout Pure",
        }],
        // Present for the same reason `boot` below is, and read even less: menu
        // layout is the front end's business, and this crate opens archives.
        menu: &oag_title::MenuSkin {
            menu_x: 0.0,
            menu_scale: 1.0,
            title_x: 0.0,
            title_y: 0.0,
            title_scale: 1.0,
            first_row_y: None,
            row_extra_leading: None,
            menu_font: None,
            text: None,
            title: None,
            selected: None,
            transition_secs: 0.0,
        },
        // Present because `Title` carries it, and never read here: `oag-assets`
        // resolves archives and has no business in a boot sequence. One screen is
        // enough to be a valid profile.
        boot: &oag_title::BootProfile {
            chain: &[oag_title::BootStep::screen("Language Selection")],
            reel: None,
            menu_backdrop: None,
            picker_backdrop_parent: None,
            fallback_globals: &[],
        },
        // Present for the same reason and equally unread: resolving archives has
        // nothing to do with which circuit a race opens on.
        race: &oag_title::RaceDefaults {
            track: r"Data\Environments\00_Nowhere\track.vex",
            team: "Nobody",
        },
    };

    /// The bulk archive's PSP and PS2 names, as the fixture spells them.
    const PSP_DATA: &str = TITLE.archives.data[0].0;
    const PS2_DATA: &str = TITLE.archives.data[1].0;

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
            archive_spec("data/images/pulse-psp-usa.chd", PSP_DATA),
            "data/images/pulse-psp-usa.chd:PSP_GAME/USRDIR/Data.wad"
        );
    }

    #[test]
    fn an_extracted_directory_is_joined_with_a_separator() {
        // The current directory always exists, so it stands in for an extract.
        let spec = archive_spec(".", PSP_DATA);
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

        let layout = Layout::resolve(source, TITLE).unwrap();
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
        let error = reject_foreign_title("pure-psp-usa.chd", "UCUS-98612", TITLE).unwrap_err();
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
        assert!(reject_foreign_title("some.chd", "ULUS-99999", TITLE).is_ok());
    }

    #[test]
    fn pulse_s_own_serials_are_not_rejected() {
        // Regression guard: this table must never grow a Pulse serial by
        // mistake, or a legitimately-owned disc would hard-reject.
        for serial in ["UCUS-98712", "SCES-54748"] {
            assert!(
                reject_foreign_title("pulse.chd", serial, TITLE).is_ok(),
                "{serial}"
            );
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

        assert!(Layout::resolve(source, TITLE).is_ok());

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

        let layout = Layout::resolve(source, TITLE).unwrap();
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

        let layout = Layout::resolve(source, TITLE).unwrap();
        assert_eq!(layout.platform, Platform::Ps2);
        assert_eq!(layout.data, format!("{source}/WADS2.WAD"));
        assert_eq!(layout.fe, None);

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn a_source_with_no_archive_says_what_it_looked_for() {
        let root = extract("empty", &["SYSTEM.CNF", "IOP/LIBSD.IRX"]);
        let source = root.to_str().unwrap();

        let error = Layout::resolve(source, TITLE).unwrap_err().to_string();
        assert!(error.contains("PS2"), "{error}");
        assert!(error.contains(PSP_DATA), "{error}");
        assert!(error.contains(PS2_DATA), "{error}");

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn an_archive_name_is_matched_on_a_component_boundary() {
        assert!(names("54748/WADS2.WAD", PS2_DATA));
        assert!(names("54748/wads2.wad", PS2_DATA));
        assert!(names("WADS2.WAD", PS2_DATA));
        assert!(names(r"54748\WADS2.WAD", PS2_DATA));
        assert!(names("PSP_GAME/USRDIR/Data.wad", PSP_DATA));

        // A name that merely ends with the candidate's characters is not that
        // candidate, which is the whole reason this is not `ends_with`.
        assert!(!names("54748/NOTWADS2.WAD", PS2_DATA));
        assert!(!names("WADS2.WADX", PS2_DATA));
        // The PSP candidate is a path, so a bare file of the same name in some
        // other directory does not answer for it.
        assert!(!names("elsewhere/Data.wad", PSP_DATA));
    }
}
