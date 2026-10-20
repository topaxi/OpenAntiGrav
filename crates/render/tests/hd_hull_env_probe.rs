//! Draws one hull triangle through the mesh pipeline with the environment-probe array bound
//! and asserts what the probe does to the picture: Wipeout HD's hull is lit from
//! `4 * probe[n]` where it names a craft, and adds `4 * probe(reflect(-V, N)) * alpha *
//! reflectivity` where its material is tagged.
//!
//! Dropping any of the wiring turns a case red: the array not bound, the layer index not
//! `2 * craft`, the uniform field not read, the reflection's glow-table tag ignored.
//! Skips when there is no adapter, like `hd_emissive_lit_path.rs`. Needs no game content.

use std::sync::Arc;

use oag_mesh::mesh::{
    Bounds, DrawCall, Emissive, GpuVertex, Model, ModelTexture, REFLECTION_RATE, Texels, slots,
};
use oag_mesh::mesh_render::{
    self, Anisotropy, ENV_PROBE_FORMAT, ENV_PROBE_LAYERS, ENV_PROBE_SIZE, Light, Scene, ShadowMaps,
    UNIFORMS_SIZE,
};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const SIZE: u32 = 64;

fn vertex(position: [f32; 3], slots: u32, normal: [f32; 3]) -> GpuVertex {
    GpuVertex {
        quake: 0.0,
        position,
        normal,
        colour: [1.0, 1.0, 1.0, 1.0],
        texcoord: [0.0, 0.0],
        lightmap_texcoord: [0.0, 0.0],
        lit: 1.0,
        anim: 0,
        xform: 0,
        sun_mask: 1.0,
        slots,
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
fn model(slots: u32, emissive: Vec<Emissive>, normal: [f32; 3]) -> Model {
    Model {
        airbrakes: [None, None],
        node_vertex_ranges: Vec::new(),
        lod_groups: Default::default(),
        label: "hd hull environment probe triangle".into(),
        indices: vec![0, 1, 2],
        draws: vec![DrawCall {
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
        transparent_draws: Vec::new(),
        textures: vec![Some(texture("white albedo", [255, 255, 255, 255]))],
        lightmaps: vec![None],
        pad_masks: Vec::new(),
        wave_maps: Vec::new(),
        material_slots: Vec::new(),
        material_specular_exponent: Vec::new(),
        material_colour_factor: Vec::new(),
        material_quake: Vec::new(),
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
        emissive,
        vertices: vec![
            vertex([-0.9, -0.9, 0.5], slots, normal),
            vertex([0.9, -0.9, 0.5], slots, normal),
            vertex([0.0, 0.9, 0.5], slots, normal),
        ],
    }
}

/// An authored rig with no sun and a dim constant ambient: what a hull draws without a
/// probe is that ambient alone.
fn rig() -> Scene {
    let mut scene = Scene::off();
    scene.light = Light::authored([0.0, 0.0, 1.0], [0.0; 3], [0.1; 3], [1.0; 3], [1.0; 3], 0.0);
    scene.fog.camera = [0.0, 0.0, 100.5];
    scene
}

/// The array with `fill(layer)` written into every layer.
fn array(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    fill: impl Fn(u32, u32, u32) -> [u8; 4],
) -> wgpu::TextureView {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("environment probes"),
        size: wgpu::Extent3d {
            width: ENV_PROBE_SIZE.0,
            height: ENV_PROBE_SIZE.1,
            depth_or_array_layers: ENV_PROBE_LAYERS,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: ENV_PROBE_FORMAT,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    for layer in 0..ENV_PROBE_LAYERS {
        let texels: Vec<u8> = (0..ENV_PROBE_SIZE.0 * ENV_PROBE_SIZE.1)
            .flat_map(|at| fill(layer, at % ENV_PROBE_SIZE.0, at / ENV_PROBE_SIZE.0))
            .collect();
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: 0,
                    y: 0,
                    z: layer,
                },
                aspect: wgpu::TextureAspect::All,
            },
            &texels,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(ENV_PROBE_SIZE.0 * 4),
                rows_per_image: Some(ENV_PROBE_SIZE.1),
            },
            wgpu::Extent3d {
                width: ENV_PROBE_SIZE.0,
                height: ENV_PROBE_SIZE.1,
                depth_or_array_layers: 1,
            },
        );
    }
    texture.create_view(&wgpu::TextureViewDescriptor {
        dimension: Some(wgpu::TextureViewDimension::D2Array),
        ..Default::default()
    })
}

fn gpu() -> Option<(wgpu::Device, wgpu::Queue)> {
    let instance = wgpu::Instance::default();
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&Default::default())) else {
        eprintln!("no GPU adapter: skipping");
        return None;
    };
    Some(
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("requesting the device"),
    )
}

/// What the probe arrays hold. Craft 0's irradiance (layer 0) lives in the top-left 64x32
/// corner, as the engine writes it: a dim blue-grey over its top half, a dim brown over its
/// bottom half, and a loud red everywhere outside the corner, which a hull must never see. Its
/// reflection (layer 1) is a dim red throughout. Craft 1's two layers are black.
fn fill(layer: u32, x: u32, y: u32) -> [u8; 4] {
    match layer {
        0 if x >= 64 || y >= 32 => [200, 0, 0, 255],
        0 if y < 16 => [13, 26, 38, 255],
        0 => [38, 26, 13, 255],
        1 => [64, 0, 0, 255],
        _ => [0, 0, 0, 255],
    }
}

fn reflective() -> Vec<Emissive> {
    vec![Emissive {
        tint: [0.5, 0.0, 0.0],
        offset: 0.0,
        scale: 1.0,
        rate: REFLECTION_RATE,
    }]
}

const UP: [f32; 3] = [0.0, 1.0, 0.0];
const DOWN: [f32; 3] = [0.0, -1.0, 0.0];

#[test]
fn a_hull_is_lit_from_its_crafts_probe_and_reflects_where_tagged() {
    let Some((device, queue)) = gpu() else { return };
    let plain = |normal| model(slots::DEFAULT, Vec::new(), normal);
    let tagged = model(
        slots::DEFAULT | (1 << slots::MATERIAL_SHIFT),
        reflective(),
        UP,
    );
    let view = array(&device, &queue, fill);
    let maps = ShadowMaps {
        env_probes: Some(&view),
        ..ShadowMaps::NONE
    };

    // No craft named: the constant ambient, 0.1 of every channel, and nothing of the probe.
    let constant = draw(&device, &queue, &plain(UP), rig(), maps, 0.0);
    assert!(
        constant[0] > 0 && constant[0] == constant[1],
        "{constant:?}"
    );

    // Craft 0, facing up: the paraboloid's top row, `4 * (0.05, 0.1, 0.15)` of the blue-grey
    // half, so the hull takes its hue and is brighter than the ambient it replaces. A shader
    // that addressed the corner without halving its coordinate would read the loud red.
    let lit = draw(&device, &queue, &plain(UP), rig(), maps, 1.0);
    assert!(
        lit[2] > lit[1] && lit[1] > lit[0],
        "the probe's hue: {lit:?}"
    );
    assert!(lit[1] > constant[1], "{lit:?} against {constant:?}");

    // Facing down: the bottom row, the brown half. The normal's `y` is the vertical of the
    // map, so this is the `v = 0.5 - 0.5 n.y` law, and a flipped one swaps the two.
    let down = draw(&device, &queue, &plain(DOWN), rig(), maps, 1.0);
    assert!(
        down[0] > down[1] && down[1] > down[2],
        "the lower half's hue: {down:?}"
    );

    // Craft 1 owns layers 2 and 3, which are black: a hull that read craft 0's layer instead
    // would not go dark.
    let other = draw(&device, &queue, &plain(UP), rig(), maps, 2.0);
    assert_eq!((other[0], other[1], other[2]), (0, 0, 0), "{other:?}");

    // A tagged material adds the craft's reflection layer, red, weighted by the texel's alpha
    // and its reflectivity: red rises and the other channels stay where they were.
    let shiny = draw(&device, &queue, &tagged, rig(), maps, 1.0);
    assert!(shiny[0] > lit[0] + 40, "{shiny:?} against {lit:?}");
    assert_eq!(
        (shiny[1], shiny[2]),
        (lit[1], lit[2]),
        "only red is reflected"
    );

    // Tagged, but no craft named: no reflection either.
    let off = draw(&device, &queue, &tagged, rig(), maps, 0.0);
    assert_eq!(off, constant);
}

/// Identity camera and model, so clip space is model space.
fn uniforms(env_layer: f32) -> Vec<u8> {
    let identity: [[f32; 4]; 4] = [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ];
    let mut bytes = Vec::new();
    bytes.extend_from_slice(bytemuck::cast_slice(&identity)); // view_projection
    bytes.extend_from_slice(bytemuck::cast_slice(&identity)); // model
    bytes.extend_from_slice(&[0u8; 16]); // sun-occlusion layer, clock, colour, hull scale
    bytes.extend_from_slice(bytemuck::cast_slice(&[env_layer, 4.0, 0.0, 0.0]));
    bytes.extend_from_slice(&[0u8; 32]); // hull point lights
    bytes.extend_from_slice(bytemuck::cast_slice(&identity)); // prev_mvp
    assert_eq!(bytes.len() as u64, UNIFORMS_SIZE);
    bytes
}

/// Draws the model once into a `SIZE`x`SIZE` target and reads the centre back.
fn draw(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    model: &Model,
    scene: Scene,
    maps: ShadowMaps<'_>,
    env_layer: f32,
) -> [u8; 4] {
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
        maps,
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
    queue.write_buffer(&uniform_buffer, 0, &uniforms(env_layer));
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
