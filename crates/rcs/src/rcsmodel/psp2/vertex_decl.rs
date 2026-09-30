//! Wipeout 2048's own vertex declaration.
//!
//! Structurally identical to Wipeout HD's own
//! ([`crate::rcsmodel::vertex_decl`]) - a 4-byte header (`count`, `stride`,
//! two reserved bytes) then `count` 8-byte attribute records - with one twist:
//! **`name_hash` is little-endian, like the rest of this container, but the
//! record's own repeated stride mirrors HD's port and stays big-endian.**
//! Consistent with a serializer that writes scalar fields through explicit
//! byte shifts (host-endian-independent) while `name_hash` is a straight
//! native-endian word copy the port never adjusted. See
//! `docs/formats/2048-rcsmodel.md`.
//!
//! # How a declaration is found
//!
//! **By anchoring on the `position` hash and scanning the whole CPU
//! section**, the way this reading was *discovered*
//! (`crates/game/examples/vita_rcsmodel_rosetta.rs`) - not through a
//! per-submesh pointer. A submesh record does carry a word at `+0x28` that
//! is a valid declaration offset on some files (the ship this reading was
//! built against among them), but it resolves for only 14.2% of the
//! corpus's submeshes where the anchor scan resolves 92.5% - so it is a
//! real but unreliable field, not the mechanism this reading uses. Instead
//! [`find_by_stride`] builds one map per file, keyed by the header's own
//! stride, and [`super::submeshes`] looks a submesh's already-measured
//! stride up in it directly.
//!
//! **Confidence 92**, the same bar HD's identical shape carries: every
//! declaration found this way restates its own stride at each attribute
//! record (`+0x04`, big-endian - see the module doc), and only a
//! self-consistent one is kept.
//!
//! # The PS4 keeps the per-submesh pointer, and it is exact there
//!
//! Keyed by stride, one declaration stands for every chunk of that stride, and
//! on a PS4 circuit that is wrong: `tech_de_ra`'s 3,186 submeshes are 8 strides
//! and **43 distinct layouts** (stride 24 alone has 18), and the layouts differ
//! in *where the diffuse coordinate is* and in whether a `lightmapUV` follows
//! it. A PS4 record holds an 8-byte pointer at [`PS4_DECLARATION_POINTER`] that
//! is the address of its own declaration's header, and it resolves on **3,186 of
//! 3,186** submeshes of that circuit with the declared stride equal to the
//! buffer-derived one on every one. [`find_by_header`] is that lookup.
//!
//! **Control group.** A submesh's material names a `lightmap` sampler if and
//! only if its own declaration carries `lightmapUV`: 856 submeshes both, 2,330
//! neither, **0 in either off-diagonal cell** - two independently read fields
//! agreeing on every submesh, which a wrong pointer offset cannot do. The
//! Vita's `u32` at the same offset resolves for 14.2 % and is left alone.

use std::collections::HashMap;

/// Bytes of declaration header before the first attribute record.
const HEADER_LEN: usize = 4;

/// Bytes per attribute record.
const ATTRIBUTE_LEN: usize = 8;

/// Where a PS4 submesh record holds the address of its own declaration's
/// header: a 64-bit word, `+0x28` from [`super::INDEX_POINTER`]'s record start.
/// See the module doc for the measurement.
pub const PS4_DECLARATION_POINTER: usize = 0x28;

/// The largest attribute count any declaration measured on this title uses,
/// plus room - the same bound HD's own reading takes for the same reason: a
/// count read out of the wrong place would otherwise walk the section as
/// attribute records.
const MAX_ATTRIBUTES: usize = 16;

/// `~crc32("position")` - HD's own hash, carried over unchanged. See
/// `crates/formats/src/rcsmodel/vertex_decl.rs`.
pub const POSITION_HASH: u32 = 0xb9d3_1b0a;
/// `~crc32("normal")`.
pub const NORMAL_HASH: u32 = 0xde7a_971b;
/// `~crc32("tangent")`.
pub const TANGENT_HASH: u32 = 0xdbe5_f417;
/// `~crc32("Uv1")` - the diffuse texture coordinate.
pub const UV1_HASH: u32 = 0x4272_14fc;
/// `~crc32("lightmapUV")`.
pub const LIGHTMAP_HASH: u32 = 0x26a7_b665;

/// The type nibble of a PS4 two-component half-float attribute: `Uv1`,
/// `lightmapUV` and every other coordinate set on all 43 layouts of
/// `tech_de_ra`, which also carries two `f32` pairs (`0`) that are not
/// coordinates of the diffuse or lightmap sets.
pub const PS4_HALF2: u8 = 1;

/// One attribute of a vertex, as the declaration spells it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Attribute {
    /// `~crc32` of the attribute's name, little-endian - see the module doc.
    pub name_hash: u32,
    /// How many components, out of the type byte's high nibble.
    pub components: u8,
    /// Which SceGxm vertex type, out of the type byte's low nibble.
    ///
    /// **Not HD's RSX enum** - 2048 runs on the Vita's SceGxm, a different
    /// hardware type table. Only `normal`'s code (`5`, three signed bytes -
    /// see [`super::unpack_normal`]) and `Uv1`/`lightmapUV`'s (`8`, two
    /// little-endian `f16`s - see [`super::unpack_texcoord`]) are decoded;
    /// see `docs/formats/2048-rcsmodel.md` for what is not.
    pub gxm_type: u8,
    /// Byte offset of the attribute within the vertex.
    pub offset: u8,
}

/// One chunk's vertex layout: the header's own stride, and its attributes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VertexDecl {
    /// Bytes per vertex, restated by every attribute record - see [`parse`].
    pub stride: usize,
    /// Every attribute the declaration names, in file order.
    pub attributes: Vec<Attribute>,
}

impl VertexDecl {
    /// The attribute of this name, if the declaration carries one.
    #[must_use]
    pub fn attribute(&self, name_hash: u32) -> Option<&Attribute> {
        self.attributes.iter().find(|a| a.name_hash == name_hash)
    }

    /// The diffuse texture coordinate, named `Uv1` outright - unlike HD's own
    /// reading, this container never has to guess which of several
    /// two-component attributes it is; only one carries this hash.
    #[must_use]
    pub fn diffuse_texcoord(&self) -> Option<&Attribute> {
        self.attribute(UV1_HASH)
    }

    /// The lightmap texture coordinate, on the same terms.
    #[must_use]
    pub fn lightmap_texcoord(&self) -> Option<&Attribute> {
        self.attribute(LIGHTMAP_HASH)
    }

    /// The diffuse coordinate of a declaration known to be **this submesh's
    /// own** - `Uv1` where it is named, otherwise the first two-component
    /// half-float attribute that is not the lightmap's.
    ///
    /// **Only sound on a declaration reached through the record's own pointer**
    /// ([`PS4_DECLARATION_POINTER`]). Keyed by stride, "the first that names a
    /// `Uv1`" was the best available and [`Self::diffuse_texcoord`] still is
    /// there; on a PS4 circuit the same stride also carries layouts with no
    /// `Uv1` at all, where the diffuse set is the file's own `uv1`
    /// (`0x7a3f521c`), `Uv2` (`0xdb7b4546`) or an unnamed hash sitting first
    /// and the lightmap's `lightmapUV` after it - HD's own rule, "a
    /// two-component coordinate that is not `lightmapUV`"
    /// (`crate::rcsmodel::vertex_decl::VertexDecl::diffuse_texcoord`).
    #[must_use]
    pub fn own_diffuse_texcoord(&self) -> Option<&Attribute> {
        self.diffuse_texcoord().or_else(|| {
            self.attributes.iter().find(|a| {
                a.name_hash != LIGHTMAP_HASH && a.components == 2 && a.gxm_type == PS4_HALF2
            })
        })
    }
}

/// Reads the declaration at `header_at` within `cpu_bytes` (the CPU section's
/// own bytes, not the whole file).
///
/// # What "checked" means
///
/// Every attribute record restates the vertex stride at `+0x04`, read
/// **big-endian** - see the module doc. A record whose restated stride
/// disagrees with the header's own is not trusted as a real declaration -
/// the same cross-check HD's own `VertexDecl::parse` rests on.
#[must_use]
pub fn parse(cpu_bytes: &[u8], header_at: usize) -> Option<VertexDecl> {
    let count = usize::from(*cpu_bytes.get(header_at)?);
    let stride = usize::from(*cpu_bytes.get(header_at + 1)?);
    if count == 0 || count > MAX_ATTRIBUTES || stride == 0 {
        return None;
    }
    let end = header_at
        .checked_add(HEADER_LEN)?
        .checked_add(count.checked_mul(ATTRIBUTE_LEN)?)?;
    if end > cpu_bytes.len() {
        return None;
    }
    let mut attributes = Vec::with_capacity(count);
    for k in 0..count {
        let base = header_at + HEADER_LEN + k * ATTRIBUTE_LEN;
        let record_stride = u16::from_be_bytes([cpu_bytes[base + 4], cpu_bytes[base + 5]]) as usize;
        if record_stride != stride {
            return None;
        }
        let ty = cpu_bytes[base + 6];
        attributes.push(Attribute {
            name_hash: u32::from_le_bytes(cpu_bytes[base..base + 4].try_into().unwrap()),
            components: ty >> 4,
            gxm_type: ty & 0xf,
            offset: cpu_bytes[base + 7],
        });
    }
    Some(VertexDecl { stride, attributes })
}

/// Every declaration in `cpu_bytes` with the offset of its header, in the order
/// the anchor is found.
///
/// Found by anchoring on every occurrence of [`POSITION_HASH`] and decoding
/// outward - see the module doc for why this, rather than a per-submesh
/// pointer, is what the Vita's [`super::submeshes`] uses.
fn scan(cpu_bytes: &[u8]) -> impl Iterator<Item = (usize, VertexDecl)> + '_ {
    let anchor = POSITION_HASH.to_le_bytes();
    cpu_bytes
        .windows(4)
        .enumerate()
        .filter(move |(_, w)| *w == anchor)
        .filter_map(move |(i, _)| {
            let header_at = i.checked_sub(HEADER_LEN)?;
            Some((header_at, parse(cpu_bytes, header_at)?))
        })
}

/// Every declaration in `cpu_bytes`, keyed by its own stride.
///
/// **Not every declaration of a given stride is identical** - two chunks
/// can share a stride while packing different attributes into it, so the
/// first one found is not always the useful one. This keeps the first *that
/// names a `Uv1` attribute* over the first found outright, since that is the
/// one thing every caller of this map wants; an earlier version that kept
/// strictly the first declaration measured 27.6% of the corpus's submeshes
/// resolving a texcoord where this measures 92.5%.
#[must_use]
pub fn find_by_stride(cpu_bytes: &[u8]) -> HashMap<usize, VertexDecl> {
    let mut out: HashMap<usize, VertexDecl> = HashMap::new();
    for (_, decl) in scan(cpu_bytes) {
        match out.get(&decl.stride) {
            Some(existing) if existing.diffuse_texcoord().is_some() => {}
            _ => {
                out.insert(decl.stride, decl);
            }
        }
    }
    out
}

/// Every declaration in `cpu_bytes`, keyed by the offset of its header - what
/// a PS4 submesh's pointer at [`PS4_DECLARATION_POINTER`] holds. See the module
/// doc.
#[must_use]
pub fn find_by_header(cpu_bytes: &[u8]) -> HashMap<usize, VertexDecl> {
    scan(cpu_bytes).collect()
}
