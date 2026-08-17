//! The material table a `.rcsmodel` carries beside its geometry.
//!
//! # What this is for
//!
//! **Which of a circuit's surfaces are see-through.** Wipeout HD draws glass
//! tunnels, cloud plates, fences and crowd billboards through the same chunk
//! list as its road, and nothing in the geometry tells the two apart - a chunk
//! is a pile of quantised positions either way. The material a chunk points at
//! is what says which, and that is the whole of what is decoded here.
//!
//! # What is *not* decoded
//!
//! The `.rcsmaterial` file the [`Material::name`] path leads to is compiled RSX
//! shader code and is not read - so *which* of a material's two textures the
//! shader samples for what is unknown, and only the first is painted. The
//! textures themselves are read: [`Material::texture`] names a `.gtf` and
//! `oag_formats::gtf` decodes it, which is where a surface's **alpha** comes
//! from and why [`Blend`] can be drawn at all.
//!
//! # Layout
//!
//! The file header names the table:
//!
//! ```text
//! +0x2c  u32   material count
//! +0x30  u32   offset of the material offset table: `count` big-endian u32s
//! ```
//!
//! and one material record, at an offset that table gives:
//!
//! ```text
//! +0x04  u32   file offset of the material's own path, NUL-terminated
//! +0x10  u32   state word; the low two bits are [`Material::transparency`]
//! +0x14  u16   source blend factor
//! +0x16  u16   destination blend factor
//! ```
//!
//! Records are not a fixed size: consecutive offsets differ by 96 to 768 bytes
//! across the disc, most often 128. Everything above sits inside the smallest of
//! them.

use crate::{ByteOrder, vex};

/// How many low bits of the state word select the transparency mode.
const TRANSPARENCY_BITS: u32 = 0x3;

/// What a texture path ends in, lowercased.
///
/// Used as a *check* on the two pointer fields rather than as a search key -
/// see [`Material::parse`].
const TEXTURE_SUFFIX: &str = ".gtf";

/// The blend factor this module reads as "one", `GL_ONE`.
///
/// **The enum values are an inference, not a decode - confidence 70.** The four
/// values the disc uses are `0x0001`, `0x0300`, `0x0302` and `0x0303`, and those
/// are exactly `GL_ONE`, `GL_SRC_COLOR`, `GL_SRC_ALPHA` and
/// `GL_ONE_MINUS_SRC_ALPHA`, whose numbering the RSX inherits from OpenGL. That
/// four distinct values land on four consecutive, meaningful members of a
/// published enum is strong, but nothing here has read the code that consumes
/// them, so it is short of the 90 an executed branch would carry.
pub const FACTOR_ONE: u16 = 0x0001;

/// `GL_ONE_MINUS_SRC_ALPHA`. See [`FACTOR_ONE`] for how the enum was read.
pub const FACTOR_ONE_MINUS_SRC_ALPHA: u16 = 0x0303;

/// One entry of the material table: what the file says about how a surface is
/// drawn, short of the shader itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Material {
    /// The `.rcsmaterial` path, out of the file's own string pool.
    ///
    /// Kept whole rather than reduced to a leaf name, because the directory is
    /// part of the identity: `glass_texture_n` appears under several circuits.
    pub name: String,
    /// The word at `+0x10`, whose low two bits are [`Self::transparency`].
    ///
    /// The other bits take 17 distinct combinations disc-wide and none of them
    /// is decoded; the raw word is kept so a later reading starts from the data
    /// rather than from this module's summary of it.
    pub state: u32,
    /// The source blend factor at `+0x14`.
    pub src_factor: u16,
    /// The destination blend factor at `+0x16`.
    pub dst_factor: u16,
    /// The `.gtf` path at `+0x58`: the texture this material paints with.
    ///
    /// **In the clear, and on every material of both models measured** - 4 of 4
    /// on Assegai, 200 of the first 200 on Talon's Junction. `WindscreenShape`'s
    /// material names `data/ships/assegai/livery1/assegai_glass.gtf`, which is
    /// what a windscreen should be painted with, and the circuit's name
    /// `talons_support_struts.gtf` and `tunnel_fx_diffuse.gtf`.
    ///
    /// Empty when the pointer leaves the file, which nothing measured does.
    pub texture: String,
    /// A second `.gtf` path at `+0x78`, on the materials that carry one.
    ///
    /// **Present on 337 of Talon's Junction's 442 and 2 of Assegai's 4, and it
    /// is not one thing.** The first look at it suggested a normal map, since
    /// Assegai's two are `assegai_n.gtf` and `assegai_glass_n.gtf`. Widening the
    /// sample by four materials refuted that outright - one slot, at least four
    /// uses:
    ///
    /// | material | first texture | second |
    /// | --- | --- | --- |
    /// | `diffuse_with_specular_from_alpha_n_vcol` | `assegai_tp_1024.gtf` | `assegai_n.gtf` |
    /// | `tunnel_fx_noalpha` | `tunnel_fx_diffuse.gtf` | `tunnel_fx_emissive.gtf` |
    /// | `clouds` | `clouds_new.gtf` | `cloud mask.gtf` |
    /// | `diffusewithalphachannel` | `holebaralpha.gtf` | an `lmaps/...-lmap.gtf` |
    ///
    /// A normal map, an emissive map, a coverage mask and a lightmap. Which it
    /// is per material is in the `.rcsmaterial`'s shader, which is not read, so
    /// **nothing samples this** and a surface whose coverage lives here paints
    /// solid - the cloud plate above being the one that shows.
    pub second_texture: Option<String>,
}

/// Which of three modes the low two bits of [`Material::state`] select.
///
/// **The field is measured; the meaning of value 2 is not.** Across all 15,762
/// materials on the HD disc the field takes only 0, 1 and 2, never 3.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Transparency {
    /// `0`: opaque.
    ///
    /// **This is what says the field gates the factor pair**, and it is a
    /// structural argument rather than one from material names: on 13,183 of
    /// these 13,188 materials the factor pair is exactly `0302`/`0303`, the
    /// default a state block is left holding when nothing consumes it, while
    /// the other two modes range over eight distinct pairs between them.
    Opaque,
    /// `1`: see-through, blended with the equation [`Material::blend`] reads
    /// out of the factor pair. 2,362 materials disc-wide.
    Blended,
    /// `2`: see-through as well, and **not** distinguished from
    /// [`Self::Blended`] by its equation - 211 of these 212 materials carry the
    /// same `0302`/`0303` pair.
    ///
    /// What *does* distinguish it is unrecovered. The obvious hypothesis is an
    /// alpha test, because `jd_alphalambert_alphatest`, `fence_alpha` and
    /// `nr_crowd_bustle` are here and are the kind of surface a cutout is for -
    /// but `hd_bombfire_glow` and `zone_death_electricity` are here too and are
    /// not cutouts, so the name evidence does not hold up and this value stays
    /// unnamed. See `docs/formats/rcsmodel.md`.
    Mode2,
}

/// What a material's factor pair asks for, once [`Transparency`] says the pair
/// is consumed at all.
///
/// # What draws through it, and what still does not
///
/// **A blend needs an alpha, and the alpha is in the texture.** That is why
/// this table sat unwired when it was first read: `oag_formats::rcsmodel`
/// decodes positions, indices and normals, and the four bytes at `+0x0a` were
/// measured against the vertex-colour hypothesis and are not one - no lane is
/// `0xff`-dominant the way an alpha would be, and on stride 22 the last is the
/// bimodal `0`/`255` of a packed tangent's handedness. With `alpha = 1.0`,
/// [`vex::BlendClass::AlphaOver`] paints exactly the opaque pixels while losing
/// depth ordering and [`vex::BlendClass::Additive`] blows a glass panel white.
///
/// `oag_formats::gtf` closed that: [`Material::texture`] names a `.gtf`, its
/// `to_rgba` carries the alpha, and `oag_render::mesh::rcs` draws these surfaces
/// blended with the class below.
///
/// **One case still paints solid, and it is a property of the data rather than
/// of this module.** Where a material's coverage lives in the *second* texture
/// at [`Material::second_texture`] - Talon's Junction's cloud plate names
/// `cloud mask.gtf` beside `clouds_new.gtf` - nothing here samples it, because
/// what that slot is for varies per shader and no shader has been read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Blend {
    /// [`Transparency::Opaque`]: the factor pair is not consumed.
    Opaque,
    /// See-through, with an equation this module maps onto the class
    /// `oag_formats::vex` already recovered from Pulse.
    Class(vex::BlendClass),
    /// See-through, with a factor pair that is **not** mapped.
    ///
    /// Three materials disc-wide - `hologram` at `0300`/`0302`,
    /// `dg_zonelights1` at `0302`/`0300`, `zone_death_electricity` at
    /// `0001`/`0302` - and none of them on any circuit's road, which is what
    /// makes leaving them unmapped cheap. Reported rather than folded into the
    /// nearest class, because `Pulse`'s three classes are a recovered fact about
    /// *Pulse* and there is no evidence HD's equations are a subset of them.
    Unmapped {
        /// [`Material::src_factor`].
        src: u16,
        /// [`Material::dst_factor`].
        dst: u16,
    },
}

impl Material {
    /// Reads one record, or `None` if it does not fit inside `data`.
    pub(super) fn parse(data: &[u8], at: usize) -> Option<Self> {
        if at + 0x18 > data.len() {
            return None;
        }
        // **Checked to be a texture path rather than assumed to be one.** These
        // two words sit past the shortest record this reader tolerates, and
        // `+0x78` is absent on 296 of Talon's Junction's 442 materials - where
        // it is absent the word is not a pointer at all, so following it blindly
        // would put arbitrary bytes in a `String`. A field that does not lead to
        // a printable `.gtf` path is reported as no texture.
        let path = |off: usize| {
            let s = (at + off + 4 <= data.len())
                .then(|| cstr(data, ByteOrder::Big.u32(data, at + off) as usize))?;
            let looks_like_one = s.len() < 256
                && s.bytes().all(|b| (0x20..0x7f).contains(&b))
                && s.to_ascii_lowercase().ends_with(TEXTURE_SUFFIX);
            looks_like_one.then_some(s)
        };
        Some(Self {
            name: cstr(data, ByteOrder::Big.u32(data, at + 0x04) as usize),
            state: ByteOrder::Big.u32(data, at + 0x10),
            src_factor: ByteOrder::Big.u16(data, at + 0x14),
            dst_factor: ByteOrder::Big.u16(data, at + 0x16),
            texture: path(0x58).unwrap_or_default(),
            second_texture: path(0x78),
        })
    }

    /// Which of the three modes this material's state word selects, or `None`
    /// for the fourth encoding, which nothing on the disc uses.
    #[must_use]
    pub fn transparency(&self) -> Option<Transparency> {
        match self.state & TRANSPARENCY_BITS {
            0 => Some(Transparency::Opaque),
            1 => Some(Transparency::Blended),
            2 => Some(Transparency::Mode2),
            _ => None,
        }
    }

    /// Whether this material is drawn see-through at all.
    ///
    /// The one question a caller usually has, and the one the evidence is
    /// strongest on: it does not depend on what separates
    /// [`Transparency::Blended`] from [`Transparency::Mode2`], only on whether
    /// the field is zero. An unrecognised fourth encoding reads as opaque, which
    /// is the direction that draws geometry rather than hiding it.
    #[must_use]
    pub fn is_see_through(&self) -> bool {
        matches!(
            self.transparency(),
            Some(Transparency::Blended | Transparency::Mode2)
        )
    }

    /// The blend equation this material asks for.
    ///
    /// **Keyed on the destination factor alone**, which is what separates the
    /// two families the disc actually uses: a destination of [`FACTOR_ONE`]
    /// leaves what is already there and adds to it whatever the source factor
    /// weights, and a destination of [`FACTOR_ONE_MINUS_SRC_ALPHA`] replaces it
    /// in proportion to coverage. The source factor varies within each family -
    /// `0302`, `0001` and `0300` all appear against a destination of `0001` -
    /// and `oag_formats::vex::BlendClass` has no member for that distinction, so
    /// folding it in would be inventing precision.
    ///
    /// Nothing draws through this; see [`Blend`].
    #[must_use]
    pub fn blend(&self) -> Blend {
        if !self.is_see_through() {
            return Blend::Opaque;
        }
        match self.dst_factor {
            FACTOR_ONE => Blend::Class(vex::BlendClass::Additive),
            FACTOR_ONE_MINUS_SRC_ALPHA => Blend::Class(vex::BlendClass::AlphaOver),
            dst => Blend::Unmapped {
                src: self.src_factor,
                dst,
            },
        }
    }
}

/// The NUL-terminated string at `at`, or empty when the pointer leaves the file.
fn cstr(data: &[u8], at: usize) -> String {
    if at >= data.len() {
        return String::new();
    }
    let end = data[at..]
        .iter()
        .position(|&b| b == 0)
        .map_or(data.len(), |n| at + n);
    String::from_utf8_lossy(&data[at..end]).into_owned()
}
