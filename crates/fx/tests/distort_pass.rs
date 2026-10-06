//! The blend class 8 pass on a real device, read back.
//!
//! `distort.wesl` is the heat-haze program (`docs/ghidra/functions/ps4-omega-eu/
//! heat-haze.md`): the fragment writes `colour.rg * (colour.a * fade * sprite.a) *
//! (sprite.rg - 0.5)` with `fade = 2 / |w|^0.75`, into a target cleared to zero and
//! drawn into additively, and a pixel the scene's depth is nearer than is rejected.
//! Each assertion below fails if one of those is dropped.
//!
//! **Skips when there is no adapter.**

use oag_core::math::camera::perspective;
use oag_fx::psys::Pipeline;
use oag_fx::psys::sprite::{Sheet, Sprite};
use oag_mesh::mesh::GpuVertex;
use oag_mesh::mesh_render::{DEPTH_FORMAT, Velocity};

const SIZE: u32 = 16;
const NEAR: f32 = 0.1;
const FAR: f32 = 100.0;

fn f16(bits: u16) -> f32 {
    let sign = if bits >> 15 == 1 { -1.0 } else { 1.0 };
    let exponent = i32::from((bits >> 10) & 0x1f);
    let fraction = f32::from(bits & 0x3ff);
    match exponent {
        0 => sign * fraction * 2.0_f32.powi(-24),
        0x1f => sign * f32::INFINITY,
        e => sign * (1.0 + fraction / 1024.0) * 2.0_f32.powi(e - 15),
    }
}

/// What the scene's depth attachment holds for a surface `w` away.
fn depth_of(w: f32) -> f32 {
    FAR * (w - NEAR) / (w * (FAR - NEAR))
}

struct Quad {
    /// Distance along the view axis, which is the clip `w`.
    w: f32,
    /// Left edge, in units; the quad runs to `+20`, so `0.0` covers the right
    /// half of the 16-pixel frame at `w = 10`.
    left: f32,
    strength: f32,
    alpha: f32,
    sprite: [f32; 4],
}

struct Gpu {
    device: wgpu::Device,
    queue: wgpu::Queue,
}

fn gpu() -> Option<Gpu> {
    let instance = wgpu::Instance::default();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).ok()?;
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).ok()?;
    Some(Gpu { device, queue })
}

/// Draws `quads` against a depth plane `occluder` away, returns the `(r, g)` of
/// every pixel.
fn draw(gpu: &Gpu, quads: &[Quad], occluder: f32) -> Vec<[f32; 2]> {
    let Gpu { device, queue } = gpu;
    device.on_uncaptured_error(std::sync::Arc::new(|e| panic!("wgpu validation: {e}")));
    let mut pipeline = Pipeline::new(device, wgpu::TextureFormat::Rgba8Unorm, 1, Velocity::None);
    let mut sheet = Sheet::default();
    let vertices: Vec<GpuVertex> = quads
        .iter()
        .flat_map(|quad| {
            let texel = quad.sprite.map(|c| (c * 255.0).round() as u8);
            let rect = sheet
                .place(&Sprite {
                    width: 4,
                    height: 4,
                    rgba: texel.repeat(16).into(),
                })
                .expect("the sprite fits");
            let uv = [(rect[0] + rect[2]) / 2.0, (rect[1] + rect[3]) / 2.0];
            let corner = |x: f32, y: f32| GpuVertex {
                position: [x, y, -quad.w],
                normal: [1.0, 0.0, 0.0],
                colour: [quad.strength, quad.strength, quad.strength, quad.alpha],
                texcoord: uv,
                ..bytemuck::Zeroable::zeroed()
            };
            let (l, r) = (quad.left, 20.0 * quad.w / 10.0);
            let (b, t) = (-r, r);
            [
                corner(l, b),
                corner(r, b),
                corner(l, t),
                corner(r, b),
                corner(r, t),
                corner(l, t),
            ]
        })
        .collect();
    pipeline.sync_sheet(queue, &sheet);
    let projection = perspective(std::f32::consts::FRAC_PI_2, 1.0, NEAR, FAR);
    pipeline.upload(queue, &projection.to_cols_array_2d(), &[], &[]);
    pipeline.upload_distort(queue, &vertices);

    let depth = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("scene depth"),
        size: wgpu::Extent3d {
            width: SIZE,
            height: SIZE,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: DEPTH_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let depth_view = depth.create_view(&Default::default());
    let mut encoder = device.create_command_encoder(&Default::default());
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("scene"),
        color_attachments: &[],
        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
            view: &depth_view,
            depth_ops: Some(wgpu::Operations {
                load: wgpu::LoadOp::Clear(depth_of(occluder)),
                store: wgpu::StoreOp::Store,
            }),
            stencil_ops: None,
        }),
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    let drew = pipeline
        .encode_distort(
            device,
            &mut encoder,
            &depth_view,
            (SIZE, SIZE),
            (0.0, 0.0, SIZE as f32, SIZE as f32),
        )
        .is_some();
    assert!(drew, "the pass drew nothing");
    let texture = pipeline.offset_texture().expect("a target exists");
    let row = 256;
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback"),
        size: u64::from(row * SIZE),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(row),
                rows_per_image: None,
            },
        },
        texture.size(),
    );
    queue.submit([encoder.finish()]);
    buffer.slice(..).map_async(wgpu::MapMode::Read, |_| {});
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("the readback completes");
    let bytes = buffer
        .slice(..)
        .get_mapped_range()
        .expect("the mapped range")
        .to_vec();
    (0..SIZE * SIZE)
        .map(|i| {
            let at = ((i / SIZE) * row + (i % SIZE) * 4) as usize;
            let half = |o: usize| f16(u16::from_le_bytes([bytes[at + o], bytes[at + o + 1]]));
            [half(0), half(2)]
        })
        .collect()
}

fn at(pixels: &[[f32; 2]], x: u32, y: u32) -> [f32; 2] {
    pixels[(y * SIZE + x) as usize]
}

/// The law for one fragment: `strength * alpha * fade * sprite.a * (sprite.rg - 0.5)`.
fn law(w: f32, strength: f32, alpha: f32, sprite: [f32; 4]) -> [f32; 2] {
    let texel = |c: f32| (c * 255.0).round() / 255.0;
    let fade = 2.0 / w.abs().powf(0.75);
    let k = strength * alpha * fade * texel(sprite[3]);
    [k * (texel(sprite[0]) - 0.5), k * (texel(sprite[1]) - 0.5)]
}

fn close(got: [f32; 2], want: [f32; 2], what: &str) {
    for channel in 0..2 {
        assert!(
            (got[channel] - want[channel]).abs() < 2e-3,
            "{what}: channel {channel} drew {} where the law says {}",
            got[channel],
            want[channel]
        );
    }
}

const SPRITE: [f32; 4] = [192.0 / 255.0, 64.0 / 255.0, 0.0, 1.0];

fn quad(w: f32, strength: f32, alpha: f32, sprite: [f32; 4]) -> Quad {
    Quad {
        w,
        left: 0.0,
        strength,
        alpha,
        sprite,
    }
}

#[test]
fn a_quad_writes_the_law_and_nothing_where_it_is_not() {
    let Some(gpu) = gpu() else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let pixels = draw(&gpu, &[quad(10.0, 2.0, 1.0, SPRITE)], 50.0);
    let want = law(10.0, 2.0, 1.0, SPRITE);
    assert!(
        want[0] > 0.1 && want[1] < -0.1,
        "the case has a sign each way"
    );
    close(at(&pixels, 12, 8), want, "inside the quad");
    assert_eq!(at(&pixels, 3, 8), [0.0, 0.0], "outside the quad: the clear");
}

#[test]
fn the_fade_the_alpha_and_the_sprites_alpha_each_scale_the_offset() {
    let Some(gpu) = gpu() else {
        return;
    };
    let near = draw(&gpu, &[quad(10.0, 2.0, 1.0, SPRITE)], 90.0);
    let far = draw(&gpu, &[quad(20.0, 2.0, 1.0, SPRITE)], 90.0);
    close(at(&far, 12, 8), law(20.0, 2.0, 1.0, SPRITE), "twice as far");
    assert!(
        at(&far, 12, 8)[0] < 0.7 * at(&near, 12, 8)[0],
        "the fade falls with w"
    );
    let faint = draw(&gpu, &[quad(10.0, 2.0, 0.5, SPRITE)], 90.0);
    close(
        at(&faint, 12, 8),
        law(10.0, 2.0, 0.5, SPRITE),
        "half the vertex alpha",
    );
    let thin = [SPRITE[0], SPRITE[1], 0.0, 128.0 / 255.0];
    let thin = draw(&gpu, &[quad(10.0, 2.0, 1.0, thin)], 90.0);
    close(
        at(&thin, 12, 8),
        law(10.0, 2.0, 1.0, [SPRITE[0], SPRITE[1], 0.0, 128.0 / 255.0]),
        "half the sprite's alpha",
    );
    let strong = draw(&gpu, &[quad(10.0, 1.0, 1.0, SPRITE)], 90.0);
    close(
        at(&strong, 12, 8),
        law(10.0, 1.0, 1.0, SPRITE),
        "the strength is the colour",
    );
}

#[test]
fn overlapping_quads_add() {
    let Some(gpu) = gpu() else {
        return;
    };
    let one = law(10.0, 2.0, 1.0, SPRITE);
    let pixels = draw(
        &gpu,
        &[quad(10.0, 2.0, 1.0, SPRITE), quad(10.0, 2.0, 1.0, SPRITE)],
        50.0,
    );
    close(
        at(&pixels, 12, 8),
        [2.0 * one[0], 2.0 * one[1]],
        "two quads",
    );
}

#[test]
fn a_scene_surface_nearer_than_the_quad_rejects_it() {
    let Some(gpu) = gpu() else {
        return;
    };
    let behind = draw(&gpu, &[quad(10.0, 2.0, 1.0, SPRITE)], 5.0);
    assert_eq!(at(&behind, 12, 8), [0.0, 0.0], "the scene is nearer");
    let level = draw(&gpu, &[quad(10.0, 2.0, 1.0, SPRITE)], 11.0);
    close(
        at(&level, 12, 8),
        law(10.0, 2.0, 1.0, SPRITE),
        "the scene is behind",
    );
}
