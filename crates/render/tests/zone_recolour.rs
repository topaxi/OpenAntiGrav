//! Draws one surface through the mesh pipeline with Wipeout HD's Zone
//! parameters bound, reads the frame back, and asserts the pixel.
//!
//! # Why a pixel and not the uniform
//!
//! `crates/game/tests/zone_grade_ground_truth.rs` asserts the *assembly* of
//! `zoneColourTint.xy` and `zoneEffect.rgb` off the disc. Nothing there
//! reaches the GPU, so a wiring error - the stage texture on the wrong
//! binding, `zone_sample` unreachable, the uniform's `zone` field at a
//! different offset in Rust than in WGSL - passes every one of those tests
//! while drawing nothing.
//!
//! It also pins the **colour space**, which is the failure this test exists
//! for. `mesh.wgsl` decodes its samples with `pow(texel, 2.2)` and shades in
//! linear light; `zoneTex` is a texture and wants that decode, `zoneEffect` is
//! a shader parameter and does not. Summing before the decode instead of after
//! computes `pow(zoneTex * zoneEffect, 2.2)` and distorts every stage colour -
//! silently, because the two agree exactly when `zoneEffect` is `1.0`. The
//! effect below is deliberately `2.0` on red so the two answers are 175 and
//! 255.
//!
//! Skips when there is no adapter, like `msaa_resolve.rs` and
//! `velocity_target.rs`.

use std::sync::Arc;

use oag_render::mesh::{Bounds, DrawCall, GpuVertex, Model, ModelTexture, Texels, slots};
use oag_render::mesh_render::{self, Anisotropy, Scene, UNIFORMS_SIZE, Zone};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const SIZE: u32 = 64;

/// The zone texture's one texel, and the multiplier bound beside it.
const ZONE_TEXEL: u8 = 128;
const ZONE_EFFECT: [f32; 4] = [2.0, 1.0, 0.0, 0.0];

fn vertex(position: [f32; 3], lit: f32) -> GpuVertex {
    GpuVertex {
        position,
        normal: [0.0, 0.0, 1.0],
        colour: [1.0, 1.0, 1.0, 1.0],
        texcoord: [0.0, 0.0],
        lightmap_texcoord: [0.0, 0.0],
        // `1.0` takes the authored path, which shades in linear light; `0.0`
        // takes the stand-in path, which is gamma throughout.
        lit,
        anim: 0,
        xform: 0,
        sun_mask: 1.0,
        // **Emissive**, so `authored` is exactly `vec3(1.0)` and the specular
        // is switched off: the light rig is then the identity and the pixel
        // measures the Zone term and the sRGB round trip alone. That is the
        // material's own declaration in the original too - a program fed
        // neither a constant ambient nor a directional light.
        slots: slots::DEFAULT | slots::EMISSIVE,
    }
}

fn model(albedo: Arc<ModelTexture>, lit: f32) -> Model {
    Model {
        airbrakes: [None, None],
        node_vertex_ranges: Vec::new(),
        label: "zone recolour test quad".into(),
        indices: vec![0, 1, 2],
        draws: vec![DrawCall {
            moving: false,
            blend: None,
            blend_state: None,
            layer: oag_formats::vex::LAYER_DEFAULT,
            culled: false,
            range: 0..3,
            texture: Some(0),
            bounds: Bounds {
                centre: [0.0, 0.0, 0.5],
                radius: 1.5,
            },
            node: None,
            chunk: None,
        }],
        alpha_tested_draws: Vec::new(),
        transparent_draws: Vec::new(),
        textures: vec![Some(albedo)],
        lightmaps: vec![None],
        material_slots: Vec::new(),
        material_variants: Vec::new(),
        vertex_colour_is_light: false,
        flame: None,
        centre: [0.0, 0.0, 0.5],
        radius: 2.0,
        mesh_count: 1,
        anim_tracks: Vec::new(),
        anim_nodes: Vec::new(),
        vertices: vec![
            vertex([-0.9, -0.9, 0.5], lit),
            vertex([0.9, -0.9, 0.5], lit),
            vertex([0.0, 0.9, 0.5], lit),
        ],
    }
}

fn texture(label: &str, rgba: [u8; 4]) -> Arc<ModelTexture> {
    Arc::new(ModelTexture {
        label: label.into(),
        width: 1,
        height: 1,
        texels: Texels::Rgba8(rgba.to_vec()),
    })
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

/// What `mesh.wgsl` must produce for one channel: the zone sample decoded,
/// scaled by the parameter, then encoded back.
fn expected(effect: f32) -> u8 {
    let sample = f32::from(ZONE_TEXEL) / 255.0;
    let linear = (sample.powf(2.2) * effect).clamp(0.0, 1.0);
    (linear.powf(1.0 / 2.2) * 255.0).round() as u8
}

/// **The Zone recolour reaches the frame, in the right colour space, and only
/// where the albedo is black.**
#[test]
fn the_zone_recolour_paints_a_black_albedo_and_leaves_a_white_one() {
    let instance = wgpu::Instance::default();
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&Default::default())) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("requesting the device");

    // A pure-black diffuse, which is exactly what the original's artists paint
    // where they want Zone mode to light a surface up.
    let black = model(texture("black albedo", [0, 0, 0, 255]), 1.0);
    let stage = texture("zone stage", [ZONE_TEXEL, ZONE_TEXEL, ZONE_TEXEL, 255]);

    let mut scene = Scene::off();
    // The authored rig, switched on so the `lit` vertices take it. `Emissive`
    // above makes its own terms the identity.
    scene.light.enabled = 1.0;
    scene.zone = Zone {
        // `zoneColourTint.xy`. `1.0` on both lanes is what
        // `zonemode.effectsettings` authors.
        uv_scale: [1.0, 1.0],
        enabled: 1.0,
        _pad: 0.0,
        effect: ZONE_EFFECT,
    };

    // Black albedo: the recolour lands.
    let painted = draw(&device, &queue, &black, Some(&stage), scene);
    let (r, g, b) = (expected(ZONE_EFFECT[0]), expected(ZONE_EFFECT[1]), 0u8);
    assert_eq!(
        (painted[0], painted[1], painted[2]),
        (r, g, b),
        "the Zone term must be summed in linear light: summing before the \
         sRGB decode would clip red to 255 instead of {r}"
    );
    // The whole point of the constants: the two answers differ.
    assert_ne!(
        painted[0], 255,
        "red saturated - the sum is in the wrong space"
    );

    // A white albedo: `blackMask` is 1, so the recolour adds nothing and the
    // surface draws exactly as it would outside Zone mode.
    let white = model(texture("white albedo", [255, 255, 255, 255]), 1.0);
    let untouched = draw(&device, &queue, &white, Some(&stage), scene);
    assert_eq!(
        (untouched[0], untouched[1], untouched[2]),
        (255, 255, 255),
        "the recolour must land only where the albedo is black"
    );

    // **The stand-in path, which is gamma throughout.** Prelit geometry
    // (`lit == 0`) takes it even under an authored rig, because its light is
    // baked into its vertex colours in the space the asset authors - so the
    // Zone term is summed there *undecoded*. Green is the discriminating
    // channel: `128` is the raw sample, and `56` is what a linear summand
    // would have left.
    let prelit = model(texture("black albedo", [0, 0, 0, 255]), 0.0);
    let gamma = draw(&device, &queue, &prelit, Some(&stage), scene);
    let raw =
        |effect: f32| ((f32::from(ZONE_TEXEL) / 255.0 * effect).min(1.0) * 255.0).round() as u8;
    assert_eq!(
        (gamma[0], gamma[1], gamma[2]),
        (raw(ZONE_EFFECT[0]), raw(ZONE_EFFECT[1]), 0),
        "the stand-in path must sum in gamma, not fold in a linear term"
    );

    // The same black surface with the Zone term switched off draws black,
    // which is what every draw outside a Zone race gets.
    let off = draw(&device, &queue, &black, None, Scene::off());
    assert_eq!(
        (off[0], off[1], off[2]),
        (0, 0, 0),
        "no Zone stage must add nothing at all"
    );
}

/// Draws `model` once into a `SIZE`x`SIZE` target and answers the centre texel.
fn draw(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    model: &Model,
    stage: Option<&Arc<ModelTexture>>,
    scene: Scene,
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
        stage,
    )
    .expect("building the mesh pipeline");
    queue.write_buffer(&built.fog_buffer, 0, bytemuck::bytes_of(&scene));

    let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("zone uniforms"),
        size: UNIFORMS_SIZE,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&uniform_buffer, 0, &uniforms());
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("zone uniforms"),
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
            label: Some("zone"),
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
        // Slot 1: the model's own first texture. Slot 0 is `build`'s white 1x1.
        pass.set_bind_group(1, &built.texture_binds[1], &[]);
        pass.set_bind_group(2, &built.fog_bind, &[]);
        pass.set_bind_group(3, &built.anim_bind, &[]);
        pass.set_vertex_buffer(0, built.vertex_buffer.slice(..));
        pass.set_index_buffer(built.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..3, 0, 0..1);
    }

    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("zone readback"),
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
