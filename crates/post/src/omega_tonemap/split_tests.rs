//! The chain run per view of a split: each seat adapts to its own pixels. The same shape as
//! `hd_bloom::split_tests`: two views of one size, one dark and one bright, each resolved as
//! the same content would be alone.

use super::*;
use crate::split_fixture::{SIZE, device, frame, mean, paint, read_red};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

fn dark_over_bright(_x: u32, y: u32) -> f32 {
    if y < SIZE.1 / 2 { 0.25 } else { 0.75 }
}

fn params() -> Params {
    Params {
        luminance_a: 0.18,
        luminance_b: 0.05,
        exposure_minimum: 0.25,
        exposure_maximum: 4.0,
        exposure_response: 3.0,
        exposure_time: 4.0,
        source_end_a: 2.0,
        source_end_b: 0.5,
        start_angle: 10.0,
        end_angle: -20.0,
    }
}

/// One whole picture of a single brightness, run as a picture of its own.
fn alone(device: &wgpu::Device, queue: &wgpu::Queue, level: f32) -> f64 {
    let chain = Chain::new(device, FORMAT, SIZE, params()).expect("pipelines build");
    paint(queue, chain.scene_texture(), |_, _| level);
    let output = frame(device);
    let target = output.create_view(&Default::default());
    let mut encoder = device.create_command_encoder(&Default::default());
    chain.run(device, queue, &mut encoder, &target, (0.0, 0.0), SIZE, None);
    queue.submit(Some(encoder.finish()));
    mean(&read_red(device, queue, &output), 0..SIZE.1)
}

/// Each view resolves as it would alone: a shared adaptation (one luminance for the frame)
/// puts both views on the exposure of their mean; a scene tap that ignores the view's offset
/// resolves the bottom view from the top view's pixels; a uniform keyed on the rectangle's
/// size alone gives the second view the first's rectangle.
#[test]
fn each_view_resolves_as_it_would_alone() {
    let Some((device, queue)) = device() else {
        return;
    };
    let chain = Chain::new(&device, FORMAT, SIZE, params()).expect("pipelines build");
    paint(&queue, chain.scene_texture(), dark_over_bright);
    let output = frame(&device);
    let target = output.create_view(&Default::default());
    let half = SIZE.1 / 2;
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
    let red = read_red(&device, &queue, &output);
    let top = mean(&red, 2..half - 2);
    let bottom = mean(&red, half + 2..SIZE.1 - 2);
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
    assert_eq!(chain.seat_lanes(), 2);
    chain.retire_seats();
    assert_eq!(chain.seat_lanes(), 0);
}
