//! Loads a `.vex` model out of an archive and flattens it for the GPU.

use anyhow::{Context, Result, bail};
use oag_assets::Container;
use oag_vex::vex;

/// How many distinct texture-transform tracks one model may carry, matching
/// `mesh.wgsl`'s `TexAnims` array.
///
/// Slot 0 is the identity, so a model gets `ANIM_TRACK_LIMIT - 1` real tracks.
///
/// **The headroom is measured, not guessed**: swept over all 52 `.vex` files
/// the PSP disc ships that this project builds - every circuit's four layout
/// variants plus both models of all eight teams - the worst is `04_Track` at
/// **18** distinct tracks, against `16_Track`'s 17. So this is a factor of
/// three. `crates/render/tests/authored_uv_ground_truth.rs` re-measures it, and
/// fails rather than silently overflowing if a file ever exceeds it.
///
/// Exceeding it is graceful either way: the surfaces past the ceiling draw
/// unanimated rather than the build failing, in [`build_with_textures`] and in
/// [`merge`] alike.
pub const ANIM_TRACK_LIMIT: usize = 64;

/// A model flattened into one vertex and one index buffer.
///
/// `Clone` so that one mesh can back several [`crate::mesh_render::Drawable`]s -
/// eight craft on a starting grid are eight uniform buffers over the same
/// geometry, and a `Drawable` owns its buffers. Real instancing would upload the
/// vertices once instead; a ship is a few thousand triangles against a track's
/// hundred and forty thousand, so the copy is not what to optimise first.
#[derive(Debug, Clone)]
pub struct Model {
    /// Human-readable source, for the window title.
    pub label: String,
    pub vertices: Vec<GpuVertex>,
    pub indices: Vec<u32>,
    /// One per material run, in draw order. Opaque; drawn with depth write on
    /// and no blending. Neither `is_transparent()` nor `is_alpha_tested()`.
    pub draws: Vec<DrawCall>,
    /// Cutout batches, indexing the same `vertices`/`indices` as
    /// [`Self::draws`]. Drawn with a `discard` below a threshold rather than a
    /// hardcoded alpha of 1.0, but otherwise opaque - depth write stays on, so
    /// overlapping cutout surfaces still occlude each other and the geometry
    /// behind them correctly.
    ///
    /// **Two titles' batches, reaching it two different ways.** A Pulse or
    /// Pure batch is tagged `is_alpha_tested()` in its own `pass_mask` (see
    /// [`build_with_textures`]) and carries no reference of its own; a Wipeout
    /// HD chunk is here because its material is
    /// `oag_rcs::rcsmodel::Transparency::Mode2`, and *does* carry one -
    /// see [`Self::alpha_test_ref`] and `mesh::rcs::cutout`.
    pub alpha_tested_draws: Vec<DrawCall>,
    /// Batches tagged `is_transparent()` (see [`build_with_textures`]),
    /// indexing the same `vertices`/`indices` as [`Self::draws`]. Meant to be
    /// drawn after every other list, blended and with depth write off.
    pub transparent_draws: Vec<DrawCall>,
    /// Textures embedded in the model, one slot per `Texture` node.
    ///
    /// `None` where the node exists but this build cannot decode it. The slots
    /// are positional because materials name a texture by its ordinal, so
    /// compacting them would re-skin the model.
    pub textures: TextureSlots,
    /// Each material's **second** texture, positionally beside
    /// [`Self::textures`], under whatever role it plays there.
    ///
    /// Named for the common case, not the only one: most of the time this is
    /// the circuit's baked lighting atlas (`oag_rcs::rcsmodel::Material::lightmap`),
    /// but it is loaded whenever the material names one at all, and
    /// [`Self::material_slots`] is what says which role a given entry
    /// actually plays - see `mesh::rcs::skin::skin` and [`slots`].
    ///
    /// **Parallel and never compacted**, for the same reason `textures` is: a
    /// draw names its material by ordinal, so a missing entry has to stay a
    /// hole. Empty on every title but Wipeout HD, and `None` on the majority of
    /// its materials.
    pub lightmaps: TextureSlots,
    /// Each HD pad material's `_ne` mask, bound third (`mesh::rcs::pad_ne`), and each
    /// magstrip material's emissive picture in the same slot (`mesh::rcs::mag_wave`).
    pub pad_masks: TextureSlots,
    /// Each magstrip material's scrolling wave texture, bound fourth; empty elsewhere.
    pub wave_maps: TextureSlots,
    /// What each material slot's own microcode says its two texture units are
    /// for, packed as [`slots`], in the same order as [`Self::textures`].
    ///
    /// Empty for every title but Wipeout HD, whose materials are the only ones
    /// carrying a shader table to read. A slot with no entry draws as
    /// [`slots::DEFAULT`], which is what this renderer did before the reading
    /// existed. See `mesh::rcs::skin::roles`.
    pub material_slots: Vec<u32>,
    /// Each material slot's specular exponent, positionally beside
    /// [`Self::textures`] exactly as [`Self::material_slots`] is - the
    /// per-material `pow(N.H, e)` value read off the same resolved fragment
    /// program, in place of `mesh.wgsl`'s shared fallback.
    ///
    /// Empty for every title but Wipeout HD, the only one with a shader
    /// table to read this from. See `mesh::rcs::skin::roles` and
    /// `oag_rcs::rcsmaterial::fragment::Program::specular_exponent`.
    pub material_specular_exponent: Vec<f32>,
    /// Which shader variant each material slot resolves to, positionally
    /// beside [`Self::textures`], or `None` where the material could not be
    /// read or ships no row for the key.
    ///
    /// **Read, not yet acted on.** `mesh.wgsl` still shades every surface one
    /// way; this is what a per-material path would key on, and what
    /// `mesh/rcs.rs`'s report counts so a reading that reaches most of a
    /// circuit can be told from one that does not. Empty on every title but
    /// Wipeout HD - see `oag_rcs::rcsmaterial`.
    pub material_variants: Vec<Option<oag_rcs::rcsmaterial::Variant>>,
    /// Which of [`Self::anim_tracks`] each material slot drives, plus one -
    /// positionally beside [`Self::textures`] exactly as [`Self::material_slots`]
    /// is, and `0` (no track) for a material with nothing authored to animate.
    ///
    /// Empty for every title but Wipeout HD: on the PSP/PS2 path
    /// [`GpuVertex::anim`] is assigned per **batch** as [`build_class`] walks
    /// them (a Pulse/Pure material's own `TEXOFFSET` block), not per material
    /// slot, so this table has nothing to hold there. On HD it is what
    /// [`mesh::rcs`] resolves an [`oag_rcs::rcsmodel::material::Curve`]
    /// through - see `mesh::rcs::curve_track`.
    pub material_anim: Vec<u32>,
    /// The `0x2000` **extra pass**'s draws: every batch whose `pass_mask` has
    /// that bit and whose material names a second texture
    /// (`oag_vex::vex::Material::second_texture`), as a copy of the batch's own
    /// draw - the same index range, so the same vertices - with
    /// [`DrawCall::texture`] the **second** texture.
    ///
    /// The original draws each such batch twice: first `FUN_0890db54` under
    /// `TEXMAPMODE` 2 (environment mapping) with this texture, then the
    /// batch's own pass. Nothing in [`Self::draws`], [`Self::alpha_tested_draws`]
    /// or [`Self::transparent_draws`] changes; this list is for the caller that
    /// draws the second pass - see `oag_render::shine`. Empty for a model that
    /// authors none, which is every model but a Pulse hull. Only the `.vex`
    /// path fills it.
    pub shine_draws: Vec<DrawCall>,
    /// Whether [`GpuVertex::colour`] holds a **baked light** rather than a tint.
    ///
    /// True only for a Wipeout HD `.rcsmodel`, whose fragment programs *add*
    /// the interpolated colour to the lightmap term before multiplying the
    /// albedo - so multiplying by it, as every other title's assets intend,
    /// darkens instead of tinting. `mesh.wgsl` reads it as a pipeline override.
    ///
    /// **A property of the model, not of the render target.** Keying it off the
    /// target's colour space was tried and was wrong both ways: HD's capture
    /// and viewer paths draw these models into a *gamma* target, where the
    /// guard never fired and they rendered dark, and on the linear target it
    /// over-fired onto every other model including the sky cube. `model_probe`
    /// is what caught the first half.
    pub vertex_colour_is_light: bool,
    /// Whether this model's draws stamp the bloom's glow mask the way the
    /// original's stencil does - see `mesh_render::GlowMask::Stamped`, which
    /// a drawable of this model is built with whatever its caller asked for.
    ///
    /// Set by a race loader for Pulse on the PSP, the one source measured;
    /// `false` everywhere else, and for a model cloned into another effect.
    pub stamps_glow: bool,
    /// The PS2's rule, see `mesh_render::GlowMask::StampedByTexel`. With
    /// [`Self::stamps_glow`]: the mask value is the fragment's own alpha,
    /// the texel's times the vertex colour's, on a batch with the glow bits,
    /// and nothing anywhere else - the **PS2**'s rule, read off GS dumps, in
    /// place of the PSP's constant-per-batch stencil reference. See
    /// `mesh_render::GlowMask::StampedByTexel`. **Without** `stamps_glow` it
    /// means a PS2 model that writes no mask at all, whatever its call site asks.
    pub glow_by_texel: bool,
    /// Wipeout HD's engine-flare shading, for the one model that is one, and
    /// `None` for every other model of every title. See [`Flame`].
    pub flame: Option<Flame>,
    /// Wipeout HD's absorb shell program (`hd_absorbinternal`), for the one
    /// model that is one. See `oag_fx::absorb_shell`.
    pub absorb_shell: bool,
    /// The alpha-test reference [`Self::alpha_tested_draws`] is compared
    /// against, when the model's own materials author one.
    ///
    /// **The disc's number, not this renderer's.** A Wipeout HD material in
    /// `oag_rcs::rcsmodel::Transparency::Mode2` carries an
    /// `alpha_func`/`alpha_ref` pair the RSX programs into
    /// `NV4097_SET_ALPHA_FUNC`/`SET_ALPHA_REF`, and every one of the disc's is
    /// `GL_GREATER`/`0.5` - see `mesh::rcs::cutout`, which reads it and
    /// reports anything else. `mesh.wgsl` takes it as the `alpha_test_ref`
    /// pipeline override, the same way [`Self::flame`]'s numbers reach it.
    ///
    /// `None` for every PSP and PS2 model, whose alpha-tested batches carry no
    /// reference of their own that this project has recovered - the shader's
    /// own default stands there, and `mesh.wgsl`'s `ALPHA_TEST_THRESHOLD`
    /// carries the evidence for it.
    pub alpha_test_ref: Option<f32>,
    /// Centre of the bounding box, so the camera can frame the model.
    pub centre: [f32; 3],
    /// Radius of the bounding sphere.
    pub radius: f32,
    /// How many meshes contributed.
    pub mesh_count: usize,
    /// The two authored airbrake flaps, left then right, when this model has
    /// them. `None` on a track, on a ship whose file carries no `Airbrake`
    /// node, and on either side that is absent.
    pub airbrakes: [Option<Flap>; 2],
    /// The distinct texture-transform tracks this model's materials author,
    /// indexed by [`GpuVertex::anim`] minus one.
    ///
    /// Deduplicated: `16_Track`'s 90 animated meshes author 17 distinct
    /// tracks, and identical tracks share an entry so the per-frame table the
    /// shader reads stays small. Order is first-seen, which keeps a rebuild of
    /// the same file byte-identical.
    pub anim_tracks: Vec<AnimTrack>,
    /// One vertex range per node of the class this model was built for, in
    /// the same node-file order [`oag_vex::pads::volumes`] walks the same
    /// file with the same class id - [`Flap::vertices`] generalised from two
    /// fixed slots to however many nodes a class authors, for a caller that
    /// wants trigger *i*'s own geometry and has only its position to reach it
    /// by. Populated by [`build_class`] and `mesh::rcs::build_pad_class`;
    /// empty otherwise (a ribbon, a collision overlay, a merge).
    pub node_vertex_ranges: Vec<std::ops::Range<u32>>,
    /// The `Anim Transform` nodes this model's geometry hangs under, indexed by
    /// [`GpuVertex::xform`] minus one.
    ///
    /// Not deduplicated, unlike [`Self::anim_tracks`]: two nodes authoring the
    /// same channels still sit at different places in the tree, so they are
    /// different matrices and cannot share a slot.
    pub anim_nodes: Vec<AnimNode>,
    /// The scrolling glow each material adds to its albedo, indexed by
    /// [`slots::material_index`] minus one.
    ///
    /// Empty on every title but Wipeout HD, whose materials are the only ones
    /// that **add** their second texture rather than selecting it. Deduplicated
    /// by value, like [`Self::anim_tracks`] and for the same reason: a
    /// circuit's animated materials collapse to a handful of distinct
    /// `(tint, rate)` pairs and the shader's table stays small.
    pub emissive: Vec<Emissive>,
    /// The authored `LodGroup`s and which child each node sits under, for the
    /// per-frame switch - see [`LodGroups`]. Empty after
    /// [`Model::keep_nearest`].
    pub lod_groups: LodGroups,
}

impl Model {
    /// A model with no geometry, named so a report can still say which file it
    /// came from.
    ///
    /// Not a `Default` impl: "no geometry" is a *result* here - the file authors
    /// none, or its class id is unrecovered for this version - and every caller
    /// already reads `indices.is_empty()` as exactly that. A `Default` would also
    /// be constructible by accident, which this must not be.
    #[must_use]
    pub fn none(label: &str) -> Self {
        Self {
            label: label.to_string(),
            vertices: Vec::new(),
            indices: Vec::new(),
            draws: Vec::new(),
            alpha_tested_draws: Vec::new(),
            transparent_draws: Vec::new(),
            textures: Vec::new(),
            lightmaps: Vec::new(),
            pad_masks: Vec::new(),
            wave_maps: Vec::new(),
            material_slots: Vec::new(),
            material_specular_exponent: Vec::new(),
            material_variants: Vec::new(),
            material_anim: Vec::new(),
            shine_draws: Vec::new(),

            vertex_colour_is_light: false,
            stamps_glow: false,
            glow_by_texel: false,

            flame: None,
            absorb_shell: false,
            alpha_test_ref: None,
            centre: [0.0; 3],
            radius: 0.0,
            mesh_count: 0,
            airbrakes: [None, None],
            anim_tracks: Vec::new(),
            node_vertex_ranges: Vec::new(),
            anim_nodes: Vec::new(),
            emissive: Vec::new(),
            lod_groups: LodGroups::default(),
        }
    }
}

mod lod;
pub use lod::{LodEye, LodGroups, LodSwitch, ModelDetail, REFERENCE_FOV_DEGREES};

mod anim_track;
pub use anim_track::AnimTrack;

mod vertex;
pub use vertex::{DEFAULT_SPECULAR_EXPONENT, GpuVertex, slots};

mod anim_node;
pub use anim_node::{AnimNode, Motion, NODE_ANIM_LIMIT};

mod emissive;
pub use emissive::Emissive;

mod external;
pub use external::geometry_is_external;

mod flap;
pub use flap::Flap;

mod batch_placement;
pub use batch_placement::{BatchPlacement, batch_placements};

pub mod rcs;
pub mod ship_skin;
pub mod sky_cube;

/// Reads one named blob out of an archive inside a disc image.
///
/// `spec` is `<image>:<path-on-disc>`, matching `oag-wad`. Either container
/// opens - a WAD wants the game's own `Data\...` spelling, a PS3 `.psarc` its
/// own stored path - and which it is comes off the archive's magic rather than
/// its extension. See [`oag_assets::Container`], whose errors name the archive.
pub fn read_blob(spec: &str, name: &str) -> Result<Vec<u8>> {
    Ok(Container::open(spec)?.read_entry(name)?)
}

/// Reads one `.vex` entry out of an archive inside a disc image.
pub fn load(spec: &str, name: &str) -> Result<Model> {
    let data = read_blob(spec, name)?;
    build(name, &data)
}

/// Flattens every mesh in a `.vex` into one buffer pair, keeping only the
/// finest tier of every authored `LodGroup`.
///
/// For a model with no race camera to switch it - a front-end preview, a HUD
/// model, a probe: [`Model::keep_nearest`] drops the coarse tiers, which is
/// what the original shows up close. A race model that switches per frame
/// comes from [`build_with_textures`] instead.
pub fn build(label: &str, data: &[u8]) -> Result<Model> {
    let mut model = build_with_textures(label, data, None)?;
    model.keep_nearest();
    Ok(model)
}

/// As [`build`], with an external texture set replacing the embedded one, and
/// every `LodGroup` tier kept with the [`LodGroups`] table the per-frame
/// switch reads - a race model, switched per drawn instance by [`LodSwitch`].
///
/// PS2 models need the texture set: their embedded texture block is empty by
/// design, so there is nothing for a material to resolve to unless the set is
/// supplied from outside. Passing `None` uses whatever the file embeds, which
/// is what every PSP model wants.
pub fn build_with_textures(
    label: &str,
    data: &[u8],
    external: Option<&Ps2TextureSet>,
) -> Result<Model> {
    build_class(label, data, external, |c| c.mesh)
}

/// The track's sky, as a model in its own right.
///
/// A [`vex::CLASS_SKYCUBE`] node's payload *is* a mesh payload - same header,
/// same bounding-box pair, same material array - so this is
/// [`build_with_textures`] pointed at a different class, not a second decoder.
/// The sky comes out separate from [`build`]'s track model on purpose: it is
/// drawn camera-centred and without depth, which is a different draw rather than
/// a different mesh.
///
/// The sky's textures are the track file's own, indexed by the same ordinals, so
/// this must be built from the same `data` the track model was, and - on PS2,
/// where the file embeds no texture block at all - from the same `external`
/// set the caller resolved for the track model too. Passing `None` uses
/// whatever the file embeds, which is what every PSP model wants.
///
/// Returns a model with no meshes when the file authors no sky. That is not an
/// error: 4 of the 40 PSP track files have no `fogCube` and a `.vex` that is not
/// a track has neither.
pub fn build_sky(label: &str, data: &[u8], external: Option<&Ps2TextureSet>) -> Result<Model> {
    build_optional_class(label, data, external, |c| c.skycube)
}

/// The track's speedup pads, as a model in their own right.
///
/// A [`vex::CLASS_SPEEDUP_PAD`] node's payload is a mesh payload for the same
/// reason a `Skycube`'s is (the pad's bind handler calls the `Mesh` bind first),
/// so this is [`build_with_textures`] pointed at a third class, not a third
/// decoder. See [`oag_vex::pads`] for the trigger volume that shares those
/// bytes.
///
/// Separate from [`build`]'s track model on purpose, and not because it is a
/// different kind of draw: it is the same pipeline and the same textures. The
/// track model's draw calls are what `oag_render::pvs` indexes its per-section
/// visibility against, and pads belong to no `section`, so folding them in would
/// put geometry into that mapping that the mapping cannot describe.
///
/// The textures are the track file's own, indexed by the same ordinals, so this
/// must be built from the same `data` the track model was, and from the same
/// `external` set on PS2 - see [`build_sky`].
///
/// Returns a model with no meshes when the file authors no pads, which is
/// ordinary: every Pure track and every `.vex` that is not a track.
pub fn build_pads(label: &str, data: &[u8], external: Option<&Ps2TextureSet>) -> Result<Model> {
    build_optional_class(label, data, external, |c| c.speedup_pad)
}

/// The track's `Weapon Pad` geometry, the same way as [`build_pads`].
///
/// A separate model rather than a second class in the same buffer, because the
/// two are separate *gameplay* objects: a speed pad pushes and a weapon pad
/// hands something out, and a renderer that merged them could not later show one
/// without the other. They share everything else - the same pipeline, the same
/// textures, the same absence from the visibility partition.
///
/// `Weapon Pad` `0x3be` is a `Mesh` subclass exactly as `Speedup Pad` `0x3bd`
/// is, so its geometry ships inside the track file and this is the mesh builder
/// pointed at a different class id. See `docs/formats/pads.md`.
pub fn build_weapon_pads(
    label: &str,
    data: &[u8],
    external: Option<&Ps2TextureSet>,
) -> Result<Model> {
    build_optional_class(label, data, external, |c| c.weapon_pad)
}

/// [`build_class`] for a node type whose absence is ordinary.
///
/// The sky and the two pad classes are all optional geometry: plenty of shipped
/// `.vex` files author none, and every caller already treats an empty model as
/// "there is none here". **An id this project has not recovered for the file's
/// version is folded into that same empty answer**, deliberately, because the
/// alternative is failing a whole race over a sky - and the caller is the one
/// holding the class table, so it is the caller that can tell the two apart and
/// word its report accordingly. `oag_raceplay::load` does exactly that.
///
/// [`build`] itself does **not** go through this: a track or ship whose `Mesh`
/// id is unrecovered has no geometry at all, and returning an empty model there
/// would be a black screen reported as a success.
fn build_optional_class(
    label: &str,
    data: &[u8],
    external: Option<&Ps2TextureSet>,
    pick: fn(vex::classes::Classes) -> Option<u32>,
) -> Result<Model> {
    let Ok(classes) = vex::classes_of(data) else {
        // Let `build_class` produce the real complaint about the file rather
        // than swallowing it as "authors none".
        return build_class(label, data, external, pick).map(|mut model| {
            model.keep_nearest();
            model
        });
    };
    let Some(id) = pick(classes) else {
        return Ok(Model::none(label));
    };

    // **A file that authors no node of the class has none of this geometry, and
    // that is the empty answer rather than an error.** Every doc comment above
    // says so and every caller already reads an empty model that way; until a
    // Zone circuit was loaded, nothing exercised it, because `build_class`
    // bails on empty indices and each of the three classes had been met only on
    // files that author it. `Data\Environments\16_Track\zone_track.vex` is the
    // counter-example: it authors a `Skycube` and 17 `Speedup Pad`s and **no
    // `Weapon Pad` at all**, which is the disc agreeing with
    // `oag_race::Mode::weapons_enabled` being false for Zone - and it made a
    // whole Zone race fail to load with `decoded to no triangles`.
    //
    // Checked by walking the tree rather than by catching that error, so the
    // two stay distinguishable: a class nothing authors returns empty here, and
    // a class that *is* authored but decodes to nothing still fails loudly in
    // `build_class`, which is a decoder problem and should not be quiet.
    if vex::nodes(data).is_ok_and(|nodes| !nodes.iter().any(|node| node.class_id == id)) {
        return Ok(Model::none(label));
    }

    // No race camera switches these models per frame, so they keep the
    // finest tier of any `LodGroup` they sit under, as the track model does
    // up close. Chosen, not measured: none has been seen under a group.
    let mut model = build_class(label, data, external, pick)?;
    model.keep_nearest();
    Ok(model)
}

/// Flattens every node of one class into one buffer pair.
///
/// The class is a parameter because `Skycube` and `Mesh` share a payload layout
/// exactly; see [`build_sky`].
fn build_class(
    label: &str,
    data: &[u8],
    external: Option<&Ps2TextureSet>,
    pick: fn(vex::classes::Classes) -> Option<u32>,
) -> Result<Model> {
    if !vex::has_magic(data) {
        bail!("{label} is not a .vex file (no VEXX magic)");
    }

    // **The ids come from the file's own version word, not from a constant.**
    // Every `vex::CLASS_*` spells version 6's numbering, and a class id is a
    // table index rather than a stable enum - so comparing one against a
    // version-4 file matches nothing at best and the wrong node type at worst.
    // Not hypothetical: this function's `CLASS_MESH` is exactly why a Pure ship
    // reported `decoded to no triangles` while its node tree parsed perfectly.
    // See [`vex::classes`], which states the rule and the evidence for it.
    let classes = vex::classes_of(data).with_context(|| format!("{label}: class table"))?;
    let Some(class_id) = pick(classes) else {
        bail!(
            "{label} is .vex version {}, and the class id for this node type has not \
             been recovered for it - see docs/formats/pure-status.md",
            classes.version
        );
    };

    let nodes = vex::nodes(data).context("walking the node tree")?;
    let glow_bytes = glow::texture_bytes(data);

    // Positional: materials name a texture by its ordinal among the `Texture`
    // nodes, so an entry this build cannot decode has to stay in place as `None`
    // rather than shift every later index.
    let embedded: TextureSlots = vex::textures(data)
        .context("extracting textures")?
        .into_iter()
        .map(|slot| {
            slot.map(|t| {
                let label = t
                    .asset_path
                    .as_deref()
                    .or(t.name.as_deref())
                    .and_then(|n| n.rsplit(['/', '\\']).next())
                    .unwrap_or("?")
                    .to_string();
                let (width, height) = (u32::from(t.width), u32::from(t.height));
                // The levels the disc authors, when they were read: the chain
                // is the game's and so is the rule that picks among them -
                // see `Texels::Chain`. A texture whose levels are not read (a
                // pre-swizzled Pure one) keeps the box-filtered chain, capped
                // at the depth it declares; `.max(1)`: a 0 would upload no
                // base level at all.
                if t.levels.len() + 1 == usize::from(t.mip_count.max(1)) {
                    std::sync::Arc::new(ModelTexture::chain(label, width, height, t.levels_rgba()))
                } else {
                    std::sync::Arc::new(ModelTexture::rgba8(
                        label,
                        width,
                        height,
                        t.to_rgba(),
                        Some(u32::from(t.mip_count).max(1)),
                    ))
                }
            })
        })
        .collect();
    // The original resolves every `Texture` node by its own declared name,
    // never by position - see [`Ps2TextureSet::resolve`] and
    // `docs/formats/ps2-texture.md`. A node whose name resolves to nothing
    // stays `None` rather than falling back to a neighbour's texture, the way
    // a flat ordinal index used to.
    let textures = match external {
        Some(set) => ps2_textures::resolve_texture_slots(data, &nodes, classes.texture, set),
        None => embedded,
    };

    // Mesh vertices are in the local space of whichever transform encloses them,
    // nested up to 25 deep on a track, so a model is only assembled once these
    // are composed. A ship has one transform and looks the same either way,
    // which is why this was not missed sooner.
    //
    // **Anchored, not absolute**: geometry under an `Anim Transform` is baked in
    // that node's own space and moved by the shader, because the node's matrix
    // changes every frame while the bake happens once. For everything else
    // `Anchored::local` *is* the world matrix, so the two cases share one array.
    // See `vex::anim_anchors`.
    let anchors = vex::anim_anchors(data, &nodes);
    let (anim_nodes, anim_slot) = anim_node::collect(data, &nodes, &anchors, classes);
    // Where each anchor sits at time zero, for the draw-call bounds below: the
    // vertices are in anchor space, and a bounding sphere has to be in the
    // space the frustum test is done in.
    let anchor_world = vex::anchor_world(data, &nodes, 0.0);
    // Every tier is built; which one draws is chosen per frame. See `lod`.
    let lod_groups = LodGroups::collect(data, &nodes, classes, &anchors, &anchor_world);

    let mut vertices: Vec<GpuVertex> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    let mut draws: Vec<DrawCall> = Vec::new();
    let mut alpha_tested_draws: Vec<DrawCall> = Vec::new();
    let mut transparent_draws: Vec<DrawCall> = Vec::new();
    let mut shine_draws: Vec<DrawCall> = Vec::new();
    let mut mesh_count = 0;
    let mut airbrakes: [Option<Flap>; 2] = [None, None];
    let mut anim_tracks: Vec<AnimTrack> = Vec::new();
    let mut node_vertex_ranges: Vec<std::ops::Range<u32>> = Vec::new();

    for (index, node) in nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| n.class_id == class_id)
    {
        let payload = &data[node.payload()];
        let anim_node::Placement {
            to_world,
            xform,
            bounds_matrix,
        } = anim_node::placement(&anchors, &anchor_world, &anim_slot, index);
        let mut contributed = false;
        // Where this node's own vertices start, for the airbrake flaps below.
        // Taken per *node* rather than per batch: a flap is one mesh node and
        // has to move as one, whatever it is split into for drawing.
        let node_first_vertex = vertices.len() as u32;

        // A batch belongs to list A while `pass_mask & 1` is set, and to list B
        // while `pass_mask & 2` is set - one bit split of one contiguous batch
        // array, not two independent passes over the same geometry (see
        // `docs/formats/vex.md`, "Geometry is pre-batched GE display lists").
        // `vex::mesh_batches` already stops at the first batch tagged for the
        // other list, so reading both here never draws a shared batch twice;
        // a mesh made up entirely of list-B batches has nothing in list A at
        // all, so skipping either list would drop that mesh's geometry
        // completely rather than avoid a duplicate.
        //
        // Which list a batch is *in* is not the same question as which pipeline
        // it needs: on `16_Track`'s PSP build every blended batch happens to
        // live in list B alone, but the PS2 build tags its tree billboards
        // `is_alpha_tested()` while leaving them in list A. Destination is
        // decided from the batch's own attributes, not from which list produced
        // it - `is_transparent()` and `is_alpha_tested()` were never observed
        // set together on the same batch, but transparent takes priority if
        // they ever are, since blending is the more permissive of the two.
        // Which render layer this mesh's batches are drawn in. The original
        // derives it once at load from the payload header and it is the top
        // twelve bits of the sort key every batch set is queued with, so it is
        // a property of the mesh rather than of a batch - see
        // `vex::mesh_layer` and `Model::sort_by_layer`.
        let layer = vex::mesh_layer(payload, vex::LAYER_SCENE).unwrap_or(vex::LAYER_DEFAULT);
        let materials = vex::mesh_materials(payload);
        // One authored keyframe block per material, and a material without one
        // is simply not animated - the engine's own identity default. Read
        // here rather than per batch because several batches share a material.
        let transforms = vex::mesh_tex_transforms(payload);
        let mesh_flags = u16::from_le_bytes([payload[0], payload[1]]);

        for batch_list in [0u8, 1u8] {
            for batch in vex::mesh_batches(payload, batch_list).context("decoding batches")? {
                let out: &mut Vec<DrawCall> = if batch.is_transparent() {
                    &mut transparent_draws
                } else if batch.is_alpha_tested() {
                    &mut alpha_tested_draws
                } else {
                    &mut draws
                };
                let base = vertices.len() as u32;
                let first_index = indices.len() as u32;

                // material index -> texture ordinal -> a texture we decoded. Any
                // link in that chain can be missing, and a missing one draws
                // untextured rather than borrowing a neighbour's skin.
                let material_texture = materials
                    .get(usize::from(batch.material_index))
                    .copied()
                    .flatten()
                    .map(|m| m.texture);
                let second_texture = materials
                    .get(usize::from(batch.material_index))
                    .copied()
                    .flatten()
                    .map(|m| m.second_texture);
                let texture = material_texture
                    .map(|t| t as usize)
                    .filter(|&t| textures.get(t).is_some_and(Option::is_some));
                // The material's own authored track, deduplicated into
                // `anim_tracks`. `0` means "no transform", so a real track is
                // its index plus one.
                let anim = transforms
                    .get(usize::from(batch.material_index))
                    .and_then(Option::as_ref)
                    .and_then(|transform| {
                        anim_tracks
                            .iter()
                            .position(|seen| matches!(seen, AnimTrack::Psp(t) if t == transform))
                            .or_else(|| {
                                // Past the shader's array, the surface draws
                                // unanimated rather than the build failing.
                                (anim_tracks.len() + 1 < ANIM_TRACK_LIMIT).then(|| {
                                    anim_tracks.push(AnimTrack::Psp(transform.clone()));
                                    anim_tracks.len() - 1
                                })
                            })
                    })
                    .map_or(0, |at| u32::try_from(at + 1).unwrap_or(0));

                let mut lo = [f32::MAX; 3];
                let mut hi = [f32::MIN; 3];
                for v in &batch.vertices {
                    let position = vex::transform_point(&to_world, v.position);
                    for k in 0..3 {
                        lo[k] = lo[k].min(position[k]);
                        hi[k] = hi[k].max(position[k]);
                    }
                    vertices.push(GpuVertex {
                        position,
                        // A batch without normals is prelit, so face it at the
                        // camera rather than leaving it black.
                        normal: v.normal.unwrap_or([0.0, 0.0, 1.0]),
                        colour: v.colour.map_or([0.75, 0.78, 0.82, 1.0], |c| {
                            [
                                f32::from(c[0]) / 255.0,
                                f32::from(c[1]) / 255.0,
                                f32::from(c[2]) / 255.0,
                                f32::from(c[3]) / 255.0,
                            ]
                        }),
                        texcoord: v.texcoord.unwrap_or([0.0, 0.0]),
                        // No PSP or PS2 model authors a lightmap; the white 1x1
                        // bound for them makes the shader's multiply identity.
                        lightmap_texcoord: [0.0, 0.0],
                        lit: if v.colour.is_some() { 0.0 } else { 1.0 },
                        anim,
                        xform,
                        sun_mask: 1.0,
                        slots: if batch.pass_mask & glow::STAMP_BITS != 0 {
                            crate::mesh::slots::DEFAULT | crate::mesh::slots::GLOW_BATCH
                        } else {
                            crate::mesh::slots::DEFAULT
                        },
                        specular_exponent: crate::mesh::DEFAULT_SPECULAR_EXPONENT,
                        glow: glow::batch_value(&batch, material_texture, &glow_bytes),
                    });
                }
                for tri in batch.triangles() {
                    indices.extend([base + tri[0], base + tri[1], base + tri[2]]);
                    contributed = true;
                }

                let last_index = indices.len() as u32;
                if last_index > first_index {
                    let centre = [
                        (lo[0] + hi[0]) * 0.5,
                        (lo[1] + hi[1]) * 0.5,
                        (lo[2] + hi[2]) * 0.5,
                    ];
                    // The AABB's own circumscribing sphere: not the tightest
                    // possible fit, but cheap, always conservative, and exact
                    // from data already being walked for the vertices above.
                    let radius = (0..3)
                        .map(|k| (hi[k] - centre[k]).powi(2))
                        .sum::<f32>()
                        .sqrt();
                    // The vertices are in anchor space when this batch moves,
                    // so the sphere is too. Lifting it by the anchor's own
                    // time-zero matrix makes it a world-space statement about
                    // one instant - which is all it can ever be, and why
                    // `moving` turns the tests off rather than trusting it.
                    let centre = bounds_matrix.map_or(centre, |m| vex::transform_point(&m, centre));
                    let draw = DrawCall {
                        range: first_index..last_index,
                        texture,
                        bounds: Bounds { centre, radius },
                        moving: xform != 0,
                        culled: batch.is_culled(),
                        blend: batch.blend_class(),
                        // A PSP batch names a class and no factor pair.
                        blend_state: None,
                        // The reference the batch's own two flag bits ask for,
                        // recovered from `Gfx_BuildBatchStateList` - see
                        // `vex::Batch::alpha_test_reference`. Read for every
                        // batch and not only the cutout ones, because the
                        // method already answers `None` for a batch the
                        // original draws with the test off; the cutout
                        // pipeline is the only one that reads it.
                        alpha_test_ref: batch.alpha_test_reference().map(|r| f32::from(r) / 255.0),
                        layer,
                        node: Some(index as u32),
                        // Not a `.rcsmodel` chunk - see `DrawCall::chunk`.
                        chunk: None,
                    };
                    // The extra pass: the same indices again under the
                    // material's second texture - see `Model::shine_draws`.
                    // Gated as `Mesh_CompileDisplayLists` and `FUN_0890db54`
                    // gate it: the mesh word and the batch's own `0x2000`.
                    if mesh_flags & 0x2000 != 0
                        && batch.pass_mask & 0x2000 != 0
                        && let Some(second) = second_texture
                            .map(|t| t as usize)
                            .filter(|&t| textures.get(t).is_some_and(Option::is_some))
                    {
                        shine_draws.push(DrawCall {
                            texture: Some(second),
                            ..draw.clone()
                        });
                    }
                    out.push(draw);
                }
            }
        }
        if contributed {
            mesh_count += 1;
        }

        // See `Model::node_vertex_ranges`'s own doc comment.
        node_vertex_ranges.push(node_first_vertex..vertices.len() as u32);

        Flap::collect(
            &mut airbrakes,
            &nodes,
            classes,
            &anchors,
            node,
            node_first_vertex..vertices.len() as u32,
        );
    }

    if indices.is_empty() {
        if geometry_is_external(data) {
            bail!(
                "{label}: a PS3 .vex carries no render geometry - it is in the .rcsmodel \
                 beside it, which nothing here decodes"
            );
        }
        bail!("{label} decoded to no triangles");
    }

    let mut lo = [f32::MAX; 3];
    let mut hi = [f32::MIN; 3];
    for v in &vertices {
        for i in 0..3 {
            lo[i] = lo[i].min(v.position[i]);
            hi[i] = hi[i].max(v.position[i]);
        }
    }
    let centre = [
        (lo[0] + hi[0]) * 0.5,
        (lo[1] + hi[1]) * 0.5,
        (lo[2] + hi[2]) * 0.5,
    ];
    let radius = (0..3)
        .map(|i| (hi[i] - lo[i]) * 0.5)
        .fold(0.0f32, f32::max)
        .max(0.001);

    let mut model = Model {
        anim_nodes,
        // A PSP or PS2 material selects its textures and never adds one; the
        // additive glow is Wipeout HD's, and `mesh::rcs` fills this in.
        emissive: Vec::new(),
        label: label.to_string(),
        vertices,
        indices,
        draws,
        alpha_tested_draws,
        transparent_draws,
        shine_draws,
        textures,
        lightmaps: Vec::new(),
        pad_masks: Vec::new(),
        wave_maps: Vec::new(),
        material_slots: Vec::new(),
        material_specular_exponent: Vec::new(),
        material_variants: Vec::new(),
        material_anim: Vec::new(),

        vertex_colour_is_light: false,
        stamps_glow: false,
        glow_by_texel: false,

        flame: None,
        absorb_shell: false,
        alpha_test_ref: None,
        centre,
        radius,
        mesh_count,
        airbrakes,
        anim_tracks,
        node_vertex_ranges,
        lod_groups,
    };
    // The order the original's render queue dispatches these in, and the last
    // thing done to the lists: `pvs::DrawSections` is built from them and is
    // index-parallel, so it has to see them already sorted.
    model.sort_by_layer();
    Ok(model)
}

mod ps2_textures;
pub use ps2_textures::{Ps2TextureSet, resolve_texture_slots};

mod draw_call;
pub use draw_call::{Bounds, DrawCall};

mod model_texture;
pub use model_texture::{BlockFormat, GnfCounts, GnfForm, ModelTexture, Texels, TextureSlots};

mod flame;
pub mod groups;
pub use flame::Flame;
pub mod glow;
mod merge;
mod order;
pub use merge::merge;
