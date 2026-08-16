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
mod tests;
