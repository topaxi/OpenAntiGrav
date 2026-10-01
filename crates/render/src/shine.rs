//! The hull's **extra pass**: the batches a `.vex` marks `pass_mask & 0x2000`
//! drawn a second time under environment mapping with their material's second
//! texture - on a Pulse hull, `Data\Tex\envtest4bit.tga`, the grey ramp that
//! gives the yellow spine and the inner wings their gloss.
//!
//! Recovered from `FUN_0890db54` (the pass's batch loop) and
//! `Mesh_SetBatchDrawState` (`0x0890d994`), bracketed by
//! `Mesh_BeginTransparentPass` (`0x0890d904`), and read live on a race on
//! Talon's Junction: see `docs/ghidra/functions/psp-pulse-usa/mesh-draw.md`,
//! "The hull's extra pass", for the evidence. What the pass is:
//!
//! - **Which batches**: a batch with `pass_mask & 0x2000` inside a mesh whose
//!   own flag word has it, drawn with the texture at material `+0x08`
//!   ([`oag_vex::vex::Material::second_texture`]) instead of the one at `+0x04`.
//!   On Assegai that is `shipShape`'s three blended batches, both airbrakes'
//!   small one and `canopyShape`'s glass - six of the hull's fourteen.
//! - **The coordinates** are generated, not authored: `TEXMAPMODE` 2 with
//!   `LS0 = light 0`, `LS1 = light 1`, and the hull model's two light lists
//!   (`model+0x48`) hold the fixed world-space pair [`crate::texgen::ENV_BASIS_0`]
//!   and [`crate::texgen::ENV_BASIS_1`] - `model+0x1a8` was read as `1` on the
//!   live hull and its `+0x1ac..+0x1c0` held exactly those six floats. So the
//!   coordinates follow the ship's rotation and never the camera.
//! - **Lighting is off** and the texture is drawn as it is, white vertex colour.
//! - **Blending** is `Mesh_SetBatchDrawState`'s, chosen by header byte 3 bit
//!   `0x10`, which the *loader* sets: `Mesh_CountBatchesPerList` (`0x0890e7a8`)
//!   ORs it into every list-A batch with `0x2000` and without `0x0800`. The
//!   five hull-body batches therefore *replace* what is under them and write
//!   depth; the canopy (list B) is additive and writes none. The batch's own
//!   ordinary pass then follows, additively and at equal depth, so the pixel is
//!   `env + lit * texture` either way.
//!
//! # Measured against the original, 2026-10-01
//!
//! A recorded GE list of a live Metropia-grid hull (`scripts/psp-ge-dump.py`):
//!
//! - **Replace-then-add equals one additive redraw.** The five replace batches
//!   run `BLENDMODE` `0xaa` with `FIXA` white and `FIXB` black (`src` alone),
//!   depth writes on; the ordinary pass over the same batch runs the same
//!   blend with `FIXB` white (`src + dst`), depth writes off, depth function
//!   `EQUAL`. The pixel is `env + lit * texture` clamped once, which is what
//!   our opaque hull plus an additive redraw gives. The canopy is additive in
//!   both with the depth **test on** (function 6, as the ordinary passes), not
//!   off as an earlier page read it.
//! - **Fog is on, and its colour register is `0`** where the ordinary pass
//!   carries the circuit's: the pass fades to nothing with distance, not into
//!   the haze. The scene uniform is written with a black fog colour.
//!
//! # What is ours
//!
//! Chosen, not measured:
//!
//! - The redraw's depth function is `LessEqual` where the original's is the
//!   strict one; they differ only at equal depth, which the pass never writes.
//! - **The airbrake batches are not deflected** with their flaps: they are five
//!   vertices each.
//!
//! # A circuit's own pass
//!
//! A circuit's `*_shinemap` batches carry the same bit and a chrome map as
//! their second texture (`07_chromemap_02.tga`, `07_env.tga`), and the same
//! mode-2 state, but the lights are not the fixed pair: in the same GE list all
//! 88 mode-2 circuit draws of a frame carry lights 0 and 1 equal to **rows 0 and
//! 1 of the view matrix** (confirmed at a second, yawed camera where rows and
//! columns differ), the matcap `Vex_UpdateLightLists_q` writes for a model whose
//! `+0x1a8` is 0. [`build_track`] and [`write_view`] draw it; animated nodes are
//! not drawn (their normals need the node's matrix) and say so.

use oag_core::math::{Mat4, Vec3};

use crate::mesh::{DrawCall, GpuVertex, Model};
use crate::texgen;

/// Whether a race draws the pass at all: **on**, because the original draws it
/// in play (six batches of a live Assegai hull were read in the recorded GE
/// list, each under `TEXMAPMODE` 2).
pub const DRAWN: bool = true;

/// The redraw's blend: `src + dst` on colour, the destination's alpha kept.
///
/// The original's factors are `GU_FIX` white on both sides for the additive
/// canopy; for the replace batches the GE's `src` alone lands on an empty
/// framebuffer and the ordinary pass then adds - see the module docs. The
/// alpha channel is the bloom's glow mask in this renderer and the original
/// masks it on this pass (`Gu_PixelMask` `0xff000000`, read live), so it is
/// left as it was.
pub const BLEND: wgpu::BlendState = wgpu::BlendState {
    color: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::One,
        dst_factor: wgpu::BlendFactor::One,
        operation: wgpu::BlendOperation::Add,
    },
    alpha: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::Zero,
        dst_factor: wgpu::BlendFactor::One,
        operation: wgpu::BlendOperation::Add,
    },
};

/// The extra pass's model for `hull`: only its [`Model::shine_draws`], on the
/// hull's own vertices and indices, unlit and white, through the second
/// texture each draw already names. `None` for a hull that authors no such
/// batch, which is every model but a Pulse hull.
///
/// The texture coordinates are the authored ones until [`write`] replaces them
/// each frame, because they depend on the ship's rotation.
#[must_use]
pub fn build(hull: &Model) -> Option<Model> {
    if hull.shine_draws.is_empty() {
        return None;
    }
    let mut model = hull.clone();
    model.label = format!("{} (shine pass)", hull.label);
    model.draws = std::mem::take(&mut model.shine_draws);
    plain(&mut model);
    Some(model)
}

/// Turns `model` into the extra pass's: unlit white vertices, no animation,
/// glow or flame, and only its [`Model::draws`] left to draw.
fn plain(model: &mut Model) {
    for vertex in &mut model.vertices {
        // A vertex that authors a colour keeps it: the pass is lit by nothing
        // and the texture is modulated by what the vertex carries, as the
        // original's is. One that authors none (a hull's) is white.
        if vertex.lit != 0.0 {
            vertex.colour = [1.0; 4];
        }
        vertex.lit = 0.0;
        vertex.anim = 0;
        vertex.glow = 0.0;
    }
    model.alpha_tested_draws.clear();
    model.transparent_draws.clear();
    model.lightmaps.clear();
    model.material_slots.clear();
    model.material_specular_exponent.clear();
    model.material_variants.clear();
    model.material_anim.clear();
    model.anim_tracks.clear();
    model.emissive.clear();
    model.vertex_colour_is_light = false;
    model.stamps_glow = false;
    model.flame = None;
}

/// A circuit's extra pass: the `0x2000` batches of its static meshes, on a
/// vertex buffer of their own, and where each came from.
#[derive(Debug)]
pub struct TrackPass {
    /// Draws the shine batches only, under their second texture, unlit and
    /// white. Its vertices are the batches' own, re-indexed.
    pub model: Model,
    /// For each of `model.draws`, the index of the circuit draw it redraws in
    /// the circuit's own [`Model::draws`] - the key its section mask, its
    /// level-of-detail child and its frustum bound are looked up by.
    pub sources: Vec<usize>,
    /// Shine batches left undrawn because they sit outside the opaque list,
    /// whose pipeline this pass does not have.
    pub skipped: usize,
}

/// The extra pass of a circuit, `None` when it authors no drawable batch.
///
/// **Opaque batches only.** A batch in the cutout or blended list would need
/// that list's pipeline; it is counted in [`TrackPass::skipped`] so the loader
/// report can say what was left out, rather than drawing a guess. An animated
/// node's batch is kept with its `xform` and the model's [`Model::anim_nodes`],
/// so the vertex shader places it as it places the circuit's own, and
/// [`write_view`] turns its normal by the same node matrix.
#[must_use]
pub fn build_track(track: &Model) -> Option<TrackPass> {
    let mut remap: Vec<Option<u32>> = vec![None; track.vertices.len()];
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let mut draws = Vec::new();
    let mut sources = Vec::new();
    let mut skipped = 0;
    for shine in &track.shine_draws {
        let source = track
            .draws
            .iter()
            .position(|d| d.range == shine.range && d.node == shine.node);
        let Some(source) = source else {
            skipped += 1;
            continue;
        };
        let first = indices.len() as u32;
        for &index in &track.indices[shine.range.start as usize..shine.range.end as usize] {
            let slot = &mut remap[index as usize];
            let mapped = *slot.get_or_insert_with(|| {
                vertices.push(track.vertices[index as usize]);
                vertices.len() as u32 - 1
            });
            indices.push(mapped);
        }
        draws.push(DrawCall {
            range: first..indices.len() as u32,
            ..shine.clone()
        });
        sources.push(source);
    }
    if draws.is_empty() {
        return None;
    }
    let mut model = Model::none(&format!("{} (shine pass)", track.label));
    model.vertices = vertices;
    model.indices = indices;
    model.draws = draws;
    // Only the textures a kept draw names stay on the GPU: the circuit's
    // other hundred are not this pass's to upload twice.
    let used: std::collections::HashSet<usize> =
        model.draws.iter().filter_map(|d| d.texture).collect();
    model.textures = track
        .textures
        .iter()
        .enumerate()
        .map(|(i, t)| used.contains(&i).then(|| t.clone()).flatten())
        .collect();
    model.anim_nodes = track.anim_nodes.clone();
    model.centre = track.centre;
    model.radius = track.radius;
    plain(&mut model);
    Some(TrackPass {
        model,
        sources,
        skipped,
    })
}

/// The vertices of a [`build_track`] model with texture coordinates generated
/// for the camera `view` (world to view): the equation over the view matrix's
/// first two **rows**, which are the camera's right and up in world space, so
/// a normal facing right of the camera reaches the far end of `u`.
///
/// Read off a recorded GE list at two camera yaws: the circuit's `TEXMAPMODE`
/// 2 batches carry lights 0 and 1 equal to rows 0 and 1 of the view matrix's
/// rotation, not its columns, and a hull in the same frame is placed by the
/// same matrix at view-space `(0, -1.8, -11.5)`, centred and ahead.
pub fn write_view(model: &Model, out: &mut Vec<GpuVertex>, view: Mat4, seconds: f32) {
    let right = Vec3::new(view.x_axis.x, view.y_axis.x, view.z_axis.x);
    let up = Vec3::new(view.x_axis.y, view.y_axis.y, view.z_axis.y);
    let nodes = model.sample_anim_nodes(seconds);
    if nodes.is_empty() {
        texgen::environment_map(&model.vertices, out, Mat4::IDENTITY, right, up);
        return;
    }
    // An animated node's vertices are in the node's space; its matrix takes the
    // normal to the circuit's. Not an inverse transpose, as the vertex shader's
    // own normal is not: the animated meshes under a non-uniformly scaled node
    // are the ones this skews.
    let placed: Vec<GpuVertex> = model
        .vertices
        .iter()
        .map(|v| {
            let mut v = *v;
            if let Some(m) = (v.xform as usize).checked_sub(1).and_then(|i| nodes.get(i)) {
                let n = Vec3::new(
                    m[0] * v.normal[0] + m[4] * v.normal[1] + m[8] * v.normal[2],
                    m[1] * v.normal[0] + m[5] * v.normal[1] + m[9] * v.normal[2],
                    m[2] * v.normal[0] + m[6] * v.normal[1] + m[10] * v.normal[2],
                );
                v.normal = n.normalize_or_zero().to_array();
            }
            v
        })
        .collect();
    texgen::environment_map(&placed, out, Mat4::IDENTITY, right, up);
}

/// The vertices of `model` (a [`build`] result) with their texture coordinates
/// generated for a ship posed by `ship`, into `out`: the uvgen-2 equation over
/// the two fixed basis vectors, the ship's rotation and nothing of the camera.
pub fn write(model: &Model, out: &mut Vec<GpuVertex>, ship: Mat4) {
    texgen::environment_map(
        &model.vertices,
        out,
        ship,
        texgen::ENV_BASIS_0,
        texgen::ENV_BASIS_1,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mesh::{Bounds, DrawCall, slots};
    use oag_core::math::Vec3;

    fn vertex(normal: [f32; 3]) -> GpuVertex {
        GpuVertex {
            position: [0.0; 3],
            normal,
            colour: [0.3, 0.4, 0.5, 0.6],
            texcoord: [0.25, 0.75],
            lightmap_texcoord: [0.0, 0.0],
            lit: 1.0,
            anim: 2,
            xform: 0,
            sun_mask: 1.0,
            slots: slots::DEFAULT,
            specular_exponent: crate::mesh::DEFAULT_SPECULAR_EXPONENT,
            glow: 1.0,
        }
    }

    fn draw(range: std::ops::Range<u32>, texture: usize) -> DrawCall {
        DrawCall {
            range,
            texture: Some(texture),
            bounds: Bounds {
                centre: [0.0; 3],
                radius: 1.0,
            },
            moving: false,
            culled: true,
            blend: None,
            blend_state: None,
            alpha_test_ref: None,
            layer: oag_vex::vex::LAYER_DEFAULT,
            node: Some(3),
            chunk: None,
        }
    }

    fn hull() -> Model {
        let mut hull = Model::none("hull");
        hull.vertices = vec![vertex([0.0, 0.0, 1.0]); 6];
        hull.indices = vec![0, 1, 2, 3, 4, 5];
        hull.draws = vec![draw(0..3, 0), draw(3..6, 1)];
        hull.transparent_draws = vec![draw(3..6, 5)];
        hull.shine_draws = vec![draw(3..6, 2)];
        hull
    }

    #[test]
    fn a_hull_with_no_extra_pass_batch_builds_no_shine_model() {
        let mut bare = hull();
        bare.shine_draws.clear();
        assert!(build(&bare).is_none());
    }

    #[test]
    fn the_shine_model_draws_only_the_extra_pass_batches_under_their_second_texture() {
        let model = build(&hull()).expect("a hull with an extra pass");
        assert_eq!(model.draws.len(), 1);
        assert_eq!(model.draws[0].range, 3..6, "the batch's own vertices");
        assert_eq!(model.draws[0].texture, Some(2), "the second texture");
        assert!(model.alpha_tested_draws.is_empty() && model.transparent_draws.is_empty());
        assert!(
            model.shine_draws.is_empty(),
            "moved into `draws`, not kept twice"
        );
    }

    #[test]
    fn the_pass_is_unlit_white_and_writes_no_glow() {
        let model = build(&hull()).unwrap();
        for v in &model.vertices {
            assert_eq!(v.colour, [1.0; 4]);
            assert_eq!((v.lit, v.anim, v.glow), (0.0, 0, 0.0));
        }
        assert!(!model.stamps_glow);
    }

    /// The equation over the fixed basis: a normal along light 0 is the far
    /// end of `u` and a normal along light 1 is the far end of `v`.
    #[test]
    fn a_normal_along_each_basis_vector_reaches_the_far_end_of_its_axis() {
        let mut model = build(&hull()).unwrap();
        model.vertices[0].normal = texgen::ENV_BASIS_0.to_array();
        model.vertices[1].normal = texgen::ENV_BASIS_1.to_array();
        let mut out = Vec::new();
        write(&model, &mut out, Mat4::IDENTITY);
        assert!((out[0].texcoord[0] - 1.0).abs() < 1.0e-5, "{:?}", out[0]);
        assert!((out[1].texcoord[1] - 1.0).abs() < 1.0e-5, "{:?}", out[1]);
        assert_eq!(out.len(), model.vertices.len());
    }

    /// The coordinates follow the ship: a turned ship moves the same vertex's
    /// coordinate, and nothing the camera does can reach `write`.
    #[test]
    fn turning_the_ship_moves_the_coordinates() {
        let model = build(&hull()).unwrap();
        let (mut still, mut turned) = (Vec::new(), Vec::new());
        write(&model, &mut still, Mat4::IDENTITY);
        write(&model, &mut turned, Mat4::from_rotation_x(1.2));
        assert_ne!(still[0].texcoord, turned[0].texcoord);
        // The translation half of the matrix is not a rotation of the normal.
        let mut moved = Vec::new();
        write(
            &model,
            &mut moved,
            Mat4::from_translation(Vec3::new(5.0, 6.0, 7.0)),
        );
        assert_eq!(still[0].texcoord, moved[0].texcoord);
    }

    fn circuit() -> Model {
        let mut track = Model::none("circuit");
        track.vertices = (0..9).map(|i| vertex([i as f32, 0.0, 1.0])).collect();
        track.indices = (0..9).collect();
        track.textures = vec![None, None, None];
        track.draws = vec![draw(0..3, 0), draw(3..6, 1), draw(6..9, 0)];
        // The second names a range no opaque draw has: a cutout or blended one.
        track.shine_draws = vec![draw(3..6, 2), draw(9..12, 2)];
        track
    }

    #[test]
    fn a_circuit_pass_keeps_every_opaque_shine_batch_and_says_which_it_left() {
        let pass = build_track(&circuit()).expect("a circuit with a shine batch");
        assert_eq!(pass.model.draws.len(), 1);
        assert_eq!(pass.sources, vec![1], "the circuit draw it redraws");
        assert_eq!(pass.skipped, 1, "the one outside the opaque list");
        assert_eq!(pass.model.draws[0].texture, Some(2), "the second texture");
        assert_eq!(
            pass.model.draws[0].range,
            0..3,
            "re-indexed to its own buffer"
        );
        assert_eq!(pass.model.vertices.len(), 3, "only the batch's vertices");
        assert!(
            pass.model
                .vertices
                .iter()
                .all(|v| v.colour == [1.0; 4] && v.lit == 0.0)
        );
    }

    #[test]
    fn a_circuit_with_only_non_opaque_shine_batches_builds_no_pass() {
        let mut track = circuit();
        track.shine_draws.remove(0);
        assert!(build_track(&track).is_none());
    }

    /// Rows, not columns: a view that turns the camera away from the world
    /// axes has different rows and columns, and the coordinate follows the row.
    #[test]
    fn the_circuit_coordinate_follows_the_cameras_right_and_up_in_world_space() {
        let pass = build_track(&circuit()).unwrap();
        // The camera yawed 90 degrees about y: its right axis is world +z (forward x up).
        let view = Mat4::look_to_rh(Vec3::ZERO, Vec3::X, Vec3::Y);
        let mut out = Vec::new();
        let mut model = pass.model;
        model.vertices[0].normal = [0.0, 0.0, 1.0];
        model.vertices[1].normal = [0.0, 1.0, 0.0];
        model.vertices[2].normal = [1.0, 0.0, 0.0];
        write_view(&model, &mut out, view, 0.0);
        assert!(
            (out[0].texcoord[0] - 1.0).abs() < 1.0e-5,
            "toward the camera's right: {:?}",
            out[0]
        );
        assert!(
            (out[1].texcoord[1] - 1.0).abs() < 1.0e-5,
            "toward the camera's up: {:?}",
            out[1]
        );
        assert!(
            (out[2].texcoord[0] - 0.5).abs() < 1.0e-5,
            "along its line of sight: {:?}",
            out[2]
        );
    }

    /// A circuit's shine vertices carry authored colours (`vtype 0x139`), and
    /// the pass modulates the chrome map by them; only a vertex that authors
    /// none is white.
    #[test]
    fn a_circuit_vertex_keeps_the_colour_it_authors() {
        let mut track = circuit();
        track.vertices[3].lit = 0.0;
        track.vertices[3].colour = [0.2, 0.3, 0.4, 1.0];
        let pass = build_track(&track).unwrap();
        assert_eq!(pass.model.vertices[0].colour, [0.2, 0.3, 0.4, 1.0]);
        assert_eq!(pass.model.vertices[1].colour, [1.0; 4]);
    }

    /// An animated node's normal is turned by the node's matrix before the
    /// coordinate is generated, so the highlight rides the moving mesh.
    #[test]
    fn an_animated_nodes_normal_is_turned_by_its_matrix() {
        use crate::mesh::{AnimNode, Motion};
        let mut track = circuit();
        track.vertices[3].xform = 1;
        track.vertices[3].normal = [1.0, 0.0, 0.0];
        // A quarter turn about z: x becomes y.
        let mut quarter = oag_vex::vex::IDENTITY;
        quarter[0..2].copy_from_slice(&[0.0, 1.0]);
        quarter[4..6].copy_from_slice(&[-1.0, 0.0]);
        track.anim_nodes = vec![AnimNode {
            transform: Motion::Vex(Box::default()),
            static_above: quarter,
            parent: None,
        }];
        let pass = build_track(&track).unwrap();
        let (mut still, mut turned) = (Vec::new(), Vec::new());
        // Camera right is +x, up is +y.
        let view = Mat4::IDENTITY;
        let mut flat = pass.model.clone();
        flat.anim_nodes.clear();
        write_view(&flat, &mut still, view, 0.0);
        write_view(&pass.model, &mut turned, view, 0.0);
        assert!(
            (still[0].texcoord[0] - 1.0).abs() < 1.0e-5,
            "faces right: {:?}",
            still[0]
        );
        assert!(
            (turned[0].texcoord[1] - 1.0).abs() < 1.0e-5,
            "turned to face up: {:?}",
            turned[0]
        );
    }
}
