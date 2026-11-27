//! A fully sun-occluded 2048 vertex-lit surface draws one smooth shade, not a binary speckle.
//!
//! 2048's vertex-lit surfaces (`NOVAColor`) carry `2 + mask` in the vertex's `sun_mask`, and a
//! hull's factor mask `-1 - mask`; the shader tells them from a plain mask by a threshold on the
//! interpolated value. A mostly occluded surface sits within a byte of `2.0` (and a plain one of
//! `0.0`), so the interpolation's rounding put pixels on either side of a threshold at the
//! boundary, and the occluded surface read as the other kind on the wrong side: a grey-white stipple over Altima's shore,
//! banners and rock faces (`docs/rendering/water.md`, "The stipple"). These tests fail if the
//! thresholds go back to the boundary. Skips when there is no adapter; needs no game content.

use std::sync::Arc;

use oag_mesh::mesh::{Bounds, DrawCall, GpuVertex, Model, ModelTexture, Texels, slots};
use oag_mesh::mesh_render::{self, Anisotropy, Light, Scene, UNIFORMS_SIZE};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const SIZE: u32 = 128;

fn vertex(position: [f32; 3], sun_mask: f32) -> GpuVertex {
    GpuVertex {
        quake: 0.0,
        position,
        normal: [0.0, 0.0, 1.0],
        colour: [0.0, 0.0, 0.0, 1.0],
        texcoord: [0.0, 0.0],
        lightmap_texcoord: [0.0, 0.0],
        // The authored path.
        lit: 1.0,
        anim: 0,
        xform: 0,
        sun_mask,
        slots: slots::DEFAULT,
        specular_exponent: oag_mesh::mesh::DEFAULT_SPECULAR_EXPONENT,
        glow: 0.0,
        texcoord2: [0.0, 0.0],
        tangent: 0,
        depth_bias: 0.0,
    }
}

fn texture(label: &str, rgba: [u8; 4]) -> Arc<ModelTexture> {
    Arc::new(ModelTexture {
        label: label.into(),
        width: 1,
        height: 1,
        texels: Texels::Rgba8(rgba.to_vec()),
        mip_count: None,
        lod_code: 0.0,
    })
}

/// One large triangle at `z = 0.5`, a white albedo and no lightmap, each vertex carrying its `sun_mask`.
fn model(sun_mask: [f32; 3], depth: [f32; 3]) -> Model {
    Model {
        airbrakes: [None, None],
        node_vertex_ranges: Vec::new(),
        lod_groups: Default::default(),
        label: "nova sun mask triangle".into(),
        indices: vec![0, 1, 2],
        draws: vec![DrawCall {
            writes_depth: false,
            moving: false,
            blend: None,
            blend_state: None,
            depth_point: None,
            layer: oag_vex::vex::LAYER_DEFAULT,
            culled: false,
            range: 0..3,
            texture: Some(0),
            bounds: Bounds {
                centre: [0.0, 0.0, -5.0],
                radius: 7.0,
            },
            node: None,
            chunk: None,
            alpha_test_ref: None,
        }],
        alpha_tested_draws: Vec::new(),
        transparent_draws: Vec::new(),
        textures: vec![Some(texture("white albedo", [255, 255, 255, 255]))],
        lightmaps: Vec::new(),
        pad_masks: Vec::new(),
        wave_maps: Vec::new(),
        fifth_maps: Vec::new(),
        sixth_maps: Vec::new(),
        material_slots: Vec::new(),
        material_specular_exponent: Vec::new(),
        material_glow: Vec::new(),
        material_colour_factor: Vec::new(),
        material_flip_v: Vec::new(),
        material_quake: Vec::new(),
        material_tangent: Vec::new(),
        material_variants: Vec::new(),
        material_anim: Vec::new(),
        shine_draws: Vec::new(),
        caustic: oag_mesh::mesh::caustic_pass::Caustic::default(),
        vertex_colour_is_light: false,
        stamps_glow: false,
        glow_by_texel: false,
        glow_from_program: false,
        flame: None,
        absorb_shell: false,
        shield_shell: false,
        shield_lit: false,
        own_shield: false,
        material_lod_bias: 0.0,
        material_state: Vec::new(),
        per_batch_cull: false,
        sky_reflection: None,
        alpha_test_ref: None,
        centre: [0.0, 0.0, -5.0],
        radius: 7.0,
        mesh_count: 1,
        anim_tracks: Vec::new(),
        anim_nodes: Vec::new(),
        emissive: Vec::new(),
        vertices: vec![
            vertex([-3.0, -3.0, depth[0]], sun_mask[0]),
            vertex([3.0, -3.0, depth[1]], sun_mask[1]),
            vertex([0.0, 3.0, depth[2]], sun_mask[2]),
        ],
    }
}

fn adapter() -> Option<(wgpu::Device, wgpu::Queue)> {
    let instance = wgpu::Instance::default();
    let Some(adapter) = oag_gpu::test_adapter::request(&instance) else {
        eprintln!("no GPU adapter: skipping");
        return None;
    };
    Some(
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("requesting the device"),
    )
}

/// A white sun straight onto the triangle and nothing else: a surface shows its sun term
/// through the mask, and an occluded one shows black.
fn rig() -> Scene {
    let mut scene = Scene::off();
    scene.light = Light::authored([0.0, 0.0, 1.0], [1.0; 3], [0.0; 3], [1.0; 3], [1.0; 3], 0.0);
    scene.fog.camera = [0.0, 0.0, 0.0];
    scene
}

/// The whole target, row by row, of a perspective-tilted triangle whose vertices carry
/// `sun_mask`; an uncovered pixel is the clear colour, pure green. The far corner is 80 deep
/// against 4 for the near two: the perspective division is where a varying stops coming out
/// bit-exact.
fn frame(sun_mask: [f32; 3]) -> Option<Vec<[u8; 4]>> {
    let (device, queue) = adapter()?;
    Some(draw(
        &device,
        &queue,
        &model(sun_mask, [-4.0, -4.5, -80.0]),
        rig(),
    ))
}

fn is_covered(p: &[u8; 4]) -> bool {
    p[0] != 0 || p[1] != 255
}

/// Two vertices at `2 + 0` and one at `2 + 1/255`: the surface is occluded to within a byte, so the
/// sun term is at most one byte of mask, which the gamma encode lifts to about 20 levels. Before
/// the thresholds moved off the boundary, pixels whose interpolated mask rounded just under
/// `2.0` read as a plain mask of about 2 and drew the sun at full strength (255).
#[test]
fn a_mostly_occluded_nova_surface_never_draws_the_sun_at_full_strength() {
    let Some(pixels) = frame([2.0, 2.0, 2.0 + 1.0 / 255.0]) else {
        return;
    };
    let covered: Vec<&[u8; 4]> = pixels.iter().filter(|p| is_covered(p)).collect();
    assert!(
        covered.len() > 1000,
        "the triangle covered {} pixels",
        covered.len()
    );
    let brightest = covered.iter().map(|p| p[0]).max().unwrap();
    assert!(
        brightest <= 24,
        "a surface occluded to within a byte reads {brightest} somewhere: the speckle"
    );
}

/// A perspective camera at the origin looking down -z, so the triangle's three corners sit at
/// different depths and the interpolation is perspective-correct: that division is where a
/// constant varying stops coming out bit-exact.
fn uniforms() -> Vec<u8> {
    let identity: [[f32; 4]; 4] = [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ];
    let (near, far) = (0.1_f32, 100.0_f32);
    let projection: [[f32; 4]; 4] = [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, far / (near - far), -1.0],
        [0.0, 0.0, near * far / (near - far), 0.0],
    ];
    let mut bytes = Vec::new();
    bytes.extend_from_slice(bytemuck::cast_slice(&projection)); // view_projection
    bytes.extend_from_slice(bytemuck::cast_slice(&identity)); // model
    bytes.extend_from_slice(&[0u8; 32]); // the four pad floats, then the environment-probe layer and its padding
    bytes.extend_from_slice(&[0u8; 32]); // hull point lights
    bytes.extend_from_slice(bytemuck::cast_slice(&identity)); // prev_mvp
    assert_eq!(bytes.len() as u64, UNIFORMS_SIZE);
    bytes
}

/// Draws the model once into a `SIZE`x`SIZE` target and reads every pixel back; an uncovered
/// pixel is the clear colour, pure green.
fn draw(device: &wgpu::Device, queue: &wgpu::Queue, model: &Model, scene: Scene) -> Vec<[u8; 4]> {
    let built = mesh_render::build(
        device,
        queue,
        model,
        FORMAT,
        Anisotropy::Off,
        1,
        mesh_render::Depth::Scene,
        mesh_render::TRANSPARENT_BLEND,
        mesh_render::GlowMask::Protected,
        mesh_render::Velocity::None,
        &mesh_render::zone::StageArt::NONE,
        mesh_render::ShadowMaps::NONE,
        mesh_render::ShadowReceiver::Never,
    )
    .expect("building the mesh pipeline");
    queue.write_buffer(&built.fog_buffer, 0, bytemuck::bytes_of(&scene));

    let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("glow uniforms"),
        size: UNIFORMS_SIZE,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&uniform_buffer, 0, &uniforms());
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("glow uniforms"),
        layout: &built.pipeline.get_bind_group_layout(0),
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: uniform_buffer.as_entire_binding(),
        }],
    });

    let extent = wgpu::Extent3d {
        width: SIZE,
        height: SIZE,
        depth_or_array_layers: 1,
    };
    let make = |label, format, usage| {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage,
            view_formats: &[],
            texture_binding_view_dimension: None,
        })
    };
    let colour = make(
        "colour",
        FORMAT,
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
    );
    let depth = make(
        "depth",
        mesh_render::DEPTH_FORMAT,
        wgpu::TextureUsages::RENDER_ATTACHMENT,
    );
    let colour_view = colour.create_view(&Default::default());
    let depth_view = depth.create_view(&Default::default());

    let mut encoder = device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("glow"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &colour_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.0,
                        g: 1.0,
                        b: 0.0,
                        a: 0.5,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&built.pipeline);
        pass.set_bind_group(0, &bind_group, &[0]);
        // Slot 1: the model's own textures. Slot 0 is `build`'s white 1x1.
        pass.set_bind_group(1, &built.texture_binds[1], &[]);
        pass.set_bind_group(2, &built.fog_bind, &[0]);
        pass.set_bind_group(3, &built.anim_bind, &[]);
        pass.set_vertex_buffer(0, built.geometry.sole().slice(..));
        pass.set_index_buffer(built.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..3, 0, 0..1);
    }

    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("glow readback"),
        size: u64::from(SIZE * SIZE * 4),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        colour.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(SIZE * 4),
                rows_per_image: None,
            },
        },
        extent,
    );
    queue.submit(Some(encoder.finish()));
    readback.slice(..).map_async(wgpu::MapMode::Read, |_| {});
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("draining the queue");
    let mapped = readback.slice(..).get_mapped_range().expect("mapping");
    mapped.as_chunks::<4>().0.to_vec()
}
