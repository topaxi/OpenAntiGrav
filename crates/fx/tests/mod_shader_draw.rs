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
    stage_in(gpu, FORMAT, SceneStore::Computed, shaders)
}

fn stage_in(
    gpu: &Gpu,
    format: wgpu::TextureFormat,
    store: SceneStore,
    shaders: &[Option<Shader>],
) -> Stage {
    let stage = Stage::new(
        &gpu.device,
        &gpu.queue,
        format,
        store,
        1,
        Velocity::None,
        shaders,
    );
    assert_eq!(stage.failures(), &[] as &[String]);
    stage
}

/// A target cleared to black, `stage` drawn over it after `upload`, as `(r, g, b, a)` texels.
fn render(gpu: &Gpu, stage: &mut Stage, frame: &Frame<'_>) -> Vec<[u8; 4]> {
    render_bytes(gpu, stage, frame, FORMAT, 4)
        .as_chunks::<4>()
        .0
        .to_vec()
}

/// [`render`] into a `format` target of `texel_bytes` a texel, as the target's own bytes.
fn render_bytes(
    gpu: &Gpu,
    stage: &mut Stage,
    frame: &Frame<'_>,
    format: wgpu::TextureFormat,
    texel_bytes: u32,
) -> Vec<u8> {
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
        format,
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
        size: u64::from(SIZE * SIZE * texel_bytes),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        colour.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(SIZE * texel_bytes),
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
    let mapped = readback.slice(..).get_mapped_range().unwrap();
    mapped.to_vec()
}

/// An IEEE half float, widened.
fn half(bits: u16) -> f32 {
    let sign = if bits & 0x8000 == 0 { 1.0 } else { -1.0 };
    let exponent = i32::from((bits >> 10) & 0x1f);
    let mantissa = f32::from(bits & 0x3ff);
    match exponent {
        0 => sign * mantissa * 2f32.powi(-24),
        31 if mantissa == 0.0 => sign * f32::INFINITY,
        31 => f32::NAN,
        _ => sign * (1.0 + mantissa / 1024.0) * 2f32.powi(exponent - 15),
    }
}

/// [`render`] into a half-float scene target, as `(r, g, b, a)` values.
fn render_scene(gpu: &Gpu, stage: &mut Stage, frame: &Frame<'_>) -> Vec<[f32; 4]> {
    let bytes = render_bytes(gpu, stage, frame, oag_gpu::formats::SCENE_FORMAT, 8);
    bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|b| half(u16::from_le_bytes(*b)))
        .collect::<Vec<_>>()
        .as_chunks::<4>()
        .0
        .to_vec()
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
        engine_view_projection: None,
        craft: IDENTITY,
        lens: [1.0, 1.0],
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
    stage.set_engine_texture(&gpu.device, Some(&view));
    // A view with no picture of its own stays empty, whatever the slot holds.
    let texels = render(&gpu, &mut stage, &frame(None, &surfaces, &[]));
    assert_eq!(at(&texels, 32, 32), [0, 0, 255, 255]);
    let viewed = Frame {
        engine_view_projection: Some(IDENTITY),
        ..frame(None, &surfaces, &[])
    };
    let texels = render(&gpu, &mut stage, &viewed);
    assert_eq!(at(&texels, 32, 32), [9, 200, 40, 255]);
}

/// A render target's row 0 is the top of the picture and `engine_uv` is y down: the upper half
/// of the view reads the texture's upper half. A flipped `v` draws the portal upside down.
/// A picture's own alpha, the bloom's glow mask in a scene target, never reaches the shader.
#[test]
fn engine_uv_reads_the_top_of_the_picture_at_the_top_of_the_view() {
    let Some(gpu) = gpu() else { return };
    let text = "fn mod_fragment(input: ModInput) -> vec4<f32> {
    return engine_texture(engine_uv(input.world));
}\n";
    let mut stage = stage(&gpu, &[shader(text)]);
    let surfaces = [(0, quad([1.0; 4]))];
    let picture = gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
            width: 1,
            height: 2,
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
        &[255, 0, 0, 0, 0, 255, 0, 0],
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(4),
            rows_per_image: None,
        },
        picture.size(),
    );
    let view = picture.create_view(&Default::default());
    stage.set_engine_texture(&gpu.device, Some(&view));
    let viewed = Frame {
        engine_view_projection: Some(IDENTITY),
        ..frame(None, &surfaces, &[])
    };
    let texels = render(&gpu, &mut stage, &viewed);
    let (top, bottom) = (at(&texels, 32, 18), at(&texels, 32, 45));
    assert!(top[0] > 200 && top[1] < 50 && top[3] == 255, "{top:?}");
    assert!(bottom[1] > 200 && bottom[0] < 50, "{bottom:?}");
}

/// The fragment a test draws: one channel in range, one above 1 and one below 0, so what the
/// target keeps of each is read off one texel.
const OVER_RANGE: &str = "fn mod_fragment(input: ModInput) -> vec4<f32> {
    return vec4<f32>(0.5, 2.0, -1.0, 1.0);
}\n";

/// The two linear float scenes that keep a value above 1, with the power each decodes a
/// gamma-authored colour by: Omega's computed one (2.2) and 2048's gamma-2 one.
const FLOAT_SCENES: [(SceneStore, f32); 2] =
    [(SceneStore::Computed, 2.2), (SceneStore::Squared, 2.0)];

/// A float scene target that computes in linear light keeps a shader's value above 1 (the
/// bloom and the tonemap read it there), and still holds a negative one at 0.
#[test]
fn a_float_target_keeps_a_value_above_one() {
    let Some(gpu) = gpu() else { return };
    let fill = Some(Fill {
        shader: 0,
        colour: [0.0, 0.0, 0.0, 1.0],
    });
    for (store, power) in FLOAT_SCENES {
        let mut float = stage_in(
            &gpu,
            oag_gpu::formats::SCENE_FORMAT,
            store,
            &[shader(OVER_RANGE)],
        );
        let kept =
            render_scene(&gpu, &mut float, &frame(fill, &[], &[]))[(32 * SIZE + 32) as usize];
        assert!(
            (kept[0] - 0.5f32.powf(power)).abs() < 0.001,
            "{store:?} {kept:?}"
        );
        assert!(
            (kept[1] - 2.0f32.powf(power)).abs() < 0.02 * 2.0f32.powf(power),
            "a value above 1 is kept: {store:?} {kept:?}"
        );
        assert_eq!(kept[2], 0.0, "a negative value is not: {store:?} {kept:?}");
    }
}

/// An 8-bit target is the clamp it always was, byte for byte.
#[test]
fn an_eight_bit_target_still_clamps_a_value_above_one() {
    let Some(gpu) = gpu() else { return };
    let fill = Some(Fill {
        shader: 0,
        colour: [0.0, 0.0, 0.0, 1.0],
    });
    let mut eight = stage(&gpu, &[shader(OVER_RANGE)]);
    let clamped = at(&render(&gpu, &mut eight, &frame(fill, &[], &[])), 32, 32);
    assert_eq!(clamped, [128, 255, 0, 255]);
}

/// Wipeout HD's scene stores what the console's packer wrote: encoded and clamped to 1. A
/// float target of that kind holds no value above 1 for a portal to lose or keep.
#[test]
fn an_encoded_scene_target_still_stops_at_white() {
    let Some(gpu) = gpu() else { return };
    let fill = Some(Fill {
        shader: 0,
        colour: [0.0, 0.0, 0.0, 1.0],
    });
    let mut encoded = stage_in(
        &gpu,
        oag_gpu::formats::SCENE_FORMAT,
        SceneStore::Encoded,
        &[shader(OVER_RANGE)],
    );
    let stored =
        render_scene(&gpu, &mut encoded, &frame(fill, &[], &[]))[(32 * SIZE + 32) as usize];
    assert!((0.999..=1.0).contains(&stored[1]), "{stored:?}");
    assert_eq!(stored[2], 0.0, "{stored:?}");
}

/// A highlight in the portal's picture (4.0 in the float scene) comes back out of
/// `engine_texture` and is written to the scene as it was, not as white; a shader's runaway
/// value stays under the half float's ceiling instead of becoming infinity.
#[test]
fn a_highlight_in_the_engine_texture_survives_a_pass_through() {
    let Some(gpu) = gpu() else { return };
    let text = "fn mod_fragment(input: ModInput) -> vec4<f32> {
    let seen = engine_texture(engine_uv(input.world));
    return vec4<f32>(seen.rgb, 1.0) + vec4<f32>(0.0, 0.0, 1000.0 * input.uv.x, 0.0);
}\n";
    for (store, _) in FLOAT_SCENES {
        pass_through(&gpu, store, text);
    }
}

fn pass_through(gpu: &Gpu, store: SceneStore, text: &str) {
    let mut stage = stage_in(gpu, oag_gpu::formats::SCENE_FORMAT, store, &[shader(text)]);
    let surfaces = [(0, quad([1.0; 4]))];
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
        format: oag_gpu::formats::SCENE_FORMAT,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    // Half floats 4.0, 0.25, 1.0 and 1.0.
    let texel: Vec<u8> = [0x4400u16, 0x3400, 0x3c00, 0x3c00]
        .iter()
        .flat_map(|h| h.to_le_bytes())
        .collect();
    gpu.queue.write_texture(
        picture.as_image_copy(),
        &texel,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(8),
            rows_per_image: None,
        },
        picture.size(),
    );
    stage.set_engine_texture(&gpu.device, Some(&picture.create_view(&Default::default())));
    let viewed = Frame {
        engine_view_projection: Some(IDENTITY),
        ..frame(None, &surfaces, &[])
    };
    let texels = render_scene(gpu, &mut stage, &viewed);
    let kept = texels[(32 * SIZE + 18) as usize];
    assert!(
        (kept[0] - 4.0).abs() < 0.02,
        "the highlight: {store:?} {kept:?}"
    );
    assert!(
        (kept[1] - 0.25).abs() < 0.002,
        "the rest: {store:?} {kept:?}"
    );
    let runaway = texels[(32 * SIZE + 45) as usize];
    assert!(
        runaway[2].is_finite() && runaway[2] > 10_000.0,
        "capped, not infinite: {store:?} {runaway:?}"
    );
}

/// The half float bits nearest `value`, for a finite value of 0 or more (the half's ordering follows the
/// bits' on that range).
fn half_bits(value: f32) -> u16 {
    (0..=0x7bffu16)
        .min_by(|a, b| {
            (half(*a) - value)
                .abs()
                .total_cmp(&(half(*b) - value).abs())
        })
        .unwrap()
}

/// What a shader that returns `engine_texture` unchanged writes to a scene of `store`, for a
/// picture holding each of `stored` (the scene's own values), one texel apiece.
fn engine_round_trip(gpu: &Gpu, store: SceneStore, stored: &[[f32; 3]]) -> Vec<[f32; 3]> {
    let text = "fn mod_fragment(input: ModInput) -> vec4<f32> {
    return engine_texture(engine_uv(input.world));
}\n";
    let mut stage = stage_in(gpu, oag_gpu::formats::SCENE_FORMAT, store, &[shader(text)]);
    let surfaces = [(0, quad([1.0; 4]))];
    stored
        .iter()
        .map(|value| {
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
                format: oag_gpu::formats::SCENE_FORMAT,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            let texel: Vec<u8> = [value[0], value[1], value[2], 1.0]
                .iter()
                .flat_map(|v| half_bits(*v).to_le_bytes())
                .collect();
            gpu.queue.write_texture(
                picture.as_image_copy(),
                &texel,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(8),
                    rows_per_image: None,
                },
                picture.size(),
            );
            stage.set_engine_texture(&gpu.device, Some(&picture.create_view(&Default::default())));
            let viewed = Frame {
                engine_view_projection: Some(IDENTITY),
                ..frame(None, &surfaces, &[])
            };
            let written = render_scene(gpu, &mut stage, &viewed)[(32 * SIZE + 32) as usize];
            [written[0], written[1], written[2]]
        })
        .collect()
}

/// Grey from 0 to 255 in steps of 32 (and 255), and three saturated colours, as 8-bit levels.
fn ramp() -> Vec<[f32; 3]> {
    let mut levels: Vec<f32> = (0..8).map(|i| (i * 32) as f32).collect();
    levels.push(255.0);
    let mut ramp: Vec<[f32; 3]> = levels.iter().map(|l| [l / 255.0; 3]).collect();
    ramp.push([1.0, 64.0 / 255.0, 13.0 / 255.0]);
    ramp.push([8.0 / 255.0, 200.0 / 255.0, 24.0 / 255.0]);
    ramp.push([20.0 / 255.0, 40.0 / 255.0, 230.0 / 255.0]);
    ramp
}

/// A shader that returns `engine_texture` as it is writes back what Wipeout HD's scene held:
/// the scene stores encoded values and the read decodes with the curve the write encodes with,
/// within one level of 255 on every channel.
#[test]
fn an_encoded_scene_passes_the_engine_texture_through_within_one_level() {
    let Some(gpu) = gpu() else { return };
    let ramp = ramp();
    let written = engine_round_trip(&gpu, SceneStore::Encoded, &ramp);
    let mut worst = [0.0f32; 3];
    for (was, now) in ramp.iter().zip(&written) {
        for c in 0..3 {
            worst[c] = worst[c].max((now[c] - was[c]).abs() * 255.0);
        }
    }
    eprintln!("encoded scene pass-through, worst error in levels (r, g, b): {worst:?}");
    assert!(
        worst.iter().all(|e| *e <= 1.0),
        "worst error in levels {worst:?}: {ramp:?} came back {written:?}"
    );
}

/// The float scenes that compute in linear light round-trip a pass-through too, to a part in
/// a hundred of the stored value.
#[test]
fn a_float_scene_passes_the_engine_texture_through() {
    let Some(gpu) = gpu() else { return };
    let ramp: Vec<[f32; 3]> = ramp().iter().map(|v| v.map(|c| c * c * 3.0)).collect();
    for (store, _) in FLOAT_SCENES {
        let written = engine_round_trip(&gpu, store, &ramp);
        for (was, now) in ramp.iter().zip(&written) {
            for c in 0..3 {
                assert!(
                    (now[c] - was[c]).abs() <= 0.01 * was[c] + 0.0005,
                    "{store:?} {was:?} came back {now:?}"
                );
            }
        }
    }
}
