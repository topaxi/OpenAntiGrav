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

use std::sync::{Arc, Mutex};

use oag_disc::DiscImage;
use oag_title::Title;

/// Re-exported because [`Layout::platform`] is one, and a caller that reads that
/// field should not have to depend on `oag-disc` to name what it read.
pub use oag_disc::Platform;

use crate::blob_source::POISONED;
use crate::{Container, Error, Result};

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
    /// the data**: `oag_fx::psys::ColourScale`, the byte value a
    /// particle effect's colour table treats as fully bright. The PSP
    /// authors `0..=255` and the PS2 `0..=127.5` - the same effect, exported
    /// twice at scales differing by exactly two - and no `.pob` header word
    /// distinguishes them. See that type for the corpus evidence and why a
    /// per-file heuristic is unsafe. A second such case would be worth
    /// looking at hard before adding it - see [`Self::serial`] below for the
    /// one that turned up.
    pub platform: Platform,
    /// The bulk archive: tracks, ships, handling, and on the PSP the movies,
    /// screens and string tables too.
    pub data: String,
    /// Update archives mounted over [`Self::data`], in search order.
    ///
    /// See [`ArchiveCandidates::patch`](oag_title::ArchiveCandidates::patch).
    pub patch: Vec<String>,
    /// The companion archive, when the source has one.
    pub fe: Option<String>,
    /// Every other archive this release ships, all of them, in candidate order.
    ///
    /// Empty for both PSP titles. See
    /// [`ArchiveCandidates::extra`](oag_title::ArchiveCandidates::extra) for why
    /// a third role exists and why it is a set rather than alternatives.
    pub extra: Vec<String>,
    /// The release's own serial, normalised to `AAAA-NNNNN`: a disc image's
    /// volume id, or the `TITLE_ID` of an extracted package's `param.sfo`
    /// (`PCSF-00007`). `None` for a directory with neither - see [`survey`].
    ///
    /// **The second exception [`Self::platform`]'s own doc predicted.** Wipeout
    /// Pure's `Title Screen->TitleFrame` (the "wipEout pure" wordmark) is
    /// assigned a texture by a literal, region-suffixed name baked into each
    /// pressing's own executable at compile time - `FMV_last_frame_EU.mip` on
    /// the EU disc, `FMV_last_frame_US.mip` on the USA one - and **both
    /// pressings ship both textures, byte-identical, in the same shared
    /// `Data.wad`**, so nothing in the archive's own data says which one a
    /// given disc actually draws. See `docs/formats/pure-status.md`'s "The
    /// Title screen wordmark" section and
    /// `docs/ghidra/functions/psp-pure-eu/title-screen.md`. Consulted by
    /// `oag_pure::frontend`'s per-pressing `TITLE_FRAME_EU`/`TITLE_FRAME_USA`
    /// through `oag_game::boot::load_shell`.
    pub serial: Option<String>,
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
        Ok(Self::resolve_on(source, title)?.0)
    }

    /// As [`Layout::resolve`], also handing back the disc image it walked so
    /// the caller's archives can be mounted on it rather than on images of
    /// their own. `None` for a directory source. See [`survey`].
    ///
    /// # Errors
    ///
    /// As [`Layout::resolve`].
    pub(crate) fn resolve_on(
        source: &str,
        title: &Title,
    ) -> Result<(Self, Option<Arc<Mutex<DiscImage>>>)> {
        let (mut platform, files, disc, serial) = survey(source, title)?;

        let data = pick(source, &files, title.archives.data).ok_or_else(|| Error::NoArchive {
            looked_in: source.to_string(),
            platform: platform.to_string(),
            looked_for: title.archive_names(),
        })?;
        let patch = pick_all(source, &files, title.archives.patch);
        let fe = pick(source, &files, title.archives.fe);
        let mut extra = pick_all(source, &files, title.archives.extra);
        // A candidate list may name one archive in two roles (HD's `DATA03` is a
        // PSN install's bulk archive and a disc's extra); it mounts once, in the
        // role that came first.
        extra.retain(|spec| *spec != data.0 && fe.as_ref().is_none_or(|(fe, _)| fe != spec));

        // An extracted directory that holds only `USRDIR` has no `UMD_DATA.BIN`
        // and no `SYSTEM.CNF` to identify it, so the archive that matched is the
        // only thing left that knows. Never the other way round: a disc that
        // says what it is is believed.
        if platform == Platform::Unknown {
            platform = data.1;
        }

        Ok((
            Self {
                platform,
                data: data.0,
                patch,
                fe: fe.map(|(spec, _)| spec),
                extra,
                serial,
            },
            disc,
        ))
    }

    /// One line for a load report: what was found and what it was found on.
    #[must_use]
    pub fn describe(&self) -> String {
        let head = match &self.fe {
            Some(fe) => format!("{} source: {} and {}", self.platform, self.data, fe),
            None => format!(
                "{} source: {}, with no companion archive",
                self.platform, self.data
            ),
        };
        let head = match self.patch.len() {
            0 => head,
            n => format!("{head}, with {n} update archive(s) mounted over it"),
        };
        match self.extra.len() {
            0 => head,
            n => format!("{head}, and {n} more"),
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
    /// The bulk archive: `Data.wad` on the PSP, `WADS2.WAD` on the PS2,
    /// `DATA00.PSARC` on the PS3.
    pub data: Container,
    /// Update archives searched before [`Self::data`], in the order the layout
    /// lists them. Empty unless the title declares
    /// [`ArchiveCandidates::patch`](oag_title::ArchiveCandidates::patch).
    pub patch: Vec<Container>,
    /// The companion archive: `FE.wad` on the PSP, `WADSP.WAD` on the PS2.
    pub fe: Option<Container>,
    /// Every other archive the release ships, all mounted, searched after
    /// [`Self::data`] and [`Self::fe`] and before [`Self::packs`].
    ///
    /// Empty for both PSP titles. Five archives on Wipeout HD, which is what
    /// this exists for; see
    /// [`ArchiveCandidates::extra`](oag_title::ArchiveCandidates::extra).
    pub extra: Vec<Container>,
    /// Mounted [downloadable content](crate::dlc), searched after the source's
    /// own archives so the disc always wins a collision.
    ///
    /// Empty unless the caller passed packs to [`Archives::open_with_packs`].
    /// Every shipped pack is a WAD; the type is a [`Container`] so that the
    /// search below is one rule rather than one rule and an exception.
    pub packs: Vec<Container>,
    /// What those packs declare: the `PI_Team` and `PI_Track` fragments out of
    /// [`crate::dlc::Pack::manifests`], concatenated in mount order.
    ///
    /// Carried here rather than handed back separately because every caller
    /// that mounts packs also needs to know what they added, and threading two
    /// values through the same six call sites is two chances to thread one.
    pub manifests: Vec<String>,
    /// What [`Self::read_name`] has already read, once
    /// [`Self::memoising_reads`] asked for it. `None` reads every time.
    pub read_memo: Option<crate::read_memo::ReadMemo>,
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
        let (layout, disc) = Layout::resolve_on(source, title)?;
        // On a disc source every archive is mounted on the one image `survey`
        // already opened and walked; a directory source has no image and each
        // archive is a file of its own. `mount` is the only difference.
        let mount = |spec: &str| match &disc {
            Some(disc) => Container::open_on(disc, spec),
            None => Container::open(spec),
        };
        let data = match mount(&layout.data) {
            Err(error) => {
                return Err(match &disc {
                    Some(disc) if layout.platform == Platform::Ps3 => {
                        let encrypted = disc
                            .lock()
                            .expect(POISONED)
                            .ps3_declares_encrypted_regions()
                            .unwrap_or(false);
                        if encrypted {
                            Error::EncryptedDisc {
                                looked_in: source.to_string(),
                            }
                        } else {
                            error
                        }
                    }
                    _ => error,
                });
            }
            Ok(data) => data,
        };
        let patch = layout
            .patch
            .iter()
            .map(|spec| mount(spec))
            .collect::<Result<Vec<_>>>()?;
        let fe = layout.fe.as_deref().map(mount).transpose()?;
        let extra = layout
            .extra
            .iter()
            .map(|spec| mount(spec))
            .collect::<Result<Vec<_>>>()?;

        let mut manifests = Vec::new();
        let mut mounted = Vec::new();
        for pack in packs {
            manifests.extend(pack.manifests);
            mounted.extend(
                pack.archives
                    .into_iter()
                    .map(|archive| Container::Wad(Box::new(archive))),
            );
        }

        Ok(Self {
            layout,
            data,
            patch,
            fe,
            extra,
            packs: mounted,
            manifests,
            read_memo: None,
        })
    }

    /// These archives, with [`Self::read_name`] answering a name it has read
    /// before from memory - see [`crate::read_memo`] for what is kept. For a
    /// load that drops the archives when it is done, so the memo goes with them.
    #[must_use]
    pub fn memoising_reads(mut self) -> Self {
        self.read_memo = Some(crate::read_memo::ReadMemo::default());
        self
    }

    /// [`crate::read_memo::ReadMemo::summary`], when reads are memoised.
    #[must_use]
    pub fn read_memo_summary(&self) -> Option<String> {
        self.read_memo
            .as_ref()
            .map(crate::read_memo::ReadMemo::summary)
    }

    /// Which mounted archive holds this name, if any.
    ///
    /// The one place the search order lives. Returning a position rather than a
    /// reference keeps it usable from `&self` and from `&mut self` alike, which
    /// is what lets [`Self::locate`] stay immutable while the readers below
    /// share the same rule.
    ///
    /// **The question is put to each archive rather than answered here**, so a
    /// WAD hashes the name and a PSARC normalises it to the path spelling it
    /// stored - see [`Container::contains`]. This used to take a `u32` hash,
    /// which built the WAD's addressing into the search itself.
    fn holder_of(&self, name: &str) -> Option<Held> {
        if let Some(index) = self.patch.iter().position(|patch| patch.contains(name)) {
            return Some(Held::Patch(index));
        }
        if self.data.contains(name) {
            return Some(Held::Data);
        }
        if self.fe.as_ref().is_some_and(|fe| fe.contains(name)) {
            return Some(Held::Fe);
        }
        if let Some(index) = self.extra.iter().position(|extra| extra.contains(name)) {
            return Some(Held::Extra(index));
        }
        self.packs
            .iter()
            .position(|pack| pack.contains(name))
            .map(Held::Pack)
    }

    /// Every archive holding `name`, in [`Self::holder_of`]'s own order.
    ///
    /// The same rule applied to completion instead of stopped at the first hit,
    /// so the two cannot disagree about precedence: the head of this is always
    /// [`Self::holder_of`]'s answer.
    fn holders_of(&self, name: &str) -> Vec<Held> {
        let mut out: Vec<Held> = self
            .patch
            .iter()
            .enumerate()
            .filter(|(_, patch)| patch.contains(name))
            .map(|(index, _)| Held::Patch(index))
            .collect();
        if self.data.contains(name) {
            out.push(Held::Data);
        }
        if self.fe.as_ref().is_some_and(|fe| fe.contains(name)) {
            out.push(Held::Fe);
        }
        out.extend(
            self.extra
                .iter()
                .enumerate()
                .filter(|(_, extra)| extra.contains(name))
                .map(|(index, _)| Held::Extra(index)),
        );
        out.extend(
            self.packs
                .iter()
                .enumerate()
                .filter(|(_, pack)| pack.contains(name))
                .map(|(index, _)| Held::Pack(index)),
        );
        out
    }

    /// The specifier of the archive a [`Held`] names.
    fn label_at(&self, at: Held) -> &str {
        match at {
            Held::Patch(index) => self.patch[index].label(),
            Held::Data => self.data.label(),
            Held::Fe => self.fe.as_ref().expect("holder_of found it here").label(),
            Held::Extra(index) => self.extra[index].label(),
            Held::Pack(index) => self.packs[index].label(),
        }
    }

    /// Which mounted archive holds this name *hash*, if any.
    ///
    /// The same search order as [`Self::holder_of`], for the entries whose names
    /// are not recovered. **PSARC archives are skipped rather than consulted**:
    /// a PSARC stores paths and no hashes, and its own per-entry digest is an
    /// MD5 of the path rather than anything a WAD hash could be compared to. A
    /// source made of them therefore answers `None` here, which is honest - the
    /// entries this addresses are Pulse's unmined ones.
    fn holder_of_hash(&self, hash: u32) -> Option<Held> {
        let has = |container: &Container| match container {
            Container::Wad(wad) => wad.index_of_hash(hash).is_some(),
            Container::Psarc(_) => false,
        };
        if let Some(index) = self.patch.iter().position(has) {
            return Some(Held::Patch(index));
        }
        if has(&self.data) {
            return Some(Held::Data);
        }
        if self.fe.as_ref().is_some_and(has) {
            return Some(Held::Fe);
        }
        if let Some(index) = self.extra.iter().position(has) {
            return Some(Held::Extra(index));
        }
        self.packs.iter().position(has).map(Held::Pack)
    }

    fn held(&mut self, at: Held) -> &mut Container {
        match at {
            Held::Patch(index) => &mut self.patch[index],
            Held::Data => &mut self.data,
            Held::Fe => self.fe.as_mut().expect("holder_of found it here"),
            Held::Extra(index) => &mut self.extra[index],
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
        Some(self.label_at(self.holder_of(name)?))
    }

    /// Every archive holding `name`, in the same order [`Self::locate`] searches.
    ///
    /// **[`Self::locate`] answers "which copy wins"; this answers "how many are
    /// there", and on one source the difference decides what a player reads.**
    /// Wipeout HD ships its language string table in five of its seven
    /// archives, and the copies disagree: `DATA02`'s numbers the circuits the
    /// way the base game did and `DATA06`'s the way the Fury-era front-end
    /// definition does. Precedence lands on the first, the circuit list comes
    /// out of the last, and the result was a confidently wrong name on eight
    /// circuits - see `oag_game::boot::load_circuit_names`, which uses this to
    /// pick the copy that agrees with the list rather than the copy that
    /// happens to be mounted first.
    ///
    /// Empty when nothing holds `name`. Both PSP titles return at most one
    /// element for any front-end file, so a caller written against this behaves
    /// there exactly as it would against [`Self::locate`].
    #[must_use]
    pub fn locations(&self, name: &str) -> Vec<&str> {
        self.holders_of(name)
            .into_iter()
            .map(|at| self.label_at(at))
            .collect()
    }

    /// Reads and decompresses `name` out of whichever archive holds it.
    ///
    /// # Errors
    ///
    /// [`Error::NoSuchEntry`] against the bulk archive when nothing has it, so
    /// the message names the archive a reader would look in first.
    pub fn read_name(&mut self, name: &str) -> Result<Vec<u8>> {
        if let Some(blob) = self.read_memo.as_mut().and_then(|memo| memo.get(name)) {
            return Ok(blob);
        }
        let blob = match self.holder_of(name) {
            Some(at) => self.held(at).read_entry(name),
            None => self.data.read_entry(name),
        }?;
        if let Some(memo) = self.read_memo.as_mut() {
            memo.keep(name, &blob);
        }
        Ok(blob)
    }

    /// Reads **every** archive's copy of `name`, labelled with the archive it
    /// came from, in [`Self::locations`] order.
    ///
    /// The many-copy counterpart of [`Self::read_name`], for a caller that has
    /// to choose between copies on their content rather than take the first.
    /// See [`Self::locations`] for the source that needs it.
    ///
    /// A copy that will not read is dropped rather than failing the lot: the
    /// question this answers is "what copies are there", and one unreadable
    /// archive does not make the others unusable. An empty result means either
    /// no copy or no *readable* copy, which a caller falling back to
    /// [`Self::read_name`] handles the same way.
    pub fn read_every_name(&mut self, name: &str) -> Vec<(String, Vec<u8>)> {
        self.holders_of(name)
            .into_iter()
            .filter_map(|at| {
                let label = self.label_at(at).to_string();
                self.held(at)
                    .read_entry(name)
                    .ok()
                    .map(|blob| (label, blob))
            })
            .collect()
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
        let at = self.holder_of_hash(hash).unwrap_or(Held::Data);
        self.held(at).as_wad_mut("a name hash")?.read_hash(hash)
    }

    /// Reads a `.fnt` and returns it with its glyph atlas attached, wherever
    /// that atlas lives on this source.
    ///
    /// The two builds differ and the file itself says which is which: a PSP
    /// `.fnt` carries its atlas inside it, a PS2 `.fnt` stops where the atlas
    /// would begin and keeps the glyph sheet in the **next archive entry**, as
    /// a [`PSMT4`](oag_texture::ps2_texture::Layout::Psmt4) texture. So the
    /// embedded atlas is tried first and the following entry is only read when
    /// there is none, which means a PSP source never touches the fallback and
    /// this needs no platform test.
    ///
    /// # Errors
    ///
    /// [`Error::NoSuchEntry`] when `name` is not on this source, and
    /// [`Error::Font`] when the metrics, or the atlas the following entry is
    /// supposed to hold, do not decode.
    pub fn read_font(&mut self, name: &str) -> Result<oag_texture::fnt::Font> {
        use oag_texture::{fnt, ps2_texture};

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
        let at = self.holder_of(name).unwrap_or(Held::Data);
        // A PSARC's entry order is its manifest's and carries no meaning, so it
        // is refused here rather than answered with the entry that happens to
        // sit next to this one. See `Error::NotAWad`.
        let archive = self.held(at).as_wad_mut("the neighbouring entry")?;
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
    Patch(usize),
    Data,
    Fe,
    Extra(usize),
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
/// **The candidate order is the caller's preference and is honoured**, which
/// this took until 2026-09-08 to actually do: it used to walk the *source's*
/// entries and take the first one matching any candidate, so the disc's own
/// directory order decided. The only caller passing more than one candidate is
/// `oag_game::boot`'s movie loader, which lists the PS2's 60 Hz cut ahead of
/// its 50 Hz one and silently got `BG512.IPF` anyway, because that is what
/// `DATA/MOVIES/` lists first.
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
        let Some(file) = candidates
            .iter()
            .find_map(|c| files.iter().find(|f| names(f, c)))
        else {
            return Ok(None);
        };
        let data = std::fs::read(path.join(file))?;
        return Ok(Some((file.clone(), data)));
    }

    let mut disc = DiscImage::open(source)?;
    let entries = disc.entries()?;
    let found = candidates
        .iter()
        .find_map(|c| {
            entries
                .iter()
                .find(|entry| !entry.is_directory && names(&entry.path, c))
        })
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
fn survey(source: &str, title: &Title) -> Result<Surveyed> {
    let path = std::path::Path::new(source);
    if path.is_dir() {
        let mut files = Vec::new();
        collect(path, "", &mut files);
        // Sorted so that two candidates matching the same role resolve the same
        // way on every filesystem, rather than in readdir order.
        files.sort();
        let platform = platform_of(&files);
        let serial = package_title_id(path, &files);
        return Ok((platform, files, None, serial));
    }

    let disc = Arc::new(Mutex::new(DiscImage::open(source)?));
    let info = disc.lock().expect(POISONED).identify()?;
    if let Some(serial) = &info.serial {
        reject_foreign_title(source, serial, title)?;
    }
    let files = disc
        .lock()
        .expect(POISONED)
        .entries()?
        .iter()
        .filter(|entry| !entry.is_directory)
        .map(|entry| entry.path.clone())
        .collect();
    // **Handed back rather than dropped here.** This walk is the expensive part
    // of opening a source, and dropping the image threw away both the entry
    // list it just built and the CHD hunk it warmed - so every archive mounted
    // behind it paid for the walk again. `Archives::open_with_packs` mounts two
    // on a Pulse disc and eight on a Wipeout HD one.
    Ok((info.platform, files, Some(disc), info.serial))
}

/// What [`survey`] found: the platform, the files, the disc image it walked
/// them out of - `None` for a directory source, which has no image to share -
/// and the disc's own serial, `None` for the same reason. See [`Layout::serial`].
type Surveyed = (
    Platform,
    Vec<String>,
    Option<Arc<Mutex<DiscImage>>>,
    Option<String>,
);

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

/// The release an extracted package names in its own `param.sfo`, first match in
/// the sorted file list (a Vita tree lists `base` before its patch and DLC).
///
/// `None` when the tree kept no `param.sfo`, as this project's PS4 extract does
/// not; the caller then has no release to key anything on.
fn package_title_id(root: &std::path::Path, files: &[String]) -> Option<String> {
    files
        .iter()
        .filter(|f| names(f, "PARAM.SFO"))
        .find_map(|f| oag_disc::sfo::title_id(&std::fs::read(root.join(f)).ok()?))
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

/// Every candidate present in `files`, in candidate order.
///
/// The set-shaped counterpart of [`pick`], for
/// [`ArchiveCandidates::extra`](oag_title::ArchiveCandidates::extra). A
/// candidate that is absent is skipped rather than reported: the field says
/// what a release *might* carry, and Fury's archives are on a disc that a
/// base-game pressing is not obliged to match.
fn pick_all(source: &str, files: &[String], candidates: &[(&str, Platform)]) -> Vec<String> {
    candidates
        .iter()
        .filter_map(|(candidate, _)| {
            files
                .iter()
                .find(|file| names(file, candidate))
                .map(|file| archive_spec(source, file))
        })
        .collect()
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
