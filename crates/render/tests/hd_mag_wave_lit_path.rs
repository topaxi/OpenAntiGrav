//! Draws one magstrip-wave surface through the mesh pipeline and asserts the
//! wave moves with `scene.time`, on a fixture needing no game content.
//!
//! `hd_mag_wave_ground_truth.rs` checks the real circuit builds the bindings;
//! this one checks `mesh.wgsl` reads them: the emissive picture dodged by a
//! wave sampled at `uv * k + time`, `d = e / (1 - lerp(e, wave, e.a) * Colour)`
//! (`mesh::rcs::mag_wave`). The wave texture is four texels across, three black
//! and one white, so the clock sweeps the white texel across the sampled point
//! and the dodge swings from `e` to its clamp.
//!
//! Skips when there is no adapter. Runs in `just test`.

use std::sync::Arc;

use oag_mesh::mesh::{Bounds, DrawCall, Emissive, GpuVertex, Model, ModelTexture, Texels, slots};
use oag_mesh::mesh_render::{self, Anisotropy, Scene, UNIFORMS_SIZE};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const SIZE: u32 = 64;

/// `Colour`: only green takes the dodge.
const TINT: [f32; 3] = [0.0, 1.0, 0.0];
const GLOW_SLOT: u32 = 1 << slots::MATERIAL_SHIFT;

fn vertex(position: [f32; 3], wave: bool) -> GpuVertex {
    GpuVertex {
        position,
        normal: [0.0, 0.0, 1.0],
        colour: [1.0, 1.0, 1.0, 1.0],
        texcoord: [0.0, 0.0],
        lightmap_texcoord: [0.0, 0.0],
        lit: 1.0,
        anim: 0,
        xform: 0,
        sun_mask: 1.0,
        slots: slots::DEFAULT
            | slots::EMISSIVE
            | GLOW_SLOT
            | if wave { slots::MAG_WAVE } else { 0 },
        specular_exponent: oag_mesh::mesh::DEFAULT_SPECULAR_EXPONENT,
        glow: 0.0,
    }
}

fn texture(label: &str, width: u32, rgba: Vec<u8>) -> Arc<ModelTexture> {
    Arc::new(ModelTexture {
        label: label.into(),
        width,
        height: 1,
        texels: Texels::Rgba8(rgba),
        mip_count: None,
    })
}

fn model(wave: bool) -> Model {
    Model {
        airbrakes: [None, None],
        node_vertex_ranges: Vec::new(),
        lod_groups: Default::default(),
        label: "hd mag wave quad".into(),
        indices: vec![0, 1, 2],
        draws: vec![DrawCall {
            moving: false,
            blend: None,
            blend_state: None,
            layer: oag_vex::vex::LAYER_DEFAULT,
            culled: false,
            range: 0..3,
            texture: Some(0),
            bounds: Bounds {
                centre: [0.0, 0.0, 0.5],
                radius: 1.5,
            },
            node: None,
            chunk: None,
            alpha_test_ref: None,
        }],
        alpha_tested_draws: Vec::new(),
        transparent_draws: Vec::new(),
        textures: vec![Some(texture("black albedo", 1, vec![0, 0, 0, 255]))],
        lightmaps: Vec::new(),
        pad_masks: vec![Some(texture("emissive", 1, vec![128, 128, 128, 255]))],
        wave_maps: vec![Some(texture(
            "wave",
            4,
            vec![0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 255, 255],
        ))],
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
        centre: [0.0, 0.0, 0.5],
        radius: 2.0,
        mesh_count: 1,
        anim_tracks: Vec::new(),
        anim_nodes: Vec::new(),
        emissive: vec![Emissive {
            tint: TINT,
            offset: 0.0,
            scale: 1.0,
            rate: 1.0,
        }],
        vertices: vec![
            vertex([-0.9, -0.9, 0.5], wave),
            vertex([0.9, -0.9, 0.5], wave),
            vertex([0.0, 0.9, 0.5], wave),
        ],
    }
}

#[test]
fn the_wave_moves_with_the_clock_and_repeats_each_second() {
    let instance = wgpu::Instance::default();
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&Default::default())) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("requesting the device");

    let at = |model: &Model, seconds: f32| {
        let mut scene = Scene::off();
        scene.light.enabled = 1.0;
        scene.fog.camera = [0.0, 0.0, 100.5];
        scene.time = [seconds; 4];
        draw(&device, &queue, model, scene)
    };
    // Texel 0's centre is at 0.125 and the white texel 3's at 0.875.
    let dark = at(&model(true), 0.125);
    let bright = at(&model(true), 0.875);
    assert!(
        bright[1] > dark[1],
        "the wave's white texel must brighten the dodge: {dark:?} then {bright:?}"
    );
    assert_eq!(
        dark[0], bright[0],
        "red is not in Colour, so the wave leaves it at the emissive picture"
    );
    assert!(
        dark[0] > 0,
        "the emissive picture reaches the pixel: {dark:?}"
    );
    assert_eq!(
        at(&model(true), 1.125),
        dark,
        "the wave repeats once a second: the clock plus one is the same pixel"
    );
    let still = at(&model(false), 0.875);
    assert_eq!(
        (still[0], still[1], still[2]),
        (0, 0, 0),
        "without the wave bit the surface is its black albedo"
    );
}

/// **A Zone race draws the strip as the plain Zone surface.** Every `ZoneMode`
/// variant of the wave materials declares no wave, emissive, clock or ramp, so
/// with `scene.zone.enabled` set the wave bit must change nothing: the pixel
/// equals the same surface without the bit, and does not move with the clock.
/// Without the gate the wave escapes the grade, which is what a Zone race
/// showed on Talon's Junction (the strip drew as it does in Time Trial).
#[test]
fn a_zone_race_draws_the_strip_without_its_wave() {
    let instance = wgpu::Instance::default();
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&Default::default())) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("requesting the device");

    let at = |model: &Model, seconds: f32, zone: bool| {
        let mut scene = Scene::off();
        scene.light.enabled = 1.0;
        scene.fog.camera = [0.0, 0.0, 100.5];
        scene.time = [seconds; 4];
        if zone {
            scene.zone.enabled = 1.0;
        }
        draw(&device, &queue, model, scene)
    };
    let off_zone = at(&model(true), 0.875, false);
    assert!(
        off_zone[1] > 0,
        "the fixture's wave must be visible off Zone: {off_zone:?}"
    );
    let plain = at(&model(false), 0.875, true);
    for seconds in [0.125, 0.875, 1.125] {
        assert_eq!(
            at(&model(true), seconds, true),
            plain,
            "in Zone the wave bit must not reach the pixel (t = {seconds})"
        );
    }
}

/// Identity camera and model, so clip space is model space.
fn uniforms() -> Vec<u8> {
    let identity: [[f32; 4]; 4] = [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ];
    let mut bytes = Vec::new();
    bytes.extend_from_slice(bytemuck::cast_slice(&identity)); // view_projection
    bytes.extend_from_slice(bytemuck::cast_slice(&identity)); // model
    bytes.extend_from_slice(&[0u8; 16]); // the four pad floats
    bytes.extend_from_slice(bytemuck::cast_slice(&identity)); // prev_mvp
    assert_eq!(bytes.len() as u64, UNIFORMS_SIZE);
    bytes
}

/// Draws the model once into a `SIZE`x`SIZE` target and reads the centre back.
fn draw(device: &wgpu::Device, queue: &wgpu::Queue, model: &Model, scene: Scene) -> [u8; 4] {
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
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
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
        pass.set_bind_group(0, &bind_group, &[]);
        // Slot 1: the model's own textures. Slot 0 is `build`'s white 1x1.
        pass.set_bind_group(1, &built.texture_binds[1], &[]);
        pass.set_bind_group(2, &built.fog_bind, &[]);
        pass.set_bind_group(3, &built.anim_bind, &[]);
        pass.set_vertex_buffer(0, built.vertex_buffer.slice(..));
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
    // Dead centre of the triangle.
    let at = (((SIZE / 2) * SIZE + SIZE / 2) * 4) as usize;
    [mapped[at], mapped[at + 1], mapped[at + 2], mapped[at + 3]]
}
