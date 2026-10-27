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
    let row = 256;
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
    let at = ((SIZE.1 / 2) * row + (SIZE.0 / 2) * 4) as usize;
    let exposure = f32::from_le_bytes([words[0], words[1], words[2], words[3]]);
    (
        [bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]],
        exposure,
    )
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
