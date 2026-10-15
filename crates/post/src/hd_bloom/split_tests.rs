//! The chain run per view of a split: each view adapts to its own pixels.
//!
//! Every test here draws two views of the same size into one scene target, a dark one and
//! a bright one, and compares what `Chain::run_view` resolves against the same content run
//! as a picture of its own (`Chain::run`). The two views are the same size on purpose:
//! a cache keyed on the rectangle's size alone would give the second view the first's
//! rectangle.

use super::*;

const SIZE: (u32, u32) = (64, 64);

fn device() -> Option<(wgpu::Device, wgpu::Queue)> {
    let instance = wgpu::Instance::default();
    let adapters = pollster::block_on(instance.enumerate_adapters(wgpu::Backends::PRIMARY));
    let Some(adapter) = adapters.first() else {
        eprintln!("no GPU adapter: skipping");
        return None;
    };
    Some(
        pollster::block_on(adapter.request_device(&Default::default()))
            .expect("a device with no extra features"),
    )
}

fn params() -> Params {
    Params {
        // A real adaptation: the exposure falls with the mean, so a bright view resolves
        // darker than the same pixels would in a dark picture.
        adaption_rate: 0.5,
        tone_adaption_boost: 1.0,
        tone_darkening_clamp: 0.9,
        tone_maximum_brightness: 1.0,
        // No glow: the exposure is what is under test.
        frame_contribution: 0.0,
        alpha_contribution: 0.0,
        ..super::tests::params()
    }
}

/// Writes `level(x, y)` (linear grey) into the scene target of `chain`.
fn paint(queue: &wgpu::Queue, chain: &Chain, level: impl Fn(u32, u32) -> f32) {
    let mut texels = Vec::new();
    for y in 0..SIZE.1 {
        for x in 0..SIZE.0 {
            let v = half(level(x, y));
            for channel in [v, v, v, half(0.0)] {
                texels.extend_from_slice(&channel);
            }
        }
    }
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: chain.scene_texture(),
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &texels,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(SIZE.0 * 8),
            rows_per_image: Some(SIZE.1),
        },
        wgpu::Extent3d {
            width: SIZE.0,
            height: SIZE.1,
            depth_or_array_layers: 1,
        },
    );
}

/// An `f16` of a value in `0..=1`, as the scene target stores it.
fn half(value: f32) -> [u8; 2] {
    let table = [
        (0.0f32, 0x0000u16),
        (0.25, 0x3400),
        (0.5, 0x3800),
        (0.75, 0x3a00),
        (1.0, 0x3c00),
    ];
    let (_, bits) = table
        .iter()
        .find(|(v, _)| (*v - value).abs() < 1e-6)
        .expect("a level the table holds");
    bits.to_le_bytes()
}

/// Reads the red bytes of the frame `view` was resolved into.
fn read_red(device: &wgpu::Device, queue: &wgpu::Queue, output: &wgpu::Texture) -> Vec<u8> {
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("hd split test readback"),
        size: u64::from(SIZE.0 * SIZE.1 * 4),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: output,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(SIZE.0 * 4),
                rows_per_image: Some(SIZE.1),
            },
        },
        wgpu::Extent3d {
            width: SIZE.0,
            height: SIZE.1,
            depth_or_array_layers: 1,
        },
    );
    queue.submit(Some(encoder.finish()));
    readback
        .slice(..)
        .map_async(wgpu::MapMode::Read, |r| r.expect("the readback maps"));
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("the GPU");
    let data = readback.slice(..).get_mapped_range().expect("mapped");
    let red = data
        .as_chunks::<4>()
        .0
        .iter()
        .map(|texel| texel[0])
        .collect();
    drop(data);
    readback.unmap();
    red
}

fn frame(device: &wgpu::Device) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("hd split test frame"),
        size: wgpu::Extent3d {
            width: SIZE.0,
            height: SIZE.1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    })
}

/// The mean of `red` over the rows `rows` of the frame.
fn mean(red: &[u8], rows: std::ops::Range<u32>) -> f64 {
    let (from, to) = (rows.start * SIZE.0, rows.end * SIZE.0);
    let slice = &red[from as usize..to as usize];
    slice.iter().map(|&v| f64::from(v)).sum::<f64>() / slice.len() as f64
}

/// The top half of the scene is dark, the bottom half bright.
fn dark_over_bright(_x: u32, y: u32) -> f32 {
    if y < SIZE.1 / 2 { 0.25 } else { 0.75 }
}

/// Two 64x32 views of `dark_over_bright`, each run on its own, several frames over so the
/// adaptation settles. Returns the frame's red bytes.
fn split_frames(device: &wgpu::Device, queue: &wgpu::Queue, frames: usize) -> Vec<u8> {
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let chain = Chain::new(device, format, SIZE, params(), Glow::Drawn).expect("pipelines build");
    paint(queue, &chain, dark_over_bright);
    let output = frame(device);
    let target = output.create_view(&Default::default());
    let half_height = SIZE.1 / 2;
    for _ in 0..frames {
        for (seat, y) in [(0, 0), (1, half_height)] {
            let mut encoder = device.create_command_encoder(&Default::default());
            chain.run_view(
                queue,
                device,
                &mut encoder,
                &target,
                View {
                    seat,
                    rect: (0, y, SIZE.0, half_height),
                    area: SIZE,
                    reach: (1.0, 1.0),
                    first: seat == 0,
                },
            );
            queue.submit(Some(encoder.finish()));
        }
    }
    read_red(device, queue, &output)
}

/// One whole picture of a single brightness, run as a picture of its own, settled.
fn alone(device: &wgpu::Device, queue: &wgpu::Queue, level: f32, frames: usize) -> f64 {
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let chain = Chain::new(device, format, SIZE, params(), Glow::Drawn).expect("pipelines build");
    paint(queue, &chain, |_, _| level);
    let output = frame(device);
    let target = output.create_view(&Default::default());
    for _ in 0..frames {
        let mut encoder = device.create_command_encoder(&Default::default());
        chain.run(queue, &mut encoder, &target, (0.0, 0.0), SIZE, None);
        queue.submit(Some(encoder.finish()));
    }
    mean(&read_red(device, queue, &output), 0..SIZE.1)
}

/// **Each view resolves as it would alone.** A dark view and a bright one, the same size,
/// one scene target: each view's resolved pixels equal the same brightness drawn as a
/// picture of its own. A shared exposure (one adaptation for the frame) puts both views at
/// the mean of the two; a scene tap that ignores the view's offset resolves the bottom view
/// from the top view's pixels; a uniform cache keyed on size alone gives the second view
/// the first's rectangle. Any of the three fails the comparison below.
#[test]
fn each_view_resolves_as_it_would_alone() {
    let Some((device, queue)) = device() else {
        return;
    };
    let red = split_frames(&device, &queue, 12);
    let top = mean(&red, 2..SIZE.1 / 2 - 2);
    let bottom = mean(&red, SIZE.1 / 2 + 2..SIZE.1 - 2);
    let dark = alone(&device, &queue, 0.25, 12);
    let bright = alone(&device, &queue, 0.75, 12);
    assert!(
        (top - dark).abs() < 2.0,
        "the dark view resolved to {top}, the same picture alone to {dark}"
    );
    assert!(
        (bottom - bright).abs() < 2.0,
        "the bright view resolved to {bottom}, the same picture alone to {bright}"
    );
}

/// **A seat keeps its own adaptation from frame to frame.** After a bright seat has settled,
/// a dark seat that draws for the first time starts from its own first frame (the rate-1
/// jump) and not from the bright seat's state: its first frame equals the dark picture's
/// first frame alone.
#[test]
fn a_new_seat_starts_from_its_own_first_frame() {
    let Some((device, queue)) = device() else {
        return;
    };
    let first_frame_dark = alone(&device, &queue, 0.25, 1);
    let red = split_frames(&device, &queue, 1);
    let top = mean(&red, 2..SIZE.1 / 2 - 2);
    assert!(
        (top - first_frame_dark).abs() < 2.0,
        "the dark seat's first frame resolved to {top}, alone to {first_frame_dark}"
    );
}

/// **The cut stays clean.** Nothing a view resolves reaches into the other's rows: with
/// the glow on, a bright view next to a dark one spills no light across the cut.
#[test]
fn a_views_glow_stops_at_its_rectangle() {
    let Some((device, queue)) = device() else {
        return;
    };
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let glowing = Params {
        frame_contribution: 4.0,
        frame_exponent: 1.0,
        horizontal_size: 4.0,
        vertical_size: 4.0,
        ..params()
    };
    let spill = |bright_top: bool| {
        let chain =
            Chain::new(&device, format, SIZE, glowing, Glow::Drawn).expect("pipelines build");
        paint(&queue, &chain, |_, y| {
            if (y < SIZE.1 / 2) == bright_top {
                1.0
            } else {
                0.0
            }
        });
        let output = frame(&device);
        let target = output.create_view(&Default::default());
        let half_height = SIZE.1 / 2;
        for (seat, y) in [(0, 0), (1, half_height)] {
            let mut encoder = device.create_command_encoder(&Default::default());
            chain.run_view(
                &queue,
                &device,
                &mut encoder,
                &target,
                View {
                    seat,
                    rect: (0, y, SIZE.0, half_height),
                    area: SIZE,
                    reach: (1.0, 1.0),
                    first: seat == 0,
                },
            );
            queue.submit(Some(encoder.finish()));
        }
        read_red(&device, &queue, &output)
    };
    // A black view beside a white one: its glow-free resolve is black through and through.
    let red = spill(true);
    let dark_rows = mean(&red, SIZE.1 / 2..SIZE.1);
    assert!(
        dark_rows < 1.0,
        "the dark view read {dark_rows}: the bright view's glow crossed the cut"
    );
}
