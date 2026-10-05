//! Draws one light-cone surface through the mesh pipeline on a fixture needing
//! no game content, and pins what `mesh.wgsl` does with the combine
//! `mesh::rcs::light_cone` reads off `dc_lightcone.rcsmaterial`:
//!
//! - the colour is `noise * K`, **saturated** before it blends (Talon's
//!   Junction's `K` is 100);
//! - the alpha is `noise * s * ramp` where the variant's ramp tap executes and
//!   `noise * s * noise` where it is predicated away (`Emissive::rate` 1 and
//!   a zero normal `y`).
//!
//! `hd_light_cone_ground_truth.rs` checks a real circuit reaches this shader.
//! Skips when there is no adapter. Runs in `just test`.

use std::sync::Arc;

use oag_mesh::mesh::{Bounds, DrawCall, Emissive, GpuVertex, Model, ModelTexture, Texels, slots};
use oag_mesh::mesh_render::{self, Anisotropy, Scene, UNIFORMS_SIZE};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const SIZE: u32 = 64;
const GLOW_SLOT: u32 = 1 << slots::MATERIAL_SHIFT;

fn vertex(position: [f32; 3]) -> GpuVertex {
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
        slots: slots::DEFAULT | slots::EMISSIVE | GLOW_SLOT | slots::LIGHT_CONE,
        specular_exponent: oag_mesh::mesh::DEFAULT_SPECULAR_EXPONENT,
        glow: 0.0,
    }
}

fn texture(label: &str, value: u8) -> Arc<ModelTexture> {
    Arc::new(ModelTexture {
        label: label.into(),
        width: 1,
        height: 1,
        texels: Texels::Rgba8(vec![value, value, value, 255]),
        mip_count: None,
    })
}

/// A cone quad: `ramp` bound as the albedo, `noise` as the second texture.
fn model(noise: u8, ramp: u8, entry: Emissive) -> Model {
    Model {
        airbrakes: [None, None],
        node_vertex_ranges: Vec::new(),
        lod_groups: Default::default(),
        label: "hd light cone quad".into(),
        indices: vec![0, 1, 2],
        draws: Vec::new(),
        transparent_draws: vec![DrawCall {
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
        textures: vec![Some(texture("ramp", ramp))],
        lightmaps: vec![Some(texture("noise", noise))],
        pad_masks: Vec::new(),
        wave_maps: Vec::new(),
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
        emissive: vec![entry],
        vertices: vec![
            vertex([-0.9, -0.9, 0.5]),
            vertex([0.9, -0.9, 0.5]),
            vertex([0.0, 0.9, 0.5]),
        ],
    }
}

fn entry(k: f32, rate: f32) -> Emissive {
    Emissive {
        tint: [k; 3],
        offset: 0.0,
        scale: 1.0,
        rate,
    }
}

#[test]
fn the_cone_saturates_its_colour_and_skips_a_predicated_ramp() {
    let instance = wgpu::Instance::default();
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&Default::default())) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("requesting the device");
    let at = |model: &Model| {
        let mut scene = Scene::off();
        scene.light.enabled = 1.0;
        scene.fog.camera = [0.0, 0.0, 100.5];
        draw(&device, &queue, model, scene)
    };

    // The cone is drawn into a black target through the blend list, so the red
    // channel is `colour * alpha * 255`.
    //
    // Noise 0.5, ramp 1.0. K = 100 saturates the colour to 1, and the ramp tap
    // runs on the ungated variant, so alpha is 0.5 * 1.0 * 1.0: red is 128.
    let ungated = at(&model(128, 255, entry(100.0, 0.0)));
    assert!(
        (i32::from(ungated[0]) - 128).abs() <= 3,
        "saturated white at alpha noise * ramp = 0.5: {ungated:?}"
    );

    // The gated variant (the normal's y is 0) leaves the noise in the register
    // the ramp would have overwritten: alpha is noise * noise = 0.25, whatever
    // the ramp holds.
    let gated = at(&model(128, 255, entry(100.0, 1.0)));
    assert!(
        (i32::from(gated[0]) - 64).abs() <= 3,
        "a predicated-away ramp leaves alpha at noise^2 = 0.25: {gated:?}"
    );
    let gated_dark_ramp = at(&model(128, 0, entry(100.0, 1.0)));
    assert_eq!(gated, gated_dark_ramp, "the skipped ramp is never read");

    // K = 1 does not saturate: the colour is the noise's own 0.5, so the red is
    // 0.5 * alpha 0.5.
    let plain = at(&model(128, 255, entry(1.0, 0.0)));
    assert!(
        (i32::from(plain[0]) - 64).abs() <= 3,
        "K = 1 leaves the noise's mid grey: {plain:?}"
    );
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
        pass.set_pipeline(&built.blend_pipeline[0]);
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
