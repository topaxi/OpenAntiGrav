//! **The vertex layout a chunk declares**, at the word `+0x58` points at.
//!
//! # Why this exists
//!
//! Everything a reader had been *solving* for is written down. Until this was
//! read, the stride came from a search over three widths judged against the
//! authored bounding box ([`super::stride`]), and the texture coordinate was
//! assumed to be the last four bytes of a vertex, in one of two types told
//! apart by what the bytes decoded to. The file states the stride, and states
//! for each attribute its name, its type and its byte offset - so none of that
//! has to be inferred, and two of the inferences were wrong:
//!
//! - **The last four bytes are usually not the texture coordinate.** On the
//!   commonest stride-18 layout they are `lightmapUV`, and the diffuse
//!   coordinate `Uv1` is at `+0x0a`; on others they are `tangent` or a colour
//!   set. Painting a diffuse texture through a lightmap's atlas-packed
//!   coordinates is what smeared a quarter of Talon's Junction into streaks.
//! - **The stride is not one of three widths.** The disc declares seven - 10,
//!   14, 18, 22, 26, 34 and 38 - and the search gives up on 3,382 of the
//!   disc's 39,372 described chunks, which then draw nothing at all.
//!
//! # Which chunks have one
//!
//! [`super::LAYOUT_DESCRIBED`] chunks only. On a [`super::LAYOUT_INLINE`] chunk
//! the same word is the index count, so reading it as a pointer there is how
//! this reading first came back as noise; [`VertexDecl::parse`] is only called
//! down the described branch.
//!
//! # Layout
//!
//! ```text
//! +0x00  u8   attribute count
//! +0x01  u8   vertex stride
//! +0x02  u16  zero
//! ```
//!
//! then `count` records of eight bytes:
//!
//! ```text
//! +0x00  u32  attribute name hash - see `name_of`
//! +0x04  u16  the vertex stride again
//! +0x06  u8   type: component count in the high nibble, RSX type in the low
//! +0x07  u8   byte offset of the attribute within the vertex
//! ```
//!
//! **Confidence 92.** Measured over every `.rcsmodel` on the disc: all 39,372
//! described chunks resolve a declaration that lies inside the file, and on
//! every one of their 118,000-odd attribute records the repeated stride at
//! `+0x04` equals the header's - an internal cross-check that a wrong offset or
//! a wrong record size could not pass. The declared stride agrees with the
//! search wherever the search settles on one, 35,983 of 35,990.

use oag_formats::ByteOrder;

/// Bytes of declaration header before the first attribute record.
const HEADER_LEN: usize = 4;

/// Bytes per attribute record.
const ATTRIBUTE_LEN: usize = 8;

/// The largest attribute count any declaration on the disc uses, plus room.
///
/// A bound rather than a decode: a count read out of the wrong place is how
/// this module would otherwise walk a megabyte of geometry as records. Seven is
/// the disc's maximum.
const MAX_ATTRIBUTES: usize = 32;

/// `CELL_GCM_VERTEX_F`: 32-bit floats.
pub const RSX_FLOAT: u8 = 2;

/// `CELL_GCM_VERTEX_SF`: 16-bit floats, IEEE halves.
pub const RSX_HALF: u8 = 3;

/// `CELL_GCM_VERTEX_UB`: bytes over `0..=1`.
pub const RSX_UBYTE_NORM: u8 = 4;

/// `CELL_GCM_VERTEX_S32K`: 16-bit signed integers, **not** normalised.
///
/// What a position is written in, dequantised through the chunk's own bias and
/// scale.
pub const RSX_SHORT: u8 = 5;

/// `CELL_GCM_VERTEX_CMP`: one word holding a compressed vector.
///
/// What a normal is written in; [`super::normal`] is the unpacking, recovered
/// before this declaration was.
pub const RSX_COMPRESSED: u8 = 6;

/// One attribute of a vertex, as the declaration spells it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Attribute {
    /// `~crc32` of the attribute's name - see [`Attribute::name`].
    pub name_hash: u32,
    /// How many components, out of the type byte's high nibble.
    pub components: u8,
    /// Which RSX vertex type, out of the type byte's low nibble.
    ///
    /// One of the `RSX_*` constants above on everything the disc uses; kept as
    /// the raw nibble so a value outside them is reported rather than rounded.
    pub rsx_type: u8,
    /// Byte offset of the attribute within the vertex.
    pub offset: u8,
}

impl Attribute {
    /// The attribute's name, for the hashes that have been recovered.
    ///
    /// **`~crc32(name)`, the hash `docs/ghidra/functions/ps3-hdfury-eu/renderer.md`
    /// recovered from `Crc32_HashString`.** The names below were found by
    /// searching candidate strings for that hash, which is a wordlist problem
    /// rather than a decode - so each is only as good as the agreement between
    /// the name and what the attribute demonstrably holds. Three of them are
    /// controls: `position` carries `RSX_SHORT` x3 at offset 0 on all 39,372
    /// chunks, `normal` carries `RSX_COMPRESSED` x1 at offset 6 on all of them,
    /// and `tangent` carries the four normalised bytes an earlier reading had
    /// already measured as a packed tangent's handedness. That three
    /// independently-derived names land on the three attributes whose content
    /// was already known is what makes the rest credible.
    ///
    /// **They are Maya's names**, which agrees with the `.ma` source paths the
    /// `.vex` files carry: `map1`, `map2`, `colorSet1` and `VertexColour1` are
    /// that package's own vocabulary.
    ///
    /// `None` for the 37 hashes still unnamed, the commonest of which is
    /// `0x1aaf7631` - 13,485 uses, always four normalised bytes.
    #[must_use]
    pub fn name(&self) -> Option<&'static str> {
        Some(match self.name_hash {
            0xb9d3_1b0a => "position",
            0xde7a_971b => "normal",
            0xdbe5_f417 => "tangent",
            0x4272_14fc => "Uv1",
            0xdb7b_4546 => "Uv2",
            0x7a3f_521c => "uv1",
            0x49f7_6806 => "Uvset1",
            0x26a7_b665 => "lightmapUV",
            0x2003_d7e6 => "map1",
            0xb90a_865c => "map2",
            0xce5c_d9d9 => "colorSet1",
            0x7493_d450 => "VertexColour1",
            _ => return None,
        })
    }

    /// Whether this attribute is a two-component texture coordinate.
    ///
    /// Both encodings the disc uses: halves on 54,120 attributes and 32-bit
    /// floats on 230. **This is the "two types" an earlier reading was reaching
    /// for**, and it is declared rather than guessed from content; see
    /// [`super::Mesh::texcoords`].
    #[must_use]
    pub fn is_texcoord(&self) -> bool {
        self.components == 2 && matches!(self.rsx_type, RSX_HALF | RSX_FLOAT)
    }

    /// How many bytes the attribute occupies, or `None` for a type whose width
    /// this reading does not know.
    #[must_use]
    pub fn width(&self) -> Option<usize> {
        let each = match self.rsx_type {
            RSX_FLOAT => 4,
            RSX_HALF | RSX_SHORT => 2,
            RSX_UBYTE_NORM => 1,
            RSX_COMPRESSED => 4,
            _ => return None,
        };
        Some(usize::from(self.components) * each)
    }
}

/// What a chunk says its vertices are made of.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VertexDecl {
    /// Bytes per vertex, out of the declaration's own `+0x01`.
    pub stride: usize,
    /// The attributes, in the order the file lists them - which is **not**
    /// offset order, and which lists aliases: a four-half attribute may cover
    /// the same bytes two two-half ones do, so a shader can bind either.
    pub attributes: Vec<Attribute>,
}

impl VertexDecl {
    /// Reads the declaration at `at`, or `None` when it does not fit or does
    /// not cross-check.
    ///
    /// The cross-check is the repeated stride at each record's `+0x04`: it
    /// equals the header's on every record of every declaration on the disc, so
    /// a disagreement means this is not a declaration and the caller should
    /// carry on without one rather than act on the bytes.
    #[must_use]
    pub fn parse(data: &[u8], at: usize) -> Option<Self> {
        let count = usize::from(*data.get(at)?);
        let stride = usize::from(*data.get(at + 1)?);
        if count == 0 || count > MAX_ATTRIBUTES || stride == 0 {
            return None;
        }
        if at + HEADER_LEN + count * ATTRIBUTE_LEN > data.len() {
            return None;
        }
        let mut attributes = Vec::with_capacity(count);
        for k in 0..count {
            let base = at + HEADER_LEN + k * ATTRIBUTE_LEN;
            if usize::from(ByteOrder::Big.u16(data, base + 4)) != stride {
                return None;
            }
            let ty = data[base + 6];
            attributes.push(Attribute {
                name_hash: ByteOrder::Big.u32(data, base),
                components: ty >> 4,
                rsx_type: ty & 0xf,
                offset: data[base + 7],
            });
        }
        Some(Self { stride, attributes })
    }

    /// The attribute a diffuse texture is sampled through.
    ///
    /// **A rule, and it is this module's rather than the file's.** The file
    /// names several texture coordinates per vertex and nothing here reads the
    /// shader that chooses between them, so the rule is: the first declared
    /// two-component coordinate that is not `lightmapUV`. That is right on the
    /// evidence available - a lightmap's coordinates are atlas-packed and
    /// painting a diffuse texture through them streaks visibly, while `Uv1`
    /// (28,298 attributes disc-wide) is the one every layout carrying a
    /// lightmap also carries - and it is a rule rather than a measurement until
    /// a `.rcsmaterial`'s shader is read.
    ///
    /// `None` on a chunk that declares no texture coordinate at all, which 984
    /// of the disc's chunks do; a caller counts those rather than substituting
    /// one.
    #[must_use]
    pub fn diffuse_texcoord(&self) -> Option<&Attribute> {
        let named = |a: &&Attribute| a.name() != Some("lightmapUV");
        self.attributes
            .iter()
            .filter(|a| a.is_texcoord())
            .find(named)
            .or_else(|| self.attributes.iter().find(|a| a.is_texcoord()))
    }

    /// The chunk's second diffuse coordinate set, `Uv2`.
    ///
    /// Named, like [`Self::lightmap_texcoord`]. It is the high half of the
    /// four-component attribute `0x1aefe524` the vertex programs read as one
    /// `vec4` (`TC4.xy` is `Uv1`, `TC4.zw` is this); `hd_water_decl` lists the
    /// Sebenco ice chunks, which declare `Uv1` at byte 18 and `Uv2` at 22.
    #[must_use]
    pub fn second_texcoord(&self) -> Option<&Attribute> {
        self.attributes
            .iter()
            .find(|a| a.is_texcoord() && a.name() == Some("Uv2"))
    }

    /// The attribute a lightmap is sampled through.
    ///
    /// **Named rather than ruled**, unlike [`Self::diffuse_texcoord`]: the
    /// declaration calls this one `lightmapUV` outright, and the name is a
    /// `~crc32` preimage checked three ways - see [`Attribute::name`]. What
    /// makes it more than a plausible name is the correspondence with the
    /// material beside it: **every chunk whose material's second texture is one
    /// of the circuit's `lmaps/*-lmap.gtf` declares this attribute, with no
    /// exceptions on any circuit on the disc**, and no chunk declares one
    /// without the other. Two files decoded independently agreeing exactly is
    /// what a wrong reading of either would break.
    #[must_use]
    pub fn lightmap_texcoord(&self) -> Option<&Attribute> {
        self.attributes
            .iter()
            .find(|a| a.is_texcoord() && a.name() == Some("lightmapUV"))
    }

    /// The attribute a chunk carries its **per-vertex colour data** in,
    /// whatever it means to the material that reads it.
    ///
    /// **Shape, not hash: four normalised bytes, not `tangent`.** Five
    /// distinct attributes disc-wide have that shape
    /// (`crates/render/examples/hd_colour_attr_census.rs`, walking all 643
    /// `.rcsmodel` files): the unnamed `0x1aaf7631` (13,485 chunk
    /// attributes), `VertexColour1` (2,595), `colorSet1` (484), `tangent`
    /// (5,535, excluded by name) and a second unnamed `0xed9a85ea` (18).
    /// **Two different consumers read this method's answer for two different
    /// reasons, and both need every non-`tangent` hash it can return:**
    /// [`super::Mesh::vertex_light`] decodes the raw bytes for whatever the
    /// caller does with them (HD's baked ambient light on a track chunk,
    /// `oag_game::livery::flare`'s `alpha_ramp` repurposing the same
    /// mechanism to pull `VertexColour1.w` as the engine flame's own alpha
    /// ramp - see `docs/ghidra/functions/ps3-hdfury-eu/engine-flare.md` and
    /// `crates/mesh/src/mesh/flame.rs`), while [`Self::light_colour_set`]
    /// below answers the narrower, semantic question of whether the chunk
    /// has the *specific* attribute a lit-race shader treats as a baked
    /// light term. Narrowing this method to that same hash set once broke
    /// the flame ramp outright - `hd_engine_flare_ground_truth.rs`'s
    /// `the_flame_carries_an_opacity_ramp_in_its_vertex_alpha` caught it - so
    /// the split exists precisely so a future narrowing of one does not
    /// silently narrow the other.
    #[must_use]
    pub fn vertex_colour(&self) -> Option<&Attribute> {
        self.attributes.iter().find(|a| {
            a.components == 4 && a.rsx_type == RSX_UBYTE_NORM && a.name() != Some("tangent")
        })
    }

    /// Whether a chunk carries the **specific** attribute a lit-race shader
    /// treats as HD's baked per-vertex light - `colorSet1` or the unnamed
    /// `0x1aaf7631`, and **not** `VertexColour1` or the second unnamed
    /// `0xed9a85ea`.
    ///
    /// **Why this exists separately from [`Self::vertex_colour`], since
    /// 2026-09-13.** `Features::chunk_word`/`Features::for_chunk` ask this
    /// question to pick the `IleVertex` half of a material's shader-variant
    /// key, and every HD ship hull material with a livery/paint attribute
    /// declares `VertexColour1` - a **named, distinct** Maya attribute the
    /// doc comment this replaced never saw, because it was measured only
    /// over Talon's Junction, which has no ship file to carry it. Answering
    /// "yes" for `VertexColour1` there asked every one of those materials
    /// for a shader permutation no ship material on the disc ships under any
    /// `Class`: 78 of 178 drawn ship materials disc-wide, none of which a
    /// different `Class` would have resolved
    /// (`crates/render/examples/hd_ship_class_census.rs`). `VertexColour1`
    /// is a real, separately-used channel - the engine flame's own alpha
    /// ramp reads it through [`Self::vertex_colour`] - just not this one.
    ///
    /// What makes `0x1aaf7631`/`colorSet1` a light rather than a tint is the
    /// shader that reads it: the vertex program moves it into `o[TC1]` and
    /// the fragment program **adds** `f[TC1]` to the lightmap term before
    /// multiplying the albedo - see
    /// `docs/ghidra/functions/ps3-hdfury-eu/renderer.md`, "The lit track
    /// material".
    ///
    /// **`0x1aaf7631` and `colorSet1` are treated as one attribute here, and
    /// that is an inference, not a read**: nothing seen so far ties the two
    /// hashes together directly. What supports it, over Talon's Junction: the
    /// arithmetic (297 + 54 was exactly the circuit's 351 colour-set chunks)
    /// and the exact complementarity with `lightmap_texcoord`, which no chunk
    /// declares alongside either.
    #[must_use]
    pub fn light_colour_set(&self) -> bool {
        self.attributes.iter().any(|a| {
            a.components == 4
                && a.rsx_type == RSX_UBYTE_NORM
                && matches!(a.name_hash, 0x1aaf_7631 | 0xce5c_d9d9)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One stride-18 declaration as Talon's Junction writes it, at `0x0003c820`:
    /// position, normal, `Uv1` and `lightmapUV`.
    fn talons_stride_18() -> Vec<u8> {
        let mut out = vec![0x04, 0x12, 0x00, 0x00];
        for (hash, ty, off) in [
            (0xb9d3_1b0au32, 0x35u8, 0x00u8),
            (0xde7a_971b, 0x16, 0x06),
            (0x4272_14fc, 0x23, 0x0a),
            (0x26a7_b665, 0x23, 0x0e),
        ] {
            out.extend_from_slice(&hash.to_be_bytes());
            out.extend_from_slice(&0x0012u16.to_be_bytes());
            out.push(ty);
            out.push(off);
        }
        out
    }

    #[test]
    fn reads_the_stride_and_every_attribute() {
        let decl = VertexDecl::parse(&talons_stride_18(), 0).expect("a declaration");
        assert_eq!(decl.stride, 18);
        assert_eq!(decl.attributes.len(), 4);
        let names: Vec<_> = decl.attributes.iter().map(Attribute::name).collect();
        assert_eq!(
            names,
            [
                Some("position"),
                Some("normal"),
                Some("Uv1"),
                Some("lightmapUV")
            ]
        );
    }

    #[test]
    fn the_diffuse_coordinate_is_not_the_last_four_bytes() {
        let decl = VertexDecl::parse(&talons_stride_18(), 0).expect("a declaration");
        let uv = decl.diffuse_texcoord().expect("a coordinate");
        assert_eq!(uv.name(), Some("Uv1"));
        // The trap this whole module exists for: `stride - 4` is 0x0e, and
        // 0x0e is the lightmap's.
        assert_eq!(uv.offset, 0x0a);
        assert_ne!(usize::from(uv.offset), decl.stride - 4);
    }

    #[test]
    fn a_position_is_three_shorts_and_a_normal_is_one_compressed_word() {
        let decl = VertexDecl::parse(&talons_stride_18(), 0).expect("a declaration");
        let position = decl.attributes[0];
        assert_eq!((position.components, position.rsx_type), (3, RSX_SHORT));
        assert_eq!(position.width(), Some(6));
        let normal = decl.attributes[1];
        assert_eq!((normal.components, normal.rsx_type), (1, RSX_COMPRESSED));
        assert_eq!(normal.width(), Some(4));
    }

    #[test]
    fn a_record_whose_repeated_stride_disagrees_is_not_a_declaration() {
        let mut bytes = talons_stride_18();
        // The word at the third record's +0x04, which the file repeats.
        bytes[4 + 2 * ATTRIBUTE_LEN + 5] = 0x16;
        assert_eq!(VertexDecl::parse(&bytes, 0), None);
    }

    #[test]
    fn a_count_or_stride_of_zero_is_not_a_declaration() {
        let mut bytes = talons_stride_18();
        bytes[0] = 0;
        assert_eq!(VertexDecl::parse(&bytes, 0), None);
        let mut bytes = talons_stride_18();
        bytes[1] = 0;
        assert_eq!(VertexDecl::parse(&bytes, 0), None);
    }

    #[test]
    fn a_declaration_that_leaves_the_file_is_not_one() {
        let bytes = talons_stride_18();
        assert_eq!(VertexDecl::parse(&bytes[..12], 0), None);
    }

    /// A declaration with `position`, `normal`, `tangent`, `VertexColour1`
    /// and `Uv1` - a ship chunk's own shape (`hd_ship_vcol_dump.rs`).
    fn ship_stride_with_vertex_colour1() -> Vec<u8> {
        let mut out = vec![0x05, 0x14, 0x00, 0x00];
        for (hash, ty, off) in [
            (0xb9d3_1b0au32, 0x35u8, 0x00u8),
            (0xde7a_971b, 0x16, 0x06),
            (0xdbe5_f417, 0x44, 0x07),
            (0x7493_d450, 0x44, 0x0b),
            (0x4272_14fc, 0x23, 0x0f),
        ] {
            out.extend_from_slice(&hash.to_be_bytes());
            out.extend_from_slice(&0x0014u16.to_be_bytes());
            out.push(ty);
            out.push(off);
        }
        out
    }

    /// **`vertex_colour()` still matches `VertexColour1` by shape - the
    /// engine flame's own alpha ramp reads it through exactly this method**
    /// (`oag_game::livery::flare::alpha_ramp`, via `Mesh::vertex_light`).
    /// Narrowing this one to a hash allowlist is the regression
    /// `hd_engine_flare_ground_truth.rs`'s
    /// `the_flame_carries_an_opacity_ramp_in_its_vertex_alpha` caught: see
    /// [`light_colour_set_does_not_match_vertex_colour1`] for the method
    /// that *should* exclude it.
    #[test]
    fn vertex_colour_still_matches_vertex_colour1() {
        let decl = VertexDecl::parse(&ship_stride_with_vertex_colour1(), 0).expect("a declaration");
        let matched = decl
            .vertex_colour()
            .expect("VertexColour1 still matches by shape");
        assert_eq!(matched.name_hash, 0x7493_d450);
    }

    /// **The regression `light_colour_set` exists for.** `VertexColour1` is a
    /// distinct, named attribute from `colorSet1`/the unnamed `0x1aaf7631` -
    /// answering "yes" for it by shape alone made `Features::chunk_word` ask
    /// a ship material for the `IleVertex` permutation it never ships,
    /// failing every ship hull material with a `VertexColour1` channel
    /// regardless of `Class`. See
    /// `crates/render/examples/hd_ship_class_census.rs` and
    /// `hd_ship_vcol_dump.rs` for the disc-wide measurement.
    #[test]
    fn light_colour_set_does_not_match_vertex_colour1() {
        let decl = VertexDecl::parse(&ship_stride_with_vertex_colour1(), 0).expect("a declaration");
        assert!(!decl.light_colour_set());
    }

    /// `colorSet1` still answers `light_colour_set` - the attribute this
    /// reading is actually about.
    #[test]
    fn light_colour_set_matches_color_set_1() {
        let mut bytes = ship_stride_with_vertex_colour1();
        // Swap `VertexColour1`'s hash for `colorSet1`'s at its own record.
        bytes[4 + 3 * ATTRIBUTE_LEN..4 + 3 * ATTRIBUTE_LEN + 4]
            .copy_from_slice(&0xce5c_d9d9u32.to_be_bytes());
        let decl = VertexDecl::parse(&bytes, 0).expect("a declaration");
        assert!(decl.light_colour_set());
    }
}
