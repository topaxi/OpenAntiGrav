//! A PSP `.vex` model samples the mip level the GE's slope rule names, off view
//! depth, and a model without the disc's own chains does not.
//!
//! # Why a pixel
//!
//! `mesh_render::build` decides from the model's textures alone
//! ([`Texels::Chain`]) whether to push the `texlod_slope` and `texlod_bias`
//! pipeline constants, and `mesh.wgsl` has to read them by those names and
//! take `textureSampleLevel` rather than `textureSample`. Any of those dropped
//! leaves the sampler on screen-space derivatives, which for a quad with one
//! constant texture coordinate is level 0 at every depth - so this draws one
//! quad against a three-level chain whose levels are red, green and blue, at
//! four view depths, and reads which level arrived.
//!
//! The law under test is `level = log2(|z| * 1/256) + 1`, clamped at 0 and at
//! the last level: depth 64 is level 0, 256 is level 1, 512 is level 2 and a
//! depth far past the chain stays on level 2. See
//! `mesh_render::PSP_TEXLOD_SLOPE` for where the two constants come from.
//!
//! Skips when there is no adapter.

use std::sync::Arc;

use oag_render::mesh::{Bounds, DrawCall, GpuVertex, Model, ModelTexture, Texels, slots};
use oag_render::mesh_render::{self, Anisotropy, Scene, UNIFORMS_SIZE};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const SIZE: u32 = 64;

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
        specular_exponent: oag_render::mesh::DEFAULT_SPECULAR_EXPONENT,
        glow: 0.0,
    }
}

/// One quad in [`Model::draws`], scaled by `depth` so it still covers the
/// centre pixel once the projection divides by it.
fn model(albedo: Arc<ModelTexture>, depth: f32) -> Model {
    Model {
        airbrakes: [None, None],
        node_vertex_ranges: Vec::new(),
        lod_groups: Default::default(),
        label: "slope lod quad".into(),
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
                radius: 1.5 * depth,
            },
            node: None,
            chunk: None,
            alpha_test_ref: None,
        }],
        alpha_tested_draws: Vec::new(),
        transparent_draws: Vec::new(),
        textures: vec![Some(albedo)],
        lightmaps: vec![None],
        material_slots: Vec::new(),
        material_specular_exponent: Vec::new(),
        material_variants: Vec::new(),
        material_anim: Vec::new(),
        vertex_colour_is_light: false,
        stamps_glow: false,
        flame: None,
        absorb_shell: false,
        alpha_test_ref: None,
        centre: [0.0, 0.0, 0.5],
        radius: 2.0 * depth,
        mesh_count: 1,
        anim_tracks: Vec::new(),
        anim_nodes: Vec::new(),
        emissive: Vec::new(),
        vertices: vec![
            vertex([-0.9 * depth, -0.9 * depth, 0.5]),
            vertex([0.9 * depth, -0.9 * depth, 0.5]),
            vertex([0.0, 0.9 * depth, 0.5]),
        ],
    }
}

/// Levels of `4x4`, `2x2` and `1x1`: red, green, blue.
fn levels() -> Vec<Vec<u8>> {
    [
        (16, [255, 0, 0, 255]),
        (4, [0, 255, 0, 255]),
        (1, [0, 0, 255, 255]),
    ]
    .into_iter()
    .map(|(texels, rgba)| rgba.repeat(texels))
    .collect()
}

fn chain() -> Arc<ModelTexture> {
    Arc::new(ModelTexture::chain(
        "red, green, blue".into(),
        4,
        4,
        levels(),
    ))
}

/// The same base level with no chain of the disc's - the other title's shape.
fn plain() -> Arc<ModelTexture> {
    Arc::new(ModelTexture {
        label: "red, base only".into(),
        width: 4,
        height: 4,
        texels: Texels::Rgba8(levels().remove(0)),
        mip_count: None,
    })
}

/// Which level answered at each depth, as the dominant channel of the centre
/// texel: 0 red, 1 green, 2 blue.
fn level_at(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: Arc<ModelTexture>,
    depth: f32,
) -> usize {
    let texel = draw(device, queue, &model(texture, depth), depth);
    (0..3).max_by_key(|&channel| texel[channel]).unwrap()
}

#[test]
fn a_chain_is_sampled_at_the_level_the_slope_law_names() {
    let instance = wgpu::Instance::default();
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&Default::default())) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("requesting the device");

    for (depth, level) in [
        (64.0, 0),
        (128.0, 0),
        (256.0, 1),
        (512.0, 2),
        (100_000.0, 2),
    ] {
        assert_eq!(
            level_at(&device, &queue, chain(), depth),
            level,
            "view depth {depth}: log2({depth} / 256) + 1 clamped to the three levels"
        );
    }
    // The discriminating half: the same quad with no chain of the disc's stays
    // on its one level at every depth, so the levels above were the rule's.
    for depth in [64.0, 512.0, 100_000.0] {
        assert_eq!(
            level_at(&device, &queue, plain(), depth),
            0,
            "a model without Texels::Chain keeps the sampler's own selection"
        );
    }
}

/// Identity camera and model, so clip space is model space.
fn uniforms(depth: f32) -> Vec<u8> {
    // Column 3 is (0, 0, 0, depth), so `clip.w` - the view depth the shader reads - is `depth`.
    let projection: [[f32; 4]; 4] = [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, depth],
    ];
    let identity: [[f32; 4]; 4] = [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ];
    let mut bytes = Vec::new();
    bytes.extend_from_slice(bytemuck::cast_slice(&projection)); // view_projection
    bytes.extend_from_slice(bytemuck::cast_slice(&identity)); // model
    bytes.extend_from_slice(&[0u8; 16]); // the four pad floats
    bytes.extend_from_slice(bytemuck::cast_slice(&identity)); // prev_mvp
    assert_eq!(bytes.len() as u64, UNIFORMS_SIZE);
    bytes
}

/// Draws `model`'s one opaque call into a `SIZE`x`SIZE` target cleared to
/// transparent black, and answers the centre texel - so a discarded fragment
/// reads as the clear and a kept one does not.
fn draw(device: &wgpu::Device, queue: &wgpu::Queue, model: &Model, depth: f32) -> [u8; 4] {
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
        label: Some("slope uniforms"),
        size: UNIFORMS_SIZE,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&uniform_buffer, 0, &uniforms(depth));
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("slope uniforms"),
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
            label: Some("slope"),
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
        label: Some("slope readback"),
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
