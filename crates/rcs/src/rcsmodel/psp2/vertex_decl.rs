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
//! (`crates/game/examples/vita_rcsmodel_rosetta.rs`), then keyed by the
//! header's own offset ([`find_by_header`]). A submesh record's pointer at
//! [`PS4_DECLARATION_POINTER`] (a `u32` on the Vita, a 64-bit word on the PS4)
//! is the offset of its own declaration's header, and **resolves on every one
//! of the 276,239 submeshes of the Vita base, DLC and patch archives** with the
//! declared stride equal to the buffer-derived one
//! (`crates/rcs/tests/psp2_flat_census_ground_truth.rs`; an earlier 14.2%
//! figure is not reproduced). A per-stride map, which this module used to
//! build for the Vita, picked a different diffuse attribute from the record's
//! own on 28.5% of submeshes and read the wrong attribute on three in four of
//! Empire Climb's: it is gone.
//!
//! **Confidence 92**, the same bar HD's identical shape carries: every
//! declaration found this way restates its own stride at each attribute
//! record (`+0x04`, big-endian - see the module doc), and only a
//! self-consistent one is kept.
//!
//! # The pointer is exact on both containers
//!
//! Keyed by stride, one declaration stands for every chunk of that stride, and
//! that is wrong: `tech_de_ra`'s 3,186 submeshes are 8 strides and **43
//! distinct layouts** (stride 24 alone has 18), and the layouts differ in
//! *where the diffuse coordinate is* and in whether a `lightmapUV` follows it.
//! The pointer resolves on **3,186 of 3,186** submeshes of that circuit.
//!
//! **Control group.** A submesh's material names a `lightmap` sampler if and
//! only if its own declaration carries `lightmapUV`: 856 submeshes both, 2,330
//! neither, **0 in either off-diagonal cell** - two independently read fields
//! agreeing on every submesh, which a wrong pointer offset cannot do. The
//! Vita's `u32` agrees the same way (33,308 both, 136,856 neither on the base
//! archive, 0 off-diagonal).

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
/// `~crc32("uv1")`, lower case: the vertex input the `fc*` object-library programs name
/// (`fc06_lambert_simple`: `uv1` passes to the pixel program's `DiffuseMapSampler` lookup).
/// Omega spells it as an `f32` pair ([`PS4_FLOAT2`]) where 2048 has a half or float pair.
pub const LOWERCASE_UV1_HASH: u32 = 0x7a3f_521c;
/// `~crc32("Uv1_Uv2")`: four halves, two coordinate pairs, the vertex input of
/// `2048_shipshield_old`'s programs (`param1` is the first pair, `param2` the second). The
/// declaration also lists `Uv1` and `Uv2` entries, both at the offset of the second pair;
/// nothing explains them.
pub const UV1_UV2_HASH: u32 = 0x1aef_e524;
/// `~crc32("VertexColour1")`: four normalised bytes per vertex (type nibble 11 on PS4).
pub const VERTEX_COLOUR1_HASH: u32 = 0x7493_d450;
/// `~crc32("uv1_uvDirt")`: four halves, the tiling pair in `.xy` and the dirt pair in `.zw`. The
/// vertex input of Omega's `tarmac_*` road programs, which name no `Uv1`.
pub const UV1_UVDIRT_HASH: u32 = 0x273a_b13e;
/// `~crc32("uvMarkings")`: two halves, the road's decal atlas pair, a unique unwrap over the
/// whole road. The second vertex input of Omega's `tarmac_*` road programs.
pub const UV_MARKINGS_HASH: u32 = 0x5f2a_9ab4;
/// `~crc32("lightmapUV")`.
pub const LIGHTMAP_HASH: u32 = 0x26a7_b665;
/// `~crc32("NOVAColor")`: the baked per-vertex light of a Vita circuit's vertex-lit surfaces, four
/// unsigned normalised bytes (type `0x44`). The name is the vertex programs' own attribute
/// (`scripts/vita-gxp.py grep ... NOVAColor`), and its hash is this value (confidence 95).
pub const NOVA_COLOUR_HASH: u32 = 0x1aaf_7631;
/// The type nibble of [`NOVA_COLOUR_HASH`] on the Vita: four unsigned normalised bytes.
pub const VITA_UBYTE4_NORM: u8 = 4;

/// `~crc32("barsUV_blendUV")`: one four-lane half attribute, the bars' pair in `.xy` and the
/// wipe's in `.zw`. The same hash HD's `nr_twinblend` names its `v[8]` by.
pub const BARS_BLEND_HASH: u32 = 0x0e7e_a5a8;

/// The type nibble of a PS4 two-component half-float attribute: `Uv1`,
/// `lightmapUV` and every other coordinate set on all 43 layouts of
/// `tech_de_ra`, which also carries two `f32` pairs (`0`) that are not
/// coordinates of the diffuse or lightmap sets.
pub const PS4_HALF2: u8 = 1;

/// The type nibble of a Vita (SceGxm) two-component half-float attribute: `Uv1`, `lightmapUV`
/// and (four of them) `barsUV_blendUV`.
pub const VITA_HALF2: u8 = 8;

/// The type nibble of a two-component `f32` attribute on the PS4: a diffuse coordinate that is a
/// float pair (23 submeshes of Omega's `data02` name their `Uv1` this way).
pub const PS4_FLOAT2: u8 = 0;

/// The type nibble of an `f32` attribute on the Vita (the position's `3x9`; a diffuse
/// coordinate declared `2x9` is a float pair, 2,291 base submeshes).
pub const FLOAT2: u8 = 9;

/// The type nibble of a PS4 `tangent` (and `normal`): four signed bytes, read `/127`.
pub const PS4_SBYTE4_NORM: u8 = 14;

/// The type nibble of `VertexColour1` on the PS4 flyer shell's reflection (`00_flyer`'s
/// `card_reflectShape`: `ff ff ff` and an alpha of `0x4c`, `0x26` or `0`).
pub const PS4_UBYTE4_NORM: u8 = 11;

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

    /// The packed pair attribute, four halves (see [`UV1_UV2_HASH`]).
    #[must_use]
    pub fn packed_uv_pair(&self) -> Option<&Attribute> {
        self.attribute(UV1_UV2_HASH).filter(|a| a.components == 4)
    }

    /// The lightmap texture coordinate, on the same terms.
    #[must_use]
    pub fn lightmap_texcoord(&self) -> Option<&Attribute> {
        self.attribute(LIGHTMAP_HASH)
    }

    /// `VertexColour1`, four normalised bytes. Only sound on a declaration reached through the
    /// record's own pointer.
    #[must_use]
    pub fn vertex_colour1(&self) -> Option<&Attribute> {
        self.attribute(VERTEX_COLOUR1_HASH)
            .filter(|a| a.components == 4 && a.gxm_type == PS4_UBYTE4_NORM)
    }

    /// `NOVAColor`, four unsigned normalised bytes on the Vita. Only sound on a declaration
    /// reached through the record's own pointer.
    #[must_use]
    pub fn nova_colour(&self) -> Option<&Attribute> {
        self.attribute(NOVA_COLOUR_HASH)
            .filter(|a| a.components == 4 && a.gxm_type == VITA_UBYTE4_NORM)
    }

    /// The `nr_twinblend` pair attribute, four halves. Only sound on a declaration reached
    /// through the record's own pointer.
    #[must_use]
    pub fn bars_blend(&self) -> Option<&Attribute> {
        self.attribute(BARS_BLEND_HASH)
            .filter(|a| a.components == 4)
    }

    /// The tiling pair of a road declaration that carries [`UV1_UVDIRT_HASH`] (four halves of
    /// the container's half type): its `.xy`, which is what the first two components at the
    /// attribute's own offset are. Not the declaration's lower-case `uv1` entry, which sits at
    /// the dirt pair's offset. The dirt pair is the same attribute's `.zw`, four bytes on.
    #[must_use]
    pub fn uv_dirt_tiling(&self, half: u8) -> Option<&Attribute> {
        self.attribute(UV1_UVDIRT_HASH)
            .filter(|a| a.components == 4 && a.gxm_type == half)
    }

    /// `uvMarkings`, a half pair. Only sound on a declaration reached through the record's own
    /// pointer.
    #[must_use]
    pub fn markings_texcoord(&self, half: u8) -> Option<&Attribute> {
        self.attribute(UV_MARKINGS_HASH)
            .filter(|a| a.components == 2 && a.gxm_type == half)
    }

    /// The diffuse coordinate of a declaration known to be **this submesh's
    /// own** - `Uv1` where it is named, then the tiling pair of `uv1_uvDirt`
    /// ([`Self::uv_dirt_tiling`]), otherwise the first two-component
    /// attribute that is not the lightmap's and is a pair of `half`s (the
    /// container's half type: [`PS4_HALF2`] or [`VITA_HALF2`]) or of `f32`s
    /// (the container's float type: [`PS4_FLOAT2`] or [`FLOAT2`]).
    ///
    /// **Only sound on a declaration reached through the record's own pointer**
    /// (`PS4_DECLARATION_POINTER`), on both containers. Where neither is
    /// named, the diffuse set is the file's own `uv1` (`0x7a3f521c`), `Uv2`
    /// (`0xdb7b4546`) or an unnamed hash, with the lightmap's `lightmapUV`
    /// after it - HD's own rule, "a two-component coordinate that is not
    /// `lightmapUV`" (`crate::rcsmodel::vertex_decl::VertexDecl::diffuse_texcoord`);
    /// **confidence 70**, so [`Self::diffuse_texcoord_is_fallback`] says when
    /// it is the one answering. `uv1_uvDirt` is by name, not that rule: the road's
    /// `uvMarkings` is first in file order and is a unique unwrap, not a tiling set
    /// (`docs/formats/omega-status.md`, "The `tarmac_*` road families draw the wrong
    /// coordinate set", confidence 85).
    #[must_use]
    pub fn own_diffuse_texcoord(&self, half: u8, float: u8) -> Option<&Attribute> {
        self.diffuse_texcoord()
            .or_else(|| self.uv_dirt_tiling(half))
            .or_else(|| {
                self.attributes.iter().find(|a| {
                    a.name_hash != LIGHTMAP_HASH
                        && a.components == 2
                        && (a.gxm_type == half
                            || a.gxm_type == FLOAT2
                            || (a.gxm_type == float && a.name_hash == LOWERCASE_UV1_HASH))
                })
            })
    }

    /// True when [`Self::own_diffuse_texcoord`] answers by the confidence-70 rule, not by name.
    #[must_use]
    pub fn diffuse_texcoord_is_fallback(&self, half: u8, float: u8) -> bool {
        self.diffuse_texcoord().is_none()
            && self.uv_dirt_tiling(half).is_none()
            && self.own_diffuse_texcoord(half, float).is_some()
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

/// Every declaration in `cpu_bytes`, keyed by the offset of its header - what
/// a PS4 submesh's pointer at [`PS4_DECLARATION_POINTER`] holds. See the module
/// doc.
#[must_use]
pub fn find_by_header(cpu_bytes: &[u8]) -> HashMap<usize, VertexDecl> {
    scan(cpu_bytes).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn attr(name_hash: u32, components: u8, gxm_type: u8, offset: u8) -> Attribute {
        Attribute {
            name_hash,
            components,
            gxm_type,
            offset,
        }
    }

    /// Rockway Stadium's road submesh: `uvMarkings` (20), `lightmapUV` (24), `uv1_uvDirt` as
    /// four halves (28), and the lower-case `uv1` and `uvDirt` both at 32. No `Uv1`.
    fn arena_road() -> VertexDecl {
        VertexDecl {
            stride: 36,
            attributes: vec![
                attr(POSITION_HASH, 3, PS4_FLOAT2, 0),
                attr(NORMAL_HASH, 4, PS4_SBYTE4_NORM, 12),
                attr(
                    crate::rcsmaterial::name_hash("uvMarkings"),
                    2,
                    PS4_HALF2,
                    20,
                ),
                attr(LIGHTMAP_HASH, 2, PS4_HALF2, 24),
                attr(UV1_UVDIRT_HASH, 4, PS4_HALF2, 28),
                attr(LOWERCASE_UV1_HASH, 2, PS4_HALF2, 32),
                attr(crate::rcsmaterial::name_hash("uvDirt"), 2, PS4_HALF2, 32),
            ],
        }
    }

    #[test]
    fn the_uv1_uvdirt_constant_is_the_hash_of_its_name() {
        assert_eq!(UV1_UVDIRT_HASH, crate::rcsmaterial::name_hash("uv1_uvDirt"));
    }

    #[test]
    fn the_uv_markings_constant_is_the_hash_of_its_name() {
        assert_eq!(
            UV_MARKINGS_HASH,
            crate::rcsmaterial::name_hash("uvMarkings")
        );
    }

    #[test]
    fn a_road_declaration_names_its_markings_pair_at_20_and_its_dirt_pair_at_32() {
        let decl = arena_road();
        assert_eq!(decl.markings_texcoord(PS4_HALF2).unwrap().offset, 20);
        let dirt = decl.uv_dirt_tiling(PS4_HALF2).unwrap();
        assert_eq!(dirt.offset + 4, 32, "the dirt pair is the second half pair");
        let mut other = arena_road();
        other.attributes[2] = attr(UV_MARKINGS_HASH, 2, PS4_FLOAT2, 20);
        assert!(other.markings_texcoord(PS4_HALF2).is_none());
    }

    #[test]
    fn a_road_declaration_without_uv1_draws_the_tiling_pair_at_offset_28() {
        let decl = arena_road();
        let own = decl.own_diffuse_texcoord(PS4_HALF2, PS4_FLOAT2).unwrap();
        assert_eq!(own.name_hash, UV1_UVDIRT_HASH);
        assert_eq!(own.offset, 28);
        assert!(!decl.diffuse_texcoord_is_fallback(PS4_HALF2, PS4_FLOAT2));
    }

    #[test]
    fn a_named_uv1_still_wins_over_uv1_uvdirt() {
        let mut decl = arena_road();
        decl.attributes.push(attr(UV1_HASH, 2, PS4_HALF2, 16));
        let own = decl.own_diffuse_texcoord(PS4_HALF2, PS4_FLOAT2).unwrap();
        assert_eq!((own.name_hash, own.offset), (UV1_HASH, 16));
    }

    #[test]
    fn a_uv1_uvdirt_that_is_not_four_halves_is_not_taken() {
        let mut decl = arena_road();
        decl.attributes[4] = attr(UV1_UVDIRT_HASH, 2, PS4_HALF2, 28);
        let own = decl.own_diffuse_texcoord(PS4_HALF2, PS4_FLOAT2).unwrap();
        assert_eq!(own.offset, 20);
        decl.attributes[4] = attr(UV1_UVDIRT_HASH, 4, PS4_FLOAT2, 28);
        let own = decl.own_diffuse_texcoord(PS4_HALF2, PS4_FLOAT2).unwrap();
        assert_eq!(own.offset, 20);
    }
}
