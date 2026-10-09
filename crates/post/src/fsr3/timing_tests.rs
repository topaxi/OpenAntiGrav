//! The two timestamp pairs [`super::ChainTimestamps`] carries, on a real
//! device.
//!
//! Its own file beside [`super::tests`] for the reason `reset_tests.rs` is
//! one: `tests.rs` is already long, and this needs a device asked for
//! `TIMESTAMP_QUERY` where every test there takes a default one.
//!
//! **What this is for.** The split exists so a dynamic-resolution budget can
//! divide by the half of the chain that moves with the render extent
//! (ADR-0045), and a split that reports one number and a blank is worse than
//! the single pair it replaced - a claimed-and-unwritten pair does not read
//! back as zero. So the assertion is that *both* halves come back, naming the
//! same frame, on the same submission. The numbers themselves are printed
//! rather than bounded: this machine is not the Steam Deck the reading is
//! wanted from, and a threshold here would only assert how fast the developer's
//! adapter is.

use super::*;
use oag_gpu::timing::{PassTimer, Timing};

#[test]
fn both_halves_of_the_chain_read_back_naming_their_own_frame() {
    let instance = wgpu::Instance::default();
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&Default::default())) else {
        eprintln!("no adapter; skipping");
        return;
    };
    let timing = Timing::of(&adapter);
    if !timing.can_time_a_pass() {
        eprintln!("{}; skipping", timing.describe());
        return;
    }
    if !supported(&adapter) {
        eprintln!("no compute shaders; skipping");
        return;
    }
    let Ok((device, queue)) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("fsr3 timing test"),
        required_features: timing.features(),
        ..Default::default()
    })) else {
        eprintln!("no device; skipping");
        return;
    };

    let mut fsr3 = Fsr3::new(&device).expect("the FSR 3.1 shaders must compile");
    let mut scaled = PassTimer::new(&device, &queue).expect("a timer on a device that has the bit");
    let mut presented =
        PassTimer::new(&device, &queue).expect("a timer on a device that has the bit");

    let render = (128u32, 64u32);
    let upscale = (256u32, 128u32);
    let texture = |label, format, usage| {
        device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d {
                    width: render.0,
                    height: render.1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage,
                view_formats: &[],
            })
            .create_view(&wgpu::TextureViewDescriptor::default())
    };
    let usage = wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT;
    let colour = texture("scene", wgpu::TextureFormat::Rgba16Float, usage);
    let depth = texture("depth", wgpu::TextureFormat::Depth32Float, usage);
    let velocity = texture("velocity", oag_gpu::formats::VELOCITY_FORMAT, usage);

    const FRAME: u64 = 5;
    scaled.begin(FRAME);
    presented.begin(FRAME);
    let mut encoder = device.create_command_encoder(&Default::default());
    let encoded = fsr3.render(
        &device,
        &queue,
        &mut encoder,
        Frame {
            colour: &colour,
            depth: &depth,
            velocity: &velocity,
            dispatch: super::tests::dispatch(render, upscale),
        },
        Some(ChainTimestamps {
            scaled: scaled.compute_writes().expect("a slot was just claimed"),
            presented: presented.compute_writes().expect("a slot was just claimed"),
        }),
    );
    assert!(encoded, "the chain reported that it encoded nothing");
    scaled.resolve(&mut encoder);
    presented.resolve(&mut encoder);
    queue.submit([encoder.finish()]);

    // The frame loop takes what has arrived; a test has nothing to pace it, so
    // it asks until both are there. The bound is what makes a never-arriving
    // reading a failure rather than a hang - the same shape as
    // `timing::timer::tests::time_a_pass`.
    let mut readings = (None, None);
    for _ in 0..1_000 {
        device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("the GPU");
        readings.0 = readings.0.or_else(|| scaled.read(&device));
        readings.1 = readings.1.or_else(|| presented.read(&device));
        if readings.0.is_some() && readings.1.is_some() {
            break;
        }
    }
    let (Some(scaled), Some(presented)) = readings else {
        panic!("one half of the chain never came back: {readings:?}");
    };

    assert_eq!(scaled.frame, FRAME, "the scaled half named the wrong frame");
    assert_eq!(
        presented.frame, FRAME,
        "the presented half named the wrong frame"
    );
    // Loose on purpose, exactly as the timer's own probe is: the claim is that
    // both clocks moved and are in seconds, not that this machine is fast.
    assert!(
        scaled.seconds > 0.0 && scaled.seconds < 60.0,
        "the six render-resolution dispatches measured {} s",
        scaled.seconds
    );
    assert!(
        presented.seconds > 0.0 && presented.seconds < 60.0,
        "accumulate and rcas measured {} s",
        presented.seconds
    );
    eprintln!(
        "fsr3 at {}x{} -> {}x{}: scaled {:.4} ms, presented {:.4} ms",
        render.0,
        render.1,
        upscale.0,
        upscale.1,
        scaled.seconds * 1000.0,
        presented.seconds * 1000.0,
    );
}
