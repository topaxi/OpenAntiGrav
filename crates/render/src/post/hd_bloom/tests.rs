//! [`Chain::run`]'s timestamp pair, end to end.
//!
//! Needs a real device and a real readback, on the same terms
//! `crates/render/src/timing/timer/tests.rs`'s own GPU test does: every
//! adapter that claims the feature, skipping those that do not. What this
//! pins is the wiring alone - that the opening write lands on the chain's
//! first pass and the closing one on its last, and that a claimed slot
//! resolves to a positive duration naming the right frame - not the chain's
//! visual output, which is a different kind of test.

use super::*;

/// One circuit's worth of parameters, none of it load-bearing for this test -
/// only that the numbers are finite, so the constants buffer round-trips
/// without producing NaN uniforms a validation layer would reject.
fn params() -> Params {
    Params {
        alpha_contribution: 1.0,
        frame_contribution: 1.0,
        frame_exponent: 1.0,
        horizontal_size: 1.0,
        vertical_size: 1.0,
        adaption_rate: 0.1,
        adaption_boost: 1.0,
        tone_adaption_boost: 1.0,
        tone_darkening_clamp: 1.0,
        tone_maximum_brightness: 1.0,
    }
}

/// The chain's own timestamp pair reads back a positive duration naming the
/// frame it was claimed for, on both the first frame and a second one - the
/// second is what proves a slot is reusable rather than only that claiming
/// one once works, the same reason `timing::timer::tests` runs its own probe
/// twice.
#[test]
fn the_chain_reads_back_naming_its_own_frame() {
    let instance = wgpu::Instance::default();
    let adapters = pollster::block_on(instance.enumerate_adapters(wgpu::Backends::PRIMARY));
    if adapters.is_empty() {
        eprintln!("no GPU adapter: skipping");
        return;
    }
    for adapter in &adapters {
        let info = adapter.get_info();
        let timing = crate::timing::Timing::of(adapter);
        if !timing.can_time_a_pass() {
            eprintln!("{}: {}, skipping", info.name, timing.describe());
            continue;
        }
        time_the_chain(adapter, &info.name);
    }
}

fn time_the_chain(adapter: &wgpu::Adapter, name: &str) {
    let features = crate::timing::Timing::of(adapter).features();
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("hd bloom timing test"),
        required_features: features,
        ..Default::default()
    }))
    .expect("a device with what was probed");

    let output_format = wgpu::TextureFormat::Rgba8Unorm;
    let size = (16u32, 16u32);
    let chain = Chain::new(&device, output_format, size, params()).expect("the pipelines build");
    let output = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("hd bloom timing test output"),
        size: wgpu::Extent3d {
            width: size.0,
            height: size.1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: output_format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let view = output.create_view(&Default::default());

    let mut timer = crate::timing::PassTimer::new(&device, &queue)
        .expect("a timer on a device that has the bit");

    for frame in [21u64, 22] {
        timer.begin(frame);
        let mut encoder = device.create_command_encoder(&Default::default());
        // The chain reads whatever `scene_view()` last had drawn into it -
        // its own cleared-on-creation contents here, since this test is
        // about the timing wiring and not the picture.
        let timestamps = Some(ChainTimestamps {
            begin: timer
                .half_writes(crate::timing::Half::Begin)
                .expect("claimed above"),
            end: timer
                .half_writes(crate::timing::Half::End)
                .expect("claimed above"),
        });
        chain.run(&queue, &mut encoder, &view, size, timestamps);
        timer.resolve(&mut encoder);
        queue.submit(Some(encoder.finish()));

        let mut reading = None;
        for _ in 0..1_000 {
            device
                .poll(wgpu::PollType::wait_indefinitely())
                .expect("the GPU");
            if let Some(got) = timer.read(&device) {
                reading = Some(got);
                break;
            }
        }
        let reading = reading.unwrap_or_else(|| panic!("{name}: frame {frame} never came back"));
        assert_eq!(
            reading.frame, frame,
            "{name}: the reading named the wrong frame"
        );
        assert!(
            reading.seconds > 0.0 && reading.seconds < 1.0,
            "{name}: the chain measured {} s",
            reading.seconds
        );
        eprintln!(
            "{name}: hd bloom chain, frame {frame}: {:.3} ms",
            reading.seconds * 1000.0
        );
    }
}
