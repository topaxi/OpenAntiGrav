//! 2048's blended `fc17` water edge through the mesh pipeline: `shade.wesl` takes the alpha chain
//! of its fragment program (`docs/rendering/water.md`, "2048 (Vita)", `@27` to `@29`) on a fixture
//! needing no game content.
//!
//! `alpha = vc.y * (Opacity - x) + x`, `x = a * vc.x`: `a` the normal map's alpha, `vc` the
//! vertex's `VertexColour1.xy` (the second coordinate pair), `Opacity` the entry's `offset`. The
//! triangle is drawn blended over a transparent target, so a pixel is the lit colour times the
//! alpha; the same colour drawn as kind 7 (opaque water, no chain) is the reference. Drop the map's
//! alpha, `vc.x`, `vc.y` or `Opacity` from the shader and one of the readings below moves.
//!
//! The colour is `light * Watercolour` with the uniform as it is (`@23`), as the `fc15` pond's is
//! `light * ColourConstant`: a second test draws two constants on each of the three kinds and
//! holds the picture to the ratio a raw multiply gives, which a constant decoded like a picture
//! misses.
//!
//! Skips when there is no adapter. Runs in `just test`.

use std::sync::Arc;

use oag_mesh::mesh::rcs::{Tap, VITA_EDGE_KIND, VITA_SEA_KIND, VITA_WATER_KIND};
use oag_mesh::mesh::{
    Bounds, DrawCall, Emissive, GpuVertex, Model, ModelTexture, PROGRAM_WATER, Texels, slots,
};
use oag_mesh::mesh_render::{self, Anisotropy, Scene, UNIFORMS_SIZE};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const SIZE: u32 = 64;
const ENTRY: u32 = 1 << slots::MATERIAL_SHIFT;
/// The first tap's coordinate is `(u, v + Speed t)` and the map is one texel, so any coordinate
/// reads it; the tangent is `+x`, which the frame needs to turn a flat map into the vertex normal.
const TANGENT: u32 = u32::from_le_bytes([255, 128, 128, 255]);

fn vertex(position: [f32; 3], chain: [f32; 2]) -> GpuVertex {
    GpuVertex {
        quake: 0.0,
        position,
        normal: [0.0, 0.0, 1.0],
        colour: [1.0, 1.0, 1.0, 1.0],
        texcoord: [0.5, 0.5],
        lightmap_texcoord: [0.5, 0.5],
        lit: 1.0,
        anim: 0,
        xform: 0,
        sun_mask: 1.0,
        slots: slots::DEFAULT | ENTRY,
        specular_exponent: oag_mesh::mesh::DEFAULT_SPECULAR_EXPONENT,
        glow: 0.0,
        texcoord2: chain,
        tangent: TANGENT,
        depth_bias: 0.0,
    }
}

fn texture(label: &str, texel: [u8; 4]) -> Option<Arc<ModelTexture>> {
    Some(Arc::new(ModelTexture {
        label: label.into(),
        width: 1,
        height: 1,
        texels: Texels::Rgba8(texel.to_vec()),
        mip_count: None,
        lod_code: 0.0,
    }))
}

/// A water triangle: glow entry `kind` with `Watercolour` `[colour; 3]` and `Opacity` `opacity`,
/// the normal map (flat, alpha `map_alpha`) bound third, `VertexColour1.xy` `chain` on every
/// vertex.
fn model(kind: f32, colour: f32, opacity: f32, map_alpha: u8, chain: [f32; 2]) -> Model {
    let mut model = Model::none("fc17 edge lit-path triangle");
    model.indices = vec![0, 1, 2];
    model.transparent_draws = vec![DrawCall {
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
    }];
    model.textures = vec![texture("watercolour", [128, 128, 128, 255])];
    model.lightmaps = vec![None];
    model.pad_masks = vec![texture("normal map", [128, 128, 255, map_alpha])];
    model.vertices = vec![
        vertex([-0.9, -0.9, 0.5], chain),
        vertex([0.9, -0.9, 0.5], chain),
        vertex([0.0, 0.9, 0.5], chain),
    ];
    // The entry `sea_block` lays: the header, then one per tap (`tint` the scale and the `u`
    // scroll, `offset` the `v` scroll, `scale` the weight, the specular colour in `program`).
    let [tap, none] = Tap::vita_water(0.05);
    let entry = |tap: &Tap, program: [f32; 2]| Emissive {
        tint: [tap.scale[0], tap.scale[1], tap.scroll[0]],
        offset: tap.scroll[1],
        scale: tap.weight,
        rate: 0.0,
        program,
    };
    let block = vec![
        Emissive {
            tint: [colour; 3],
            offset: opacity,
            scale: 250.0,
            rate: 0.0,
            program: [PROGRAM_WATER, kind],
        },
        entry(&tap, [0.0, 0.0]),
        entry(&none, [0.0, 0.0]),
    ];
    model.emissive = block;
    model
}

/// The device and the scene both tests draw under: the sun down the triangle's normal.
fn stage() -> Option<(wgpu::Device, wgpu::Queue, Scene)> {
    let instance = wgpu::Instance::default();
    let Some(adapter) = oag_gpu::test_adapter::request(&instance) else {
        eprintln!("no GPU adapter: skipping");
        return None;
    };
    let (device, queue) = pollster::block_on(
        adapter.request_device(&mesh_render::device_descriptor("fc17 edge", &adapter)),
    )
    .expect("requesting the device");
    let mut scene = Scene::off();
    scene.light =
        mesh_render::Light::authored([0.0, 0.0, 1.0], [0.4; 3], [0.2; 3], [1.0; 3], [1.0; 3], 0.0);
    scene.fog.camera = [0.0, 0.0, 100.5];
    Some((device, queue, scene))
}

/// `light * Watercolour` (and `fc15`'s `light * ColourConstant`, kind 6) takes the uniform as it
/// is, where a picture is decoded first: doubling the constant doubles the light the target
/// encodes, `2^(1/2.2)` in the picture. Decoded like a picture the constant would double the
/// picture itself. The map's alpha is zero, which takes the pond's highlight out of the sum.
#[test]
fn the_water_colour_multiplies_the_light_undecoded() {
    let Some((device, queue, scene)) = stage() else {
        return;
    };
    let at = |kind, colour| {
        let model = model(kind, colour, 1.0, 0, [0.0, 1.0]);
        f32::from(draw(&device, &queue, &model, scene)[0])
    };
    for kind in [VITA_SEA_KIND, VITA_WATER_KIND, VITA_EDGE_KIND] {
        let (dim, bright) = (at(kind, 0.2), at(kind, 0.4));
        assert!(dim > 40.0 && bright < 250.0, "kind {kind}: {dim}, {bright}");
        let ratio = bright / dim;
        assert!(
            (ratio - 2f32.powf(1.0 / 2.2)).abs() < 0.04,
            "kind {kind}: {dim} to {bright} is {ratio}, a raw multiply gives 1.37 and a decoded constant 2"
        );
    }
}

#[test]
fn the_edge_alpha_is_vc_y_times_opacity_minus_x_plus_x() {
    let Some((device, queue, scene)) = stage() else {
        return;
    };
    let at = |kind, opacity, map_alpha, chain| {
        draw(
            &device,
            &queue,
            &model(kind, 0.25, opacity, map_alpha, chain),
            scene,
        )
    };
    let reference = at(VITA_WATER_KIND, 1.0, 255, [0.0, 0.0])[0];
    assert!(reference > 40, "the water draws nothing: {reference}");
    let alpha = |opacity: f32, map_alpha: u8, chain: [f32; 2]| {
        f32::from(at(VITA_EDGE_KIND, opacity, map_alpha, chain)[0]) / f32::from(reference)
    };
    let near = |got: f32, want: f32, what: &str| {
        assert!(
            (got - want).abs() < 0.03,
            "{what}: alpha {got}, want {want}"
        );
    };
    // vc.y = 1 selects Opacity alone: the map and vc.x drop out.
    near(alpha(0.8, 255, [0.0, 1.0]), 0.8, "vc.y 1 reads Opacity");
    near(
        alpha(0.4, 0, [0.3, 1.0]),
        0.4,
        "vc.y 1 reads Opacity, map alpha 0",
    );
    // vc.y = 0 selects the map's alpha times vc.x.
    near(
        alpha(0.8, 128, [1.0, 0.0]),
        128.0 / 255.0,
        "vc.y 0 reads a * vc.x",
    );
    near(
        alpha(0.8, 255, [0.5, 0.0]),
        0.5,
        "vc.x scales the map's alpha",
    );
    near(
        alpha(0.8, 255, [0.0, 0.0]),
        0.0,
        "vc.x 0 and vc.y 0 is clear",
    );
    // Between, the lerp: 0.5 * (0.8 - 0.5) + 0.5 = 0.65.
    near(alpha(0.8, 255, [0.5, 0.5]), 0.65, "the lerp");
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
    // Dead centre of the triangle.
    let at = (((SIZE / 2) * SIZE + SIZE / 2) * 4) as usize;
    [mapped[at], mapped[at + 1], mapped[at + 2], mapped[at + 3]]
}
