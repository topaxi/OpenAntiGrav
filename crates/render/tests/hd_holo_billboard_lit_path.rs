//! Draws one holographic-billboard surface through the mesh pipeline on a
//! fixture needing no game content, and pins what `holo.wesl` does with the
//! combine `mesh::rcs::holo_billboard` reads off
//! `nr_billboardholographicscanlines.rcsmaterial`:
//!
//! - the colour is the **overlay** of `advert * layer` under the scanline
//!   texture: `2ab` where the base is below one half, `1 - 2(1 - a)(1 - b)`
//!   where it is not;
//! - the alpha is the scanline texture's blue channel;
//! - dropping the role bit draws the generic reading instead, which is a
//!   different colour on the same textures.
//!
//! `hd_holo_billboard_ground_truth.rs` checks a real circuit reaches this
//! shader. Skips when there is no adapter. Runs in `just test`.

use std::sync::Arc;

use oag_mesh::mesh::{Bounds, DrawCall, Emissive, GpuVertex, Model, ModelTexture, Texels, slots};
use oag_mesh::mesh_render::{self, Anisotropy, Scene, UNIFORMS_SIZE};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const SIZE: u32 = 64;
const GLOW_SLOT: u32 = 1 << slots::MATERIAL_SHIFT;

fn vertex(position: [f32; 3], layer_uv: [f32; 2], role: u32) -> GpuVertex {
    GpuVertex {
        position,
        normal: [0.0, 0.0, 1.0],
        colour: [1.0, 1.0, 1.0, 1.0],
        texcoord: [0.0, 0.0],
        lightmap_texcoord: layer_uv,
        lit: 1.0,
        anim: 0,
        xform: 0,
        sun_mask: 1.0,
        slots: slots::DEFAULT | slots::EMISSIVE | GLOW_SLOT | role,
        specular_exponent: oag_mesh::mesh::DEFAULT_SPECULAR_EXPONENT,
        glow: 0.0,
        texcoord2: [0.0, 0.0],
    }
}

fn texture(label: &str, rgb: [u8; 3]) -> Arc<ModelTexture> {
    Arc::new(ModelTexture {
        label: label.into(),
        width: 1,
        height: 1,
        texels: Texels::Rgba8(vec![rgb[0], rgb[1], rgb[2], 255]),
        mip_count: None,
    })
}

/// A billboard quad: the advert is the albedo, the scanlines the second
/// texture and the layer the third.
fn model(advert: u8, scanlines: [u8; 3], layer: u8, role: u32) -> Model {
    let tint = [0.25, -5.625, 1.0];
    model_with(advert, scanlines, texture("layer", [layer; 3]), (tint, [0.0; 2]), role)
}

/// The general fixture: any layer texture, the tint the material path writes
/// (drift, signed scroll, scale) and the layer coordinate the vertices carry.
fn model_with(
    advert: u8,
    scanlines: [u8; 3],
    layer: Arc<ModelTexture>,
    (tint, layer_uv): ([f32; 3], [f32; 2]),
    role: u32,
) -> Model {
    Model {
        airbrakes: [None, None],
        node_vertex_ranges: Vec::new(),
        lod_groups: Default::default(),
        label: "hd holographic billboard quad".into(),
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
        textures: vec![Some(texture("advert", [advert; 3]))],
        lightmaps: vec![Some(texture("scanlines", scanlines))],
        pad_masks: vec![Some(layer)],
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
        centre: [0.0, 0.0, 0.5],
        radius: 2.0,
        mesh_count: 1,
        anim_tracks: Vec::new(),
        anim_nodes: Vec::new(),
        emissive: vec![Emissive {
            tint,
            offset: 0.0,
            scale: 1.0,
            rate: 0.0,
        }],
        vertices: vec![
            vertex([-0.9, -0.9, 0.5], layer_uv, role),
            vertex([0.9, -0.9, 0.5], layer_uv, role),
            vertex([0.0, 0.9, 0.5], layer_uv, role),
        ],
    }
}

#[test]
fn the_billboard_is_an_overlay_of_the_advert_layer_product_under_the_scanlines() {
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

    // The scanline texel is (0.5, 0.5, 1): its blue is the alpha, so the quad
    // is opaque over the black target and the red channel is the colour.
    let lines = [128, 128, 255];

    // The fixture's target is a gamma one, so the colour is encoded.
    let encoded = |v: f32| (v.powf(1.0 / 2.2) * 255.0).round() as i32;

    // advert 0.5 * layer 0.5 = 0.25, below one half: `2 * 0.25 * 0.5` = 0.25.
    let low = at(&model(128, lines, 128, slots::HOLO_BILLBOARD));
    assert!(
        (i32::from(low[0]) - encoded(0.252)).abs() <= 3,
        "below one half the overlay multiplies: {low:?}"
    );

    // advert 1 * layer 0.8 = 0.8, above: `1 - 2 * 0.2 * 0.498` = 0.80.
    let high = at(&model(255, lines, 204, slots::HOLO_BILLBOARD));
    assert!(
        (i32::from(high[0]) - encoded(0.8008)).abs() <= 3,
        "above one half the overlay screens: {high:?}"
    );

    // The scanline texture's blue is 1: `1 - 2 * (1 - 0.8) * 0` = 1.
    let blue = at(&model(255, lines, 204, slots::HOLO_BILLBOARD));
    assert!(
        i32::from(blue[2]) >= 252,
        "the screen branch saturates where the scanline layer is full: {blue:?}"
    );

    // Without the role bit the same textures draw as the generic reading,
    // which does not combine them this way.
    let generic = at(&model(128, lines, 128, 0));
    assert_ne!(low, generic, "the role bit is what selects the combine");
}

/// A two-texel-wide layer: dark on the left half, bright on the right.
fn halves() -> Arc<ModelTexture> {
    Arc::new(ModelTexture {
        label: "layer halves".into(),
        width: 2,
        height: 1,
        texels: Texels::Rgba8(vec![51, 51, 51, 255, 204, 204, 204, 255]),
        mip_count: None,
    })
}

#[test]
fn the_tints_scale_moves_the_layer_coordinate_and_unity_leaves_it() {
    let instance = wgpu::Instance::default();
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&Default::default())) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("requesting the device");
    let at = |scale: f32| {
        let mut scene = Scene::off();
        scene.light.enabled = 1.0;
        scene.fog.camera = [0.0, 0.0, 100.5];
        // The layer coordinate (0.2, 0.5) sits in the dark half; times 3 it is
        // (0.6, 1.5), in the bright half. The scanlines are (0.5, 0.5, 1).
        let m = model_with(
            255,
            [128, 128, 255],
            halves(),
            ([0.0, 0.0, scale], [0.2, 0.5]),
            slots::HOLO_BILLBOARD,
        );
        draw(&device, &queue, &m, scene)
    };
    let (kept, scaled) = (at(1.0), at(3.0));
    assert!(
        scaled[0] > kept[0] + 20,
        "scale 3 samples the bright half, scale 1 the dark: {kept:?} vs {scaled:?}"
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
