//! The PSP bloom's numbers, run on a real device against the same arithmetic
//! done by hand.
//!
//! `bloom.md` recovers four passes and `post::bloom` ports them; nothing ran
//! the port against the recovered arithmetic, so a gain lost anywhere in the
//! chain read as a plausible but dim glow. This feeds a frame with a known
//! masked strip through [`Bloom::render`] and compares what comes back with
//! the passes' own maths in `f32`: a 2x2 box downsample of `rgb * a`, the
//! 11-tap kernel along y then x with each pass clamped to `0..=1` like the
//! GE's 8-bit target, a bilinear 2x upscale, and `+ 0xaf / 255` of it.
//!
//! `#[ignore]`d because it needs a GPU adapter:
//! `cargo nextest run -p oag-render --run-ignored all -E 'test(bloom_gain)'`

use oag_render::post::bloom::{BLUR_WEIGHTS, Bloom, COMPOSITE_STRENGTH, Frame};

const W: u32 = 480;
const H: u32 = 272;
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

/// The strip: rows `STRIP.0..STRIP.1`, every column, glow mask `0xff`.
const STRIP: (u32, u32) = (100, 110);
const STRIP_RGB: [u8; 3] = [20, 60, 70];
/// What an unstamped pixel holds: the stencil base the original clears to.
const BASE_MASK: u8 = 4;

fn scene_pixels() -> Vec<u8> {
    let mut px = vec![0u8; (W * H * 4) as usize];
    for y in 0..H {
        for x in 0..W {
            let at = ((y * W + x) * 4) as usize;
            let on = (STRIP.0..STRIP.1).contains(&y);
            let rgb = if on { STRIP_RGB } else { [10, 10, 10] };
            px[at..at + 3].copy_from_slice(&rgb);
            px[at + 3] = if on { 0xff } else { BASE_MASK };
        }
    }
    px
}

fn clamp01(v: f32) -> f32 {
    v.clamp(0.0, 1.0)
}

/// The recovered passes in `f32`, for one channel, down a single column (the
/// strip is constant along x so only the vertical profile matters, and the
/// horizontal blur of a constant row is a gain of the kernel's sum).
fn expected_add(channel: usize, y: u32) -> f32 {
    let px = scene_pixels();
    let sample = |x: u32, y: u32| {
        let at = ((y * W + x) * 4) as usize;
        f32::from(px[at + channel]) / 255.0 * f32::from(px[at + 3]) / 255.0
    };
    // Bright pass at 240x136: the corner shared by a 2x2 block.
    let bright = |by: i32| {
        let by = by.clamp(0, H as i32 / 2 - 1) as u32;
        let x = 100;
        (sample(x * 2, by * 2)
            + sample(x * 2 + 1, by * 2)
            + sample(x * 2, by * 2 + 1)
            + sample(x * 2 + 1, by * 2 + 1))
            / 4.0
    };
    let kernel: Vec<f32> = BLUR_WEIGHTS.iter().map(|&w| f32::from(w) / 255.0).collect();
    let row_gain: f32 = kernel.iter().sum();
    // Blur x first (the original's order): a constant row, so the kernel sum.
    let after_x = |by: i32| clamp01(bright(by) * row_gain);
    // Blur y over the half-resolution column.
    let after_y = |by: i32| {
        clamp01(
            kernel
                .iter()
                .enumerate()
                .map(|(i, w)| after_x(by + i as i32 - 5) * w)
                .sum(),
        )
    };
    // Bilinear 2x upscale back: between two half-res samples.
    let fy = (y as f32 + 0.5) / 2.0 - 0.5;
    let y0 = fy.floor() as i32;
    let t = fy - y0 as f32;
    let layer = after_y(y0) * (1.0 - t) + after_y(y0 + 1) * t;
    layer * f32::from(COMPOSITE_STRENGTH) / 255.0 * 255.0
}

#[test]
#[ignore = "needs a GPU adapter"]
fn the_bloom_adds_what_the_recovered_passes_say_it_adds() {
    let instance = wgpu::Instance::default();
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        compatible_surface: None,
        ..Default::default()
    }))
    .expect("no GPU adapter available");
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("bloom gain test"),
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

    let bloom = Bloom::new(&device, FORMAT).expect("bloom builds");
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
    for y in [90u32, 98, 100, 104, 109, 112, 120, 135] {
        let at = (y * padded + 100 * 4) as usize;
        for channel in 0..3 {
            let base = f32::from(pixels[((y * W + 100) * 4) as usize + channel]);
            let got = f32::from(data[at + channel]) - base;
            let want = expected_add(channel, y);
            println!("y {y:3} ch {channel}: added {got:6.1}, recovered passes say {want:6.1}");
            worst = worst.max((got - want).abs());
        }
    }
    assert!(
        worst < 8.0,
        "the bloom adds up to {worst:.1} levels away from the recovered arithmetic"
    );
}
