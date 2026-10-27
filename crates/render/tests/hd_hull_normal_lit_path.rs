//! HD's hull program under an authored rig: the `_n` map is read as a normal
//! through the authored tangent frame, and a flat map or a missing tangent is
//! the identity.
//!
//! The technique of `hd_pad_ne_lit_path.rs`: the authored rig is only reachable
//! under `scene.light.enabled`. Skips when there is no adapter and needs no
//! game content, so it runs in `just test`.
//!
//! One triangle in the `z = 0.5` plane facing the camera, vertex tangent `+x`
//! (`T`), so `B = cross(N, T) * w` is `+y` where `w = 1` and zero where
//! `w = 0`: the vertex program's own formula, with the tangent's fourth byte
//! as the factor.

use std::sync::Arc;

use oag_mesh::mesh::{Bounds, DrawCall, GpuVertex, Model, ModelTexture, Texels, slots};
use oag_mesh::mesh_render::{self, Anisotropy, Scene, UNIFORMS_SIZE};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const SIZE: u32 = 64;
/// A hull's slots word: the first texture's colour, no lightmap.
const HULL: u32 = slots::DEFAULT;

fn texture(label: &str, rgba: [u8; 4]) -> Arc<ModelTexture> {
    Arc::new(ModelTexture {
        label: label.into(),
        width: 1,
        height: 1,
        texels: Texels::Rgba8(rgba.to_vec()),
        mip_count: None,
    })
}

/// The authored tangent `+x`: `2b - 1` of `(255, 128, 128)` is `(1, 0.004, 0.004)`;
/// `w` is the byte that multiplies the binormal.
fn tangent(w: u8) -> u32 {
    u32::from_le_bytes([255, 128, 128, w])
}

fn vertex(position: [f32; 3], tangent: u32) -> GpuVertex {
    GpuVertex {
        quake: 0.0,
        position,
        normal: [0.0, 0.0, 1.0],
        colour: [0.0, 0.0, 0.0, 1.0],
        texcoord: [(position[0] + 0.9) / 1.8, (position[1] + 0.9) / 1.8],
        lightmap_texcoord: [0.5, 0.5],
        lit: 1.0,
        anim: 0,
        xform: 0,
        sun_mask: 1.0,
        slots: HULL,
        specular_exponent: oag_mesh::mesh::DEFAULT_SPECULAR_EXPONENT,
        glow: 0.0,
        texcoord2: [0.0, 0.0],
        tangent,
    }
}

/// The map bound third, and the tangent every vertex carries (`0` for none).
fn model(map: [u8; 4], tangent: u32) -> Model {
    let mut model = Model::none("hd hull normal lit-path triangle");
    model.indices = vec![0, 1, 2];
    model.draws = vec![DrawCall {
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
    model.textures = vec![Some(texture("albedo", [128, 128, 128, 255]))];
    model.lightmaps = vec![None];
    model.pad_masks = vec![Some(texture("_n", map))];
    model.vertices = vec![
        vertex([-0.9, -0.9, 0.5], tangent),
        vertex([0.9, -0.9, 0.5], tangent),
        vertex([0.0, 0.9, 0.5], tangent),
    ];
    model
}

fn luma(px: [u8; 4]) -> f32 {
    0.299 * f32::from(px[0]) + 0.587 * f32::from(px[1]) + 0.114 * f32::from(px[2])
}

#[test]
fn a_hull_reads_its_normal_map_through_the_authored_tangent_frame() {
    let instance = wgpu::Instance::default();
    let Some(adapter) = oag_gpu::test_adapter::request(&instance) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("requesting the device");

    let mut scene = Scene::off();
    scene.light.enabled = 1.0;
    // Toward the light: `+x`, `+y` and `+z`, so a normal tilted toward either
    // tangent axis faces it more squarely than the flat one does.
    scene.light.direction = [0.6, 0.6, 0.529];
    scene.light.sun = [0.6, 0.6, 0.6];
    scene.fog.camera = [0.0, 0.0, 100.5];
    let lit = |map: [u8; 4], tangent: u32| draw(&device, &queue, &model(map, tangent), scene);

    let flat = [128, 128, 255, 255];
    let flat_px = lit(flat, tangent(255));
    println!("flat map: {flat_px:?}");
    assert!(
        luma(flat_px) > 5.0,
        "the hull drew nothing under an authored rig"
    );

    // **No tangent is the off switch**: any map, however tilted, is ignored.
    let tilted_x = [230, 128, 200, 255];
    let ignored = lit(tilted_x, 0);
    assert_eq!(
        ignored,
        lit(flat, 0),
        "a vertex with no tangent must not read the map"
    );

    // **A flat map is the identity**: `nx = ny = 0`, `nz = 1` rebuilds the
    // vertex normal. A wrong decode (unsigned, a swapped lane) breaks this.
    assert!(
        (luma(flat_px) - luma(ignored)).abs() < 2.0,
        "a flat map must leave the hull lit as its vertex normal does: {flat_px:?} against {ignored:?}"
    );

    // **The map's red is the tangent's share.** Tilted toward the light along
    // `+x` the hull is lit more; away from it, less.
    let toward = lit(tilted_x, tangent(255));
    assert!(
        luma(toward) > luma(flat_px) + 8.0,
        "a map tilted toward the light along T must light the hull more: {toward:?} against {flat_px:?}"
    );
    let away = lit([26, 128, 200, 255], tangent(255));
    assert!(
        luma(away) + 8.0 < luma(flat_px),
        "a map tilted away from the light along T must light the hull less: {away:?}"
    );

    // **The map's green goes through `B = cross(N, T) * w`**: with `w = 255`
    // a tilt along `+y` faces the light; with `w = 0` the binormal is the zero
    // vector and the same tilt changes nothing.
    let tilted_y = [128, 230, 200, 255];
    let with_b = lit(tilted_y, tangent(255));
    let without_b = lit(tilted_y, tangent(0));
    assert!(
        luma(with_b) > luma(flat_px) + 8.0,
        "w = 1 must let the map's green tilt the normal along B: {with_b:?} against {flat_px:?}"
    );
    assert!(
        (luma(without_b) - luma(flat_px)).abs() < 2.0,
        "w = 0 must drop the green, and a rescaled vertex normal is the vertex normal: {without_b:?} against {flat_px:?}"
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
