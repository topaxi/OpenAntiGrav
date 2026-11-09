//! The resolve's half-texel read: each output pixel is the mean of four scene texels in
//! linear light, with the offset taken against the drawn rectangle at a dynamic width.
//!
//! The scene is a one-texel checker of black and white held as an encoded store (0 and 1
//! are their own encodings), so the right answer is known three ways: read on the texel
//! centres every pixel is black or white; filtered by the hardware on the encoded values
//! the mean decodes to `0.5^2.2`, far darker than light; averaged in linear light it is
//! the byte a flat linear `0.5` resolves to.

use super::*;
use crate::split_fixture::{SIZE, device, frame, paint, read_red};

/// A dynamic width: the drawn rectangle is a fraction of the scene target, as
/// `1300 / 1920` is. Not square, so a divisor taken from the wrong axis shows.
const VIEWPORT: (u32, u32) = (43, 41);

fn params(final_half_texel: bool, store: SceneStore) -> Params {
    Params {
        // No glow and no darkening: the resolve's scale is exactly one.
        tone_darkening_clamp: 0.0,
        final_half_texel,
        store,
        ..super::tests::params()
    }
}

/// The red bytes of a scene painted with `level`, resolved over `VIEWPORT`.
fn resolve(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    params: Params,
    level: impl Fn(u32, u32) -> f32,
) -> Vec<u8> {
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let chain =
        Chain::new(device, format, SIZE, params, Glow::Suppressed).expect("pipelines build");
    paint(queue, chain.scene_texture(), level);
    let output = frame(device);
    let target = output.create_view(&Default::default());
    let mut encoder = device.create_command_encoder(&Default::default());
    chain.run(queue, &mut encoder, &target, (0.0, 0.0), VIEWPORT, None);
    queue.submit(Some(encoder.finish()));
    read_red(device, queue, &output)
}

fn checker(x: u32, y: u32) -> f32 {
    if (x + y).is_multiple_of(2) { 0.0 } else { 1.0 }
}

/// Every pixel of the drawn rectangle whose four texels are all inside it: the last row
/// and column clamp to the edge instead.
fn interior(red: &[u8]) -> Vec<(u32, u32, u8)> {
    let mut out = Vec::new();
    for y in 0..VIEWPORT.1 - 1 {
        for x in 0..VIEWPORT.0 - 1 {
            out.push((x, y, red[(y * SIZE.0 + x) as usize]));
        }
    }
    out
}

#[test]
fn the_half_texel_averages_four_texels_in_linear_light() {
    let Some((device, queue)) = device() else {
        return;
    };
    let flat = resolve(
        &device,
        &queue,
        params(false, SceneStore::Computed),
        |_, _| 0.5,
    );
    let light = flat[(10 * SIZE.0 + 10) as usize];
    let shifted = resolve(&device, &queue, params(true, SceneStore::Encoded), checker);
    for (x, y, byte) in interior(&shifted) {
        assert!(
            byte.abs_diff(light) <= 1,
            "pixel ({x}, {y}) read {byte}, the mean of its four texels in light is {light}"
        );
    }
}

#[test]
fn without_the_tap_the_checker_reaches_the_screen_texel_for_texel() {
    let Some((device, queue)) = device() else {
        return;
    };
    let plain = resolve(&device, &queue, params(false, SceneStore::Encoded), checker);
    for (x, y, byte) in interior(&plain) {
        let want = if (x + y).is_multiple_of(2) { 0 } else { 255 };
        assert_eq!(byte, want, "pixel ({x}, {y})");
    }
}
