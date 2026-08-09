//! Loads a `.vex` model out of an archive and flattens it for the GPU.

use anyhow::{Context, Result, bail};
use oag_assets::Archive;
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
    /// How many whole V (row) sweeps of this vertex's texture pass under it per
    /// [`ANIM_PERIOD_TICKS`], or 0.0 for a surface that does not animate.
    ///
    /// The shader offsets the V texture coordinate by `v_cycles * anim_phase`,
    /// and that is the entire animation: the colour and brightness curve lives
    /// in the texture's own rows, not in any code here. Which textures qualify,
    /// and the evidence for each, is [`ANIMATED_TEXTURES`].
    ///
    /// A rate rather than the flag this used to be, so a surface that turns out
    /// to run at a different speed is a table entry rather than a second uniform
    /// and a second branch. It stays one `f32` because no recovered surface
    /// scrolls in U: a track model is millions of vertices, and a second
    /// component would cost that much memory to carry zeroes.
    pub v_cycles: f32,
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
#[derive(Debug)]
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

/// Which textures animate and how fast: `oag_pulse::textures::ANIMATED_TEXTURES`.
///
/// The table moved to the title package under [ADR-0021] - which surfaces move is
/// a fact about what Pulse ships, and every key is a `Data\Tex\` entry off its
/// disc. The mechanism stays here: this lookup, the per-vertex
/// [`GpuVertex::v_cycles`] attribute and the V offset in `mesh.wgsl`. The evidence
/// for each entry is on the constant itself and in
/// `crates/render/tests/animated_uv_ground_truth.rs`.
///
/// [ADR-0021]: ../../../docs/architecture/adr/0021-title-packages.md
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
#[derive(Debug)]
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
}

/// One authored airbrake flap: which vertices are its own, and what it hinges
/// about.
///
/// The flap is **not** a separate model. `Airbrake` (`0x3c5`) sits between a
/// `Transform` and a `Mesh` in `Ship.vex`, so its geometry is already in the
/// ship's buffers with everything else, and pulling it out into its own
/// `Drawable` would duplicate the ship's whole texture set for two meshes.
/// What it needs instead is for its own vertices to move, which is what
/// [`Self::vertices`] is for - see `Flap::deflect`.
#[derive(Debug, Clone, PartialEq)]
pub struct Flap {
    /// This flap's vertices, as a range into [`Model::vertices`].
    ///
    /// Contiguous because the builder appends one mesh node's vertices at a
    /// time, and an `Airbrake` has exactly one `Mesh` child in every shipped
    /// file. A flap split across two nodes would need a list here, and an
    /// assertion in the builder would be a better way to find that out than a
    /// half-moved flap.
    pub vertices: std::ops::Range<u32>,
    /// The hinge's pose in model space: the enclosing `Transform`'s composed
    /// world matrix.
    ///
    /// The locator, not the flap. Its two instances are mirror images in X
    /// (`+1.5246` and `-1.5246` on Assegai), which is what makes the sides
    /// rotate oppositely without either angle being negated by hand.
    pub hinge: Mat4,
}

impl Flap {
    /// The model-space transform that swings this flap by `angle` radians.
    ///
    /// `hinge * R * hinge^-1`, because the builder has already baked every
    /// ancestor transform into the vertices ([`build_with_textures`] composes
    /// the tree through `vex::world_transforms`). Rotating the baked vertices
    /// directly would swing them about the model's origin instead of the
    /// hinge, which on Assegai is 6.2 units behind it.
    ///
    /// # The axis is chosen, not recovered
    ///
    /// The `Airbrake` node's payload is **zero bytes** and its class
    /// descriptor carries no handler, so the file does not say which way the
    /// flap turns and there is no per-class update function in the binary to
    /// read it out of - the dispatch that drives a tagged node is indirect
    /// (`Exhaust_Update`, the worked example of the same shape, has no direct
    /// callers either). Under the confidence rubric that puts the axis below
    /// 50, so it is **not** presented as recovered: local X is picked because
    /// it swings the flap the way an airbrake looks like it should, and the
    /// mirrored hinges then take care of the sign. What *is* recovered is
    /// everything around it - the hinge pose, the deflection in radians and
    /// both rates.
    #[must_use]
    pub fn deflect(&self, angle: f32) -> Mat4 {
        // Stowed is the common case and has to be *exact*: `H * I * H^-1` is
        // only identity to about `1e-7`, which would leave a closed flap
        // permanently a fraction of a unit off the hull it was authored
        // against. Cheap to rule out, and it is the case a parked ship spends
        // all its time in.
        if angle == 0.0 {
            return Mat4::IDENTITY;
        }
        self.hinge * Mat4::from_rotation_x(angle) * self.hinge.inverse()
    }
}

/// Reads one named blob out of an archive inside a disc image.
///
/// `spec` is `<image>:<path-on-disc>`, matching `oag-wad`.
pub fn read_blob(spec: &str, name: &str) -> Result<Vec<u8>> {
    let mut archive = Archive::open(spec)?;
    archive
        .read_name(name)
        .with_context(|| format!("reading {name} from {}", archive.label()))
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
    build_class(label, data, external, lod, vex::CLASS_MESH)
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
    build_class(label, data, external, Lod::Both, vex::CLASS_SKYCUBE)
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
    build_class(label, data, external, Lod::Both, vex::CLASS_SPEEDUP_PAD)
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
    class_id: u32,
) -> Result<Model> {
    if !vex::has_magic(data) {
        bail!("{label} is not a .vex file (no VEXX magic)");
    }

    let nodes = vex::nodes(data).context("walking the node tree")?;
    let slots = nodes
        .iter()
        .filter(|n| n.class_id == vex::CLASS_TEXTURE)
        .count();

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
        for (i, node) in nodes.iter().enumerate() {
            if node.class_id != vex::CLASS_LOD_GROUP {
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
                let v_cycles = texture
                    .and_then(|t| textures.get(t))
                    .and_then(Option::as_ref)
                    .and_then(|t| animated_v_cycles(&t.label))
                    .unwrap_or(0.0);

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
                        v_cycles,
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
                .is_some_and(|n| n.class_id == vex::CLASS_AIRBRAKE)
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

/// Concatenates several models into one buffer pair.
///
/// Used to draw more than one thing at a time without the pipeline learning
/// about scenes: `oag-view --collision --with-spline` overlays the collision
/// soup on the driveable ribbon so the two can be compared in place.
///
/// Index and texture-slot references are rebased, since both are positional.
/// Empty models are skipped rather than contributing an empty draw call.
///
/// This is a *concatenation*, not a scene: the models share one depth buffer and
/// one opaque pipeline, so a solid model will occlude anything inside it. That is
/// why the collision view draws outlines by default; see
/// [`crate::collision`].
#[must_use]
pub fn merge(label: &str, models: Vec<Model>) -> Model {
    let mut out = Model {
        // `merge` folds several sources into one buffer pair, and a flap's
        // vertex range is only meaningful against the source it came from -
        // the same reason `DrawCall::node` is documented as ambiguous here.
        airbrakes: [None, None],
        label: label.to_string(),
        vertices: Vec::new(),
        indices: Vec::new(),
        draws: Vec::new(),
        alpha_tested_draws: Vec::new(),
        transparent_draws: Vec::new(),
        textures: Vec::new(),
        centre: [0.0; 3],
        radius: 1.0,
        mesh_count: 0,
    };

    for model in models {
        if model.vertices.is_empty() || model.indices.is_empty() {
            continue;
        }
        let vertex_base = out.vertices.len() as u32;
        let index_base = out.indices.len() as u32;
        let texture_base = out.textures.len();

        out.vertices.extend(model.vertices);
        out.indices
            .extend(model.indices.iter().map(|i| i + vertex_base));
        let rebase = |d: DrawCall| DrawCall {
            range: (d.range.start + index_base)..(d.range.end + index_base),
            texture: d.texture.map(|t| t + texture_base),
            bounds: d.bounds,
            node: d.node,
            blend: d.blend,
            culled: d.culled,
        };
        out.draws.extend(model.draws.into_iter().map(rebase));
        out.alpha_tested_draws
            .extend(model.alpha_tested_draws.into_iter().map(rebase));
        out.transparent_draws
            .extend(model.transparent_draws.into_iter().map(rebase));
        out.textures.extend(model.textures);
        out.mesh_count += model.mesh_count;
    }

    let mut lo = [f32::MAX; 3];
    let mut hi = [f32::MIN; 3];
    for v in &out.vertices {
        for i in 0..3 {
            lo[i] = lo[i].min(v.position[i]);
            hi[i] = hi[i].max(v.position[i]);
        }
    }
    if !out.vertices.is_empty() {
        out.centre = [
            (lo[0] + hi[0]) * 0.5,
            (lo[1] + hi[1]) * 0.5,
            (lo[2] + hi[2]) * 0.5,
        ];
        out.radius = (0..3)
            .map(|i| (hi[i] - lo[i]) * 0.5)
            .fold(0.0f32, f32::max)
            .max(0.001);
    }
    out
}

#[cfg(test)]
mod flap_tests {
    use super::*;
    use oag_core::math::Vec3;

    /// A hinge a couple of units out to one side, tilted so the test cannot
    /// pass by accident on an axis-aligned matrix.
    fn flap(x: f32) -> Flap {
        Flap {
            vertices: 0..1,
            hinge: Mat4::from_translation(Vec3::new(x, -0.5, -6.0)) * Mat4::from_rotation_y(0.3),
        }
    }

    /// A stowed flap must be exactly where the builder put it.
    ///
    /// Exactly, not nearly: the rest position is written back every frame from
    /// the model's own vertices, so any drift here would be visible as a flap
    /// that never quite closes.
    #[test]
    fn a_zero_deflection_moves_nothing() {
        assert_eq!(flap(1.5).deflect(0.0), Mat4::IDENTITY);
    }

    /// The hinge itself is the one point that does not move.
    ///
    /// This is the whole reason `deflect` is `hinge * R * hinge^-1` rather than
    /// a bare rotation: the vertices arrive with every ancestor transform
    /// already baked in, so rotating them directly would swing the flap about
    /// the model's origin - six units away on a real ship - and tear it off the
    /// hull.
    #[test]
    fn the_hinge_point_stays_put() {
        let flap = flap(1.5);
        let pivot = flap.hinge.w_axis.truncate();
        let moved = flap.deflect(0.5).transform_point3(pivot);
        assert!(
            (moved - pivot).length() < 1e-5,
            "the hinge moved to {moved:?} from {pivot:?}"
        );
    }

    /// Mirrored hinges swing the two sides apart, with one angle for both.
    ///
    /// The sides are never negated by hand anywhere: `Race` feeds both flaps
    /// the same positive deflection and the opposition comes entirely from the
    /// locators being mirror images, which
    /// `tests/airbrake_flaps_ground_truth.rs` confirms on all eight teams.
    ///
    /// **The opposition is in `x` and only in `x`**, which is worth pinning
    /// rather than assuming - an earlier version of this test looked for it in
    /// `z` and failed. Conjugating by a reflection mirrors the whole motion, so
    /// two flaps swing *apart* sideways while rising and sweeping identically:
    /// exactly what a symmetric pair of airbrakes does, and not what a naive
    /// "negate the angle on one side" would give.
    #[test]
    fn mirrored_hinges_swing_the_two_sides_apart() {
        // The artists' mirror, as a conjugation, rather than a sign flip on the
        // translation alone: a real locator's rotation is mirrored too.
        let mirror = Mat4::from_scale(Vec3::new(-1.0, 1.0, 1.0));
        let left = flap(1.5);
        let right = Flap {
            vertices: left.vertices.clone(),
            hinge: mirror * left.hinge * mirror,
        };

        let point = Vec3::new(0.0, 0.0, -6.0);
        let moved_left = left.deflect(0.4).transform_point3(point) - point;
        let moved_right = right.deflect(0.4).transform_point3(point) - point;

        assert!(
            moved_left.x.abs() > 1e-3,
            "the fixture does not move sideways at all: {moved_left:?}"
        );
        assert!(
            moved_left.x.signum() != moved_right.x.signum(),
            "both sides swung the same way in x: {moved_left:?} and {moved_right:?}"
        );
        assert!(
            (moved_left.y - moved_right.y).abs() < 1e-5
                && (moved_left.z - moved_right.z).abs() < 1e-5,
            "a mirrored pair must rise and sweep alike: {moved_left:?} and {moved_right:?}"
        );
    }
}

#[cfg(test)]
mod merge_tests {
    use super::*;

    fn model(label: &str, vertices: usize, textures: usize) -> Model {
        Model {
            airbrakes: [None, None],
            label: label.to_string(),
            vertices: (0..vertices)
                .map(|k| GpuVertex {
                    position: [k as f32, 0.0, 0.0],
                    normal: [0.0, 1.0, 0.0],
                    colour: [1.0; 4],
                    texcoord: [0.0; 2],
                    lit: 1.0,
                    v_cycles: 0.0,
                })
                .collect(),
            indices: (0..vertices as u32).collect(),
            draws: vec![DrawCall {
                blend: None,
                culled: false,
                range: 0..vertices as u32,
                texture: (textures > 0).then_some(0),
                bounds: Bounds {
                    centre: [0.0; 3],
                    radius: 1.0,
                },
                node: None,
            }],
            alpha_tested_draws: Vec::new(),
            transparent_draws: Vec::new(),
            textures: (0..textures).map(|_| None).collect(),
            centre: [0.0; 3],
            radius: 1.0,
            mesh_count: 1,
        }
    }

    /// Indices are positions in a shared buffer, so the second model's have to
    /// be rebased or it draws the first model's geometry twice.
    #[test]
    fn indices_are_rebased_onto_the_combined_buffer() {
        let out = merge("both", vec![model("a", 3, 0), model("b", 3, 0)]);
        assert_eq!(out.vertices.len(), 6);
        assert_eq!(out.indices, vec![0, 1, 2, 3, 4, 5]);
        assert_eq!(out.draws[1].range, 3..6);
    }

    /// A material names a texture by its ordinal, so the slot index has to move
    /// with the slots or the second model draws the first model's skin.
    #[test]
    fn texture_slots_are_rebased_too() {
        let out = merge("both", vec![model("a", 3, 2), model("b", 3, 1)]);
        assert_eq!(out.textures.len(), 3);
        assert_eq!(out.draws[1].texture, Some(2));
    }

    #[test]
    fn empty_models_contribute_nothing() {
        let out = merge("one", vec![model("a", 3, 0), model("empty", 0, 0)]);
        assert_eq!(out.draws.len(), 1);
        assert_eq!(out.vertices.len(), 3);
    }

    #[test]
    fn merging_nothing_is_still_a_usable_model() {
        let out = merge("none", Vec::new());
        assert!(out.vertices.is_empty());
        assert!(out.radius > 0.0);
    }
}
