//! A mod's shader drawn into a small target: what its text computes reaches the picture, on a
//! fill and on a surface, and a place that did not load draws nothing.
//!
//! **Skips when there is no adapter.**

use oag_fx::mod_shader::{self, Fill, Frame, Shader, Stage};
use oag_gpu::formats::SceneStore;
use oag_mesh::mesh::GpuVertex;
use oag_mesh::mesh_render::{DEPTH_FORMAT, Velocity};

const SIZE: u32 = 64;
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const IDENTITY: [[f32; 4]; 4] = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
    [0.0, 0.0, 0.0, 1.0],
];

fn shader(text: &str) -> Option<Shader> {
    Some(Shader {
        compiled: mod_shader::compile("test.wgsl", text).unwrap(),
        texture: None,
    })
}

struct Gpu {
    device: wgpu::Device,
    queue: wgpu::Queue,
}

fn gpu() -> Option<Gpu> {
    let instance = wgpu::Instance::default();
    let adapter = oag_gpu::test_adapter::request(&instance)?;
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).ok()?;
    device.on_uncaptured_error(std::sync::Arc::new(|e| panic!("wgpu validation: {e}")));
    Some(Gpu { device, queue })
}

fn stage(gpu: &Gpu, shaders: &[Option<Shader>]) -> Stage {
    let stage = Stage::new(
        &gpu.device,
        &gpu.queue,
        FORMAT,
        SceneStore::Computed,
        1,
        Velocity::None,
        shaders,
    );
    assert_eq!(stage.failures(), &[] as &[String]);
    stage
}

/// A target cleared to black, `stage` drawn over it after `upload`, as `(r, g, b, a)` texels.
fn render(gpu: &Gpu, stage: &mut Stage, frame: &Frame<'_>) -> Vec<[u8; 4]> {
    let size = wgpu::Extent3d {
        width: SIZE,
        height: SIZE,
        depth_or_array_layers: 1,
    };
    let target = |format, usage| {
        gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage,
            view_formats: &[],
        })
    };
    let colour = target(
        FORMAT,
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
    );
    let depth = target(DEPTH_FORMAT, wgpu::TextureUsages::RENDER_ATTACHMENT);
    stage.upload(&gpu.queue, frame);
    let drawn = stage.drawn();
    let view = colour.create_view(&Default::default());
    let depth_view = depth.create_view(&Default::default());
    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Discard,
                }),
                stencil_ops: None,
            }),
            ..Default::default()
        });
        stage.drawer().draw_surfaces(&mut pass, &drawn);
        stage.drawer().draw_fill(&mut pass, &drawn);
    }
    let readback = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
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
        size,
    );
    gpu.queue.submit([encoder.finish()]);
    readback
        .slice(..)
        .map_async(wgpu::MapMode::Read, |result| result.unwrap());
    gpu.device
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();
    let mapped = readback.slice(..).get_mapped_range();
    mapped
        .chunks_exact(4)
        .map(|c| [c[0], c[1], c[2], c[3]])
        .collect()
}

fn at(texels: &[[u8; 4]], x: u32, y: u32) -> [u8; 4] {
    texels[(y * SIZE + x) as usize]
}

fn frame<'a>(
    fill: Option<Fill>,
    surfaces: &'a [(usize, Vec<GpuVertex>)],
    params: &'a [[Option<f32>; 4]],
) -> Frame<'a> {
    Frame {
        view_projection: IDENTITY,
        time: 2.0,
        draw_alpha: 1.0,
        fill,
        surfaces,
        params,
    }
}

#[test]
fn a_fill_runs_the_mods_text_with_its_uv_time_and_scalars() {
    let Some(gpu) = gpu() else { return };
    // Red is the uv's x, green the time over four, blue the scalar, the colour's alpha over.
    let text = "//! [[param]]\n//! name = \"blue\"\n//! default = 0.25\n
fn mod_fragment(input: ModInput) -> vec4<f32> {
    return vec4<f32>(input.uv.x, input.time / 4.0, param_blue(), input.colour.a);
}\n";
    let mut stage = stage(&gpu, &[shader(text)]);
    let fill = Some(Fill {
        shader: 0,
        colour: [0.0, 0.0, 0.0, 1.0],
    });
    let texels = render(&gpu, &mut stage, &frame(fill, &[], &[]));
    let left = at(&texels, 1, 32);
    let right = at(&texels, 62, 32);
    assert!(left[0] < 12 && right[0] > 243, "{left:?} {right:?}");
    assert!((i32::from(left[1]) - 128).abs() <= 2, "{left:?}");
    assert!(
        (i32::from(left[2]) - 64).abs() <= 2,
        "the default: {left:?}"
    );
    // A sent scalar replaces the header's default.
    let sent = [[Some(1.0), None, None, None]];
    let texels = render(&gpu, &mut stage, &frame(fill, &[], &sent));
    assert_eq!(at(&texels, 32, 32)[2], 255);
}

fn quad(colour: [f32; 4]) -> Vec<GpuVertex> {
    let corner = |x: f32, y: f32, u: f32, v: f32| GpuVertex {
        position: [x, y, 0.5],
        colour,
        texcoord: [u, v],
        ..bytemuck::Zeroable::zeroed()
    };
    let (bl, br) = (corner(-0.5, -0.5, 0.0, 1.0), corner(0.5, -0.5, 1.0, 1.0));
    let (tl, tr) = (corner(-0.5, 0.5, 0.0, 0.0), corner(0.5, 0.5, 1.0, 0.0));
    vec![bl, br, tl, br, tr, tl]
}

#[test]
fn a_surface_draws_where_its_vertices_are_with_their_colour_and_uv() {
    let Some(gpu) = gpu() else { return };
    let text = "//! blend = \"add\"\n
fn mod_fragment(input: ModInput) -> vec4<f32> {
    return vec4<f32>(input.colour.r, input.uv.y, input.screen_uv.x, 1.0);
}\n";
    let mut stage = stage(&gpu, &[shader(text)]);
    let surfaces = [(0, quad([1.0, 0.0, 0.0, 1.0]))];
    let texels = render(&gpu, &mut stage, &frame(None, &surfaces, &[]));
    assert_eq!(at(&texels, 2, 2), [0, 0, 0, 255], "outside the quad");
    let top = at(&texels, 32, 18);
    let bottom = at(&texels, 32, 45);
    assert_eq!(top[0], 255);
    assert!(
        top[1] < 30 && bottom[1] > 225,
        "v runs down: {top:?} {bottom:?}"
    );
    assert!((i32::from(top[2]) - 128).abs() <= 3, "screen uv: {top:?}");
}

/// A place that did not load (the load report named it) draws nothing, and a neighbour's
/// surfaces still draw: nothing falls back to a flat colour.
#[test]
fn a_shader_that_did_not_load_draws_nothing() {
    let Some(gpu) = gpu() else { return };
    let good = "fn mod_fragment(input: ModInput) -> vec4<f32> { return input.colour; }\n";
    let mut stage = stage(&gpu, &[None, shader(good)]);
    let surfaces = [(0, quad([1.0, 1.0, 1.0, 1.0]))];
    let fill = Some(Fill {
        shader: 0,
        colour: [1.0; 4],
    });
    let texels = render(&gpu, &mut stage, &frame(fill, &surfaces, &[]));
    assert!(texels.iter().all(|t| *t == [0, 0, 0, 255]));
    let surfaces = [
        (0, quad([1.0, 1.0, 1.0, 1.0])),
        (1, quad([0.0, 1.0, 0.0, 1.0])),
    ];
    let texels = render(&gpu, &mut stage, &frame(None, &surfaces, &[]));
    assert_eq!(at(&texels, 32, 32), [0, 255, 0, 255]);
}

/// The engine slot: transparent black and unbound until a texture is set, then the texture,
/// projected through the matrix it was set with.
#[test]
fn the_engine_texture_slot_is_empty_until_the_engine_fills_it() {
    let Some(gpu) = gpu() else { return };
    let text = "fn mod_fragment(input: ModInput) -> vec4<f32> {
    if !engine_texture_bound() { return vec4<f32>(0.0, 0.0, 1.0, 1.0); }
    return vec4<f32>(engine_texture(engine_uv(input.world)).rgb, 1.0);
}\n";
    let mut stage = stage(&gpu, &[shader(text)]);
    let surfaces = [(0, quad([1.0; 4]))];
    let texels = render(&gpu, &mut stage, &frame(None, &surfaces, &[]));
    assert_eq!(at(&texels, 32, 32), [0, 0, 255, 255]);

    let picture = gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    gpu.queue.write_texture(
        picture.as_image_copy(),
        &[9, 200, 40, 255],
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(4),
            rows_per_image: None,
        },
        picture.size(),
    );
    let view = picture.create_view(&Default::default());
    stage.set_engine_texture(&gpu.device, Some((&view, IDENTITY)));
    let texels = render(&gpu, &mut stage, &frame(None, &surfaces, &[]));
    assert_eq!(at(&texels, 32, 32), [9, 200, 40, 255]);
}
