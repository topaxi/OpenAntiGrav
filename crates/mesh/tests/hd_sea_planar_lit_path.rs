//! HD's sea (`reflectplane_dc_seawater`, water kind 1) reads the planar reflection target
//! through the mesh pipeline, on a fixture needing no game content (`docs/rendering/water.md`,
//! "The planar reflection, drawn"): with `scene.sea_reflection.y` set it adds
//! `Reflectivity * R`, `R` the target's stored value at the point's own screen position; with it
//! clear the target is not read and the sky stand-in takes its place.
//!
//! The target is a flat colour bound in the behind-the-glass slot; the camera and model are the
//! identity, so the triangle's point projects to the target's centre through `scene.refraction`.
//! Drop the sample, the `Reflectivity` weight or the flag and one of the readings below moves.
//!
//! Skips when there is no adapter. Runs in `just test`.

use std::sync::Arc;

use oag_mesh::mesh::rcs::Tap;
use oag_mesh::mesh::{
    Bounds, DrawCall, Emissive, GpuVertex, Model, ModelTexture, PROGRAM_WATER, Texels, slots,
};
use oag_mesh::mesh_render::{self, Anisotropy, Scene, UNIFORMS_SIZE};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const SIZE: u32 = 64;
const ENTRY: u32 = 1 << slots::MATERIAL_SHIFT;
/// The sea's kind (`program[1]`, `mesh::rcs::water`).
const SEA_KIND: f32 = 1.0;
const TANGENT: u32 = u32::from_le_bytes([255, 128, 128, 255]);

fn vertex(position: [f32; 3]) -> GpuVertex {
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
        texcoord2: [0.0, 0.0],
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
    }))
}

/// A sea triangle: the entry `sea_block` lays (`ColourConstant` 0.2, `Reflectivity`
/// `reflectivity`, no specular colour), a flat `.ag` normal map bound third.
fn model(reflectivity: f32) -> Model {
    let mut model = Model::none("hd sea planar lit-path triangle");
    model.indices = vec![0, 1, 2];
    model.draws = vec![DrawCall {
        writes_depth: true,
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
    model.textures = vec![texture("sea colour", [128, 128, 128, 255])];
    model.lightmaps = vec![None];
    model.pad_masks = vec![texture("normal map", [128, 128, 128, 128])];
    model.vertices = vec![
        vertex([-0.9, -0.9, 0.5]),
        vertex([0.9, -0.9, 0.5]),
        vertex([0.0, 0.9, 0.5]),
    ];
    let entry = |tap: &Tap| Emissive {
        tint: [tap.scale[0], tap.scale[1], tap.scroll[0]],
        offset: tap.scroll[1],
        scale: tap.weight,
        rate: 0.0,
        program: [0.0, 0.0],
    };
    let [first, second] = Tap::sea();
    model.emissive = vec![
        Emissive {
            tint: [0.2; 3],
            offset: reflectivity,
            scale: 250.0,
            rate: 0.0,
            program: [PROGRAM_WATER, SEA_KIND],
        },
        entry(&first),
        entry(&second),
    ];
    model
}

/// A 4x4 target of one colour, as the planar reflection the sea reads.
fn target(device: &wgpu::Device, queue: &wgpu::Queue, value: u8) -> wgpu::TextureView {
    let extent = wgpu::Extent3d {
        width: 4,
        height: 4,
        depth_or_array_layers: 1,
    };
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("planar reflection"),
        size: extent,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
        texture_binding_view_dimension: None,
    });
    queue.write_texture(
        texture.as_image_copy(),
        &[value, value, value, 255].repeat(16),
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(16),
            rows_per_image: None,
        },
        extent,
    );
    texture.create_view(&Default::default())
}

/// The device and the scene: the sun down the triangle's normal, the target bound through the
/// identity (`refraction`) with `distortion` 40, which a flat normal pushes along `z` alone.
fn stage() -> Option<(wgpu::Device, wgpu::Queue, Scene)> {
    let instance = wgpu::Instance::default();
    let Some(adapter) = oag_gpu::test_adapter::request(&instance) else {
        eprintln!("no GPU adapter: skipping");
        return None;
    };
    let (device, queue) = pollster::block_on(
        adapter.request_device(&mesh_render::device_descriptor("hd sea planar", &adapter)),
    )
    .expect("requesting the device");
    let mut scene = Scene::off();
    scene.light =
        mesh_render::Light::authored([0.0, 0.0, 1.0], [0.2; 3], [0.1; 3], [1.0; 3], [1.0; 3], 0.0);
    scene.fog.camera = [0.0, 0.0, 100.5];
    scene.sea_reflection = [40.0, 1.0, 1.0, 1.0];
    Some((device, queue, scene))
}

/// Linear light from a gamma target's byte.
fn light(byte: u8) -> f32 {
    (f32::from(byte) / 255.0).powf(2.2)
}

#[test]
fn the_sea_adds_reflectivity_times_the_stored_target() {
    let Some((device, queue, scene)) = stage() else {
        return;
    };
    let (black, white) = (target(&device, &queue, 0), target(&device, &queue, 255));
    let at = |reflectivity, scene, target| {
        light(draw(&device, &queue, &model(reflectivity), scene, target)[0])
    };
    // A white target adds `Reflectivity * 1` of light over a black one.
    let gain = at(0.3, scene, &white) - at(0.3, scene, &black);
    assert!((gain - 0.3).abs() < 0.03, "Reflectivity 0.3 adds {gain}");
    // No reflectivity, no reflection.
    let none = at(0.0, scene, &white) - at(0.0, scene, &black);
    assert!(none.abs() < 0.005, "Reflectivity 0 adds {none}");
    // A grey target is read as stored: byte 128 is `128 / 255` of the white one's gain.
    let grey = target(&device, &queue, 128);
    let half = at(0.3, scene, &grey) - at(0.3, scene, &black);
    assert!(
        (half - 0.3 * 128.0 / 255.0).abs() < 0.03,
        "the stored value: {half}"
    );
}

#[test]
fn without_the_flag_the_target_is_not_read() {
    let Some((device, queue, mut scene)) = stage() else {
        return;
    };
    scene.sea_reflection = [0.0; 4];
    let (black, white) = (target(&device, &queue, 0), target(&device, &queue, 255));
    let at = |target| draw(&device, &queue, &model(0.3), scene, target);
    assert_eq!(
        at(&black),
        at(&white),
        "the stand-in reads the sky, not the target"
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

/// Draws the model once into a `SIZE`x`SIZE` target, with `target` bound as the planar
/// reflection, and reads the centre back.
fn draw(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    model: &Model,
    scene: Scene,
    target: &wgpu::TextureView,
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
        mesh_render::ShadowMaps {
            behind_glass: Some(target),
            ..mesh_render::ShadowMaps::NONE
        },
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
    // Dead centre of the triangle.
    let at = (((SIZE / 2) * SIZE + SIZE / 2) * 4) as usize;
    [mapped[at], mapped[at + 1], mapped[at + 2], mapped[at + 3]]
}
