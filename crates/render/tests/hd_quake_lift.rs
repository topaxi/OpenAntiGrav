//! Draws two triangles through the mesh pipeline, one carrying `GpuVertex::quake`,
//! and asserts what `quake.wesl`'s `quake_lifted` does to them.
//!
//! The law is Wipeout HD's `StaticQuake` vertex program, decoded from
//! `track_surface.rcsmaterial` (`docs/ghidra/functions/ps3-hdfury-eu/quake.md`):
//! a vertex rises along `quakeTrackUpNormal` by
//! `amplitude * cos(min(d_line / radius, pi / 2)) * sqrt(saturate(1 - |M - P| / radius))`.
//! Clip space is model space here (identity camera), so a lift of `L` along `+Y`
//! moves a vertex up the target by `L * SIZE / 2` rows, and the covered
//! triangle's centroid - which is the mean of its three vertices - moves by the
//! mean of their three lifts. Asserted: with no wave nothing moves; with one the
//! flagged triangle rises by the CPU mirror's mean lift to within a pixel and the
//! unflagged one does not move at all; amplitude zero is the identity whatever
//! the other registers hold.
//!
//! Skips when there is no adapter, like `hd_spu_vertex_light.rs`. Needs no game
//! content, so it runs in `just test`.

use std::sync::Arc;

use oag_mesh::mesh::{Bounds, DrawCall, GpuVertex, Model, ModelTexture, Texels, slots};
use oag_mesh::mesh_render::{self, Anisotropy, QuakeWave, Scene, UNIFORMS_SIZE};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const SIZE: u32 = 64;

/// The flagged triangle's corners (left half), then the unflagged one's (right).
const FLAGGED: [[f32; 3]; 3] = [[-0.9, -0.9, 0.5], [-0.1, -0.9, 0.5], [-0.5, -0.1, 0.5]];
const PLAIN: [[f32; 3]; 3] = [[0.1, -0.9, 0.5], [0.9, -0.9, 0.5], [0.5, -0.1, 0.5]];

fn vertex(position: [f32; 3], quake: f32) -> GpuVertex {
    GpuVertex {
        quake,
        position,
        normal: [0.0, 0.0, 1.0],
        colour: [1.0, 1.0, 1.0, 1.0],
        texcoord: [0.0, 0.0],
        lightmap_texcoord: [0.0, 0.0],
        lit: 1.0,
        anim: 0,
        xform: 0,
        sun_mask: 1.0,
        // Emissive, so the pixel is the albedo and any covered texel is non-zero.
        slots: slots::DEFAULT | slots::EMISSIVE,
        specular_exponent: oag_mesh::mesh::DEFAULT_SPECULAR_EXPONENT,
        glow: 0.0,
        texcoord2: [0.0, 0.0],
    }
}

fn texture() -> Arc<ModelTexture> {
    Arc::new(ModelTexture {
        label: "white albedo".into(),
        width: 1,
        height: 1,
        texels: Texels::Rgba8(vec![255; 4]),
        mip_count: None,
    })
}

fn model() -> Model {
    let mut vertices: Vec<GpuVertex> = FLAGGED.iter().map(|&p| vertex(p, 1.0)).collect();
    vertices.extend(PLAIN.iter().map(|&p| vertex(p, 0.0)));
    Model {
        airbrakes: [None, None],
        node_vertex_ranges: Vec::new(),
        lod_groups: Default::default(),
        label: "hd quake lift pair".into(),
        indices: vec![0, 1, 2, 3, 4, 5],
        draws: vec![DrawCall {
            moving: false,
            blend: None,
            blend_state: None,
            layer: oag_vex::vex::LAYER_DEFAULT,
            culled: false,
            range: 0..6,
            texture: Some(0),
            bounds: Bounds {
                centre: [0.0, 0.0, 0.5],
                radius: 2.0,
            },
            node: None,
            chunk: None,
            alpha_test_ref: None,
        }],
        alpha_tested_draws: Vec::new(),
        transparent_draws: Vec::new(),
        textures: vec![Some(texture())],
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
        emissive: Vec::new(),
        vertices,
    }
}

/// A wave across the lower half of the frame: the two track-edge points either
/// side of the origin at `y = -0.4`, so the road line runs along `X`.
fn wave(amplitude: f32) -> QuakeWave {
    QuakeWave {
        point_b: [0.6, -0.4, 0.5, amplitude],
        point_a: [-0.6, -0.4, 0.5, 2.0],
        offset: [1.2, 0.0, 0.0, 1.2],
        up: [0.0, 1.0, 0.0, 0.0],
    }
}

/// `quake.wesl`'s `quake_lifted`, on the CPU.
fn lift(wave: &QuakeWave, p: [f32; 3]) -> f32 {
    let sub = |a: [f32; 3], b: [f32; 3]| [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    let len = |v: [f32; 3]| (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    let (a, b) = (
        [wave.point_a[0], wave.point_a[1], wave.point_a[2]],
        [wave.point_b[0], wave.point_b[1], wave.point_b[2]],
    );
    let radius = wave.point_a[3];
    let m = [
        (a[0] + b[0]) / 2.0,
        (a[1] + b[1]) / 2.0,
        (a[2] + b[2]) / 2.0,
    ];
    let ap = sub(a, p);
    let off = [wave.offset[0], wave.offset[1], wave.offset[2]];
    let cross = [
        ap[1] * off[2] - ap[2] * off[1],
        ap[2] * off[0] - ap[0] * off[2],
        ap[0] * off[1] - ap[1] * off[0],
    ];
    let along = len(cross) / wave.offset[3] / radius;
    let falloff = (1.0 - len(sub(m, p)) / radius).clamp(0.0, 1.0).sqrt();
    wave.point_b[3] * along.clamp(0.0, std::f32::consts::FRAC_PI_2).cos() * falloff
}

/// Mean row of the covered pixels in columns `lo..hi`, counted from the top, or
/// `None` where nothing is covered.
fn centroid_row(pixels: &[[u8; 4]], lo: u32, hi: u32) -> Option<f32> {
    let (mut sum, mut n) = (0.0f32, 0.0f32);
    for y in 0..SIZE {
        for x in lo..hi {
            if pixels[(y * SIZE + x) as usize].iter().any(|&c| c != 0) {
                sum += y as f32 + 0.5;
                n += 1.0;
            }
        }
    }
    (n > 0.0).then(|| sum / n)
}

#[test]
fn the_quake_lifts_marked_surfaces_by_the_staticquake_law_and_nothing_else() {
    let instance = wgpu::Instance::default();
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&Default::default())) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("requesting the device");
    let model = model();
    let half = SIZE / 2;
    let rows = |scene: Scene| {
        let pixels = draw(&device, &queue, &model, scene);
        (
            centroid_row(&pixels, 0, half).expect("the flagged triangle is covered"),
            centroid_row(&pixels, half, SIZE).expect("the plain triangle is covered"),
            pixels,
        )
    };

    let (rest_flagged, rest_plain, rest) = rows(Scene::off());

    // Amplitude zero: the other registers are set and nothing moves.
    let mut idle = Scene::off();
    idle.quake = wave(0.0);
    let (_, _, idle_pixels) = rows(idle);
    assert_eq!(rest, idle_pixels, "a zero amplitude is the identity");

    let amplitude = 0.4;
    let mut live = Scene::off();
    live.quake = wave(amplitude);
    let (flagged, plain, _) = rows(live);

    let mean_lift = FLAGGED.iter().map(|&p| lift(&live.quake, p)).sum::<f32>() / 3.0;
    assert!(
        mean_lift > 0.15,
        "the test wave should move the triangle: {mean_lift}"
    );
    // Up the target is fewer rows: `SIZE / 2` rows per clip unit.
    let moved = rest_flagged - flagged;
    let expected = mean_lift * (SIZE / 2) as f32;
    assert!(
        (moved - expected).abs() < 1.0,
        "the flagged triangle rose {moved} rows, the law says {expected}"
    );
    assert!(
        (plain - rest_plain).abs() < 1e-3,
        "an unmarked surface must not move: {plain} against {rest_plain}"
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

/// Draws the model once into a `SIZE`x`SIZE` target and reads every pixel back.
fn draw(device: &wgpu::Device, queue: &wgpu::Queue, model: &Model, scene: Scene) -> Vec<[u8; 4]> {
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
        pass.draw_indexed(0..6, 0, 0..1);
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
    mapped
        .chunks_exact(4)
        .map(|p| [p[0], p[1], p[2], p[3]])
        .collect()
}
