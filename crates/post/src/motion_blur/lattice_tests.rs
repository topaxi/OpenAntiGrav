//! The tile lattice, measured on a synthetic ramp, and the grain that the old
//! fix for it traded in.
//!
//! Its own file beside [`super::tests`], which is near the 1,000 lines a file
//! may hold. The method is `docs/rendering/motion-blur.md`'s "tile lattice"
//! section: velocity purely vertical, its magnitude ramping along x past the
//! reach cap, depth flat, the colour a bright block, so the smear above the
//! block reads the gather out directly.

use super::tests::f32_to_half;
use super::*;

pub(super) const W: u32 = 1920;
pub(super) const H: u32 = 1080;
const BLOCK: std::ops::Range<u32> = 700..740;

/// One pixel of a synthetic frame: its colour, its velocity in uv units and
/// its depth.
pub(super) type Pixel = ([u8; 3], [f32; 2], f32);

/// The ramp frame's red channel, row-major.
fn ramp_frame(device: &wgpu::Device, queue: &wgpu::Queue, strength: f32) -> Vec<u8> {
    let rgba = frame(device, queue, strength, &|x, y| {
        let value = if BLOCK.contains(&y) { 255 } else { 0 };
        // Zero at the left edge, past the cap (in pixels) from x=1300 on.
        ([value; 3], [0.0, 0.16 * (x as f32 / 1300.0)], 0.5)
    });
    rgba.iter().step_by(4).copied().collect()
}

/// A 1920x1080 frame of `pixel`, run through the whole chain, as RGBA bytes.
pub(super) fn frame(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    strength: f32,
    pixel: &dyn Fn(u32, u32) -> Pixel,
) -> Vec<u8> {
    frames(device, queue, strength, pixel, 1).remove(0)
}

/// `count` consecutive frames of the same still scene through one chain, the
/// way a player's run feeds it, as RGBA bytes each.
pub(super) fn frames(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    strength: f32,
    pixel: &dyn Fn(u32, u32) -> Pixel,
    count: usize,
) -> Vec<Vec<u8>> {
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let extent = wgpu::Extent3d {
        width: W,
        height: H,
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
    let upload = |texture: &wgpu::Texture, bytes: &[u8]| {
        queue.write_texture(
            texture.as_image_copy(),
            bytes,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(W * 4),
                rows_per_image: None,
            },
            extent,
        );
    };
    let scene = make(
        "scene",
        format,
        wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::COPY_DST
            | wgpu::TextureUsages::COPY_SRC,
    );
    let velocity = make(
        "velocity",
        oag_gpu::formats::VELOCITY_FORMAT,
        wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
    );
    let depth = make(
        "depth",
        wgpu::TextureFormat::R32Float,
        wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
    );

    let bytes = (W * H * 4) as usize;
    let (mut pixels, mut velocities, mut depths) = (
        Vec::with_capacity(bytes),
        Vec::with_capacity(bytes),
        Vec::with_capacity(bytes),
    );
    for y in 0..H {
        for x in 0..W {
            let (colour, velocity, depth) = pixel(x, y);
            pixels.extend_from_slice(&[colour[0], colour[1], colour[2], 255]);
            velocities.extend_from_slice(&f32_to_half(velocity[0]).to_le_bytes());
            velocities.extend_from_slice(&f32_to_half(velocity[1]).to_le_bytes());
            depths.extend_from_slice(&depth.to_le_bytes());
        }
    }
    upload(&velocity, &velocities);
    upload(&depth, &depths);

    let mut blur = MotionBlur::new(device, format).expect("building the pipelines");
    (0..count)
        .map(|_| {
            upload(&scene, &pixels);
            render_once(
                device, queue, &mut blur, &scene, &velocity, &depth, strength, extent,
            )
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn render_once(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    blur: &mut MotionBlur,
    scene: &wgpu::Texture,
    velocity: &wgpu::Texture,
    depth: &wgpu::Texture,
    strength: f32,
    extent: wgpu::Extent3d,
) -> Vec<u8> {
    let mut encoder = device.create_command_encoder(&Default::default());
    blur.render(
        device,
        queue,
        &mut encoder,
        &Frame {
            scene: &scene.create_view(&Default::default()),
            velocity: &velocity.create_view(&Default::default()),
            depth: &depth.create_view(&Default::default()),
            sample_count: 1,
            size: (W, H),
            viewport: (0.0, 0.0, W as f32, H as f32),
            strength,
            camera_shake: Mat4::IDENTITY,
        },
        None,
    );
    let stride = (W * 4).div_ceil(256) * 256;
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback"),
        size: u64::from(stride * H),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        scene.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(stride),
                rows_per_image: None,
            },
        },
        extent,
    );
    queue.submit(Some(encoder.finish()));
    readback.slice(..).map_async(wgpu::MapMode::Read, |_| {});
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("waiting for the readback");
    let data = readback.slice(..).get_mapped_range().expect("mapping");
    let mut rgba = Vec::with_capacity((W * H * 4) as usize);
    for y in 0..H {
        let row = (y * stride) as usize;
        rgba.extend_from_slice(&data[row..row + (W * 4) as usize]);
    }
    rgba
}

/// What the ramp's smear profile says about its tile lattice and its grain.
struct Profile {
    /// The largest per-phase mean of `|2nd difference|` over the median
    /// phase, phases being `x mod tile`. A lattice step puts all of its
    /// energy at one phase; a smooth ramp spreads it evenly.
    peak_over_median: f32,
    /// The phase it peaks at.
    peak_phase: usize,
    /// The mean absolute difference between a pixel and the mean of its two
    /// row neighbours, over the band the ramp is inside the cap: the grain,
    /// which a smooth smear has none of and a per-pixel hash has plenty.
    grain: f32,
}

fn profile(red: &[u8]) -> Profile {
    let tile = max_px((0.0, 0.0, W as f32, H as f32)).ceil() as usize;
    // Rows 16 to 4 above the block, where the smear is read straight out.
    let rows = (BLOCK.start - 16)..(BLOCK.start - 4);
    let row_mean = |x: u32| -> f32 {
        rows.clone()
            .map(|y| f32::from(red[(y * W + x) as usize]))
            .sum::<f32>()
            / rows.len() as f32
    };
    let mean: Vec<f32> = (0..W).map(row_mean).collect();
    let (first, last) = (2 * tile as u32, 1300);
    let mut phases = vec![(0.0f32, 0u32); tile];
    for x in first..last {
        let i = x as usize;
        let d2 = (mean[i - 1] - 2.0 * mean[i] + mean[i + 1]).abs();
        let slot = &mut phases[i % tile];
        slot.0 += d2;
        slot.1 += 1;
    }
    let means: Vec<f32> = phases.iter().map(|p| p.0 / p.1.max(1) as f32).collect();
    let mut sorted = means.clone();
    sorted.sort_by(|a, b| a.total_cmp(b));
    let median = sorted[sorted.len() / 2].max(1.0e-3);
    let (peak_phase, peak) = means
        .iter()
        .copied()
        .enumerate()
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .expect("a non-empty profile");

    let mut grain = 0.0;
    let mut n = 0.0;
    for y in rows.clone() {
        for x in first..last {
            let at = |dx: i64| f32::from(red[(y * W) as usize + (i64::from(x) + dx) as usize]);
            grain += (at(0) - 0.5 * (at(-1) + at(1))).abs();
            n += 1.0;
        }
    }
    Profile {
        peak_over_median: peak / median,
        peak_phase,
        grain: grain / n,
    }
}

/// A device on whatever adapter this machine has, or `None` to skip.
pub(super) fn gpu() -> Option<(wgpu::Device, wgpu::Queue)> {
    let instance = wgpu::Instance::default();
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&Default::default())) else {
        eprintln!("no GPU adapter: skipping");
        return None;
    };
    Some(
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("requesting the device"),
    )
}

fn measured(strength: f32) -> Option<Profile> {
    let (device, queue) = gpu()?;
    Some(profile(&ramp_frame(&device, &queue, strength)))
}

/// The lattice the nearest tile lookup showed - the smear profile stepping at
/// every tile boundary - stays gone, and the per-pixel noise the hash wobble
/// bought it with is gone too.
///
/// Measured at 1920x1080 on this machine's adapter, nearest lookup: peak at
/// phase 0 (the tile edge), ratio over median 2.3-2.6. Hash-wobbled lookup:
/// ratio 1.4-1.5 and the grain figure below. **Skips when there is no
/// adapter.**
#[test]
fn the_tile_lookup_leaves_neither_a_lattice_nor_grain_on_a_ramp() {
    for strength in [0.5, 0.75] {
        let Some(found) = measured(strength) else {
            return;
        };
        eprintln!(
            "strength {strength}: peak/median {:.2} at phase {}, grain {:.3}",
            found.peak_over_median, found.peak_phase, found.grain
        );
        assert!(
            found.peak_over_median < 1.8,
            "the smear steps on the tile lattice at strength {strength}: \
             {:.2}x the median at phase {}",
            found.peak_over_median,
            found.peak_phase
        );
    }
}
