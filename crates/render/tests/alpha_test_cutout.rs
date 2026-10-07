//! Draws one surface through the **cutout** pipeline and asserts which alphas
//! survive it.
//!
//! # Why a pixel and not the field
//!
//! `Model::alpha_test_ref` is a plain `Option<f32>` and every step between it
//! and a discarded fragment is a place it can be dropped silently:
//! `mesh_render::build` has to push it as a pipeline constant, the constant's
//! name has to match `mesh.wgsl`'s `override`, and the entry point has to
//! compare against the override rather than against the constant it defaults
//! to. Each of those failures leaves Wipeout HD's mode-2 surfaces testing at
//! `mesh.wgsl`'s own default of `0` - which keeps every texel above fully
//! transparent and so looks like a working cutout while reproducing nothing
//! the disc asked for.
//!
//! So this draws the same quad twice against the same two textures, once with
//! the disc's `0.5` and once with no reference at all, and asserts the two
//! disagree on the texel between them. See `mesh::rcs::cutout` for where the
//! `0.5` comes from and `docs/formats/rcsmodel.md` for the decode.
//!
//! Skips when there is no adapter, like `msaa_resolve.rs` and
//! `velocity_target.rs`.

use std::sync::Arc;

use oag_mesh::mesh::{Bounds, DrawCall, GpuVertex, Model, ModelTexture, Texels, slots};
use oag_mesh::mesh_render::{self, Anisotropy, Scene, UNIFORMS_SIZE};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const SIZE: u32 = 64;

/// The reference every `Transparency::Mode2` material on the HD disc authors.
const AUTHORED_REF: f32 = 0.5;

/// Either side of [`AUTHORED_REF`], and both well above the `0` a model with
/// no reference of its own falls back to.
const BELOW: u8 = 100;
const ABOVE: u8 = 200;

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
        // Emissive, so the light rig is the identity on the albedo and the
        // pixel measures the discard alone - the same reason
        // `zone_recolour.rs` uses it.
        slots: slots::DEFAULT | slots::EMISSIVE,
        specular_exponent: oag_mesh::mesh::DEFAULT_SPECULAR_EXPONENT,
        glow: 0.0,
        texcoord2: [0.0, 0.0],
    }
}

/// One white quad in [`Model::alpha_tested_draws`], with `reference` as the
/// model's own alpha test.
fn model(albedo: Arc<ModelTexture>, reference: Option<f32>) -> Model {
    Model {
        airbrakes: [None, None],
        node_vertex_ranges: Vec::new(),
        lod_groups: Default::default(),
        label: "alpha test cutout quad".into(),
        indices: vec![0, 1, 2],
        draws: Vec::new(),
        alpha_tested_draws: vec![DrawCall {
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
        transparent_draws: Vec::new(),
        textures: vec![Some(albedo)],
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
        alpha_test_ref: reference,
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

fn texture(alpha: u8) -> Arc<ModelTexture> {
    Arc::new(ModelTexture {
        label: format!("white, alpha {alpha}"),
        width: 1,
        height: 1,
        texels: Texels::Rgba8(vec![255, 255, 255, alpha]),
        mip_count: None,
    })
}

/// **The disc's own reference reaches the shader, and it is not the default.**
#[test]
fn the_authored_reference_cuts_where_the_material_says_and_not_where_the_default_does() {
    let instance = wgpu::Instance::default();
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&Default::default())) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("requesting the device");

    let drawn = |alpha: u8, reference: Option<f32>| {
        draw(&device, &queue, &model(texture(alpha), reference)) != [0, 0, 0, 0]
    };

    assert!(
        drawn(ABOVE, Some(AUTHORED_REF)),
        "alpha {ABOVE}/255 is above the authored 0.5 and must survive the test"
    );
    assert!(
        !drawn(BELOW, Some(AUTHORED_REF)),
        "alpha {BELOW}/255 is below the authored 0.5 and must be discarded - a \
         cutout that keeps it is testing against some other reference"
    );

    // The same texel with no authored reference. This is the assertion that
    // makes the pair discriminating: if the override never reached the
    // pipeline, both would answer the same here.
    assert!(
        drawn(BELOW, None),
        "a model with no reference of its own keeps the shader's own default \
         of 0, which alpha {BELOW}/255 clears"
    );
    assert!(
        drawn(ABOVE, None),
        "and so does one above it - the default discards the fully transparent \
         texel alone"
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

/// Draws `model`'s one cutout call into a `SIZE`x`SIZE` target cleared to
/// transparent black, and answers the centre texel - so a discarded fragment
/// reads as the clear and a kept one does not.
fn draw(device: &wgpu::Device, queue: &wgpu::Queue, model: &Model) -> [u8; 4] {
    let built = mesh_render::build(
        device,
        queue,
        model,
        FORMAT,
        Anisotropy::Off,
        1,
        mesh_render::Depth::Scene,
        mesh_render::TRANSPARENT_BLEND,
        // `Written`, so a kept fragment's alpha of 1.0 reaches the frame and
        // "drawn" and "discarded" differ in every channel rather than in three.
        mesh_render::GlowMask::Written,
        mesh_render::Velocity::None,
        &mesh_render::zone::StageArt::NONE,
        // No shadow map, no depth map and no receiver: this test draws one
        // model against nothing.
        mesh_render::ShadowMaps::NONE,
        mesh_render::ShadowReceiver::Never,
    )
    .expect("building the mesh pipeline");
    queue.write_buffer(&built.fog_buffer, 0, bytemuck::bytes_of(&Scene::off()));

    let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("cutout uniforms"),
        size: UNIFORMS_SIZE,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&uniform_buffer, 0, &uniforms());
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("cutout uniforms"),
        layout: &built.alpha_test_pipeline.get_bind_group_layout(0),
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
            label: Some("cutout"),
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
        pass.set_pipeline(&built.alpha_test_pipeline);
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
        label: Some("cutout readback"),
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
