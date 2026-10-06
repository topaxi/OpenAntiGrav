//! Draws a craft-shaped caster into one layer of the per-craft self-shadow
//! depth array, reads the layer back, and compares the way the hull's shader
//! does: a texel of the craft under its own wing compares shadowed, a texel
//! beside it compares lit.
//!
//! **The pass has no other observable**, for the reason `sun_occlusion_map.rs`
//! gives - and this one has a failure that hides even better: the pass culls
//! front faces, so a caster of single-sided quads facing the sun is culled
//! away entirely, the map stays at far, every compare says "lit", and the
//! frame still renders. The first assertion below is that the caster reached
//! the map at all.
//!
//! Skips when there is no adapter, like `velocity_target.rs`.

use oag_core::math::{Mat4, Vec3};
use oag_mesh::mesh::GpuVertex;
use oag_mesh::mesh_render::OCCLUSION_LAYERS;
use oag_mesh::mesh_render::material_bind_group_layout;
use oag_render::shadow::map::{Caster, Fit};
use oag_render::shadow::occlusion::{Maps, REACH, Track};
use oag_render::shadow::self_shadow::SIZE;

fn vertex(position: Vec3) -> GpuVertex {
    GpuVertex {
        position: position.to_array(),
        normal: [0.0, 1.0, 0.0],
        colour: [1.0, 1.0, 1.0, 1.0],
        texcoord: [0.0, 0.0],
        lightmap_texcoord: [0.0, 0.0],
        lit: 0.0,
        anim: 0,
        xform: 0,
        sun_mask: 1.0,
        slots: oag_mesh::mesh::slots::DEFAULT,
        specular_exponent: oag_mesh::mesh::DEFAULT_SPECULAR_EXPONENT,
        glow: 0.0,
    }
}

/// One face of a slab: the quad `centre +- a +- b`, wound so its front faces
/// along `b x a`. The winding is `sun_occlusion_map.rs`'s `quad`, whose
/// `a = +x, b = +z` face is front-facing towards `+y` - measured there by
/// the occlusion pass's back-face cull drawing it.
fn face(out: &mut Vec<GpuVertex>, indices: &mut Vec<u32>, centre: Vec3, a: Vec3, b: Vec3) {
    let base = out.len() as u32;
    out.extend([
        vertex(centre - a - b),
        vertex(centre + a - b),
        vertex(centre + a + b),
        vertex(centre - a + b),
    ]);
    indices.extend([0, 2, 1, 0, 3, 2].map(|i| base + i));
}

/// A closed box, every face front-facing outwards - the shape a hull is
/// from the sun: a top the sun sees, and a far side behind it.
fn slab(out: &mut Vec<GpuVertex>, indices: &mut Vec<u32>, centre: Vec3, half: Vec3) {
    let x = Vec3::X * half.x;
    let y = Vec3::Y * half.y;
    let z = Vec3::Z * half.z;
    face(out, indices, centre + y, x, z);
    face(out, indices, centre - y, z, x);
    face(out, indices, centre + x, z, y);
    face(out, indices, centre - x, y, z);
    face(out, indices, centre + z, y, x);
    face(out, indices, centre - z, x, y);
}

/// The shader's own lookup: world through the layer's matrix to `ndc`, and
/// the texel `(ndc.x * 0.5 + 0.5, 0.5 - ndc.y * 0.5)` names.
fn project(matrix: Mat4, world: Vec3) -> (Vec3, usize) {
    let clip = matrix * world.extend(1.0);
    let ndc = clip.truncate() / clip.w;
    let u = ((ndc.x * 0.5 + 0.5) * SIZE as f32) as usize;
    let v = ((0.5 - ndc.y * 0.5) * SIZE as f32) as usize;
    (
        ndc,
        v.min(SIZE as usize - 1) * SIZE as usize + u.min(SIZE as usize - 1),
    )
}

/// `mesh.wesl`'s `SELF_SHADOW_BIAS`, restated: the compare is
/// `ndc.z - bias <= depth`.
const BIAS: f32 = 0.001;

#[test]
fn a_wing_shadows_the_fuselage_under_it_and_the_rest_of_the_hull_is_lit() {
    let instance = wgpu::Instance::default();
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&Default::default())) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("requesting the device");

    let mut maps = Maps::new(&device, &material_bind_group_layout(&device));

    // The craft: a wide lower slab (top at y = 0, underside at y = -1) and a
    // narrower one over its middle (top at y = 3, underside at y = 2).
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    slab(
        &mut vertices,
        &mut indices,
        Vec3::new(0.0, -0.5, 0.0),
        Vec3::new(5.0, 0.5, 5.0),
    );
    slab(
        &mut vertices,
        &mut indices,
        Vec3::new(0.0, 2.5, 0.0),
        Vec3::new(2.0, 0.5, 2.0),
    );
    let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("craft vertices"),
        size: std::mem::size_of_val(&vertices[..]) as u64,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&vertex_buffer, 0, bytemuck::cast_slice(&vertices));
    let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("craft indices"),
        size: std::mem::size_of_val(&indices[..]) as u64,
        usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&index_buffer, 0, bytemuck::cast_slice(&indices));
    // One opaque range covering both slabs - built by hand only because
    // clippy reads a one-element `[a..b]` literal as a mistyped `(a..b)`.
    let ranges = std::array::from_fn::<_, 1, _>(|_| 0..indices.len() as u32);
    let caster = Caster {
        vertices: &vertex_buffer,
        indices: &index_buffer,
        ranges: &ranges,
        model: Mat4::IDENTITY,
    };

    // The box the occlusion layer is rendered from is the one the self-shadow
    // layer projects through, so it comes from an occlusion render - of an
    // empty track here, which draws nothing and still sets the matrix.
    let fit = Fit {
        centre: Vec3::new(0.0, 2.0, 0.0),
        radius: 8.0,
        towards_light: Vec3::Y,
    };
    let track = Track {
        vertices: &vertex_buffer,
        indices: &index_buffer,
        draws: &[],
        cutouts: &[],
        transparent: &[],
        materials: &[],
        model: Mat4::IDENTITY,
    };
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    assert_eq!(maps.render(&queue, &mut encoder, 2, &fit, &track), 0);
    let drawn = maps.render_self_shadow(&queue, &mut encoder, 2, &caster);
    queue.submit([encoder.finish()]);
    assert_eq!(drawn, 1);
    assert_eq!(maps.self_shadow().drawn(2), 1);
    let matrix = maps.matrix(2);
    assert_eq!(
        matrix,
        fit.matrix_reaching(REACH),
        "the two maps share a matrix"
    );

    let depth = maps.self_shadow().read_back(&device, &queue, 2);
    assert!(
        depth.iter().any(|d| *d < 1.0),
        "the caster reached the map: front-face culling left something"
    );

    // What the map holds is the craft's far side. Under the upper slab it is
    // that slab's underside at y = 2, not its top at y = 3 - the front-face
    // cull, checked directly - and beside it the lower slab's underside.
    let (_, under_wing) = project(matrix, Vec3::new(0.0, 0.0, 0.0));
    let (wing_bottom, _) = project(matrix, Vec3::new(0.0, 2.0, 0.0));
    assert!(
        (depth[under_wing] - wing_bottom.z).abs() < 1e-3,
        "the map holds the wing's underside {}, got {}",
        wing_bottom.z,
        depth[under_wing]
    );
    let (_, beside) = project(matrix, Vec3::new(4.0, 0.0, 4.0));
    let (hull_bottom, _) = project(matrix, Vec3::new(4.0, -1.0, 4.0));
    assert!(
        (depth[beside] - hull_bottom.z).abs() < 1e-3,
        "the map holds the hull's underside {}, got {}",
        hull_bottom.z,
        depth[beside]
    );

    // The shader's compare, per point of the hull's sunlit faces.
    let compares_lit = |world: Vec3| {
        let (ndc, texel) = project(matrix, world);
        ndc.z - BIAS <= depth[texel]
    };
    assert!(
        !compares_lit(Vec3::new(0.0, 0.0, 0.0)),
        "the fuselage under the wing is shadowed"
    );
    assert!(
        compares_lit(Vec3::new(4.0, 0.0, 4.0)),
        "the fuselage beside the wing is lit"
    );
    assert!(
        compares_lit(Vec3::new(-4.0, 0.0, 0.0)),
        "the other side of the fuselage is lit"
    );
    assert!(
        compares_lit(Vec3::new(0.0, 3.0, 0.0)),
        "the wing's own top is lit, and does not shadow itself"
    );
    // The map's border, where nothing casts, reads far and compares lit.
    assert_eq!(depth[0], 1.0, "the corner is far");
    assert!(
        compares_lit(Vec3::new(7.0, 0.0, 7.0)),
        "a point past the craft compares lit"
    );

    // Clearing the occlusion layer clears this one to far with it.
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    maps.clear(&mut encoder, 2);
    queue.submit([encoder.finish()]);
    let cleared = maps.self_shadow().read_back(&device, &queue, 2);
    assert!(
        cleared.iter().all(|d| *d == 1.0),
        "the layer cleared to far"
    );
    assert_eq!(maps.self_shadow().drawn(2), 0);
    const { assert!(OCCLUSION_LAYERS > 2) };
}
