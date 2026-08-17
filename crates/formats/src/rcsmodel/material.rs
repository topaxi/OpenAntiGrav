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
//! shader code and is not read. Neither is the `.gtf` texture beside it, which is
//! where a surface's **alpha** lives - see the note on [`Blend`] for why that
//! matters more than it sounds like it should.
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
/// # Nothing blends through this yet, and why
///
/// **A see-through surface needs an alpha, and this project has none for HD.**
/// The alpha is in the `.gtf` texture (7,333 files, 2.4 GiB, unread) and in
/// nothing else: `oag_formats::rcsmodel` decodes positions, indices and normals,
/// and the four bytes at `+0x0a` were measured against the vertex-colour
/// hypothesis and are not one - no lane is `0xff`-dominant the way an alpha
/// would be, and on stride 22 the last is the bimodal `0`/`255` of a packed
/// tangent's handedness.
///
/// So blending an HD surface today would use `alpha = 1.0`, and
/// [`vex::BlendClass::AlphaOver`] at alpha 1 is pixel-identical to drawing it
/// opaque while additionally losing depth ordering, and
/// [`vex::BlendClass::Additive`] would blow every glass panel to white.
///
/// What the renderer does with this instead is **skip the chunk** and count it,
/// which is a wrong picture replaced by an honest absence rather than by a
/// better picture - `oag_render::mesh::rcs::see_through` carries the evidence
/// for that choice and the condition to reverse it. The class this reads is
/// what a blend will use once `.gtf` supplies the alpha.
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
        let name_at = ByteOrder::Big.u32(data, at + 0x04) as usize;
        Some(Self {
            name: cstr(data, name_at),
            state: ByteOrder::Big.u32(data, at + 0x10),
            src_factor: ByteOrder::Big.u16(data, at + 0x14),
            dst_factor: ByteOrder::Big.u16(data, at + 0x16),
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
