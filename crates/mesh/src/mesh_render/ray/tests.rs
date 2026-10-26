//! The ray tier: off means untouched, and on means a ray finds what the raster draws.
//!
//! Every GPU test skips without an adapter that reports the feature (llvmpipe does).

use super::*;
use crate::mesh::{Bounds, DrawCall, GpuVertex, Model};
use wgpu::util::DeviceExt as _;

fn vertex(position: [f32; 3]) -> GpuVertex {
    GpuVertex {
        quake: 0.0,
        position,
        normal: [0.0, 0.0, 1.0],
        colour: [1.0; 4],
        texcoord: [0.0, 0.0],
        lightmap_texcoord: [0.0, 0.0],
        lit: 0.0,
        anim: 0,
        xform: 0,
        sun_mask: 1.0,
        slots: crate::mesh::slots::DEFAULT,
        specular_exponent: crate::mesh::DEFAULT_SPECULAR_EXPONENT,
        glow: 0.0,
        texcoord2: [0.0, 0.0],
        tangent: 0,
    }
}

/// One opaque triangle, 1.6 across, at depth 0.5 under an identity camera.
fn triangle() -> Model {
    let call = DrawCall {
        moving: false,
        blend: None,
        blend_state: None,
        layer: oag_vex::vex::LAYER_DEFAULT,
        culled: false,
        range: 0..3,
        texture: None,
        bounds: Bounds {
            centre: [0.0, 0.0, 0.5],
            radius: 1.5,
        },
        node: None,
        chunk: None,
        alpha_test_ref: None,
        depth_point: None,
        writes_depth: false,
    };
    Model {
        indices: vec![0, 1, 2],
        draws: vec![call],
        centre: [0.0, 0.0, 0.5],
        radius: 2.0,
        mesh_count: 1,
        vertices: vec![
            vertex([-0.8, -0.8, 0.5]),
            vertex([0.8, -0.8, 0.5]),
            vertex([0.0, 0.8, 0.5]),
        ],
        ..Model::none("ray test triangle")
    }
}

fn adapter() -> Option<wgpu::Adapter> {
    let instance = wgpu::Instance::default();
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
            .ok()?;
    adapter.features().contains(FEATURES).then_some(adapter)
}

fn ray_device() -> Option<(wgpu::Device, wgpu::Queue)> {
    let adapter = adapter()?;
    eprintln!("ray test adapter: {}", adapter.get_info().name);
    request(true);
    let descriptor = apply(
        wgpu::DeviceDescriptor {
            required_limits: wgpu::Limits::default(),
            ..Default::default()
        },
        &adapter,
    );
    pollster::block_on(adapter.request_device(&descriptor)).ok()
}

const IDENTITY_MATRIX: [f32; 16] = [
    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
];

fn texel(image: &[u8], (w, h): (u32, u32), ndc: (f32, f32)) -> [u8; 4] {
    let x = (((ndc.0 + 1.0) / 2.0) * w as f32) as usize;
    let y = (((1.0 - ndc.1) / 2.0) * h as f32) as usize;
    let at = (y * w as usize + x) * 4;
    image[at..at + 4].try_into().unwrap()
}

#[test]
fn the_affine_is_the_column_vector_form_of_the_row_matrix() {
    // Translation in the fourth row of the row-vector matrix lands in the last column.
    let mut m = IDENTITY_MATRIX;
    m[12..15].copy_from_slice(&[3.0, 4.0, 5.0]);
    // A row-vector x axis of (0, 1, 0): the second row of M^T's first column.
    m[0..3].copy_from_slice(&[0.0, 1.0, 0.0]);
    let a = accel::affine(&m);
    assert_eq!([a[3], a[7], a[11]], [3.0, 4.0, 5.0]);
    assert_eq!(
        [a[0], a[4], a[8]],
        [0.0, 1.0, 0.0],
        "column 0 is the x axis"
    );
}

#[test]
fn a_device_that_never_asked_for_the_tier_is_left_exactly_as_it_was() {
    let Some(adapter) = adapter() else {
        return;
    };
    request(false);
    let plain = wgpu::DeviceDescriptor {
        required_limits: wgpu::Limits::default(),
        ..Default::default()
    };
    let applied = apply(plain.clone(), &adapter);
    assert_eq!(applied.required_features, plain.required_features);
    assert_eq!(applied.required_limits, plain.required_limits);
    assert!(!applied.experimental_features.is_enabled());
    let (device, _queue) = pollster::block_on(adapter.request_device(&applied)).expect("a device");
    assert!(!enabled(&device));
    assert!(
        blas_input(&device).is_empty(),
        "today's buffers, byte for byte"
    );
}

#[test]
fn a_ray_finds_the_track_where_the_raster_draws_it_and_a_craft_where_its_matrix_puts_it() {
    let Some((device, queue)) = ray_device() else {
        return;
    };
    assert!(enabled(&device));
    let model = triangle();
    let vertices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("test vertices"),
        contents: bytemuck::cast_slice(&model.vertices),
        usage: wgpu::BufferUsages::VERTEX | blas_input(&device),
    });
    let track =
        ModelAccel::build(&device, &queue, "track", &model, None, &vertices).expect("a BLAS");
    assert_eq!(track.stats.triangles, 1);
    let hull = ModelAccel::build(&device, &queue, "hull", &model, None, &vertices).expect("a BLAS");
    let mut scene = Scene::new(create_tlas(&device), track, vec![hull], 2);

    // The hull is nearer the camera and moved right by 0.8: a transposed matrix would put it
    // somewhere else.
    let mut moved = IDENTITY_MATRIX;
    moved[12] = 0.8;
    moved[14] = -0.25;
    let size = (64, 64);
    let image = Debug::new(&device).render(
        &device,
        &queue,
        &mut scene,
        &Frame {
            nodes: &[],
            craft: &[(0, moved)],
            visible: ALWAYS,
        },
        IDENTITY_MATRIX,
        size,
    );
    // Static only: left of the hull's reach, inside the track triangle.
    let track_pixel = texel(&image, size, (-0.5, -0.7));
    assert!(
        track_pixel[2] > track_pixel[0] && track_pixel[3] == 255,
        "the track reads blue: {track_pixel:?}"
    );
    // Hull only: past the track triangle's right edge, inside the moved hull.
    let hull_pixel = texel(&image, size, (0.9, -0.7));
    assert!(
        hull_pixel[0] > hull_pixel[2],
        "the hull reads orange: {hull_pixel:?}"
    );
    // Nothing: above both.
    assert_eq!(texel(&image, size, (-0.9, 0.9)), [0, 0, 0, 255]);
    assert_eq!(scene.live(), 2);
}

/// Two copies of one triangle a hair apart, in sections 0 and 1 of a track's partition:
/// the near copy (depth 0.5) is draw 0, the far one (depth 0.52, moved right by 0.4) is
/// draw 1, so a pixel left of both reads which copies the structure holds.
fn swap_pair() -> (Model, [u64; 2]) {
    let mut model = triangle();
    let mut far = model.draws[0].clone();
    far.range = 3..6;
    model.draws.push(far);
    model.indices.extend([3, 4, 5]);
    model.vertices.extend([
        vertex([-0.4, -0.8, 0.52]),
        vertex([1.2, -0.8, 0.52]),
        vertex([0.4, 0.8, 0.52]),
    ]);
    (model, [1 << 0, 1 << 1])
}

#[test]
fn each_section_is_a_part_of_its_own_and_a_model_without_sections_is_always_in() {
    let (model, sections) = swap_pair();
    let split = accel::partition(
        &model,
        Some(Sections {
            opaque: &sections,
            alpha_tested: &[],
        }),
    );
    assert_eq!(
        split.keys().copied().collect::<Vec<_>>(),
        [(0, 1 << 0), (0, 1 << 1)]
    );
    assert_eq!(split[&(0, 1 << 1)], [3, 4, 5]);
    // A hull, or a circuit with no partition: one part, in every frame.
    let whole = accel::partition(&model, None);
    assert_eq!(whole.keys().copied().collect::<Vec<_>>(), [(0, ALWAYS)]);
    // A list shorter than its draws shows too much, never too little.
    let short = accel::partition(
        &model,
        Some(Sections {
            opaque: &sections[..1],
            alpha_tested: &[],
        }),
    );
    assert_eq!(
        short.keys().copied().collect::<Vec<_>>(),
        [(0, 1 << 0), (0, ALWAYS)]
    );
    // The frame's visible set decides: one copy, the other, both only when the set names
    // both, and an unplaced part always.
    assert!(instanced(1 << 0, 0b01) && !instanced(1 << 1, 0b01));
    assert!(instanced(1 << 1, 0b10) && !instanced(1 << 0, 0b10));
    assert!(instanced(ALWAYS, 0b10));
}

#[test]
fn a_ray_meets_only_the_copy_whose_section_the_frame_shows() {
    let Some((device, queue)) = ray_device() else {
        return;
    };
    let (model, sections) = swap_pair();
    let vertices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("test vertices"),
        contents: bytemuck::cast_slice(&model.vertices),
        usage: wgpu::BufferUsages::VERTEX | blas_input(&device),
    });
    let track = ModelAccel::build(
        &device,
        &queue,
        "track",
        &model,
        Some(Sections {
            opaque: &sections,
            alpha_tested: &[],
        }),
        &vertices,
    )
    .expect("a BLAS");
    assert_eq!(track.stats.blases, 2, "one BLAS per section");
    let mut scene = Scene::new(create_tlas(&device), track, Vec::new(), 0);
    let debug = Debug::new(&device);
    let size = (64, 64);
    let mut shoot = |visible| {
        let image = debug.render(
            &device,
            &queue,
            &mut scene,
            &Frame {
                nodes: &[],
                craft: &[],
                visible,
            },
            IDENTITY_MATRIX,
            size,
        );
        (
            texel(&image, size, (-0.5, -0.7)),
            texel(&image, size, (0.9, -0.7)),
        )
    };
    let empty = [0, 0, 0, 255];
    // Left of the far copy, inside the near one; and right of the near copy, inside the far.
    let (near, far) = shoot(1 << 0);
    assert_ne!(near, empty, "section 0 shows the near copy");
    assert_eq!(far, empty, "and not the far one");
    let (near, far) = shoot(1 << 1);
    assert_eq!(near, empty, "section 1 hides the near copy");
    assert_ne!(far, empty, "and shows the far one");
    let (near, far) = shoot(ALWAYS);
    assert!(near != empty && far != empty, "everything shows both");
}
