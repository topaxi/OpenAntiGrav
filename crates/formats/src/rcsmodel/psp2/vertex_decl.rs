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

use std::collections::HashMap;

/// Bytes of declaration header before the first attribute record.
const HEADER_LEN: usize = 4;

/// Bytes per attribute record.
const ATTRIBUTE_LEN: usize = 8;

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

/// Every declaration in `cpu_bytes`, keyed by its own stride.
///
/// **Not every declaration of a given stride is identical** - two chunks
/// can share a stride while packing different attributes into it, so the
/// first one found is not always the useful one. This keeps the first *that
/// names a `Uv1` attribute* over the first found outright, since that is the
/// one thing every caller of this map wants; an earlier version that kept
/// strictly the first declaration measured 27.6% of the corpus's submeshes
/// resolving a texcoord where this measures 92.5%.
///
/// Found by anchoring on every occurrence of [`POSITION_HASH`] and decoding
/// outward - see the module doc for why this, rather than a per-submesh
/// pointer, is what [`super::submeshes`] uses.
#[must_use]
pub fn find_by_stride(cpu_bytes: &[u8]) -> HashMap<usize, VertexDecl> {
    let anchor = POSITION_HASH.to_le_bytes();
    let mut out: HashMap<usize, VertexDecl> = HashMap::new();
    for (i, _) in cpu_bytes
        .windows(4)
        .enumerate()
        .filter(|(_, w)| *w == anchor)
    {
        let Some(header_at) = i.checked_sub(HEADER_LEN) else {
            continue;
        };
        let Some(decl) = parse(cpu_bytes, header_at) else {
            continue;
        };
        match out.get(&decl.stride) {
            Some(existing) if existing.diffuse_texcoord().is_some() => {}
            _ => {
                out.insert(decl.stride, decl);
            }
        }
    }
    out
}
