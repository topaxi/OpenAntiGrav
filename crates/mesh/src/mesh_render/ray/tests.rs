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
    let track = ModelAccel::build(&device, &queue, "track", &model, &vertices).expect("a BLAS");
    assert_eq!(track.stats.triangles, 1);
    let hull = ModelAccel::build(&device, &queue, "hull", &model, &vertices).expect("a BLAS");
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
        &[],
        &[(0, moved)],
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
