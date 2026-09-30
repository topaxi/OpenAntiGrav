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
//! # What is ours
//!
//! Chosen, not measured:
//!
//! - **One additive redraw after the hull** ([`BLEND`], depth `LessEqual`, no
//!   write) stands in for the original's replace-then-add pair. For the five
//!   replace batches the sum is the same; for the canopy, which the original
//!   draws with the depth test off, this keeps it behind the hull's own
//!   occluders.
//! - **No fog** on the pass, as the absorb overlay has none.
//! - **The airbrake batches are not deflected** with their flaps: they are five
//!   vertices each.

use oag_core::math::Mat4;

use crate::mesh::{GpuVertex, Model};
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
    for vertex in &mut model.vertices {
        vertex.colour = [1.0; 4];
        vertex.lit = 0.0;
        vertex.anim = 0;
        vertex.glow = 0.0;
    }
    model.draws = std::mem::take(&mut model.shine_draws);
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
    Some(model)
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
}
