//! [`GpuVertex`]: one vertex in the layout `mesh.wesl` declares.
//!
//! Split out of `mesh.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change.
//!
//! **The field order in here is the attribute order in
//! `mesh_render::build`'s `vertex_attr_array!`**, and the two have to be
//! changed together - see [`GpuVertex::lightmap_texcoord`] for what happens
//! when they drift.

/// A vertex as the mesh shader expects it.
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub colour: [f32; 4],
    pub texcoord: [f32; 2],
    /// 1.0 to apply the viewer's light rig, 0.0 for geometry that is prelit.
    ///
    /// Track batches carry vertex colours *and* normals, and their colours are
    /// baked lighting. Lighting them again multiplies two lighting terms and the
    /// track comes out nearly black. Ship batches have no vertex colour, so they
    /// need the rig. This is the viewer's own choice, not the game's: the GE
    /// decides per draw from state we have not recovered.
    pub lit: f32,
    /// Which of [`Model::anim_tracks`] transforms this vertex's texture
    /// coordinate, plus one; `0` for a surface that does not animate.
    ///
    /// The shader looks the entry up and applies `uv * scale + offset`, the
    /// GE's own `TEXSCALE`/`TEXOFFSET` arithmetic - see
    /// `docs/ghidra/functions/psp-pulse-usa/texture-animation.md`. The curve
    /// itself is the material's authored keyframe block, sampled on the CPU
    /// once per frame per track rather than per vertex.
    ///
    /// An index rather than the scalar V rate this used to be, because the
    /// authored data is not a scalar V rate. `16_Track` alone carries 17
    /// distinct tracks, and they scroll in U as often as in V - the
    /// `col_display7` displays this project animated downwards actually run
    /// sideways - some diagonally, and the flicker panels step between held
    /// values rather than sliding at all. One `u32` carries all of that for
    /// the same four bytes per vertex the rate cost.
    pub anim: u32,
    /// Where a **lightmap** is sampled, for the surfaces that carry one.
    ///
    /// A second, separate coordinate set, because a lightmap is an atlas: its
    /// coordinates place a surface's own patch inside one texture for the whole
    /// circuit, and they have nothing to do with where the diffuse tiles. On
    /// Wipeout HD it is the `lightmapUV` attribute the chunk's vertex
    /// declaration names outright; every other title leaves it at zero and
    /// binds a white lightmap, so the multiply in `mesh.wesl` is the identity.
    ///
    /// **Field order here is attribute order, and that is load-bearing.**
    /// `wgpu::vertex_attr_array!` lays offsets out in the order the locations
    /// are written, not in field order, so a field inserted *above* an existing
    /// one silently shifts every later attribute's offset - which is exactly
    /// what putting this after `texcoord` did: the diffuse coordinates stayed
    /// right and `lit` and `anim` came out of the wrong bytes, painting the
    /// circuit in flat white. New attributes go on the end, which is why
    /// [`Self::xform`] follows this rather than sitting beside its sibling
    /// [`Self::anim`].
    pub lightmap_texcoord: [f32; 2],
    /// Which of [`Model::anim_nodes`] *moves* this vertex, plus one; `0` for
    /// geometry the file places statically.
    ///
    /// The sibling of [`Self::anim`], and deliberately the same shape: that one
    /// animates the texture coordinate, this one the position. A vertex under
    /// an `Anim Transform` is baked in that node's own space and multiplied by
    /// the node's world matrix in the shader, because the matrix changes every
    /// frame and the bake happens once.
    ///
    /// Declared last rather than next to `anim` for the reason
    /// [`Self::lightmap_texcoord`] gives: this struct's order *is* the
    /// attribute array's order.
    ///
    /// See `oag_vex::vex::anim_anchors` for the split, and
    /// `docs/rendering/scenery-animation.md` for why the class cannot simply be
    /// folded into the bake.
    pub xform: u32,
    /// HD's sun-occlusion mask, `1.0` where nothing is known to occlude it.
    ///
    /// **Not `colour.a`.** That channel is already spoken for twice over - the
    /// PSP/PS2 boost plume's baked alpha falloff, and the bloom pass's glow
    /// mask in `mesh.wesl`'s fragment output - and HD's mask means neither.
    /// `oag_rcs::rcsmodel::Mesh::vertex_light`'s fourth component for a
    /// chunk that carries a colour set, `1.0` (unmasked) for one that does
    /// not and for every non-HD title, which never reads this field at all.
    /// See `docs/ghidra/functions/ps3-hdfury-eu/renderer.md`, "The sun is real
    /// and it is masked".
    pub sun_mask: f32,
    /// Which texture the shader takes this surface's colour and coverage from,
    /// as [`slots`] packs it.
    ///
    /// **Read off the material's own fragment microcode**, not guessed from a
    /// file name: a `.rcsmaterial` variant states which unit it samples for the
    /// picture and which unit and channel it puts in the output alpha, and
    /// `oag_rcs::rcsmaterial::fragment::Program::output_texels` traces it.
    /// See [`slots::DEFAULT`] for what every title that is not Wipeout HD
    /// carries here, which is the behaviour this field replaced.
    ///
    /// Per vertex rather than per draw because the alternative is a second
    /// uniform and a second bind group, and a chunk's vertices are emitted
    /// together anyway - the same argument [`Self::anim`] makes.
    pub slots: u32,
    /// The exponent `mesh.wesl`'s specular term raises `N.H` to.
    ///
    /// **Resolved per material where it can be, off the material's own
    /// fragment microcode** -
    /// `oag_rcs::rcsmaterial::fragment::Program::specular_exponent` -
    /// rather than shared. `32.0` is only the *fallback*: for a non-HD
    /// title, which never varies this field at all; for an HD material the
    /// decoder could not resolve to a value; and for a resolved literal
    /// `0.0`, which reads as `SpecularPower` patched at draw time rather
    /// than a real shininess (`pow(x, 0) = 1` is not a plausible one) - see
    /// that method's own doc comment for the disc-wide evidence. `32.0`
    /// stays a defensible middle value in all three cases: the commonest of
    /// the disc's own round numbers, not this project's invention.
    pub specular_exponent: f32,
    /// The value this surface stamps into the bloom's glow mask, `0..=1`,
    /// where its drawable is built with `mesh_render::GlowMask::Stamped`.
    ///
    /// The original writes the mask through the stencil, never through the
    /// blend, so a surface writes a **constant** rather than its own alpha:
    /// `crate::mesh::glow` reads which one off the batch and its texture. See
    /// `docs/rendering/glow-mask.md`. `0.0` on every surface nothing reads it
    /// for. Last, for the reason [`Self::lightmap_texcoord`] gives.
    pub glow: f32,
    /// The chunk's **second** diffuse coordinate set, `Uv2` in the vertex
    /// declaration (`TC4.zw` in the programs that read it).
    ///
    /// Filled only for the surfaces whose program samples a second picture
    /// through it - HD's Sebenco ice pool, whose pond mask lives on this set
    /// (`mesh::rcs::ice`) - and `[0, 0]` everywhere else. Last, for the reason
    /// [`Self::lightmap_texcoord`] gives.
    pub texcoord2: [f32; 2],
}

/// [`GpuVertex::specular_exponent`]'s fallback - every title but Wipeout HD,
/// and every HD material the decoder could not resolve to a real value.
pub const DEFAULT_SPECULAR_EXPONENT: f32 = 32.0;

/// How [`GpuVertex::slots`] packs a material's texture roles.
///
/// Bit-for-bit the same layout `mesh.wesl`'s `fs_main` decodes; the two are
/// changed together.
pub mod slots {
    /// The second texture is the circuit's baked lighting atlas, so the prelit
    /// curve applies to it and its alpha is the sun mask.
    pub const SECOND_IS_LIGHTMAP: u32 = 1 << 0;
    /// The surface's colour comes from the **second** texture rather than the
    /// first.
    ///
    /// Rare and real: Talon's Junction's cloud plate samples `clouds_new.gtf`
    /// for one channel of alpha and never for colour, and takes its picture
    /// from the file its material calls a mask. See
    /// `docs/formats/rcsmaterial.md`.
    pub const ALBEDO_FROM_SECOND: u32 = 1 << 1;
    /// The output alpha comes from the second texture rather than the first.
    pub const ALPHA_FROM_SECOND: u32 = 1 << 2;
    /// The material's vertex program writes `1 - v` into the varying its
    /// fragment program samples with, so the coordinate has to be flipped.
    ///
    /// **Read off the vertex microcode, per material** - see
    /// `oag_rcs::rcsmaterial::vertex` and `mesh::rcs::skin::flips`. Unlike
    /// the three bits above it this one never reaches the shader: the build
    /// applies it once per vertex, because it is a property of the material
    /// and not of the pixel. It rides here because this is already the word
    /// that carries what a material's own microcode says.
    pub const FLIP_V: u32 = 1 << 5;

    /// The material's fragment program is **not** fed
    /// `constantAmbientColour`, so the circuit's constant ambient must not be
    /// added to it.
    ///
    /// **Set only when the declaration was actually read** and does not name
    /// the parameter. A material whose variant or `SHO` block this reader
    /// could not follow keeps the ambient, which is the direction that leaves
    /// a surface lit rather than black.
    ///
    /// **Read, and deliberately not acted on yet.** `mesh.wesl` still adds
    /// the ambient to everything, because gating on this bit alone was tried
    /// and is a regression: the materials without an ambient are three
    /// families, not one. Most declare `prelitBias`, `prelitScaleSpecular` and
    /// `directionalLight0*` and are lit by the lightmap and the sun, but
    /// `sign_emissive` and its kin declare only `fogColour` and are emissive -
    /// so removing the ambient turns those black and trades a brown circuit
    /// for a black one. This wants the per-material lighting branch, not one
    /// bit.
    ///
    /// The renderer adds `Lighting.Constant ambient color` to every
    /// surface, and the disc feeds it to a minority: **58 of Anulpha Pass's
    /// 309 drawn materials, 179 of its 1,101 chunks**; 24 of 283 on Talon's
    /// Junction. That is invisible where the authored ambient is neutral and
    /// glaring where it is not - Anulpha authors `0.557 0.322 0.184`, a strong
    /// orange, against a cyan sun, and the whole circuit came out brown where
    /// the original is cyan and white.
    pub const NO_AMBIENT: u32 = 1 << 6;

    /// The material's fragment program is **not** fed a directional light
    /// either - neither `directionalLight0Colour` nor
    /// `directionalLight0DirectionWorldSpace`.
    ///
    /// Set under the same rule as [`NO_AMBIENT`]: only where the declaration
    /// was read. Together the two make a three-way key, and it is the addition
    /// of *this* one that makes the split hold where an earlier attempt did
    /// not. That attempt keyed on the lightmap sampler and the ambient alone
    /// and put `track_wall` and `glasstest` - ordinary lit surfaces - in the
    /// "neither" bucket; asking about the directional light as well moves both
    /// into "sun only", which is what they are.
    ///
    /// **A program fed neither is emissive**, and the names that fall out say
    /// so without having been asked: on Anulpha Pass the 33 materials in that
    /// bucket are `sign_emissive`, `sign_emissive_glow`, `cf_glow_tube`,
    /// `cf_plasma_glow2`/`3`, `cf_startbeam_glow`, `mr_uvanim_em_alpha`,
    /// `cf_uvanim_emssive_glowtint`, `nr_holobowlparallax`, `nr_twinblend`;
    /// on Talon's Junction the 35 are `cf_billboard1`, `scanlinebillboard`,
    /// `sign_emissive_glow`, `cf_startbeam_glow`, `dc_lightcone`,
    /// `loopmaterial`, `nr_scalinguvs`. Two independent circuits, every name a
    /// sign or a glow.
    pub const NO_SUN: u32 = 1 << 7;

    /// Both of the above: the program is fed no scene light at all, so its
    /// albedo is the picture and the rig must not touch it.
    pub const EMISSIVE: u32 = NO_AMBIENT | NO_SUN;

    /// The second texture is a glow this material **adds** to its albedo,
    /// rather than one of the two it selects between.
    ///
    /// Wipeout HD's emissive family, and the one shape that separates it from
    /// every other two-texture material: `MAD H0.xyz, H0.wwww, H1, H0` -
    /// albedo plus diffuse-alpha times the tinted, scrolling sample from
    /// unit 1. Both [`ALBEDO_FROM_SECOND`] and [`ALPHA_FROM_SECOND`] above
    /// *replace*, so without this bit those surfaces draw their diffuse and
    /// their glow is simply absent.
    ///
    /// Read per material by `oag_rcs::rcsmaterial::fragment::Program::accumulates`,
    /// whose own doc carries the disc-wide evidence. **Never set together with
    /// [`SECOND_IS_LIGHTMAP`]**: adding the circuit's baked atlas paints a
    /// shadow map as a glow, the same refusal `skin::roles` already makes for
    /// albedo and coverage.
    ///
    /// The tint and the scroll this layer needs are floats rather than bits,
    /// and travel in [`super::Emissive`] through the index bits below.
    pub const ADD_SECOND: u32 = 1 << 8;

    /// This chunk is track surface, so an HD Zone race publishes the `Track`
    /// parameter set and `zoneModeTrack<n>.gtf` for it rather than the
    /// `Scene` set and `zoneMode<n>.gtf`.
    ///
    /// **Per chunk, not per material** - the one bit in this word that is:
    /// it is bit 0 of the chunk's own render-block flags
    /// (`oag_rcs::rcsmodel::Mesh::is_track`), authored in the `.rcsmodel`
    /// on 4,365 of the disc's 41,861 chunks, and `mesh::rcs::surface` ORs
    /// it in per chunk over the material's roles. `shaders/zone.wesl`'s `zone_set`
    /// decodes it; see `mesh_render::Zone` for the two publications.
    pub const ZONE_TRACK: u32 = 1 << 9;

    /// This material's resolved fragment program is the glass family's
    /// facing-ramp combine - `mesh::rcs::glass_sheen`'s own fact-based
    /// classifier, matched once at load. Where this is set, `albedo` is the
    /// facing ramp (sampled at `dot(V, N)`, not the diffuse UV) and
    /// `lightmap`/`aux` is the grid, exactly as `mesh::rcs::skin::picks`
    /// bound them - see `mesh.wesl`'s own read of this bit for the combine
    /// `docs/formats/rcsmaterial.md` traces.
    pub const FACING_RAMP_SHEEN: u32 = 1 << 10;

    /// This material's resolved fragment program is the LeachBall's glow -
    /// `hd_leachbeam_ball_glow.rcsmaterial`'s shape, matched by
    /// `mesh::rcs::rim_glow`'s fingerprint: a noise-displaced, clock-scrolled
    /// tap, faded toward the silhouette by `(0.9 (1 - rim^5))^5` and
    /// expanded by `c / (1 - c)`. No light enters it. See
    /// `docs/rendering/hd-unlit-programs.md`.
    pub const RIM_GLOW: u32 = 1 << 11;

    /// This material's resolved fragment program is the Plasma bolt's head -
    /// `plasmasphere_subtractive_glow.rcsmaterial`'s shape: the same two
    /// taps, times `1000 rim^5`, at the alpha the material authors. See
    /// [`RIM_GLOW`] and the same page.
    pub const RIM_EDGE: u32 = 1 << 12;

    /// This batch carries the glow bits (`pass_mask & 0xc0`), the one thing
    /// the PS2's mask rule reads: such a batch stamps its fragments' own
    /// alpha and every other batch stamps nothing. Set by `mesh::build` on
    /// every `.vex` batch, and read only by a model with
    /// [`Model::glow_by_texel`](super::Model::glow_by_texel); the PSP's rule
    /// keeps carrying its constant in [`GpuVertex::glow`](super::GpuVertex::glow).
    pub const GLOW_BATCH: u32 = 1 << 13;

    /// This is one of HD's pad materials, whose fragment program reads the
    /// `_ne` mask (bound third, [`Model::pad_masks`](super::Model::pad_masks))
    /// as a tangent-space normal for its own `N.L`/`N.H` and adds
    /// `_ne.a * W_Cycle` after the light - see `mesh::rcs::pad_ne`. The
    /// colour `W_Cycle` is this material's [`Model::emissive`](super::Model::emissive)
    /// tint, which a pad material has no other use for (it never carries
    /// [`ADD_SECOND`]).
    pub const PAD_NE: u32 = 1 << 14;

    /// This material's resolved fragment program is the Plasma explosion
    /// ring's (`hd_plasmaring_glow.rcsmaterial`, block `@0x1900`): its
    /// texture is addressed by the **model's own clock**, the declared
    /// `UV_offset` parameter - `AnimNode_GetTime`'s `node + 0xc0`, which
    /// `WeaponExplosions_Draw` sets to the blast's age - not the scene's
    /// `time`. Matched by `mesh::rcs::rim_glow`'s fingerprint. See
    /// `docs/ghidra/functions/ps3-hdfury-eu/plasma.md`, 2026-10-05.
    pub const CLOCK_SCROLL_RING: u32 = 1 << 15;

    /// The same for the explosion halo's program
    /// (`hd_plasmahalo_glow.rcsmaterial`, block `@0x1960`): a different
    /// set of rates over the same `UV_offset`. See [`CLOCK_SCROLL_RING`].
    pub const CLOCK_SCROLL_HALO: u32 = 1 << 16;

    /// This material's resolved fragment program takes HD's magstrip wave:
    /// the emissive picture (bound third, the same slot as [`PAD_NE`]'s mask,
    /// which no magstrip material shares) is dodged by a scrolling wave
    /// texture (bound fourth, [`Model::wave_maps`](super::Model::wave_maps)),
    /// `e / (1 - lerp(e, wave(uv * k + time), e.a) * Colour)`. The tint is
    /// `Colour`, the scale `k` and the rate 1 ride in this material's
    /// [`Model::emissive`](super::Model::emissive) entry. See
    /// `mesh::rcs::mag_wave`.
    pub const MAG_WAVE: u32 = 1 << 17;

    /// With [`MAG_WAVE`]: the strip-floor combine of `mag_effect_loop_opaque`
    /// and `mageffectloop`, where `albedo` is the grid, `lightmap` the facing
    /// ramp, and `d` enters as `vertexLight * grid * (ramp + c) +
    /// (grid + ramp) * d`. `c` rides in the glow-table entry's `offset`.
    pub const MAG_LOOP: u32 = 1 << 18;

    /// This material's resolved fragment program is HD's light cone -
    /// `dc_lightcone.rcsmaterial`'s shape, matched by
    /// `mesh::rcs::light_cone`'s fact-based classifier: `albedo` is the cone's
    /// facing ramp (sampled at `dot(V, N)`) and `lightmap` the noise
    /// (`Texture1`), the output being `(noise * K, noise * s * ramp)`. `K` rides
    /// in the glow-table entry's tint and `s` in its scale.
    pub const LIGHT_CONE: u32 = 1 << 19;

    /// This material reads the behind-the-glass target - `mesh::rcs::refraction`'s
    /// fact-based classifier, Vineta K's tunnel glass. Drawn as one opaque
    /// surface whose colour includes that target (the `refraction` module's
    /// own doc has the equation). The weight `W` rides in the glow-table
    /// entry's tint, the colour multiplier in its offset, the weight
    /// multiplier in its scale and whether the diffuse alpha weights the grab
    /// in its rate.
    ///
    /// Bit 21 above it is free: it marked one of the two blended passes the
    /// glass was drawn as before the target existed.
    pub const REFRACTION: u32 = 1 << 20;

    /// HD's Sebenco ice pool (`mesh::rcs::ice`): the albedo is a lerp of three
    /// authored colours by a facing term and a pond mask read through
    /// [`GpuVertex::texcoord2`](super::GpuVertex::texcoord2) from the third
    /// binding. The colours ride in three consecutive glow-table entries
    /// starting at this material's index.
    pub const ICE: u32 = 1 << 22;

    /// HD's Bomb fireball (`hd_bombfire_glow.rcsmaterial`, block `@0x1d90`):
    /// the LeachBall's rim-glow skeleton plus a dissolve and a brightness
    /// the blast drives - `AlphaAnim` (the model's own clock) and `ColourAnim`
    /// (a per-draw scalar, the mesh uniform's `model_colour`). Carried as
    /// [`RIM_GLOW`] and [`RIM_EDGE`] **together**, which no single material
    /// earns, rather than a bit of its own: bits 23 and up are the material
    /// index and a circuit needs hundreds of those. Every test of either bit
    /// alone excludes this pair. See `docs/rendering/hd-unlit-programs.md`,
    /// "The Bomb's fireball and shockwaves".
    pub const BOMB_FIRE: u32 = RIM_GLOW | RIM_EDGE;

    /// HD's Bomb shockwave ring (`hd_bombfire_shockwaves_glow.rcsmaterial`,
    /// block `@0x1950`), addressed by the model's own clock as
    /// `Shockwave_scalar`. Carried as [`CLOCK_SCROLL_RING`] and
    /// [`CLOCK_SCROLL_HALO`] together, for [`BOMB_FIRE`]'s reason.
    pub const BOMB_SHOCK: u32 = CLOCK_SCROLL_RING | CLOCK_SCROLL_HALO;

    /// Where a material's index into [`Model::emissive`](super::Model::emissive)
    /// sits in this word, plus one; `0` is "this material has none".
    ///
    /// **The high half, because the roles are bit flags and the index is a
    /// number.** Putting it here rather than in a tenth vertex attribute costs
    /// nothing: the word is already `u32`, already per material, and already
    /// `@interpolate(flat)`, and every shader read of the low half is a masked
    /// bit test that an index above bit 15 cannot disturb. A circuit's
    /// materials number in the hundreds against the 65,535 this allows.
    ///
    /// Twenty-three since [`ICE`] took bit 22, twenty-two since [`REFRACTION`] and a since-retired pass bit took bits 20 and 21, twenty since [`LIGHT_CONE`] took bit 19, nineteen before it, when
    /// [`MAG_WAVE`] and [`MAG_LOOP`] took bits 17 and 18; seventeen rather than sixteen since the two clock-scroll bits
    /// ([`CLOCK_SCROLL_RING`], [`CLOCK_SCROLL_HALO`]) took bits 15 and 16: the
    /// index keeps 32,767 values, against the hundreds a circuit uses.
    pub const MATERIAL_SHIFT: u32 = 23;

    /// The mask covering every role bit - the low half of the word, with
    /// [`MATERIAL_SHIFT`]'s index excluded.
    pub const ROLE_MASK: u32 = (1 << MATERIAL_SHIFT) - 1;

    /// This material's [`Model::emissive`](super::Model::emissive) index plus
    /// one, or `0` where it has no entry.
    #[must_use]
    pub const fn material_index(packed: u32) -> u32 {
        packed >> MATERIAL_SHIFT
    }

    /// Which channel of that texture the alpha is, in bits 3 and 4.
    #[must_use]
    pub const fn alpha_channel(channel: u32) -> u32 {
        (channel & 3) << 3
    }
    /// The first texture's colour and the first texture's **alpha** channel.
    ///
    /// What every PSP and PS2 source carries, and what an HD material carries
    /// whose microcode this reading could not follow - so a zero-information
    /// answer leaves the picture exactly as it was rather than guessing.
    pub const DEFAULT: u32 = alpha_channel(3);
}
