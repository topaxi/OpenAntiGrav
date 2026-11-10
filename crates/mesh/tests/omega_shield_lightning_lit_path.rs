//! Draws one `2048_shipshield_old` shell surface through the mesh pipeline and asserts
//! `shade.wesl` runs its Lightning layer, on a fixture needing no game content.
//!
//! `omega_2048_shield_colour_ground_truth.rs` checks the real shell builds the entry and the
//! binding from the program's own literals; this one checks the shader reads them
//! (`mesh::rcs::psp2::shield_lightning`): the diffuse colour is sampled at `u + A * time`
//! (the entry's tint `y`), and the third binding's colour times the weight (tint `x`) is added
//! on top. A surface whose entry is not the mode draws its first texture, still.
//!
//! Skips when there is no adapter. Runs in `just test`.

use std::sync::Arc;

use oag_mesh::mesh::{
    Bounds, DrawCall, Emissive, GpuVertex, Model, ModelTexture, PROGRAM_SHIELD_LIGHTNING, Texels,
    slots,
};
use oag_mesh::mesh_render::{self, Anisotropy, Scene, UNIFORMS_SIZE};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const SIZE: u32 = 64;
const ENTRY: u32 = 1 << slots::MATERIAL_SHIFT;
/// The program's own rate, `v_madmk_f32 .., 3.0, ..`.
const DIFFUSE_SCROLL: f32 = 3.0;

fn vertex(position: [f32; 3]) -> GpuVertex {
    GpuVertex {
        quake: 0.0,
        position,
        normal: [0.0, 0.0, 1.0],
        colour: [1.0, 1.0, 1.0, 1.0],
        texcoord: [0.25, 0.5],
        lightmap_texcoord: [0.5, 0.5],
        lit: 1.0,
        anim: 0,
        xform: 0,
        sun_mask: 1.0,
        slots: slots::DEFAULT | ENTRY,
        specular_exponent: oag_mesh::mesh::DEFAULT_SPECULAR_EXPONENT,
        glow: 0.0,
        texcoord2: [0.5, 0.5],
        tangent: 0,
    }
}

fn texture(label: &str, width: u32, texels: &[[u8; 4]]) -> Option<Arc<ModelTexture>> {
    Some(Arc::new(ModelTexture {
        label: label.into(),
        width,
        height: 1,
        texels: Texels::Rgba8(texels.concat()),
        mip_count: None,
    }))
}

/// A shell whose diffuse is a red texel beside a green one and whose lightning picture is
/// `lightning`; `mode` is the entry's program and `weight` its multiplier.
fn model(mode: f32, weight: f32, lightning: [u8; 4]) -> Model {
    Model {
        airbrakes: [None, None],
        node_vertex_ranges: Vec::new(),
        lod_groups: Default::default(),
        label: "omega shield shell quad".into(),
        indices: vec![0, 1, 2],
        draws: Vec::new(),
        transparent_draws: vec![DrawCall {
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
                centre: [0.0, 0.0, 0.5],
                radius: 1.5,
            },
            node: None,
            chunk: None,
            alpha_test_ref: None,
        }],
        alpha_tested_draws: Vec::new(),
        textures: vec![texture("diffuse", 2, &[[255, 0, 0, 255], [0, 255, 0, 255]])],
        lightmaps: Vec::new(),
        pad_masks: vec![texture("lightning", 1, &[lightning])],
        wave_maps: Vec::new(),
        fifth_maps: Vec::new(),
        sixth_maps: Vec::new(),
        material_slots: Vec::new(),
        material_specular_exponent: Vec::new(),
        material_glow: Vec::new(),
        material_colour_factor: Vec::new(),
        material_quake: Vec::new(),
        material_tangent: Vec::new(),
        material_variants: Vec::new(),
        material_anim: Vec::new(),
        shine_draws: Vec::new(),
        vertex_colour_is_light: false,
        stamps_glow: false,
        glow_by_texel: false,
        glow_from_program: false,
        flame: None,
        absorb_shell: false,
        shield_shell: false,
        own_shield: false,
        material_lod_bias: 0.0,
        material_state: Vec::new(),
        per_batch_cull: false,
        alpha_test_ref: None,
        centre: [0.0, 0.0, 0.5],
        radius: 2.0,
        mesh_count: 1,
        anim_tracks: Vec::new(),
        anim_nodes: Vec::new(),
        emissive: vec![Emissive {
            tint: [weight, DIFFUSE_SCROLL, 0.16],
            offset: 0.8,
            scale: 1.0,
            rate: 0.0,
            program: [mode, 0.0],
        }],
        vertices: vec![
            vertex([-0.9, -0.9, 0.5]),
            vertex([0.9, -0.9, 0.5]),
            vertex([0.0, 0.9, 0.5]),
        ],
    }
}

#[test]
fn the_diffuse_scrolls_at_the_programs_rate_and_the_lightning_is_added() {
    let instance = wgpu::Instance::default();
    let Some(adapter) = oag_gpu::test_adapter::request(&instance) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let (device, queue) = pollster::block_on(adapter.request_device(
        &mesh_render::device_descriptor("shield lightning", &adapter),
    ))
    .expect("requesting the device");
    let at = |model: &Model, seconds: f32| {
        let mut scene = Scene::off();
        scene.light.enabled = 1.0;
        scene.light.direction = [0.0, 0.0, 1.0];
        scene.light.sun = [0.6, 0.6, 0.6];
        scene.fog.camera = [0.0, 0.0, 100.5];
        scene.time = [seconds; 4];
        draw(&device, &queue, model, scene)
    };
    let off = [0, 0, 0, 0];
    // `u = 0.25` is the red texel; the program moves it by `3 * time`, so a sixth of a second
    // later it is on the green one. The blend list draws into a black target.
    let before = at(&model(PROGRAM_SHIELD_LIGHTNING, 0.0, off), 0.0);
    assert!(
        before[0] > 64 && before[1] < 8,
        "at time 0 the diffuse reads its red texel: {before:?}"
    );
    let after = at(
        &model(PROGRAM_SHIELD_LIGHTNING, 0.0, off),
        0.5 / DIFFUSE_SCROLL,
    );
    assert!(
        after[1] > 64 && after[0] < 8,
        "half a unit of `u` later (`0.5 / A` seconds) it reads the green one: {after:?}"
    );
    // The lightning picture is added, weighted by the entry's tint `x`; nothing else reads it.
    let blue = [0, 0, 255, 255];
    let weighted = at(&model(PROGRAM_SHIELD_LIGHTNING, 1.0, blue), 0.0);
    let unweighted = at(&model(PROGRAM_SHIELD_LIGHTNING, 0.0, blue), 0.0);
    assert!(
        weighted[2] > 64 && unweighted[2] < 8,
        "the weight scales the lightning term: {weighted:?} against {unweighted:?}"
    );
    // Without the mode the entry is inert: the first texel stays put whatever the clock says.
    let plain = at(&model(0.0, 1.0, blue), 0.5 / DIFFUSE_SCROLL);
    assert!(
        plain[0] > 64 && plain[1] < 8 && plain[2] < 8,
        "an entry that is not the mode scrolls nothing and adds nothing: {plain:?}"
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
    bytes.extend_from_slice(&[0u8; 32]); // the four pad floats, then the environment-probe layer and its padding
    bytes.extend_from_slice(&[0u8; 32]); // hull point lights
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
        pass.set_bind_group(0, &bind_group, &[0]);
        // Slot 1: the model's own textures. Slot 0 is `build`'s white 1x1.
        pass.set_bind_group(1, &built.texture_binds[1], &[]);
        pass.set_bind_group(2, &built.fog_bind, &[0]);
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
