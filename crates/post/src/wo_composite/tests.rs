//! What [`super`] is asserted to do: the law against the numbers the original
//! drew and stepped live, and the chain drawn on a real device against the law.

use super::law::{self, Exposure, Params};
use super::*;

/// Queens Mall's own keys, read live off the setters (exposure.md).
fn mall() -> Params {
    Params {
        exposure_scale: 0.4,
        exposure_max: 12.0,
        exposure_speed: 0.1,
        bloom_factor: 0.2,
        bloom_gate: 0.2,
        exposure: Exposure::Adapting,
    }
}

/// The ram-fill probe: alpha 64 written into every gate pixel, after which E
/// read 0.843, then 0.662, then 0.539 at grabs about five frames apart, and
/// settled at 0.4000. The law steps the same way, and to the same place.
#[test]
fn the_step_walks_the_exposure_as_the_filled_readback_did() {
    let p = mall();
    let a = law::readback(64.0);
    assert!((a - 1.0).abs() < 1e-6, "byte 64 reads as {a}");
    let mut e = law::RESET_EXPOSURE;
    let mut walked = Vec::new();
    for step in 1..=200 {
        e = law::adapt(e, a, &p);
        if step == 5 || step == 10 || step == 16 {
            walked.push(e);
        }
    }
    for (got, want) in walked.iter().zip([0.843, 0.662, 0.539]) {
        assert!((got - want).abs() < 2e-3, "walked {walked:?}");
    }
    assert!((e - 0.4).abs() < 1e-4, "settled at {e}");
}

#[test]
fn a_readback_at_the_floor_takes_no_step_and_the_ceiling_holds() {
    let p = mall();
    assert_eq!(law::adapt(1.15, 0.01, &p), 1.15);
    assert_eq!(law::adapt(1.15, 0.0, &p), 1.15);
    // A near-black frame drives the target to 40; one step from 11.9 lands on the cap.
    assert_eq!(law::adapt(11.9, 0.0101, &p), 12.0);
}

/// The gate writes its luminance over four into an 8-bit alpha and the
/// readback scales the byte by 1/64: the two cancel, so `a` is the luminance
/// the gate saw, to within a byte. This is what puts a Vita's settled
/// exposure at `ExposureScale / mean luminance`.
#[test]
fn the_gate_alpha_and_the_readback_scale_cancel() {
    for grey in [0.05_f32, 0.2, 0.5, 1.0, 2.5] {
        let (_, alpha) = law::gate([[grey; 3]; 4], 1.15, 0.2);
        let byte = (alpha.clamp(0.0, 1.0) * 255.0).round();
        let a = law::readback(byte);
        assert!(
            (a - grey).abs() <= 0.5 * 4.0 / 255.0 + grey * 0.004,
            "grey {grey}: readback {a}"
        );
    }
}

/// The poke probe on Queens Mall: bloom off, E from 1.15 to 2.3, and the
/// frame's grey quantiles before and after. For a grey pixel the composite is
/// `y = e / sqrt(1 + e)`, so each quantile at 1.15 predicts the one at 2.3.
/// Measured to the 90th percentile (above it the hull's colour breaks the grey
/// form and the output clips).
#[test]
fn the_composite_predicts_the_doubled_exposure_the_poke_drew() {
    let measured = [
        (0.075_f32, 0.148_f32),
        (0.124, 0.238),
        (0.166, 0.314),
        (0.218, 0.403),
        (0.367, 0.630),
    ];
    for (at_reset, drawn) in measured {
        // Invert `y^2 (1 + e) = e^2` for the pixel's `e` at E = 1.15.
        let y2 = at_reset * at_reset;
        let e = (y2 + (y2 * y2 + 4.0 * y2).sqrt()) / 2.0;
        let grey = e / 1.15;
        let predicted = law::composite([grey; 3], [0.0; 3], 2.3, 0.0)[0];
        assert!(
            (predicted - drawn).abs() < 0.015,
            "from {at_reset}: law {predicted}, drawn {drawn}"
        );
    }
}

#[test]
fn the_blur_keeps_a_flat_picture_flat_to_within_its_own_rounding() {
    let sum: f32 = law::BLUR_WEIGHTS.iter().map(|w| 2.0 * w).sum::<f32>() * law::BLUR_NORMALISE;
    assert!((sum - 0.998).abs() < 1e-3, "kernel sums to {sum}");
}

struct Gpu {
    device: wgpu::Device,
    queue: wgpu::Queue,
}

fn gpu() -> Option<Gpu> {
    let instance = wgpu::Instance::default();
    let adapter = oag_gpu::test_adapter::request(&instance)?;
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("requesting the device");
    Some(Gpu { device, queue })
}

const SIZE: (u32, u32) = (16, 16);
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

/// Clears the chain's scene to `colour`, runs one frame into a fresh
/// `Rgba8Unorm` target, and returns the centre pixel and the exposure state.
fn frame(gpu: &Gpu, chain: &Chain, colour: [f32; 3]) -> ([u8; 4], f32) {
    let (pixels, exposure) = frame_pixels(gpu, chain, colour);
    (pixel_at(&pixels, SIZE.0 / 2, SIZE.1 / 2), exposure)
}

/// The pixel at `(x, y)` of a [`frame_pixels`] buffer.
fn pixel_at(pixels: &[u8], x: u32, y: u32) -> [u8; 4] {
    let at = (y * ROW + x * 4) as usize;
    [pixels[at], pixels[at + 1], pixels[at + 2], pixels[at + 3]]
}

/// Bytes a row of the readback buffer takes.
const ROW: u32 = 256;

/// [`frame`], returning the whole readback buffer, `ROW` bytes a row.
fn frame_pixels(gpu: &Gpu, chain: &Chain, colour: [f32; 3]) -> (Vec<u8>, f32) {
    let out = gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("out"),
        size: wgpu::Extent3d {
            width: SIZE.0,
            height: SIZE.1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let out_view = out.create_view(&Default::default());
    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("scene"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: chain.scene_view(),
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color {
                    r: f64::from(colour[0]),
                    g: f64::from(colour[1]),
                    b: f64::from(colour[2]),
                    a: 1.0,
                }),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    chain.run(&gpu.queue, &mut encoder, &out_view, (0.0, 0.0), SIZE);
    let row = ROW;
    let pixels = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("pixels"),
        size: u64::from(row * SIZE.1),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let state = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("state"),
        size: STATE_SIZE,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        out.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &pixels,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(row),
                rows_per_image: None,
            },
        },
        out.size(),
    );
    encoder.copy_buffer_to_buffer(chain.window_state(), 0, &state, 0, STATE_SIZE);
    gpu.queue.submit([encoder.finish()]);
    pixels.slice(..).map_async(wgpu::MapMode::Read, |_| {});
    state.slice(..).map_async(wgpu::MapMode::Read, |_| {});
    gpu.device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("the readback completes");
    let bytes = pixels
        .slice(..)
        .get_mapped_range()
        .expect("pixels")
        .to_vec();
    let words = state.slice(..).get_mapped_range().expect("state").to_vec();
    let exposure = f32::from_le_bytes([words[0], words[1], words[2], words[3]]);
    (bytes, exposure)
}

fn byte(v: f32) -> i32 {
    (v.clamp(0.0, 1.0) * 255.0).round() as i32
}

/// The chain on a real device at a held exposure, below the gate (so no bloom)
/// and above it: each pixel is the law's, and the held exposure never moves.
/// **Skips when there is no adapter**, so a green CI run is not evidence that
/// it ran.
#[test]
fn a_held_chain_draws_the_law_on_a_real_device() {
    let Some(gpu) = gpu() else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let p = Params {
        exposure: Exposure::Held(law::RESET_EXPOSURE),
        ..mall()
    };
    let chain = Chain::new(&gpu.device, FORMAT, SIZE, p).expect("the chain builds");

    // Luminance 0.15, under the 0.2 gate: no bloom.
    let dark = [0.15_f32, 0.15, 0.15];
    let (pixel, exposure) = frame(&gpu, &chain, dark);
    assert_eq!(
        exposure,
        law::RESET_EXPOSURE,
        "a held exposure is never stepped"
    );
    let want = law::composite(dark, [0.0; 3], law::RESET_EXPOSURE, p.bloom_factor);
    for channel in 0..3 {
        let got = i32::from(pixel[channel]);
        assert!(
            (got - byte(want[channel])).abs() <= 2,
            "dark: drew {got}, the law says {}",
            byte(want[channel])
        );
    }

    // A flat 0.5 grey: the gate passes `(1.15 * 0.3)^2 * 2.0` into an 8-bit
    // target, the blurs keep it flat to 0.998 each, and the composite adds it.
    let grey = [0.5_f32; 3];
    let (pixel, _) = frame(&gpu, &chain, grey);
    let (gated, _) = law::gate([grey; 4], law::RESET_EXPOSURE, p.bloom_gate);
    let kernel: f32 = law::BLUR_WEIGHTS.iter().map(|w| 2.0 * w).sum::<f32>() * law::BLUR_NORMALISE;
    let bloom = gated.map(|g| (byte(g) as f32 / 255.0) * kernel * kernel);
    let want = law::composite(grey, bloom, law::RESET_EXPOSURE, p.bloom_factor);
    let unbloomed = law::composite(grey, [0.0; 3], law::RESET_EXPOSURE, p.bloom_factor);
    for channel in 0..3 {
        let got = i32::from(pixel[channel]);
        assert!(
            (got - byte(want[channel])).abs() <= 2,
            "grey: drew {got}, the law says {}",
            byte(want[channel])
        );
    }
    assert!(
        byte(want[0]) - byte(unbloomed[0]) > 10,
        "the bloom term is visible here: {} against {}",
        byte(want[0]),
        byte(unbloomed[0])
    );
}

/// The vignette on a real device: a flat grey picture under the gate is darkest in the
/// top corners (the far side of the mesh centred low in the frame), a little dark along the
/// top edge, and untouched in the bottom corners and the middle. Each pixel is the law's,
/// `composite * (1 - (vignetteTint * w)^2)` with `w` from the mesh's own triangles. The
/// original's top corners read 1.2 to 1.8 in luma on all three 2048 pairs, its bottom
/// corners lit (exposure.md, "The vignette"). **Skips when there is no adapter.**
#[test]
fn the_composite_draws_the_vignette_over_the_top_of_the_frame() {
    let Some(gpu) = gpu() else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let p = Params {
        exposure: Exposure::Held(law::RESET_EXPOSURE),
        ..mall()
    };
    let chain = Chain::new(&gpu.device, FORMAT, SIZE, p).expect("the chain builds");
    let grey = [0.15_f32; 3];
    let (pixels, _) = frame_pixels(&gpu, &chain, grey);
    let plain = law::composite(grey, [0.0; 3], law::RESET_EXPOSURE, p.bloom_factor);
    let drawn = |x: u32, y: u32| {
        let clip = [
            (x as f32 + 0.5) / SIZE.0 as f32 * 2.0 - 1.0,
            1.0 - (y as f32 + 0.5) / SIZE.1 as f32 * 2.0,
        ];
        let v = law::vignette(law::vignette_w(clip));
        (pixel_at(&pixels, x, y), v)
    };
    for (x, y) in [
        (0, 0),
        (15, 0),
        (8, 0),
        (3, 1),
        (0, 15),
        (15, 15),
        (8, 15),
        (8, 8),
    ] {
        let (pixel, v) = drawn(x, y);
        for channel in 0..3 {
            let want = byte(plain[channel] * v[channel]);
            let got = i32::from(pixel[channel]);
            assert!(
                (got - want).abs() <= 3,
                "pixel ({x}, {y}) channel {channel}: drew {got}, the law says {want} (v {v:?})"
            );
        }
    }
    let corner = i32::from(pixel_at(&pixels, 0, 0)[0]);
    let foot = i32::from(pixel_at(&pixels, 0, 15)[0]);
    assert_eq!(
        foot,
        byte(plain[0]),
        "the bottom corner is the plain composite"
    );
    assert!(
        corner * 2 < foot,
        "the top corner ({corner}) is under half the bottom corner's ({foot})"
    );
}

/// The mesh's `w` and the factor it makes, at the places the pairs measured.
#[test]
fn the_vignette_is_dark_at_the_top_corners_and_absent_below_the_centre() {
    // The frame's top corners, the far edge of a mesh centred at (0, -0.885).
    for x in [-1.0_f32, 1.0] {
        let w = law::vignette_w([x, 1.0]);
        assert!((0.95..=1.0).contains(&w), "w at the top corner is {w}");
        let v = law::vignette(w);
        assert!(
            v[0] < 0.1 && v[1] < 0.1,
            "red and green at the corner: {v:?}"
        );
        assert!(
            v[2] > v[0],
            "the 0.97 tint leaves blue a little lighter: {v:?}"
        );
    }
    // The top edge's middle is part of the way out.
    let top = law::vignette_w([0.0, 1.0]);
    assert!((0.5..0.7).contains(&top), "w at the top middle is {top}");
    // Nothing at the bottom, where the mesh is centred, or inside the inner circle.
    for clip in [
        [-1.0, -1.0],
        [1.0, -1.0],
        [0.0, -1.0],
        [0.0, 0.0],
        [0.9, 0.2],
    ] {
        assert_eq!(law::vignette_w(clip), 0.0, "w at {clip:?}");
        assert_eq!(law::vignette(0.0), [1.0; 3]);
    }
    // Outward from the inner circle `w` rises monotonically along the centre line.
    let along: Vec<f32> = (0..=10)
        .map(|k| law::vignette_w([0.0, law::VIGNETTE_CENTRE_Y + 1.4 + 0.075 * k as f32]))
        .collect();
    assert!(along.windows(2).all(|pair| pair[1] >= pair[0]), "{along:?}");
}

/// An adapting chain starts settled on its first frame - `min(Scale / a, Max)`
/// for the readback of that frame's gate - and stays there on the next, where
/// the step's fixed point is the same.
#[test]
fn an_adapting_chain_settles_where_the_law_does() {
    let Some(gpu) = gpu() else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let p = mall();
    let chain = Chain::new(&gpu.device, FORMAT, SIZE, p).expect("the chain builds");
    // Luminance 0.16, under the gate, so the picture is the composite alone.
    let colour = [0.2_f32, 0.15, 0.1];
    let (_, alpha) = law::gate([colour; 4], 1.0, p.bloom_gate);
    let a = law::readback((alpha * 255.0).round());
    let settled = (p.exposure_scale / a).min(p.exposure_max);

    let (pixel, first) = frame(&gpu, &chain, colour);
    assert!(
        (first - settled).abs() < 1e-3,
        "first frame at {first}, the law settles at {settled}"
    );
    let (_, second) = frame(&gpu, &chain, colour);
    assert!(
        (second - settled).abs() < 1e-3,
        "second frame moved to {second}"
    );
    assert!(
        (first - law::RESET_EXPOSURE).abs() > 0.05,
        "the exposure moved off the reset value"
    );
    // And the frame is drawn at the settled exposure, not the reset one.
    let want = law::composite(colour, [0.0; 3], first, p.bloom_factor);
    let reset = law::composite(colour, [0.0; 3], law::RESET_EXPOSURE, p.bloom_factor);
    assert!(
        (i32::from(pixel[0]) - byte(want[0])).abs() <= 3,
        "drew {}, law {}",
        pixel[0],
        byte(want[0])
    );
    assert!(
        (byte(want[0]) - byte(reset[0])).abs() > 5,
        "settled and reset pictures differ"
    );
}

mod split {
    use super::*;
    use crate::split_fixture::{SIZE, device, frame, paint, read_red};

    /// The mean of the middle half of the columns of `rows`. The vignette's top corners and
    /// edge are left out of a flat picture's mean by taking the rows below the mesh's inner
    /// circle (`w` is 0 there), which the views and the picture alone reach at different
    /// rows: a view's top is its own.
    fn mean(red: &[u8], rows: std::ops::Range<u32>) -> f64 {
        let mut sum = 0.0;
        let mut count = 0.0;
        for y in rows {
            for x in SIZE.0 / 4..SIZE.0 * 3 / 4 {
                sum += f64::from(red[(y * SIZE.0 + x) as usize]);
                count += 1.0;
            }
        }
        sum / count
    }

    fn dark_over_bright(_x: u32, y: u32) -> f32 {
        if y < SIZE.1 / 2 { 0.25 } else { 0.75 }
    }

    fn held() -> Params {
        Params {
            exposure: Exposure::Held(1.0),
            ..mall()
        }
    }

    /// One whole picture of a single brightness, run as a picture of its own.
    fn alone(device: &wgpu::Device, queue: &wgpu::Queue, level: f32) -> f64 {
        let chain = Chain::new(device, FORMAT, SIZE, held()).expect("pipelines build");
        paint(queue, chain.scene_texture(), |_, _| level);
        let output = frame(device);
        let target = output.create_view(&Default::default());
        let mut encoder = device.create_command_encoder(&Default::default());
        chain.run(queue, &mut encoder, &target, (0.0, 0.0), SIZE);
        queue.submit(Some(encoder.finish()));
        mean(
            &read_red(device, queue, &output),
            SIZE.1 / 4 + 2..SIZE.1 - 2,
        )
    }

    /// Each view of a split resolves as the same picture would alone, from its own
    /// rectangle of the scene, and the seats' lanes are built once, kept, retired one at a
    /// time and all at once.
    #[test]
    fn each_view_resolves_as_it_would_alone_and_its_lane_is_kept() {
        let Some((device, queue)) = device() else {
            return;
        };
        let chain = Chain::new(&device, FORMAT, SIZE, held()).expect("pipelines build");
        paint(&queue, chain.scene_texture(), dark_over_bright);
        let output = frame(&device);
        let target = output.create_view(&Default::default());
        let half = SIZE.1 / 2;
        let draw_views = || {
            for (seat, y) in [(0, 0), (1, half)] {
                let mut encoder = device.create_command_encoder(&Default::default());
                chain.run_view(
                    &device,
                    &queue,
                    &mut encoder,
                    &target,
                    View {
                        seat,
                        rect: (0, y, SIZE.0, half),
                        first: seat == 0,
                    },
                );
                queue.submit(Some(encoder.finish()));
            }
        };
        draw_views();
        let red = read_red(&device, &queue, &output);
        let top = mean(&red, half / 4 + 2..half - 2);
        let bottom = mean(&red, half + half / 4 + 2..SIZE.1 - 2);
        let dark = alone(&device, &queue, 0.25);
        let bright = alone(&device, &queue, 0.75);
        assert!(
            (top - dark).abs() < 2.0,
            "the dark view resolved to {top}, the same picture alone to {dark}"
        );
        assert!(
            (bottom - bright).abs() < 2.0,
            "the bright view resolved to {bottom}, the same picture alone to {bright}"
        );
        assert!(
            bottom > top + 10.0,
            "the views differ: {top} against {bottom}"
        );
        assert_eq!((chain.seat_lanes(), chain.seat_lane_builds()), (2, 2));
        draw_views();
        assert_eq!((chain.seat_lanes(), chain.seat_lane_builds()), (2, 2));
        chain.retire_seat(1);
        assert_eq!(chain.seat_lanes(), 1);
        draw_views();
        assert_eq!((chain.seat_lanes(), chain.seat_lane_builds()), (2, 3));
        chain.retire_seats();
        assert_eq!(chain.seat_lanes(), 0);
    }
}
