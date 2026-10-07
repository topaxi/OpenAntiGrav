//! Draws one triangle through the `Velocity::Write` mesh pipeline with a
//! displaced `prev_mvp`, reads the velocity attachment back, and asserts the
//! sign and magnitude of what landed.
//!
//! The single most likely bug in a velocity encoding is an invisible one: a
//! flipped y (NDC y is up, uv y is down) changes no final image until the
//! reconstruction filter smears everything the wrong way vertically -
//! `docs/rendering/motion-blur.md` calls this test out by name. Skips when
//! there is no adapter, like `msaa_resolve.rs`.

use oag_gpu::formats::VELOCITY_FORMAT;
use oag_mesh::mesh::{Bounds, DrawCall, GpuVertex, Model};
use oag_mesh::mesh_render::{self, Anisotropy, UNIFORMS_SIZE};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const SIZE: u32 = 64;

fn vertex(position: [f32; 3]) -> GpuVertex {
    GpuVertex {
        position,
        normal: [0.0, 0.0, 1.0],
        colour: [1.0, 1.0, 1.0, 1.0],
        texcoord: [0.0, 0.0],
        lightmap_texcoord: [0.0, 0.0],
        lit: 0.0,
        anim: 0,
        xform: 0,
        sun_mask: 1.0,
        slots: oag_mesh::mesh::slots::DEFAULT,
        specular_exponent: oag_mesh::mesh::DEFAULT_SPECULAR_EXPONENT,
        glow: 0.0,
        texcoord2: [0.0, 0.0],
    }
}

/// A triangle covering the middle of the frame under an identity camera:
/// clip space *is* model space, so the uniforms below choose the NDC motion
/// directly.
fn triangle_model() -> Model {
    Model {
        airbrakes: [None, None],
        node_vertex_ranges: Vec::new(),
        lod_groups: Default::default(),
        label: "velocity_target test triangle".into(),
        indices: vec![0, 1, 2],
        draws: vec![DrawCall {
            moving: false,
            blend: None,
            blend_state: None,
            layer: oag_vex::vex::LAYER_DEFAULT,
            culled: false,
            range: 0..3,
            texture: None,
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
        textures: Vec::new(),
        lightmaps: Vec::new(),
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
            vertex([-0.8, -0.8, 0.5]),
            vertex([0.8, -0.8, 0.5]),
            vertex([0.0, 0.8, 0.5]),
        ],
    }
}

/// The uniform block `mesh.wgsl` reads, laid out by hand: identity camera
/// and model, and a `prev_mvp` that claims every point sat `+0.5` NDC to the
/// right and `+0.5` NDC up one tick ago.
fn uniforms() -> Vec<u8> {
    let identity: [[f32; 4]; 4] = [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ];
    let mut prev = identity;
    prev[3][0] = 0.5;
    prev[3][1] = 0.5;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(bytemuck::cast_slice(&identity)); // view_projection
    bytes.extend_from_slice(bytemuck::cast_slice(&identity)); // model
    bytes.extend_from_slice(&[0u8; 16]); // the four pad floats
    bytes.extend_from_slice(bytemuck::cast_slice(&prev)); // prev_mvp
    assert_eq!(bytes.len() as u64, UNIFORMS_SIZE);
    bytes
}

fn half_to_f32(bits: u16) -> f32 {
    let sign = if bits & 0x8000 != 0 { -1.0f32 } else { 1.0 };
    let exponent = ((bits >> 10) & 0x1f) as i32;
    let mantissa = (bits & 0x3ff) as f32;
    match exponent {
        0 => sign * mantissa * (2.0f32).powi(-24),
        31 => sign * f32::INFINITY,
        _ => sign * (1.0 + mantissa / 1024.0) * (2.0f32).powi(exponent - 15),
    }
}

/// The pipeline writes `(cur - prev) * (0.5, -0.5)` in uv units. With the
/// previous pose `+0.5` NDC right and up, that is `x = -0.25` (it moved
/// left in uv) and `y = +0.25` (NDC up is uv *down*, so having been higher
/// means it moved down the screen). The y sign is the whole point.
#[test]
fn the_velocity_target_carries_the_ndc_delta_halved_and_y_flipped() {
    let instance = wgpu::Instance::default();
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&Default::default())) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("requesting the device");

    let model = triangle_model();
    let mesh_render::Built {
        pipeline,
        vertex_buffer,
        index_buffer,
        texture_binds,
        fog_bind,
        anim_bind,
        ..
    } = mesh_render::build(
        &device,
        &queue,
        &model,
        FORMAT,
        Anisotropy::Off,
        1,
        mesh_render::Depth::Scene,
        mesh_render::TRANSPARENT_BLEND,
        mesh_render::GlowMask::Protected,
        mesh_render::Velocity::Write,
        // No Zone stage: this test draws a model, not a race.
        &mesh_render::zone::StageArt::NONE,
        // No shadow map, no depth map and no receiver: this test draws one
        // model against nothing.
        mesh_render::ShadowMaps::NONE,
        mesh_render::ShadowReceiver::Never,
    )
    .expect("building the mesh pipeline");

    let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("velocity uniforms"),
        size: UNIFORMS_SIZE,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&uniform_buffer, 0, &uniforms());
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("velocity uniforms"),
        layout: &pipeline.get_bind_group_layout(0),
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
    let texture = |label, format, usage| {
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
    let colour = texture("colour", FORMAT, wgpu::TextureUsages::RENDER_ATTACHMENT);
    let velocity = texture(
        "velocity",
        VELOCITY_FORMAT,
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
    );
    let depth = texture(
        "depth",
        mesh_render::DEPTH_FORMAT,
        wgpu::TextureUsages::RENDER_ATTACHMENT,
    );
    let colour_view = colour.create_view(&Default::default());
    let velocity_view = velocity.create_view(&Default::default());
    let depth_view = depth.create_view(&Default::default());

    let mut encoder = device.create_command_encoder(&Default::default());
    {
        let attachment = |view| {
            Some(wgpu::RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })
        };
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("velocity"),
            color_attachments: &[attachment(&colour_view), attachment(&velocity_view)],
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
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.set_bind_group(1, &texture_binds[0], &[]);
        pass.set_bind_group(2, &fog_bind, &[]);
        pass.set_bind_group(3, &anim_bind, &[]);
        pass.set_vertex_buffer(0, vertex_buffer.slice(..));
        pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..3, 0, 0..1);
    }

    // Rg16Float is four bytes a texel, so a 64-wide row is 256 bytes -
    // exactly the copy alignment.
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("velocity readback"),
        size: u64::from(SIZE * SIZE * 4),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        velocity.as_image_copy(),
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
    let texel = |x: u32, y: u32| {
        let at = ((y * SIZE + x) * 4) as usize;
        let r = half_to_f32(u16::from_le_bytes([mapped[at], mapped[at + 1]]));
        let g = half_to_f32(u16::from_le_bytes([mapped[at + 2], mapped[at + 3]]));
        (r, g)
    };

    // Dead centre of the triangle.
    let (vx, vy) = texel(SIZE / 2, SIZE / 2);
    assert!(
        (vx + 0.25).abs() < 0.01,
        "uv x velocity: expected -0.25, got {vx}"
    );
    assert!(
        (vy - 0.25).abs() < 0.01,
        "uv y velocity: expected +0.25 (the y flip), got {vy}"
    );
    // A corner the triangle never covers keeps the cleared zero.
    let (cx, cy) = texel(1, 1);
    assert_eq!((cx, cy), (0.0, 0.0), "the clear leaked velocity");
}
