//! Draws one lit surface through the mesh pipeline with Wipeout HD's SPU
//! vertex-light list bound, and asserts what `spu_light_sum` leaves.
//!
//! The formula under test is `EdgeGeom`'s, read off the SPU job
//! (`docs/ghidra/functions/ps3-hdfury-eu/renderer.md`, "`EdgeGeom`'s light
//! path is read"): per light within range, `max(0, 1 - |d| / D)^w *
//! max(0, N . L) * colour`, summed per vertex and added to the pre-albedo
//! diffuse sum. Rather than pin one encoded pixel, which would also pin the
//! target's transfer curve, this asserts the properties that distinguish the
//! formula from its neighbours: no list draws black under a zeroed rig; a
//! light in front of the surface draws its own hue, red over green over blue;
//! a light at `D` contributes nothing at all (the falloff is a hard zero, not
//! a tail); a light behind the surface is cut by `N . L`; a longer range is
//! brighter and a higher exponent dimmer; and the array is read to `count`,
//! not to its end.
//!
//! Skips when there is no adapter, like `hd_emissive_lit_path.rs`. Needs no
//! game content, so it runs in `just test`.

use std::sync::Arc;

use oag_mesh::mesh::{Bounds, DrawCall, GpuVertex, Model, ModelTexture, Texels, slots};
use oag_mesh::mesh_render::{self, Anisotropy, Light, Scene, SpuLight, SpuLights, UNIFORMS_SIZE};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const SIZE: u32 = 64;

/// A Fury engine at rest, scaled down so the sum stays inside the target:
/// the `1 : 0.25 : 0.1` ratio is the literal's own.
const COLOUR: [f32; 3] = [0.4, 0.1, 0.04];

fn vertex(position: [f32; 3]) -> GpuVertex {
    GpuVertex {
        position,
        // Facing the camera, which is what a light on `+Z` shines on.
        normal: [0.0, 0.0, 1.0],
        colour: [1.0, 1.0, 1.0, 1.0],
        texcoord: [0.0, 0.0],
        lightmap_texcoord: [0.0, 0.0],
        // The authored path - every HD track chunk.
        lit: 1.0,
        anim: 0,
        xform: 0,
        sun_mask: 1.0,
        // `NO_AMBIENT`, so the constant ambient does not join the sum and a
        // black rig leaves the SPU term alone; not `NO_SUN`, so the chunk is
        // not `EMISSIVE`, which would replace the sum with `1.0` outright.
        slots: slots::DEFAULT | slots::NO_AMBIENT,
        specular_exponent: oag_mesh::mesh::DEFAULT_SPECULAR_EXPONENT,
        glow: 0.0,
        texcoord2: [0.0, 0.0],
    }
}

fn texture(label: &str, rgba: [u8; 4]) -> Arc<ModelTexture> {
    Arc::new(ModelTexture {
        label: label.into(),
        width: 1,
        height: 1,
        texels: Texels::Rgba8(rgba.to_vec()),
        mip_count: None,
    })
}

/// One triangle at `z = 0.5`, a white albedo, no second texture.
fn model() -> Model {
    Model {
        airbrakes: [None, None],
        node_vertex_ranges: Vec::new(),
        lod_groups: Default::default(),
        label: "hd spu vertex light quad".into(),
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
        textures: vec![Some(texture("white albedo", [255, 255, 255, 255]))],
        lightmaps: vec![None],
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
        emissive: Vec::new(),
        vertices: vec![
            vertex([-0.9, -0.9, 0.5]),
            vertex([0.9, -0.9, 0.5]),
            vertex([0.0, 0.9, 0.5]),
        ],
    }
}

/// A black authored rig: no sun, no ambient, the prelit curve at the
/// identity over a black placeholder. Everything the pixel shows is the SPU
/// term.
fn black_rig() -> Scene {
    let mut scene = Scene::off();
    scene.light = Light::authored([0.0, 0.0, 1.0], [0.0; 3], [0.0; 3], [1.0; 3], [1.0; 3], 0.0);
    // Dead in front and far away, so the eye-dependent terms vanish - see
    // `zone_recolour.rs` for what `Fog::off`'s origin does to a quad at
    // `z = 0.5` otherwise.
    scene.fog.camera = [0.0, 0.0, 100.5];
    scene
}

fn light(position: [f32; 3], range: f32) -> SpuLight {
    SpuLight {
        position: [position[0], position[1], position[2], 1.0],
        colour: [COLOUR[0], COLOUR[1], COLOUR[2], range],
    }
}

#[test]
fn the_spu_light_sum_is_edgegeoms_formula() {
    let instance = wgpu::Instance::default();
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&Default::default())) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("requesting the device");
    let model = model();

    // No list: a black rig draws black.
    let none = draw(&device, &queue, &model, black_rig());
    assert_eq!(
        (none[0], none[1], none[2]),
        (0, 0, 0),
        "no lights, black rig"
    );

    // One light half a range in front of the surface: its own hue arrives,
    // `1 : 0.25 : 0.1` in linear light - red over green over blue, none of
    // them black. Per vertex, as the original: the three corners each see
    // their own distance and `N . L`, and the centre pixel is their blend.
    let mut lit = black_rig();
    lit.spu_lights = SpuLights::from_slice(&[light([0.0, 0.0, 1.5], 2.0)]);
    let front = draw(&device, &queue, &model, lit);
    assert!(
        front[0] > front[1] && front[1] > front[2] && front[2] > 0,
        "a light in front should draw its hue, red over green over blue: {front:?}"
    );

    // The same light exactly `D` away: the falloff is a hard zero.
    let mut edge = black_rig();
    edge.spu_lights = SpuLights::from_slice(&[light([0.0, 0.0, 2.5], 2.0)]);
    let at_range = draw(&device, &queue, &model, edge);
    assert_eq!(
        (at_range[0], at_range[1], at_range[2]),
        (0, 0, 0),
        "a light at its own range contributes nothing"
    );

    // Behind the surface: cut by `N . L`.
    let mut behind = black_rig();
    behind.spu_lights = SpuLights::from_slice(&[light([0.0, 0.0, -0.5], 2.0)]);
    let back = draw(&device, &queue, &model, behind);
    assert_eq!(
        (back[0], back[1], back[2]),
        (0, 0, 0),
        "a light behind the surface is cut by N.L"
    );

    // A longer range is brighter: `1 - |d| / D` grows with `D`.
    let mut wide = black_rig();
    wide.spu_lights = SpuLights::from_slice(&[light([0.0, 0.0, 1.5], 4.0)]);
    let wider = draw(&device, &queue, &model, wide);
    assert!(
        wider[0] > front[0],
        "twice the range should be brighter: {wider:?} vs {front:?}"
    );

    // A higher exponent is dimmer: `att < 1`, so `att^2 < att`.
    let mut steep = black_rig();
    let mut record = light([0.0, 0.0, 1.5], 2.0);
    record.position[3] = 2.0;
    steep.spu_lights = SpuLights::from_slice(&[record]);
    let steeper = draw(&device, &queue, &model, steep);
    assert!(
        steeper[0] < front[0] && steeper[0] > 0,
        "w = 2 should be dimmer than w = 1 and still lit: {steeper:?} vs {front:?}"
    );

    // Only `count` records are read: a second record past the count is
    // ignored even though it sits in the array.
    let mut counted = black_rig();
    counted.spu_lights = SpuLights::from_slice(&[light([0.0, 0.0, 2.5], 2.0)]);
    counted.spu_lights.lights[1] = light([0.0, 0.0, 1.5], 2.0);
    let bounded = draw(&device, &queue, &model, counted);
    assert_eq!(
        (bounded[0], bounded[1], bounded[2]),
        (0, 0, 0),
        "a record past `count` must not be summed"
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
