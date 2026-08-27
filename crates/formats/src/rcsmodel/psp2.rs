//! Wipeout 2048's `.rcsmodel`: the same extension, a different container.
//!
//! Wipeout HD's `.rcsmodel` is an offset-table archive read big-endian by
//! [`super::Model::parse`]. **2048's is not that file.** It is a linker-style
//! image: a self-describing header section carrying relocation tables, then one
//! CPU-resident block and one GPU-resident block, laid back to back, read
//! little-endian and rebased at load. `RcsModel_Load` (`0x812f15b2`) is the
//! loader, tagged `PSP2/Psp2.RcsModelLoader.cpp`, which is where the `psp2` in
//! this module's name comes from.
//!
//! ```text
//! section A - the header, read first and thrown away after load:
//!   +0x00  u32   magic, 0xca5caded
//!   +0x08  u32   section count, 2 on every file with geometry
//!   +0x0c  u32   section A's own size
//!         then   one 0x20-byte descriptor per section, from +0x20
//!         then   each section's relocation table, in descriptor order
//!
//! one section descriptor:
//!   +0x00  u32   a tag, unread
//!   +0x04  u32   the section's size in bytes
//!   +0x08  u32   its relocation table's offset, from the end of the descriptors
//!   +0x0c  u32   how many entries that table has
//!   +0x10  u32   the base its pointers were linked against - 0 on every file
//!
//! one relocation entry, 8 bytes:
//!   +0x00  u32   offset of a pointer, within the section named at +0x04
//!   +0x04  u32   which section that pointer lives in
//! ```
//!
//! Section **B** is a serialized C++ object graph and section **C** is raw GPU
//! buffer data. `RcsModel_Load` walks table 0 rebasing every pointer *to* B and
//! table 1 rebasing every pointer *to* C, and on every shipped file the link
//! base is `0` - so **a pointer on disc is already the offset of its target
//! within its section**, and nothing here has to simulate the rebase.
//!
//! # What is decoded, and what is deliberately not
//!
//! **Positions, triangle indices and vertex normals.** That is enough to draw
//! a lit 2048 circuit and a lit 2048 craft, which nothing could before.
//!
//! **The normal is three signed bytes, `byte/127.0`, at `+0x0c` - not a
//! packed word.** Section B carries a per-chunk vertex declaration in the
//! same shape as HD's own (`docs/formats/2048-rcsmodel.md`'s "Section B
//! carries a vertex declaration" section), naming `normal`'s offset (`0x0c`,
//! immediately after the 12-byte position, on every declaration decoded) and
//! its type - a Vita SceGxm code, not one of HD's RSX ones, so HD's packed
//! 11:11:10 word was tried at this exact offset and ruled out. **Confidence
//! 96**: an index-exact vertex correspondence against Wipeout HD (1,504
//! vertices, zero ambiguity - the same shape of oracle that settled the `WO
//! Track` tail) scores this exact decode at 100% within 18 degrees, mean dot
//! 0.994, against every other candidate tried scoring at or below chance.
//! The fourth byte is `0x00` on all 1,504 - genuine padding to a 4-byte
//! attribute width, not a fourth field. See [`unpack_normal`].
//!
//! **The diffuse texture coordinate is two little-endian `f16`s**, at the
//! offset [`vertex_decl::VertexDecl::diffuse_texcoord`] names -
//! [`vertex_decl::find_by_stride`] finds every declaration a file carries by
//! anchoring on the same `position` hash, keyed by stride, and a submesh
//! looks its own already-measured stride up in that map. **Confidence 96**,
//! the same index-exact oracle that settled the normal (1,432 vertices, 9
//! same-export submeshes across twelve HD-ported circuits): decoded as
//! `f16`, mean squared distance to Wipeout HD's own diffuse UV at the same
//! vertices is indistinguishable from zero and the two decodings agree on
//! every vertex that leaves `[0, 1]` (16 of 1,432, both sides); decoded as
//! `unorm16` the distance is 0.41, chance-level. See [`unpack_texcoord`].
//!
//! **The material table is also read**, and **which submesh draws with which
//! entry** - name, technique name, every `.gxt` path found within a
//! material's own extent, and a per-submesh index into the table
//! ([`SubMesh::material`], at [`MATERIAL_INDEX_BEFORE_RECORD`]). The index is
//! **confidence 90**: measured across all 244,889 shipped submeshes with a
//! control group, not read out of the executable - see [`material`]'s module
//! doc for the search, the eighteen candidate offsets it rules out, and the
//! Ghidra path that stays open.
//!
//! **Not decoded**: the object graph's own layout, past the declaration and
//! the material table. This module finds submesh records *through the
//! relocation table* rather than by walking B - see [`submeshes`] - which is
//! honest about what is known and is what makes the reading checkable. Also
//! not decoded: the tangent (four bytes at `+0x10`, same declared type as
//! `normal` but four components rather than three - the padding argument
//! above does not apply, since 4 components exactly fill 4 bytes), the
//! lightmap coordinate's content (offset placed via the same declaration,
//! unconfirmable across titles since a lightmap atlas is baked per
//! platform), and the 64-bit hashes each record carries beside its buffer
//! pointers - along with the three words beside the material index at
//! `-0x20`, `-0x10` and `-0x08`. See
//! `docs/formats/2048-rcsmodel.md`.
//!
//! # Why the record layout is trustworthy anyway
//!
//! Every claim below closes arithmetically over **993 files** - every
//! `.rcsmodel` the three EU packages ship - and
//! `crates/formats/tests/psp2_rcsmodel_ground_truth.rs` is that argument,
//! executable. All 993 have `A + B + C` equal to the file's own length. All
//! **244,889** submeshes have an index buffer of exactly `index_count * 2`
//! bytes rounded up to a 4-byte boundary, with `index_count` divisible by
//! three. Not one of 33,335,682 indices names a vertex outside its own
//! submesh's count.

use std::fmt;

pub mod material;
pub mod vertex_decl;

/// The word every 2048 `.rcsmodel` opens with.
///
/// This container has no version field anything reads, so the magic is the
/// whole signature - and it is checked rather than assumed, because a caller
/// that handed this the Wipeout HD container would otherwise read a section
/// count out of a big-endian version word and walk the file on it.
pub const MAGIC: u32 = 0xca5c_aded;

/// Bytes per section descriptor.
pub const DESCRIPTOR_LEN: usize = 0x20;

/// Where the descriptors start.
pub const DESCRIPTOR_BASE: usize = 0x20;

/// Bytes per relocation entry.
pub const RELOCATION_LEN: usize = 8;

/// Bytes between a submesh record's index-buffer pointer and its vertex-buffer
/// pointer.
///
/// The **shape this module finds records by**: a relocation entry names a word
/// holding an offset into the GPU section, and a submesh is the pair of them
/// exactly this far apart. See [`submeshes`].
pub const BUFFER_POINTER_GAP: usize = 28;

/// Offset of the index-buffer pointer within a submesh record.
pub const INDEX_POINTER: usize = 0x10;

/// Offset of the vertex-buffer pointer within a submesh record.
pub const VERTEX_POINTER: usize = INDEX_POINTER + BUFFER_POINTER_GAP;

/// How far **before** a submesh record's own start its material index sits.
///
/// **Measured across the corpus, not read out of the loader** - see
/// [`material`]'s module doc for the search and its control group, and
/// `docs/formats/2048-rcsmodel.md` for the numbers. A `u32` here (its high
/// half is `0` on every submesh measured) indexes the file's own material
/// offset table, in table order.
///
/// **Negative because the record is found, not walked to.** [`submeshes`]
/// locates a record by the shape of its two buffer pointers, and what it calls
/// the record's start is simply where those pointers put it - not the start of
/// whatever enclosing struct section B actually serializes. The material index
/// is a field of that larger struct, so it reads back at a negative offset,
/// alongside three other still-uninterpreted words (`-0x10` is `0xffffffff`
/// and `-0x08` is `0x00010001` on every file sampled).
pub const MATERIAL_INDEX_BEFORE_RECORD: usize = 0x18;

/// Bytes a vertex's position occupies - three little-endian `f32`.
pub const POSITION_LEN: usize = 12;

/// Byte offset of the vertex normal, immediately after the position.
///
/// The same on every declared stride this reading has seen (16, 20, 28) -
/// see the module doc's confidence note. Mirrors
/// [`crate::rcsmodel::NORMAL_OFFSET`], which is `POSITION_LEN` for the same
/// reason on the unrelated HD container.
pub const NORMAL_OFFSET: usize = POSITION_LEN;

/// Turns the three bytes at [`NORMAL_OFFSET`] into a unit vector.
///
/// **Not HD's packed word.** Each byte is one signed, two's-complement
/// component - `x`, `y`, `z` in that order - divided by 127. The declaration
/// names a fourth byte after these three that this function does not read;
/// it is `0x00` on all 1,504 vertices an index-exact correspondence against
/// Wipeout HD checked, so it is padding to a 4-byte attribute width rather
/// than a fourth field. See the module doc for the confidence evidence.
#[must_use]
pub fn unpack_normal(bytes: [u8; 3]) -> [f32; 3] {
    std::array::from_fn(|i| f32::from(bytes[i] as i8) / 127.0)
}

/// Turns two little-endian `f16`s into a texture coordinate.
///
/// **Confidence 96** - see the module doc's index-exact oracle result: `f16`
/// matches Wipeout HD's own diffuse UV at the same authored vertices to
/// within quantisation noise, where `unorm16` (the only other encoding two
/// bytes per component could plausibly be) scores at chance.
#[must_use]
pub fn unpack_texcoord(bytes: [u8; 4]) -> [f32; 2] {
    let word = |o: usize| u16::from_le_bytes([bytes[o], bytes[o + 1]]);
    [
        crate::rcsmodel::unpack_half(word(0)),
        crate::rcsmodel::unpack_half(word(2)),
    ]
}

/// Something wrong with a 2048 `.rcsmodel`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The first word is not [`MAGIC`].
    BadMagic {
        /// What it was.
        magic: u32,
    },
    /// A section, table or buffer runs past the end of the file.
    OutOfBounds {
        /// What did not fit.
        what: &'static str,
        /// Where it would have ended.
        end: usize,
        /// The file's length.
        len: usize,
    },
    /// The sections do not add up to the file's own length.
    SectionsDoNotClose {
        /// What they add up to.
        sections: usize,
        /// The file's length.
        len: usize,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadMagic { magic } => {
                write!(f, "magic is {magic:#010x}, not {MAGIC:#010x}")
            }
            Self::OutOfBounds { what, end, len } => {
                write!(f, "{what} ends at {end} but the file is {len} bytes")
            }
            Self::SectionsDoNotClose { sections, len } => {
                write!(
                    f,
                    "the sections total {sections} bytes and the file is {len}"
                )
            }
        }
    }
}

impl std::error::Error for Error {}

/// Result alias for this module.
pub type Result<T> = std::result::Result<T, Error>;

/// One section of the image, as the header describes it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Section {
    /// Where the section starts in the file.
    pub at: usize,
    /// How long it is.
    pub len: usize,
    /// Where its relocation table starts in the file.
    pub table: usize,
    /// How many entries that table has.
    pub entries: usize,
}

/// One drawable run: a triangle list over a vertex buffer.
#[derive(Debug, Clone, PartialEq)]
pub struct SubMesh {
    /// Where the record sits in the CPU section, for a report or a probe.
    pub record: usize,
    /// Triangle indices, three per triangle.
    pub indices: Vec<u16>,
    /// Vertex positions, in whatever space the model is authored in - world
    /// space for a circuit, model space for a craft. See [`Model::positions`].
    pub positions: Vec<[f32; 3]>,
    /// Vertex normals, one per vertex, in the same space as [`positions`](Self::positions).
    ///
    /// Empty rather than `None` when a submesh's stride is too small to hold
    /// one (`< NORMAL_OFFSET + 3`) - not observed on any of the 993 shipped
    /// files (the smallest declared stride is 16), but not assumed away
    /// either. See [`unpack_normal`].
    pub normals: Vec<[f32; 3]>,
    /// Diffuse texture coordinates, one per vertex, decoded from
    /// [`vertex_decl::VertexDecl::diffuse_texcoord`]'s offset via
    /// [`unpack_texcoord`].
    ///
    /// Empty when the file's declaration for this submesh's stride
    /// ([`vertex_decl::find_by_stride`]) does not name a `Uv1` attribute -
    /// a real, countable state (92.5% of the corpus's submeshes have one)
    /// rather than an error, on the same terms as [`Self::normals`].
    pub texcoords: Vec<[f32; 2]>,
    /// How many of [`Self::texcoords`] decoded to a non-finite value and were
    /// substituted with the origin.
    ///
    /// **A real, measured minority (~21% of decoded vertices corpus-wide),
    /// not zero.** [`vertex_decl::find_by_stride`] keys one declaration per
    /// *stride*, and two chunks can share a stride while packing genuinely
    /// different attributes into it - a chunk whose own layout disagrees
    /// with the one this file picked for its stride decodes noise at the
    /// `Uv1` offset. Substituting rather than propagating `NaN`/`Inf` keeps
    /// this out of a GPU vertex buffer; counting it here rather than
    /// silently zeroing it is what a caller's report shows instead of a
    /// hidden regression on the normal, unrelated vertices sharing the same
    /// buffer. See `docs/formats/2048-rcsmodel.md`.
    pub non_finite_texcoords: usize,
    /// Bytes per vertex, derived from the buffer's own length rather than read
    /// from a field - see [`Model::parse`].
    pub stride: usize,
    /// Which entry of [`Model::materials`] this submesh draws with, in the
    /// file's own material-offset-table order.
    ///
    /// `None` when the file carries no material table, when the record sits
    /// too close to the section's start to hold the field, or when the value
    /// found is not a valid index into the table this reading recovered - all
    /// three are real, countable states rather than errors, on the same terms
    /// as [`Self::normals`]. Never `None` for any of the 244,889 submeshes the
    /// three EU packages ship. See [`MATERIAL_INDEX_BEFORE_RECORD`].
    pub material: Option<usize>,
}

impl SubMesh {
    /// Triangles, which is a third of [`indices`](Self::indices).
    #[must_use]
    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }
}

/// A decoded 2048 `.rcsmodel`.
#[derive(Debug, Clone, PartialEq)]
pub struct Model {
    /// The sections the header declares, in order: the CPU block first, then
    /// the GPU block where the file has one.
    pub sections: Vec<Section>,
    /// Every submesh found, in the order their records appear.
    pub submeshes: Vec<SubMesh>,
    /// Relocation entries naming a GPU-section pointer that is **not** half of
    /// a submesh's buffer pair.
    ///
    /// Carried rather than dropped because it is the honest measure of how much
    /// of the graph this reading does not account for: 5,294 of the corpus's
    /// 495,000-odd, about one in a hundred. A caller can report it.
    pub unpaired_pointers: usize,
    /// The file's own material table - see [`material`].
    ///
    /// Empty when the file-level header does not check out, which is a real
    /// state and not a decode failure: a model with no geometry at all
    /// carries no material table either. Which submesh draws with which entry
    /// is [`SubMesh::material`] - see [`material`]'s module doc.
    pub materials: Vec<material::Material>,
}

impl Model {
    /// Whether every submesh draws through the same, single material. True
    /// for 522 of the corpus's 898 models with a readable material table
    /// (58.1%).
    ///
    /// **No longer load-bearing for drawing.** Until 2026-08-27 this was the
    /// only shape a renderer could texture, because which submesh used which
    /// material was unread; it is read now ([`SubMesh::material`]), and the
    /// single-material model is simply its `n == 1` case. Kept because a
    /// census or a probe still wants to ask.
    #[must_use]
    pub fn has_one_material(&self) -> bool {
        self.materials.len() == 1
    }

    /// Whether this file even has geometry.
    ///
    /// 43 of the corpus's 993 declare one section and no GPU block at all -
    /// `RcsModel_Load` skips the whole second allocation when its size is zero,
    /// so this is an ordinary state and not a decode failure.
    #[must_use]
    pub fn has_geometry(&self) -> bool {
        self.sections.len() > 1
    }

    /// Every vertex position in the model, submesh by submesh.
    ///
    /// **Which space they are in is a property of the model, not of the
    /// format.** A circuit's come out in world coordinates - `altima`'s span
    /// the same box its `track_col.col` states - and a craft's come out about
    /// its own origin, `Assegai`'s inside 2.8 x 1.7 x 7.0 units. So a caller
    /// draws a circuit with no transform and a craft with the craft's.
    pub fn positions(&self) -> impl Iterator<Item = [f32; 3]> + '_ {
        self.submeshes
            .iter()
            .flat_map(|s| s.positions.iter().copied())
    }

    /// Every decoded vertex normal in the model, submesh by submesh.
    ///
    /// Shorter than [`Self::positions`] when a submesh's stride was too
    /// small to carry one - see [`SubMesh::normals`] - which is not observed
    /// on any of the 993 shipped files.
    pub fn normals(&self) -> impl Iterator<Item = [f32; 3]> + '_ {
        self.submeshes
            .iter()
            .flat_map(|s| s.normals.iter().copied())
    }
}

/// Decodes a 2048 `.rcsmodel`.
///
/// # The vertex stride is not in any field this reading has placed
///
/// It comes from the **buffer packing**: the GPU section is exactly the
/// buffers, back to back in pointer order, so sorting the distinct pointer
/// targets and differencing them gives every buffer's length, and a vertex
/// buffer's length divided by its own vertex count is the stride. That is the
/// same oracle `docs/formats/rcsmodel.md` records as one of four for Wipeout
/// HD, and here it is the only one needed: 244,889 of 244,889 submeshes give a
/// whole number, and the strides that come out are 16 to 64 in steps of four.
///
/// # Errors
///
/// Refuses a wrong magic, a header that runs past the end of the file, and
/// sections whose lengths do not add up to the file's own. A submesh whose
/// buffers do not lie inside the GPU section is skipped rather than refused -
/// see [`Model::unpaired_pointers`] for the same reasoning.
pub fn parse(file: &[u8]) -> Result<Model> {
    let magic = u32_at(file, 0, "magic")?;
    if magic != MAGIC {
        return Err(Error::BadMagic { magic });
    }
    let section_count = u32_at(file, 0x08, "section count")? as usize;
    let header_len = u32_at(file, 0x0c, "header length")? as usize;
    let table_base = DESCRIPTOR_BASE + section_count * DESCRIPTOR_LEN;

    let mut sections = Vec::with_capacity(section_count.min(file.len() / DESCRIPTOR_LEN));
    let mut at = header_len;
    for i in 0..section_count {
        let d = DESCRIPTOR_BASE + i * DESCRIPTOR_LEN;
        let len = u32_at(file, d + 0x04, "section size")? as usize;
        let table = table_base + u32_at(file, d + 0x08, "relocation offset")? as usize;
        let entries = u32_at(file, d + 0x0c, "relocation count")? as usize;
        let end = table
            .checked_add(entries * RELOCATION_LEN)
            .ok_or(Error::OutOfBounds {
                what: "relocation table",
                end: usize::MAX,
                len: file.len(),
            })?;
        if end > file.len() {
            return Err(Error::OutOfBounds {
                what: "relocation table",
                end,
                len: file.len(),
            });
        }
        sections.push(Section {
            at,
            len,
            table,
            entries,
        });
        at = at.checked_add(len).ok_or(Error::OutOfBounds {
            what: "section",
            end: usize::MAX,
            len: file.len(),
        })?;
    }
    if at != file.len() {
        return Err(Error::SectionsDoNotClose {
            sections: at,
            len: file.len(),
        });
    }

    let Some(&gpu) = sections.get(1) else {
        let cpu = sections[0];
        let materials = material::read(&file[cpu.at..cpu.at + cpu.len]);
        return Ok(Model {
            sections,
            submeshes: Vec::new(),
            unpaired_pointers: 0,
            materials,
        });
    };
    let cpu = sections[0];
    let (mut submeshes, unpaired_pointers) = submeshes(file, cpu, gpu)?;
    let materials = material::read(&file[cpu.at..cpu.at + cpu.len]);
    // An index this reading cannot resolve against the table it recovered is
    // dropped rather than carried: a caller binding a texture off it would be
    // painting a submesh with some other submesh's material, which is worse
    // than painting it with none.
    for submesh in &mut submeshes {
        if submesh.material.is_some_and(|i| i >= materials.len()) {
            submesh.material = None;
        }
    }
    Ok(Model {
        sections,
        submeshes,
        unpaired_pointers,
        materials,
    })
}

/// Finds every submesh, through the GPU section's own relocation table.
///
/// **The record layout is found rather than walked to**, and that is the
/// deliberate limit of this reading: section B is a serialized object graph
/// whose layout is unread, so instead of walking it this takes the one thing
/// the header states exactly - where every pointer into the GPU section lives -
/// and pairs them. A submesh record holds its index-buffer pointer at
/// [`INDEX_POINTER`] and its vertex-buffer pointer
/// [`BUFFER_POINTER_GAP`] bytes later, with its two counts at the record's own
/// `+0x00` and `+0x04`.
///
/// What makes that safe rather than a pattern match is that **it is checked**:
/// a pair is only taken when the index count is divisible by three and its
/// buffer is exactly that many `u16`s rounded up to four bytes. On the whole
/// corpus that check has never once failed on a pair, and never once passed on
/// something that was not a submesh.
fn submeshes(file: &[u8], cpu: Section, gpu: Section) -> Result<(Vec<SubMesh>, usize)> {
    let declarations_by_stride = vertex_decl::find_by_stride(&file[cpu.at..cpu.at + cpu.len]);
    let mut sites: Vec<usize> = (0..gpu.entries)
        .map(|e| u32_at(file, gpu.table + e * RELOCATION_LEN, "relocation entry"))
        .collect::<Result<Vec<u32>>>()?
        .into_iter()
        .map(|offset| offset as usize)
        .collect();
    sites.sort_unstable();

    // Every distinct buffer start, so differencing them gives every length.
    let mut targets: Vec<u32> = sites
        .iter()
        .filter_map(|&s| u32_at(file, cpu.at + s, "buffer pointer").ok())
        .collect();
    targets.sort_unstable();
    targets.dedup();
    let length_of = |target: u32| -> Option<usize> {
        let i = targets.binary_search(&target).ok()?;
        match targets.get(i + 1) {
            Some(&next) => Some((next - target) as usize),
            None => gpu.len.checked_sub(target as usize),
        }
    };

    let mut out = Vec::new();
    let mut used = 0usize;
    let mut i = 0usize;
    while i + 1 < sites.len() {
        let (first, second) = (sites[i], sites[i + 1]);
        if second - first != BUFFER_POINTER_GAP || first < INDEX_POINTER {
            i += 1;
            continue;
        }
        let record = first - INDEX_POINTER;
        let Some(mesh) = one(file, cpu, gpu, record, &length_of, &declarations_by_stride) else {
            i += 1;
            continue;
        };
        out.push(mesh);
        used += 2;
        i += 2;
    }
    Ok((out, sites.len() - used))
}

/// One submesh record, or `None` if it does not check out.
fn one(
    file: &[u8],
    cpu: Section,
    gpu: Section,
    record: usize,
    length_of: &impl Fn(u32) -> Option<usize>,
    declarations_by_stride: &std::collections::HashMap<usize, vertex_decl::VertexDecl>,
) -> Option<SubMesh> {
    let base = cpu.at.checked_add(record)?;
    let index_count = u32_at(file, base, "index count").ok()? as usize;
    let vertex_count = u32_at(file, base + 0x04, "vertex count").ok()? as usize;
    if index_count == 0 || !index_count.is_multiple_of(3) || vertex_count == 0 {
        return None;
    }
    let index_ptr = u32_at(file, base + INDEX_POINTER, "index pointer").ok()?;
    let vertex_ptr = u32_at(file, base + VERTEX_POINTER, "vertex pointer").ok()?;
    // The index buffer is exactly its own contents, rounded up to a 4-byte
    // boundary. This is the check that says the pair really is a submesh.
    let index_bytes = length_of(index_ptr)?;
    if index_bytes < index_count * 2 || index_bytes - index_count * 2 >= 4 {
        return None;
    }
    let vertex_bytes = length_of(vertex_ptr)?;
    if vertex_bytes == 0 || !vertex_bytes.is_multiple_of(vertex_count) {
        return None;
    }
    let stride = vertex_bytes / vertex_count;
    if stride < 12 {
        return None;
    }

    let index_at = gpu.at.checked_add(index_ptr as usize)?;
    let vertex_at = gpu.at.checked_add(vertex_ptr as usize)?;
    if index_at + index_count * 2 > file.len() || vertex_at + vertex_bytes > file.len() {
        return None;
    }

    let mut indices = Vec::with_capacity(index_count);
    for k in 0..index_count {
        let index = u16::from_le_bytes([file[index_at + k * 2], file[index_at + k * 2 + 1]]);
        if usize::from(index) >= vertex_count {
            return None;
        }
        indices.push(index);
    }
    let mut positions = Vec::with_capacity(vertex_count);
    for v in 0..vertex_count {
        let at = vertex_at + v * stride;
        positions.push([f32_at(file, at), f32_at(file, at + 4), f32_at(file, at + 8)]);
    }
    // Read raw here and validated against the material table in `parse`,
    // which is the only place that knows how many entries the table has.
    let material = record
        .checked_sub(MATERIAL_INDEX_BEFORE_RECORD)
        .and_then(|at| u32_at(file, cpu.at + at, "material index").ok())
        .and_then(|value| usize::try_from(value).ok());

    let mut normals = Vec::new();
    if stride >= NORMAL_OFFSET + 3 {
        normals.reserve_exact(vertex_count);
        for v in 0..vertex_count {
            let at = vertex_at + v * stride + NORMAL_OFFSET;
            normals.push(unpack_normal([file[at], file[at + 1], file[at + 2]]));
        }
    }
    let mut texcoords = Vec::new();
    let mut non_finite_texcoords = 0usize;
    let uv1_offset = declarations_by_stride
        .get(&stride)
        .and_then(|decl| decl.diffuse_texcoord())
        .map(|attr| usize::from(attr.offset));
    if let Some(off) = uv1_offset
        && stride >= off + 4
    {
        texcoords.reserve_exact(vertex_count);
        for v in 0..vertex_count {
            let at = vertex_at + v * stride + off;
            let uv = unpack_texcoord([file[at], file[at + 1], file[at + 2], file[at + 3]]);
            // A submesh whose stride is shared with a differently-laid-out
            // declaration decodes noise, not a real coordinate - see
            // `SubMesh::non_finite_texcoords`. Substituted with the origin
            // rather than left as NaN/Inf, the same "absent, not poisoned"
            // rule `normals` already follows: this value still reaches a GPU
            // vertex buffer even on a model this build draws untextured.
            if uv[0].is_finite() && uv[1].is_finite() {
                texcoords.push(uv);
            } else {
                texcoords.push([0.0, 0.0]);
                non_finite_texcoords += 1;
            }
        }
    }
    Some(SubMesh {
        record,
        material,
        indices,
        positions,
        normals,
        texcoords,
        non_finite_texcoords,
        stride,
    })
}

fn u32_at(file: &[u8], at: usize, what: &'static str) -> Result<u32> {
    file.get(at..at + 4)
        .map(|b| u32::from_le_bytes(b.try_into().expect("four bytes")))
        .ok_or(Error::OutOfBounds {
            what,
            end: at + 4,
            len: file.len(),
        })
}

fn f32_at(file: &[u8], at: usize) -> f32 {
    f32::from_le_bytes(file[at..at + 4].try_into().expect("four bytes"))
}

#[cfg(test)]
mod tests;
