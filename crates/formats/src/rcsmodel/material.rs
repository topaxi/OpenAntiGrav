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
//! shader code and is largely not read - so *which* of a material's two
//! textures the shader samples for what is unknown in general, and only the
//! first is painted. The textures themselves are read: [`Material::texture`]
//! names a `.gtf` and `oag_texture::gtf` decodes it, which is where a surface's
//! **alpha** comes from and why [`Blend`] can be drawn at all.
//!
//! **Two things have since come off that pile.** The *values* a material
//! instance supplies to its program are decoded here - see [`parameters`] - and
//! one program, the engine flare's `flame_test`, is read instruction by
//! instruction on
//! `docs/ghidra/functions/ps3-hdfury-eu/engine-flare.md`. Every other
//! `.rcsmaterial` on the disc is still compiled code nothing here follows.
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
//! +0x18  u32   alpha test comparison function
//! +0x1c  f32   alpha test reference, in `[0, 1]`
//! ```
//!
//! Records are not a fixed size: consecutive offsets differ by 96 to 768 bytes
//! across the disc, most often 128. Everything above sits inside the smallest of
//! them.

use crate::ByteOrder;

mod parameters;
pub use parameters::{KIND_SAMPLER, Parameter, parameters, samplers};

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

/// `GL_SRC_COLOR`. See [`FACTOR_ONE`] for how the enum was read.
pub const FACTOR_SRC_COLOUR: u16 = 0x0300;

/// `GL_SRC_ALPHA`. See [`FACTOR_ONE`] for how the enum was read.
pub const FACTOR_SRC_ALPHA: u16 = 0x0302;

/// `GL_ONE_MINUS_SRC_ALPHA`. See [`FACTOR_ONE`] for how the enum was read.
pub const FACTOR_ONE_MINUS_SRC_ALPHA: u16 = 0x0303;

/// One side of a blend equation, as the four values the disc uses spell it.
///
/// **A rename of a measured number, and nothing more.** Every member is one of
/// the constants above, the mapping is the identity, and a value outside the
/// four is not given a member - see [`Self::from_rsx`]. What this buys over
/// passing the `u16` around is that a renderer can translate it without
/// re-deriving which OpenGL constant each number is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Factor {
    /// [`FACTOR_ONE`].
    One,
    /// [`FACTOR_SRC_COLOUR`].
    SrcColour,
    /// [`FACTOR_SRC_ALPHA`].
    SrcAlpha,
    /// [`FACTOR_ONE_MINUS_SRC_ALPHA`].
    OneMinusSrcAlpha,
}

impl Factor {
    /// The member for one of the four values the disc uses, or `None`.
    ///
    /// `None` rather than a nearest match: a fifth value would be a fact about
    /// the data this reading has not seen, and rounding it onto a neighbour is
    /// how a wrong equation survives review.
    #[must_use]
    pub fn from_rsx(value: u16) -> Option<Self> {
        match value {
            FACTOR_ONE => Some(Self::One),
            FACTOR_SRC_COLOUR => Some(Self::SrcColour),
            FACTOR_SRC_ALPHA => Some(Self::SrcAlpha),
            FACTOR_ONE_MINUS_SRC_ALPHA => Some(Self::OneMinusSrcAlpha),
            _ => None,
        }
    }
}

/// One entry of the material table: what the file says about how a surface is
/// drawn, short of the shader itself.
///
/// **`PartialEq` but not `Eq`**, since [`Self::parameters`] holds `f32`s.
#[derive(Debug, Clone, PartialEq)]
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
    /// The alpha test comparison function at `+0x18`.
    ///
    /// **Confidence 88, from a decompiled RSX write, not a guess at the field's
    /// existence.** `FUN_005d8f68` in `ps3-hdfury-eu`'s Ghidra database reads
    /// this word and this material's [`Self::alpha_ref`] straight into
    /// `cellGcmSetAlphaFunc`'s two arguments (`NV4097_SET_ALPHA_FUNC` at RSX
    /// register `0x308`, `NV4097_SET_ALPHA_REF` at `0x30c`), gated on
    /// [`Self::state`] bit 1 - the bit [`Transparency::Mode2`] sets. See
    /// `docs/formats/rcsmodel.md` and
    /// `docs/ghidra/functions/ps3-hdfury-eu/material-state.md`.
    ///
    /// **Every material measured, [`Transparency::Mode2`] or not, holds one of
    /// exactly two values**, `0x0201` or `0x0204` - `GL_LESS` and `GL_GREATER`
    /// in the RSX's OpenGL-inherited numbering, the same family
    /// [`Factor::from_rsx`] reads. Every [`Transparency::Mode2`] material reads
    /// `0x0204`, `GL_GREATER`; nothing on the disc where the field is actually
    /// *consumed* uses `GL_LESS`. Left as the raw `u32` rather than a named
    /// enum for the same reason [`Self::state`]'s untouched bits are: two
    /// values out of eight comparison functions RSX defines is thin ground for
    /// treating this as a settled small enum the way [`Factor`] is.
    pub alpha_func: u32,
    /// The alpha test reference at `+0x1c`, authored in `[0, 1]`.
    ///
    /// Same evidence as [`Self::alpha_func`]. The same decompiled call scales
    /// this by `255.0` before writing `NV4097_SET_ALPHA_REF`, converting the
    /// authored fraction to the register's 8-bit range - confirmed by reading
    /// the constant directly (`0x437f0000`, exactly `255.0`) rather than
    /// assumed from the RSX register's usual width.
    ///
    /// **Exactly `0.5` disc-wide** - every material name across all 16
    /// circuits and all three `PSARC` archives, opaque, blended or
    /// [`Transparency::Mode2`] alike, holds this same value. It is a shared
    /// default the state block is built with, not a per-material tuned
    /// reference. See [`Transparency::Mode2`] for what that resolves about
    /// the crowd-erasure reading it replaces.
    pub alpha_ref: f32,
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
    /// The **sampler name hash** [`Self::texture`] binds to, at `+0x40`.
    ///
    /// See [`Self::second_texture_sampler`], which carries the evidence for
    /// both. Zero on a record too short to hold it.
    pub texture_sampler: u32,
    /// The same for [`Self::second_texture`], at `+0x60`.
    ///
    /// # A texture record is `(hash, ..., path)`, and the path is `+0x18` in
    ///
    /// **This is what says which texture unit a slot reaches, and the answer
    /// is not the slot's ordinal.** The value takes exactly the sampler-name
    /// hashes `docs/formats/rcsmaterial.md` recovered preimages for -
    /// `Texture1` (`0x3bdc0403`), `diffuse` (`0x515e298e`), `lightmap`
    /// (`0x37b5db58`), `paraboloidReflectionTex` (`0x9edd3243`) - and a
    /// resolved shader variant's own declaration
    /// (`crate::rcsmaterial::Declared::samplers`) maps that hash to a unit.
    ///
    /// **The pairing is the part that was wrong for a day**, and it is worth
    /// stating because the mistake is invisible: the records are `0x20` apart
    /// from `+0x40` with the hash first and the path `0x18` further in, so
    /// the hash sitting eight bytes *after* a path belongs to the **next**
    /// record, not that one. Pairing them that way put every texture on its
    /// neighbour's sampler. Talon's Junction's `track_surface` settles it
    /// against the microcode: read correctly, `ds_floor_cs.gtf` binds
    /// `0x11cb4f74` at unit 0 and `ds_floor_n_rh.gtf` binds `0x739a786e` at
    /// unit 1 - and that variant's `TEX R4.xyz, R0 unit1` is followed by
    /// `MAD R4.w, R4.yyyy, {2,-1}` twice, a normal-map decode. The other
    /// pairing puts the *diffuse* through that decode, which is nonsense.
    ///
    /// `None` where there is no second texture.
    pub second_texture_sampler: Option<u32>,
    /// **Every** sampler entry the material's own input table carries: the
    /// name hash it binds to and the `.gtf` it supplies, in table order.
    ///
    /// **There is no fixed number of these** - see
    /// [`parameters::samplers`], which carries the census. [`Self::texture`]
    /// and [`Self::second_texture`] are entries 0 and 1 of this list and
    /// nothing more; a material with five entries has three this renderer
    /// does not bind, and `tunnel_fx_glass` has seven.
    pub samplers: Vec<(u32, Option<String>)>,
    /// The named values this **instance** supplies to its shader - see
    /// [`parameters`].
    ///
    /// Empty on a record too short to carry the table, which is an ordinary
    /// state rather than a failure: most materials on the disc declare none.
    pub parameters: Vec<Parameter>,
}

/// Where a circuit keeps its baked lighting atlases.
///
/// The prefix rather than `/lmaps/`, because a circuit keeps **three** of these
/// and the narrower form saw only one: `lmaps/` on the forward models,
/// `lmaps_rev/` on the reversed ones and `lmaps_dlc/` on Fury's. Measured over
/// the 123 `.rcsmodel` of `DATA00.PSARC`: of 981 second textures ending in the
/// suffix below, the old predicate matched 493 and missed **488, every one of
/// them a `lmaps_rev/` path on a `track_reversed.rcsmodel`** - so this was
/// latent rather than live, since nothing loads a reversed circuit today, and
/// it is widened here so that it does not become live silently.
const LIGHTMAP_DIR: &str = "/lmaps";

/// What every one of them is called.
const LIGHTMAP_SUFFIX: &str = "-lmap.gtf";

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
    /// `2`: an alpha test, not a blend - **confidence 90, from a decompiled
    /// RSX write**. 211 of these 212 materials carry the same `0302`/`0303`
    /// factor pair as [`Self::Blended`], which is why the equation alone
    /// never distinguished them; what does is that state bit 1 (this value)
    /// drives `NV4097_SET_ALPHA_TEST_ENABLE` while bit 0
    /// ([`Self::Blended`]) drives `NV4097_SET_BLEND_ENABLE` - two different
    /// fixed-function GPU features, read directly out of `FUN_005d8f68` in
    /// `ps3-hdfury-eu`'s Ghidra database. The comparison and reference are
    /// [`Material::alpha_func`]/[`Material::alpha_ref`], and both turn out
    /// to be the disc-wide constants `GL_GREATER`/`0.5` - **a plain 0.5
    /// cutout, exactly the "obvious hypothesis" an earlier pass tried and
    /// believed refuted.**
    ///
    /// **That refutation was a measurement error, not a property of the
    /// data.** It read `nr_crowd_bustle`'s texture as "alpha runs 0..255 at a
    /// mean of 120, and the threshold takes most of it" - true of the mean,
    /// and the wrong statistic: the texture is a clean bimodal split, 52.9%
    /// of texels at `0` and 47.1% at `255`, nothing between. `GL_GREATER`
    /// against `0.5` keeps exactly the 47.1% - the crowd figures - and
    /// drops the transparent background between them, which is what a
    /// cutout is for. A mean near the threshold reads as "erases most of
    /// it" only if the distribution is assumed uniform; a cutout texture is
    /// built to be the opposite of uniform. See `docs/formats/rcsmodel.md`.
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
/// an alpha-over blend paints exactly the opaque pixels while losing depth
/// ordering and an additive one blows a glass panel white.
///
/// `oag_texture::gtf` closed that: [`Material::texture`] names a `.gtf`, its
/// `to_rgba` carries the alpha, and `oag_render::mesh::rcs` draws these surfaces
/// blended with the equation below.
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
    /// [`Transparency::Mode2`]: an alpha **test**, and no blend at all.
    ///
    /// **A different fixed-function feature, not a different equation.** The
    /// transparency field's two low bits drive two separate RSX registers -
    /// bit 0 `NV4097_SET_BLEND_ENABLE`, bit 1
    /// `NV4097_SET_ALPHA_TEST_ENABLE` - and mode 2 is bit 1 alone, so a
    /// mode-2 surface has blending *off* and a per-pixel keep-or-discard on.
    /// This used to answer [`Self::Factors`] here, identically to
    /// [`Transparency::Blended`], and 211 of the disc's 212 mode-2 materials
    /// carry the same `0302`/`0303` pair as a blended one, so the picture
    /// looked close on the textures that are pure cutouts and the depth
    /// buffer was wrong on all of them.
    ///
    /// **Carries nothing**, because [`Material::alpha_func`] and
    /// [`Material::alpha_ref`] are the comparison and the reference and a
    /// caller that has this value has the material. Making it a payload
    /// variant would also cost [`Blend`]'s `Eq`, since the reference is an
    /// `f32`.
    AlphaTest,
    /// See-through, with **the pair the file authors**, both sides carried.
    ///
    /// This used to be a `Class(vex::BlendClass)` keyed on the destination
    /// factor alone, and the source was dropped - which meant a renderer
    /// pairing it with Pulse's own pipelines drew every see-through surface
    /// with a source factor of `GL_SRC_ALPHA` whatever the file said. On
    /// Talon's Junction that is **96 of 353 see-through chunks**, all of them
    /// authoring `0001`/`0001` - unweighted additive - plus 3 at
    /// `0001`/`0303`, which is premultiplied alpha drawn as straight alpha.
    /// See `docs/formats/rcsmodel.md`.
    Factors {
        /// [`Material::src_factor`], named.
        src: Factor,
        /// [`Material::dst_factor`], named.
        dst: Factor,
    },
    /// See-through, with a factor value outside the four the disc uses.
    ///
    /// Nothing measured reaches this - all 15,762 materials use the four - and
    /// it exists so a fifth value is reported rather than rounded onto a
    /// neighbour. See [`Factor::from_rsx`].
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
        if at + 0x20 > data.len() {
            return None;
        }
        // **Entries 0 and 1 of the material's own input table**, which is what
        // the `+0x58`/`+0x78` pair this used to read always was - see
        // [`parameters::samplers`]. Derived rather than read at those offsets
        // so a record whose table sits elsewhere is read correctly, and so
        // a hash can never be paired with a neighbouring entry's path.
        let entries = samplers(data, at);
        Some(Self {
            name: cstr(data, ByteOrder::Big.u32(data, at + 0x04) as usize),
            state: ByteOrder::Big.u32(data, at + 0x10),
            src_factor: ByteOrder::Big.u16(data, at + 0x14),
            dst_factor: ByteOrder::Big.u16(data, at + 0x16),
            alpha_func: ByteOrder::Big.u32(data, at + 0x18),
            alpha_ref: ByteOrder::Big.f32(data, at + 0x1c),
            texture: entries
                .first()
                .and_then(|(_, p)| p.clone())
                .unwrap_or_default(),
            second_texture: entries.get(1).and_then(|(_, p)| p.clone()),
            texture_sampler: entries.first().map_or(0, |&(h, _)| h),
            // Tied to there being a path, because this answers "which unit
            // does the second **texture** reach" and an entry with no `.gtf`
            // binds nothing for a unit to receive.
            second_texture_sampler: entries.get(1).and_then(|(h, p)| p.as_ref().map(|_| *h)),
            samplers: entries,
            parameters: parameters(data, at),
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

    /// The second texture, when it is the circuit's baked lighting atlas.
    ///
    /// **The one use of [`Self::second_texture`] this project identifies**, and
    /// it is identified rather than guessed. Four signals agree and the fourth
    /// has no exceptions:
    ///
    /// 1. The path sits under the circuit's own `lmaps/` directory.
    /// 2. The file name ends `-lmap.gtf`.
    /// 3. The material's own `.rcsmaterial` names a sampler `lightmap`
    ///    (`~crc32` `0x37b5db58`) - see `docs/formats/rcsmaterial.md`.
    /// 4. **Every chunk that names such a material declares a `lightmapUV`
    ///    vertex attribute**, and no chunk declares one without the other -
    ///    76 of 76 on Talon's Junction, 128 of 128 on `amphiseum`, swept over
    ///    every circuit by `rcsmodel_material_ground_truth.rs`.
    ///
    /// The fourth is the discriminator: it is a correspondence between the
    /// material table and the vertex declaration, decoded independently, and a
    /// wrong reading of either would break it.
    ///
    /// **This says what the texture *is*, not what to do with it.** Multiplying
    /// it into the diffuse is the conventional reading and a caller's decision;
    /// nothing here reads the microcode that would confirm the operation. The
    /// other three uses of the slot - an emissive map, a normal map and a
    /// coverage mask - need the shader variant a draw selects, which is unread,
    /// so this answers `None` for them rather than guessing.
    #[must_use]
    pub fn lightmap(&self) -> Option<&str> {
        let path = self.second_texture.as_deref()?;
        (path.contains(LIGHTMAP_DIR) && path.ends_with(LIGHTMAP_SUFFIX)).then_some(path)
    }

    /// The `.gtf` of the entry bound to the **`lightmap` sampler**, wherever
    /// it sits in the table.
    ///
    /// **The declared answer, against [`Self::lightmap`]'s inferred one.**
    /// That method reads a path pattern - `lmaps/` and `-lmap.gtf` - on the
    /// *second* entry alone, which was the only entry this project could see
    /// when it was written. The file says it outright and says it anywhere:
    /// Talon's Junction names a lightmap on **118** of its 442 materials and
    /// only 85 of them put it in entry 1, so a third of the circuit's baked
    /// lighting sat in a slot nothing looked at - and a surface with no
    /// lightmap and no vertex light of its own renders near black.
    /// `track_wall`, `tunnel_fx_noalpha` and
    /// `diffuse_emissive_with_specular_from_alpha` are the ones that showed.
    #[must_use]
    pub fn lightmap_entry(&self) -> Option<&str> {
        self.samplers
            .iter()
            .find(|(hash, path)| *hash == crate::rcsmaterial::LIGHTMAP_SAMPLER && path.is_some())
            .and_then(|(_, path)| path.as_deref())
    }

    /// The blend equation this material asks for, **both factors**.
    ///
    /// This used to key on the destination alone and drop the source, on the
    /// reasoning that `oag_vex::vex::BlendClass` - Pulse's three recovered
    /// classes - has no member for the distinction. That was the right thing to
    /// say about `BlendClass` and the wrong thing to do with the data: the
    /// source factor is measured, it is not constant, and dropping it silently
    /// redrew every surface that does not author `GL_SRC_ALPHA`. Carrying the
    /// pair costs the caller one translation and invents nothing.
    ///
    /// **[`Transparency::Mode2`] answers [`Blend::AlphaTest`], not a pair.**
    /// That mode enables the RSX's alpha *test* and leaves blending off, so
    /// its factor pair is not consumed at all - the same reason
    /// [`Transparency::Opaque`] answers [`Blend::Opaque`] while still
    /// carrying one. See [`Blend::AlphaTest`] for what answering
    /// [`Blend::Factors`] here cost.
    ///
    /// `oag_render::mesh::rcs` draws through this; see [`Blend`].
    #[must_use]
    pub fn blend(&self) -> Blend {
        match self.transparency() {
            Some(Transparency::Mode2) => return Blend::AlphaTest,
            Some(Transparency::Blended) => {}
            // The unrecognised fourth encoding reads as opaque, as
            // [`Self::is_see_through`] documents.
            Some(Transparency::Opaque) | None => return Blend::Opaque,
        }
        match (
            Factor::from_rsx(self.src_factor),
            Factor::from_rsx(self.dst_factor),
        ) {
            (Some(src), Some(dst)) => Blend::Factors { src, dst },
            _ => Blend::Unmapped {
                src: self.src_factor,
                dst: self.dst_factor,
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
