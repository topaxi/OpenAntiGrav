//! The PS2 bloom's numbers, run on a real device against the GS's own integer
//! arithmetic done by hand.
//!
//! `docs/rendering/ps2-bloom.md` reads three passes off GS dumps and
//! `post::ps2_bloom` ports them. This feeds a frame with a known masked block
//! through [`Ps2Bloom::render`] and compares what comes back with the same
//! passes in integers: `Cs * As >> 7` into a 320 x 224 buffer, seven taps
//! `texel * w >> 7` per axis with nothing past the edge, and
//! `(Ct * 127 >> 7) * 64 >> 7` added back five pixels (of 512) up and left.
//!
//! `#[ignore]`d because it needs a GPU adapter:
//! `cargo nextest run -p oag-render --run-ignored all -E 'test(ps2_bloom_gain)'`

use oag_render::post::bloom::Frame;
use oag_render::post::ps2_bloom::{
    BLOOM_HEIGHT, BLOOM_WIDTH, BLUR_WEIGHTS, COMPOSITE_COLOUR, COMPOSITE_FIX, COMPOSITE_OFFSET,
    FRAME_SIZE, Ps2Bloom,
};

const W: u32 = 640;
const H: u32 = 448;
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

/// The glowing block: columns and rows, glow colour, and the mask value it
/// carries on the PS2's `0..=128` scale.
const BLOCK: ((u32, u32), (u32, u32)) = ((200, 260), (150, 200));
const BLOCK_RGB: [u8; 3] = [100, 60, 30];
const BLOCK_ALPHA: u8 = 100;
const BACKGROUND: [u8; 3] = [30, 30, 30];

fn scene_pixels() -> Vec<u8> {
    let mut px = vec![0u8; (W * H * 4) as usize];
    for y in 0..H {
        for x in 0..W {
            let at = ((y * W + x) * 4) as usize;
            let on = (BLOCK.0.0..BLOCK.0.1).contains(&x) && (BLOCK.1.0..BLOCK.1.1).contains(&y);
            let (rgb, alpha) = if on {
                (BLOCK_RGB, BLOCK_ALPHA)
            } else {
                (BACKGROUND, 0)
            };
            px[at..at + 3].copy_from_slice(&rgb);
            // The scene stores the PS2's alpha `0x80` as `1.0`.
            px[at + 3] = (f32::from(alpha) / 128.0 * 255.0).round() as u8;
        }
    }
    px
}

/// The 320 x 224 bright pass, for one channel: the block is large against a
/// bloom texel and the filter is bilinear, so a texel is the box average of the
/// scene under it, which this takes at its centre (`x * 2`, `y * 2` is not it:
/// a texel is 2 scene pixels wide and exactly 2 tall).
fn bright(channel: usize) -> Vec<u32> {
    let px = scene_pixels();
    let mut out = vec![0u32; (BLOOM_WIDTH * BLOOM_HEIGHT) as usize];
    for by in 0..BLOOM_HEIGHT {
        for bx in 0..BLOOM_WIDTH {
            // The two scene pixels the texel's centre sits between, per axis.
            let sx = (bx * 2).min(W - 2);
            let sy = (by * 2).min(H - 2);
            let mut c = 0.0;
            let mut a = 0.0;
            for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                let at = (((sy + dy) * W + sx + dx) * 4) as usize;
                c += f32::from(px[at + channel]) / 4.0;
                a += f32::from(px[at + 3]) / 4.0;
            }
            let alpha = (a / 255.0 * 128.0 + 0.5).floor() as u32;
            out[(by * BLOOM_WIDTH + bx) as usize] = (c.round() as u32 * alpha) >> 7;
        }
    }
    out
}

/// One blur pass in integers, zero past the edge.
fn blur(src: &[u32], vertical: bool) -> Vec<u32> {
    let mut out = vec![0u32; src.len()];
    let (w, h) = (BLOOM_WIDTH as i32, BLOOM_HEIGHT as i32);
    for y in 0..h {
        for x in 0..w {
            let mut sum = 0u32;
            for (i, &wt) in BLUR_WEIGHTS.iter().enumerate() {
                let k = i as i32 - 3;
                let (sx, sy) = if vertical { (x, y + k) } else { (x + k, y) };
                if sx < 0 || sy < 0 || sx >= w || sy >= h {
                    continue;
                }
                let v = src[(sy * w + sx) as usize];
                sum = (sum + ((v * u32::from(wt)) >> 7)).min(255);
            }
            out[(y * w + x) as usize] = sum;
        }
    }
    out
}

/// What the composite adds at a scene pixel, one channel, from the blurred
/// buffer: bilinear at the pixel shifted five frame pixels down and right.
fn added(buffer: &[u32], x: u32, y: u32) -> f32 {
    let shift = COMPOSITE_OFFSET / FRAME_SIZE;
    let u = (x as f32 + 0.5) / W as f32 + shift;
    let v = (y as f32 + 0.5) / H as f32 + shift;
    if u >= 1.0 || v >= 1.0 {
        return 0.0;
    }
    let fx = u * BLOOM_WIDTH as f32 - 0.5;
    let fy = v * BLOOM_HEIGHT as f32 - 0.5;
    let (x0, y0) = (fx.floor(), fy.floor());
    let (tx, ty) = (fx - x0, fy - y0);
    let at = |ix: f32, iy: f32| {
        let ix = (ix.max(0.0) as u32).min(BLOOM_WIDTH - 1);
        let iy = (iy.max(0.0) as u32).min(BLOOM_HEIGHT - 1);
        buffer[(iy * BLOOM_WIDTH + ix) as usize] as f32
    };
    let ct = (at(x0, y0) * (1.0 - tx) + at(x0 + 1.0, y0) * tx) * (1.0 - ty)
        + (at(x0, y0 + 1.0) * (1.0 - tx) + at(x0 + 1.0, y0 + 1.0) * tx) * ty;
    let cs = (ct * f32::from(COMPOSITE_COLOUR) / 128.0).floor();
    (cs * f32::from(COMPOSITE_FIX) / 128.0).floor()
}

#[test]
#[ignore = "needs a GPU adapter"]
fn the_ps2_bloom_adds_what_the_gs_arithmetic_says_it_adds() {
    let instance = wgpu::Instance::default();
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        compatible_surface: None,
        ..Default::default()
    }))
    .expect("no GPU adapter available");
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("ps2 bloom gain test"),
        ..Default::default()
    }))
    .expect("requesting the device");
    device.on_uncaptured_error(std::sync::Arc::new(|e| panic!("wgpu validation: {e}")));

    let scene = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("scene"),
        size: wgpu::Extent3d {
            width: W,
            height: H,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_DST
            | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let pixels = scene_pixels();
    queue.write_texture(
        scene.as_image_copy(),
        &pixels,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(W * 4),
            rows_per_image: Some(H),
        },
        wgpu::Extent3d {
            width: W,
            height: H,
            depth_or_array_layers: 1,
        },
    );
    let view = scene.create_view(&wgpu::TextureViewDescriptor::default());

    let bloom = Ps2Bloom::new(&device, FORMAT).expect("bloom builds");
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    bloom.render(
        &device,
        &queue,
        &mut encoder,
        Frame {
            scene: &view,
            size: (W, H),
            origin: (0.0, 0.0),
            viewport: (W, H),
        },
    );
    let padded = (W * 4).div_ceil(256) * 256;
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback"),
        size: u64::from(padded * H),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        scene.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded),
                rows_per_image: Some(H),
            },
        },
        wgpu::Extent3d {
            width: W,
            height: H,
            depth_or_array_layers: 1,
        },
    );
    queue.submit([encoder.finish()]);
    let slice = readback.slice(..);
    slice.map_async(wgpu::MapMode::Read, |r| r.expect("map"));
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("poll");
    let data = slice.get_mapped_range().expect("mapped");

    let mut worst = 0.0f32;
    let mut peak = 0.0f32;
    for channel in 0..3 {
        let blurred = blur(&blur(&bright(channel), false), true);
        // Inside the block, on its edge, in the halo, far away, and the bottom
        // right corner the five-pixel offset leaves undrawn.
        for (x, y) in [
            (230u32, 175u32),
            (205, 155),
            (196, 175),
            (264, 175),
            (230, 146),
            (230, 203),
            (180, 175),
            (150, 175),
            (230, 220),
            (230, 250),
            (20, 20),
            (W - 2, H - 2),
        ] {
            let at = (y * padded + x * 4) as usize + channel;
            let base = f32::from(pixels[((y * W + x) * 4) as usize + channel]);
            let got = f32::from(data[at]) - base;
            // The target saturates: a channel already near white cannot take it.
            let want = (base + added(&blurred, x, y)).min(255.0) - base;
            println!(
                "x {x:3} y {y:3} ch {channel}: added {got:5.1}, GS arithmetic says {want:5.1}"
            );
            worst = worst.max((got - want).abs());
            peak = peak.max(want);
        }
    }
    assert!(
        peak > 10.0,
        "the reference adds at most {peak:.1}: the probe points miss the glow"
    );
    assert!(
        worst < 4.0,
        "the bloom adds up to {worst:.1} levels away from the GS arithmetic"
    );
}
