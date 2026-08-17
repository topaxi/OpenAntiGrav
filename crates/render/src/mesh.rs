//! Loads a `.vex` model out of an archive and flattens it for the GPU.

use anyhow::{Context, Result, bail};
use oag_assets::Container;
use oag_core::math::Mat4;
use oag_formats::{ps2_texture, vex, wad};

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
}

/// A world-space bounding sphere, for frustum culling.
///
/// A sphere rather than an oriented box: cheap to test
/// ([`oag_core::math::frustum::Frustum::intersects_sphere`]), and a batch's
/// own vertices already give a centre and radius for free while they are
/// being iterated to build [`GpuVertex`]s, with no separate pass needed.
#[derive(Debug, Clone, Copy)]
pub struct Bounds {
    pub centre: [f32; 3],
    pub radius: f32,
}

/// A run of indices sharing one texture.
#[derive(Debug, Clone)]
pub struct DrawCall {
    pub range: std::ops::Range<u32>,
    /// Index into [`Model::textures`], or `None` for untextured.
    pub texture: Option<usize>,
    /// World-space bounds of this draw call's own vertices, for frustum
    /// culling - not the whole model's, which would defeat the point on a
    /// track where one `Model` is the entire circuit.
    pub bounds: Bounds,
    /// Whether this batch is drawn single-sided, from its own `pass_mask`.
    ///
    /// **The original culls most of its geometry and this crate culled none of
    /// it.** Measured on the PSP disc: `01_Track` has 1,632 of 1,734 batches
    /// culled, `16_Track` 1,613 of 2,079, and `Assegai\Ship.vex` 12 of 14 -
    /// but `shipboost.vex` culls none of its four, so it is genuinely per
    /// batch and not a global setting. It matters most on **transparent**
    /// batches, where a back face is not hidden by the depth test but blended
    /// a second time: 119 of `01_Track`'s 142 transparent batches are
    /// single-sided in the original, so drawing them two-sided doubles their
    /// contribution. See [`oag_formats::vex::Batch::is_culled`].
    pub culled: bool,
    /// Which blend equation this batch asked for, or `None` when it is not a
    /// transparent batch at all.
    ///
    /// **The three bits inside `is_transparent()`'s `0x0700` are not
    /// interchangeable, and this crate drew all of them with one equation
    /// until it was recovered.** `0x100` is an ordinary alpha blend, `0x200`
    /// is additive and source-alpha weighted, `0x400` is unblended. Carried
    /// per draw call rather than per model because a single mesh mixes them.
    /// See [`oag_formats::vex::Batch::blend_class`].
    pub blend: Option<vex::BlendClass>,
    /// Index of the scene-tree node this draw call came from, into the
    /// `vex::nodes` of the file the model was built from, or `None` for
    /// synthetic geometry (ribbons, collision overlays, fixtures).
    ///
    /// This is what lets `oag_render::pvs` place a draw call in its *authored*
    /// section - the section node sharing its ancestor group - instead of
    /// guessing from world-space bounds. Only meaningful while the model maps
    /// to one file; [`merge`] keeps the per-source values, which are ambiguous
    /// across sources.
    pub node: Option<u32>,
}

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

/// Which textures animate and how fast: `oag_pulse::textures::ANIMATED_TEXTURES`.
///
/// The table moved to the title package under [ADR-0022] - which surfaces move is
/// a fact about what Pulse ships, and every key is a `Data\Tex\` entry off its
/// disc. The evidence for each entry is on the constant itself and in
/// `crates/render/tests/animated_uv_ground_truth.rs`.
///
/// **Nothing draws through this any more.** The renderer reads each material's
/// authored keyframe block instead ([`GpuVertex::anim`]), which is the
/// original's own mechanism rather than an inference from geometry - and the
/// two disagree: the table scrolls `col_display7_GLOW` in V where the disc
/// authors it in U. What the table and its measurement are still good for is
/// the question they were built to answer, *which* surfaces on a circuit are
/// meant to move, which is a useful cross-check on the authored reading and
/// the only record of the narrow-V-band survey. Kept for that, and for
/// [`is_blink_light_texture`], which several ship paths still key off.
///
/// [ADR-0022]: ../../../docs/architecture/adr/0022-title-packages.md
pub use oag_pulse::textures::ANIMATED_TEXTURES;

/// The V scroll rate for a decoded texture, or `None` if it does not animate.
#[must_use]
pub fn animated_v_cycles(label: &str) -> Option<f32> {
    let key = label.to_ascii_lowercase();
    // Longest match first, so `col_display7_BLEND_GLOW` is not claimed by the
    // `col_display7_GLOW` entry through a shared prefix.
    ANIMATED_TEXTURES
        .iter()
        .filter(|(name, _)| key.contains(name))
        .max_by_key(|(name, _)| name.len())
        .map(|&(_, cycles)| cycles)
}

/// Whether a decoded texture's name identifies its surface as the shared
/// blink-light palette, animated by scrolling its V (row) coordinate.
///
/// **Confidence: 85.** Every one of the 8 playable PSP ships carries a mesh
/// named `glowingShape` whose material resolves to the exact same shared
/// texture, `Data\Tex\colours_flashing_GLOW.tga` - not a per-ship asset, a
/// common one. The ship-specific mesh names first noticed on Feisar
/// (`underbrake_flashrightShape`/`underbrake_flashleftShape`) and Triakis
/// (`flasherShape`/`flasher1Shape`) resolve to the identical texture, which is
/// why matching by mesh name generalised badly (each ship names its extra
/// copies of this light differently, or not at all) while matching by the
/// texture it actually paints generalises to all of them.
///
/// The texture's rows turned out to be the animation itself - see
/// `docs/formats/vex.md`, "The animation is authored in the texture, on its V
/// axis", for the full survey and the capture that confirmed it
/// (`crates/render/tests/blink_lights_ground_truth.rs` checks the texture
/// match against every real ship).
///
/// Matching on the texture rather than the mesh name also means a mesh with
/// more than one material - Feisar's `self_illuminatedShape` has one batch on
/// this texture and another on the ship's own steady-lit skin - is judged
/// batch by batch instead of being wrongly all-or-nothing.
///
/// Kept as its own predicate, rather than folded into [`animated_v_cycles`],
/// because the ship claim is evidenced far more strongly than any track entry:
/// this one is worth naming and citing separately even though the table would
/// match the same label. `blink_lights_ground_truth.rs` asserts it against every
/// real ship.
#[must_use]
pub fn is_blink_light_texture(label: &str) -> bool {
    label.to_ascii_lowercase().contains("flashing_glow")
}

#[cfg(test)]
mod blink_texture_tests {
    use super::{ANIMATED_TEXTURES, animated_v_cycles, is_blink_light_texture};

    #[test]
    fn matches_the_shared_blink_texture_however_it_is_cased() {
        assert!(is_blink_light_texture("colours_flashing_GLOW.tga"));
        assert!(is_blink_light_texture("COLOURS_FLASHING_GLOW.TGA"));
    }

    #[test]
    fn excludes_unrelated_textures() {
        assert!(!is_blink_light_texture("engine_general.tga"));
        assert!(!is_blink_light_texture("texture1.tga"));
    }

    #[test]
    fn the_blink_palette_animates_through_the_table_too() {
        assert_eq!(animated_v_cycles("colours_flashing_GLOW.tga"), Some(2.0));
    }

    #[test]
    fn track_entries_are_matched_case_insensitively() {
        assert!(animated_v_cycles("col_display7_GLOW.tga").is_some());
        assert!(animated_v_cycles("07_Pulse_light_BLEND_GLOW.TGA").is_some());
        assert!(animated_v_cycles("rf_cyclegrad3_GLOW.tga").is_some());
    }

    /// The static sponsor art that a `_GLOW`/`_ADD` suffix rule would have
    /// swept up. Each of these is a real texture on a real circuit.
    #[test]
    fn static_art_never_animates() {
        for label in [
            "hub_banner_GLOW.tga",
            "col_banners2_ADD.tga",
            "FEISAR3_GLOW.tga",
            "Harimau_Glow.tga",
            "billboard1.tga",
            "banner2.tga",
            "tunnelanim_sb.tga",
            "flicker1nonalpha_GLOW.tga",
            "Plasma_scroll_ADD_GLOW.tga",
            "SL_stripwindows_shinemap.tga",
        ] {
            assert_eq!(animated_v_cycles(label), None, "{label} must not animate");
        }
    }

    /// `col_display7_GLOW` is a prefix of `col_display7_BLEND_GLOW` in neither
    /// direction, but both contain `col_display7`, so the longest-match rule is
    /// what keeps a future shorter entry from swallowing a longer one.
    #[test]
    fn the_longest_matching_entry_wins() {
        let table_keys: Vec<&str> = ANIMATED_TEXTURES.iter().map(|&(n, _)| n).collect();
        assert!(table_keys.contains(&"col_display7_glow"));
        assert!(table_keys.contains(&"col_display7_blend_glow"));
        assert_eq!(
            animated_v_cycles("col_display7_BLEND_GLOW.tga"),
            Some(2.0),
            "matched by the more specific entry"
        );
    }
}

/// A texture decoded from the model.
#[derive(Debug, Clone)]
pub struct ModelTexture {
    pub label: String,
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// Whether [`build_with_textures`] draws every child of an authored
/// `LodGroup` (class `0x2ee`), or only the first.
///
/// **This is not a quality tier and it does not switch by distance.** The
/// original PSP binary never does either: it registers the class but never
/// reads its `child_count` or switch-distance fields, and its generic tree
/// walker draws every child of every node unconditionally, always, with no
/// live re-evaluation against the camera - see `docs/formats/vex.md`,
/// "`LodGroup`: authored, but never switched at runtime". Ten of the eleven
/// `LodGroup` instances on `16_Track` carry two children with real,
/// differently-detailed mesh geometry in both, so [`Self::Both`] (matching
/// the original) means genuinely overlapping duplicate geometry, not a
/// "higher quality" picture. [`Self::Single`] is a one-time choice made when
/// the model is built, not a live switch: it keeps the higher-detail tier and
/// permanently discards the other, removing that duplication at the cost of
/// no longer matching the original.
///
/// A real distance-based switch (the original's authored switch-distance
/// value, re-checked against the camera every frame) would need `Model` to
/// carry per-node bounds and the render loop to re-evaluate them each frame -
/// the same live-camera mechanism the roadmap's unimplemented frustum-culling
/// entry needs, and not yet built.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Lod {
    /// Draw every child of every `LodGroup`, exactly as the original does -
    /// including the duplicate geometry that results.
    #[default]
    Both,
    /// Draw only the first child of a two-child `LodGroup` - the
    /// higher-triangle-count tier in all ten measured cases - and skip the
    /// rest. A permanent, load-time choice, not a live switch. Removes real
    /// duplicate geometry the original always draws twice, at the cost of no
    /// longer matching it.
    Single,
}

impl Lod {
    /// The spelling used in a settings file and on a menu row.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Both => "both",
            Self::Single => "single",
        }
    }

    /// Every mode, for the menus and for error messages.
    pub const ALL: [Self; 2] = [Self::Both, Self::Single];
}

impl std::str::FromStr for Lod {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|lod| lod.name().eq_ignore_ascii_case(text))
            .ok_or_else(|| format!("{text:?} is not a level-of-detail mode; try both or single"))
    }
}

impl std::fmt::Display for Lod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

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
    /// Batches tagged `is_alpha_tested()` (see [`build_with_textures`]),
    /// indexing the same `vertices`/`indices` as [`Self::draws`]. Meant to be
    /// drawn with a cutout (`discard` below a threshold) rather than a
    /// hardcoded alpha of 1.0, but otherwise opaque - depth write stays on, so
    /// overlapping cutout surfaces still occlude each other and the geometry
    /// behind them correctly.
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
    pub textures: Vec<Option<ModelTexture>>,
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
    pub anim_tracks: Vec<vex::TexTransform>,
    /// One vertex range per node of the class this model was built for, in
    /// the same node-file order [`oag_formats::pads::volumes`] walks the same
    /// file with the same class id - [`Flap::vertices`] generalised from two
    /// fixed slots to however many nodes a class authors, for a caller that
    /// wants trigger *i*'s own geometry and has only its position to reach it
    /// by. Empty outside [`build_class`] (a ribbon, a collision overlay, a
    /// merge).
    pub node_vertex_ranges: Vec<std::ops::Range<u32>>,
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
            centre: [0.0; 3],
            radius: 0.0,
            mesh_count: 0,
            airbrakes: [None, None],
            anim_tracks: Vec::new(),
            node_vertex_ranges: Vec::new(),
        }
    }
}

mod flap;
pub use flap::Flap;

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

/// Flattens every mesh in a `.vex` into one buffer pair.
///
/// Draws every child of an authored `LodGroup`, matching the original's own
/// behaviour - see [`build_with_textures`] to choose [`Lod::Single`] instead.
pub fn build(label: &str, data: &[u8]) -> Result<Model> {
    build_with_textures(label, data, None, Lod::Both)
}

/// As [`build`], with an external texture set replacing the embedded one, and
/// a choice of [`Lod`] behaviour.
///
/// PS2 models need the texture set: their embedded texture block is empty by
/// design, so there is nothing for a material to resolve to unless the set is
/// supplied from outside. Passing `None` uses whatever the file embeds, which
/// is what every PSP model wants.
pub fn build_with_textures(
    label: &str,
    data: &[u8],
    external: Option<Vec<Option<ModelTexture>>>,
    lod: Lod,
) -> Result<Model> {
    build_class(label, data, external, lod, |c| c.mesh)
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
pub fn build_sky(
    label: &str,
    data: &[u8],
    external: Option<Vec<Option<ModelTexture>>>,
) -> Result<Model> {
    build_optional_class(label, data, external, |c| c.skycube)
}

/// The track's speedup pads, as a model in their own right.
///
/// A [`vex::CLASS_SPEEDUP_PAD`] node's payload is a mesh payload for the same
/// reason a `Skycube`'s is (the pad's bind handler calls the `Mesh` bind first),
/// so this is [`build_with_textures`] pointed at a third class, not a third
/// decoder. See [`oag_formats::pads`] for the trigger volume that shares those
/// bytes.
///
/// Separate from [`build`]'s track model on purpose, and not because it is a
/// different kind of draw: it is the same pipeline and the same textures. The
/// track model's draw calls are what [`crate::pvs`] indexes its per-section
/// visibility against, and pads belong to no `section`, so folding them in would
/// put geometry into that mapping that the mapping cannot describe.
///
/// The textures are the track file's own, indexed by the same ordinals, so this
/// must be built from the same `data` the track model was, and from the same
/// `external` set on PS2 - see [`build_sky`].
///
/// Returns a model with no meshes when the file authors no pads, which is
/// ordinary: every Pure track and every `.vex` that is not a track.
pub fn build_pads(
    label: &str,
    data: &[u8],
    external: Option<Vec<Option<ModelTexture>>>,
) -> Result<Model> {
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
    external: Option<Vec<Option<ModelTexture>>>,
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
/// word its report accordingly. `oag_game::race::load` does exactly that.
///
/// [`build`] itself does **not** go through this: a track or ship whose `Mesh`
/// id is unrecovered has no geometry at all, and returning an empty model there
/// would be a black screen reported as a success.
fn build_optional_class(
    label: &str,
    data: &[u8],
    external: Option<Vec<Option<ModelTexture>>>,
    pick: fn(vex::classes::Classes) -> Option<u32>,
) -> Result<Model> {
    if vex::classes_of(data).is_ok_and(|classes| pick(classes).is_none()) {
        return Ok(Model::none(label));
    }
    build_class(label, data, external, Lod::Both, pick)
}

/// Whether this `.vex`'s render geometry lives outside the file.
///
/// True for a PS3 `.vex`, and that is the whole of the test: **Wipeout HD moved
/// its meshes into a `.rcsmodel` beside each `.vex`** - 643 of them, 686 MiB,
/// undecoded - and left the node tree, the transforms, the spline, the collision
/// soup and the pad volumes exactly where they were. So an HD model parses
/// perfectly and yields no triangles, which is indistinguishable from a decoder
/// bug unless something says so.
///
/// Byte order stands in for the console because it is read from the file's own
/// magic (`XXEV` against `VEXX`) rather than from anything a caller was told,
/// which is the rule everywhere else in this project. A big-endian `.vex` is a
/// PS3 one; no PSP or PS2 pressing ships one.
#[must_use]
pub fn geometry_is_external(data: &[u8]) -> bool {
    vex::has_magic(data) && vex::byte_order(data) == oag_formats::ByteOrder::Big
}

/// Flattens every node of one class into one buffer pair.
///
/// The class is a parameter because `Skycube` and `Mesh` share a payload layout
/// exactly; see [`build_sky`].
fn build_class(
    label: &str,
    data: &[u8],
    external: Option<Vec<Option<ModelTexture>>>,
    lod: Lod,
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
    let slots = classes.texture.map_or(0, |texture| {
        nodes.iter().filter(|n| n.class_id == texture).count()
    });

    // `Lod::Single` skips every node under a two-child `LodGroup`'s second
    // child - see `Lod`'s own doc comment for why this is an invented
    // divergence rather than a recovered one. Built as a mark-and-skip set
    // over the whole tree up front, the same shape a mesh's own descendants
    // would need if this ever grows past direct children.
    let skip: std::collections::HashSet<usize> = if lod == Lod::Single {
        let mut children: Vec<Vec<usize>> = vec![Vec::new(); nodes.len()];
        for (i, node) in nodes.iter().enumerate() {
            if let Some(p) = node.parent {
                children[p].push(i);
            }
        }
        fn mark(children: &[Vec<usize>], root: usize, out: &mut std::collections::HashSet<usize>) {
            out.insert(root);
            for &child in &children[root] {
                mark(children, child, out);
            }
        }
        let mut skip = std::collections::HashSet::new();
        // `None` for a version whose `LodGroup` id is unrecovered, and then
        // nothing is skipped - which is `Lod::Both`, the safe direction. Matching
        // version 6's `0x2ee` against a version-4 file would be the unsafe one:
        // that number is some *other* class there, so the skip would delete real
        // geometry rather than a level of detail.
        for (i, node) in nodes.iter().enumerate() {
            if Some(node.class_id) != classes.lod_group {
                continue;
            }
            let payload = &data[node.payload()];
            let Some(child_count) = payload.get(0x50..0x54) else {
                continue;
            };
            if u32::from_le_bytes(child_count.try_into().expect("checked len 4")) == 2
                && let Some(&second) = children[i].get(1)
            {
                mark(&children, second, &mut skip);
            }
        }
        skip
    } else {
        std::collections::HashSet::new()
    };

    // Positional: materials name a texture by its ordinal among the `Texture`
    // nodes, so an entry this build cannot decode has to stay in place as `None`
    // rather than shift every later index.
    let embedded: Vec<Option<ModelTexture>> = vex::textures(data)
        .context("extracting textures")?
        .into_iter()
        .map(|slot| {
            slot.map(|t| ModelTexture {
                // The runtime path first: it is present on tracks *and* ships,
                // where the node-header name is the artists' `Z:/...` authoring
                // path and is absent altogether on every track texture.
                label: t
                    .asset_path
                    .as_deref()
                    .or(t.name.as_deref())
                    .and_then(|n| n.rsplit(['/', '\\']).next())
                    .unwrap_or("?")
                    .to_string(),
                width: u32::from(t.width),
                height: u32::from(t.height),
                rgba: t.to_rgba(),
            })
        })
        .collect();
    // A short external set leaves the tail untextured rather than misaligning
    // the ordinals a material indexes with.
    let textures = external.map_or(embedded, |mut set| {
        set.resize_with(set.len().max(slots), || None);
        set
    });

    // Mesh vertices are in the local space of whichever transform encloses them,
    // nested up to 25 deep on a track, so a model is only assembled once these
    // are composed. A ship has one transform and looks the same either way,
    // which is why this was not missed sooner.
    let world = vex::world_transforms(data, &nodes);

    let mut vertices: Vec<GpuVertex> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    let mut draws: Vec<DrawCall> = Vec::new();
    let mut alpha_tested_draws: Vec<DrawCall> = Vec::new();
    let mut transparent_draws: Vec<DrawCall> = Vec::new();
    let mut mesh_count = 0;
    let mut airbrakes: [Option<Flap>; 2] = [None, None];
    let mut anim_tracks: Vec<vex::TexTransform> = Vec::new();
    let mut node_vertex_ranges: Vec<std::ops::Range<u32>> = Vec::new();

    for (index, node) in nodes
        .iter()
        .enumerate()
        .filter(|(i, n)| n.class_id == class_id && !skip.contains(i))
    {
        let payload = &data[node.payload()];
        let to_world = world[index];
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
        let materials = vex::mesh_materials(payload);
        // One authored keyframe block per material, and a material without one
        // is simply not animated - the engine's own identity default. Read
        // here rather than per batch because several batches share a material.
        let transforms = vex::mesh_tex_transforms(payload);

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
                let texture = materials
                    .get(usize::from(batch.material_index))
                    .copied()
                    .flatten()
                    .map(|m| m.texture as usize)
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
                            .position(|seen| seen == transform)
                            .or_else(|| {
                                // Past the shader's array, the surface draws
                                // unanimated rather than the build failing.
                                (anim_tracks.len() + 1 < ANIM_TRACK_LIMIT).then(|| {
                                    anim_tracks.push(transform.clone());
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
                        lit: if v.colour.is_some() { 0.0 } else { 1.0 },
                        anim,
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
                    out.push(DrawCall {
                        range: first_index..last_index,
                        texture,
                        bounds: Bounds { centre, radius },
                        culled: batch.is_culled(),
                        blend: batch.blend_class(),
                        node: Some(index as u32),
                    });
                }
            }
        }
        if contributed {
            mesh_count += 1;
        }

        // See `Model::node_vertex_ranges`'s own doc comment.
        node_vertex_ranges.push(node_first_vertex..vertices.len() as u32);

        // An airbrake flap is a `Mesh` whose parent is an `Airbrake` node. The
        // hinge is the `Airbrake`'s own parent - the locator `Transform` that
        // carries the 4x4 - and `world` already holds its composed pose, which
        // is why this reads the ancestor rather than re-composing anything.
        //
        // Left and right by the artists' own node names, not by the sign of a
        // translation: `Airbrake_Left` sits on the `+x` locator in Assegai's
        // file, so a reimplementation that inferred sides from geometry would
        // have to decide what `+x` means and could get it backwards silently.
        if let Some(brake) = node.parent
            && nodes
                .get(brake)
                // Version-keyed for the same reason the `LodGroup` check above
                // is: `None` leaves a ship with no recovered flaps rather than
                // mounting whatever version 4 happens to number `0x3c0`.
                .is_some_and(|n| Some(n.class_id) == classes.airbrake)
            && let Some(hinge) = nodes[brake].parent
        {
            let name = nodes[brake].name.as_deref().unwrap_or_default();
            let side = if name.eq_ignore_ascii_case("Airbrake_Left") {
                Some(0)
            } else if name.eq_ignore_ascii_case("Airbrake_Right") {
                Some(1)
            } else {
                None
            };
            let span = node_first_vertex..vertices.len() as u32;
            if let Some(side) = side
                && !span.is_empty()
            {
                airbrakes[side] = Some(Flap {
                    vertices: span,
                    hinge: Mat4::from_cols_array(&world[hinge]),
                });
            }
        }
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

    Ok(Model {
        label: label.to_string(),
        vertices,
        indices,
        draws,
        alpha_tested_draws,
        transparent_draws,
        textures,
        centre,
        radius,
        mesh_count,
        airbrakes,
        anim_tracks,
        node_vertex_ranges,
    })
}

/// Decodes a PS2 texture set into slots a model can be re-skinned with.
///
/// PS2 `.vex` files declare a texture block of zero length: their textures are
/// separate archive entries, and a model's set is gathered into a **nested WAD**
/// of its own, one entry per `Texture` node and in the same order. Each entry is
/// a Graphics Synthesizer upload packet, see
/// [`oag_formats::ps2_texture`].
///
/// Slots are positional for the same reason [`build`] keeps them positional: a
/// material names a texture by its ordinal, so an entry this build cannot decode
/// stays `None` in place rather than shifting every later one.
pub fn ps2_texture_set(blob: &[u8]) -> Result<Vec<Option<ModelTexture>>> {
    let count = wad::Directory::peek_entry_count(blob).context("not a nested WAD")?;
    let directory = wad::Directory::parse(blob, Some(blob.len() as u64))
        .map_err(|e| anyhow::anyhow!("{e}"))
        .context("parsing the texture set directory")?;

    let mut out = Vec::with_capacity(count as usize);
    for (index, entry) in directory.entries.iter().enumerate() {
        let start = entry.offset as usize;
        let end = start + entry.size as usize;
        let Some(stored) = blob.get(start..end) else {
            out.push(None);
            continue;
        };
        let decompressed = match entry.compression {
            wad::Compression::None => stored.to_vec(),
            wad::Compression::Lzss => {
                match oag_formats::lzss::decompress(stored, entry.size_uncompressed as usize) {
                    Ok(bytes) => bytes,
                    Err(_) => {
                        out.push(None);
                        continue;
                    }
                }
            }
            wad::Compression::Zlib => {
                out.push(None);
                continue;
            }
        };
        out.push(
            ps2_texture::parse(&decompressed)
                .ok()
                .map(|texture| ModelTexture {
                    label: format!("#{index} {:08x}", entry.name_hash),
                    width: u32::from(texture.width),
                    height: u32::from(texture.height),
                    rgba: texture.to_rgba(),
                }),
        );
    }
    Ok(out)
}

mod merge;
pub use merge::merge;
