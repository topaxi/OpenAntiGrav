//! A lit surface under a **non-uniformly scaled moving node** is shaded the way
//! the baked path shades it.
//!
//! `mesh.wgsl` moves a node's vertices through the node-table matrix, and used
//! to turn the normal through the same matrix: right for a rotation or a
//! uniform scale, wrong under a per-axis one, where a normal must go through
//! the inverse transpose. Wipeout 2048 has 51 such lit meshes across its
//! fourteen race circuits. The baked (static) 2048 path already did the inverse
//! transpose on the CPU, so the two ways of placing one oblique quad under
//! `scale (0.5, 1, 1)` have to agree:
//!
//! - **moving**: vertices and normal authored in the node's space, the node
//!   table's matrix (`xform = 1`) scaling them in the shader;
//! - **baked**: the vertices scaled and the normal put through the inverse
//!   transpose on the CPU, `xform = 0`.
//!
//! Measured 2026-10-05: before the shader change the two differ by up to 8 in a
//! channel (7,968 summed over the frame); after it, by 0.
//!
//! Skips when there is no adapter, like `zone_recolour.rs`.

use oag_mesh::mesh::{
    AnimNode, Bounds, DrawCall, Emissive, GpuVertex, Model, ModelTexture, Motion, Texels, slots,
};
use oag_mesh::mesh_render::Anisotropy;
use oag_rcs::rcsskeleton::{IDENTITY, Node};
use std::sync::Arc;

const SCALE: [f32; 3] = [0.5, 1.0, 1.0];
const SIZE: u32 = 96;

fn vertex(position: [f32; 3], normal: [f32; 3], xform: u32) -> GpuVertex {
    GpuVertex {
        position,
        normal,
        colour: [1.0, 1.0, 1.0, 1.0],
        texcoord: [0.0, 0.0],
        lightmap_texcoord: [0.0, 0.0],
        lit: 1.0,
        anim: 0,
        xform,
        sun_mask: 1.0,
        slots: slots::DEFAULT,
        specular_exponent: oag_mesh::mesh::DEFAULT_SPECULAR_EXPONENT,
        glow: 0.0,
        texcoord2: [0.0, 0.0],
    }
}

fn model(vertices: Vec<GpuVertex>, moving: bool) -> Model {
    let node = Node {
        id: 1,
        parent: None,
        scale: SCALE,
        rotation: [0.0, 0.0, 0.0, 1.0],
        translation: [0.0; 3],
        visible: true,
        pivot: [0.0; 3],
        pivot_translate: [0.0; 3],
        above: IDENTITY,
        kinds: Default::default(),
    };
    Model {
        airbrakes: [None, None],
        node_vertex_ranges: Vec::new(),
        lod_groups: Default::default(),
        label: "moving normal test".into(),
        indices: vec![0, 1, 2],
        draws: vec![DrawCall {
            moving,
            blend: None,
            blend_state: None,
            layer: oag_vex::vex::LAYER_DEFAULT,
            culled: false,
            range: 0..3,
            texture: Some(0),
            bounds: Bounds {
                centre: [0.0; 3],
                radius: 2.0,
            },
            node: None,
            chunk: None,
            alpha_test_ref: None,
        }],
        alpha_tested_draws: Vec::new(),
        transparent_draws: Vec::new(),
        textures: vec![Some(Arc::new(ModelTexture {
            label: "white".into(),
            width: 1,
            height: 1,
            texels: Texels::Rgba8(vec![255, 255, 255, 255]),
            mip_count: None,
        }))],
        lightmaps: vec![None],
        pad_masks: Vec::new(),
        wave_maps: Vec::new(),
        material_slots: Vec::new(),
        material_specular_exponent: Vec::new(),
        material_colour_factor: Vec::new(),
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
        radius: 2.0,
        mesh_count: 1,
        anim_tracks: Vec::new(),
        anim_nodes: if moving {
            vec![AnimNode {
                transform: Motion::Rig(Box::new(oag_rcs::rig::NodeMotion { node, track: None })),
                static_above: IDENTITY,
                parent: None,
            }]
        } else {
            Vec::new()
        },
        emissive: Vec::new(),
        vertices,
    }
}

/// The oblique quad's corners in node space, in the plane `x + z = 0`, whose
/// normal is `(1, 0, 1) / sqrt 2`.
const CORNERS: [[f32; 3]; 3] = [[-0.6, -0.7, 0.6], [0.6, -0.7, -0.6], [0.0, 0.7, 0.0]];

fn render(model: &Model) -> Option<Vec<u8>> {
    oag_mesh::capture::capture_pixels_from(model, SIZE, SIZE, 0.6, 0.2, Anisotropy::Off, 0.0).ok()
}

#[test]
fn a_lit_surface_under_a_non_uniform_scale_is_shaded_as_the_baked_path_shades_it() {
    let s = std::f32::consts::FRAC_1_SQRT_2;
    let node_normal = [s, 0.0, s];
    let moving = model(
        CORNERS.iter().map(|&c| vertex(c, node_normal, 1)).collect(),
        true,
    );
    // The CPU inverse transpose of `diag(SCALE)` is `diag(1 / SCALE)`.
    let n = [node_normal[0] / SCALE[0], 0.0, node_normal[2] / SCALE[2]];
    let len = (n[0] * n[0] + n[2] * n[2]).sqrt();
    let baked_normal = [n[0] / len, 0.0, n[2] / len];
    let baked = model(
        CORNERS
            .iter()
            .map(|&c| {
                vertex(
                    [c[0] * SCALE[0], c[1] * SCALE[1], c[2] * SCALE[2]],
                    baked_normal,
                    0,
                )
            })
            .collect(),
        false,
    );
    let (Some(a), Some(b)) = (render(&moving), render(&baked)) else {
        println!("skipping: no GPU adapter");
        return;
    };
    let covered = a.chunks(4).filter(|p| p[..3] != a[..3]).count();
    assert!(covered > 100, "the quad drew only {covered} pixels");
    let worst = a
        .iter()
        .zip(&b)
        .map(|(x, y)| x.abs_diff(*y))
        .max()
        .unwrap_or(0);
    let total: u64 = a
        .iter()
        .zip(&b)
        .map(|(x, y)| u64::from(x.abs_diff(*y)))
        .sum();
    assert!(
        worst <= 1,
        "moving and baked shade the same quad differently: worst channel {worst}, total {total}"
    );
}

fn flat(diffuse_alpha: u8, layered: bool) -> Model {
    let n = [0.0, 0.0, 1.0];
    let mut vertices: Vec<GpuVertex> = [[-0.6, -0.6, 0.0], [0.6, -0.6, 0.0], [0.0, 0.7, 0.0]]
        .iter()
        .map(|&c| vertex(c, n, 0))
        .collect();
    let mut m = model(Vec::new(), false);
    m.textures = vec![Some(Arc::new(ModelTexture {
        label: "diffuse".into(),
        width: 1,
        height: 1,
        texels: Texels::Rgba8(vec![200, 200, 200, diffuse_alpha]),
        mip_count: None,
    }))];
    if layered {
        for v in &mut vertices {
            v.slots |= slots::ADD_SECOND | (1 << slots::MATERIAL_SHIFT);
        }
        m.lightmaps = vec![Some(Arc::new(ModelTexture {
            label: "emissive".into(),
            width: 1,
            height: 1,
            texels: Texels::Rgba8(vec![255, 0, 0, 255]),
            mip_count: None,
        }))];
        m.emissive = vec![Emissive {
            tint: [1.0, 1.0, 1.0],
            offset: 0.0,
            scale: 1.0,
            rate: 0.0,
        }];
    }
    m.vertices = vertices;
    m
}

/// The glow layer is gated by the diffuse alpha: a Wipeout 2048 material whose
/// diffuse is clear adds nothing, and a model that binds an emissive texture
/// beside a diffuse does not change how a surface without the flag draws.
#[test]
fn a_glow_layer_is_gated_by_the_diffuse_alpha() {
    let (Some(plain_clear), Some(layered_clear), Some(layered_opaque), Some(plain_opaque)) = (
        render(&flat(0, false)),
        render(&flat(0, true)),
        render(&flat(255, true)),
        render(&flat(255, false)),
    ) else {
        println!("skipping: no GPU adapter");
        return;
    };
    assert_eq!(
        plain_clear, layered_clear,
        "a clear diffuse still took the glow"
    );
    assert_ne!(
        plain_opaque, layered_opaque,
        "an opaque diffuse took no glow at all"
    );
}
