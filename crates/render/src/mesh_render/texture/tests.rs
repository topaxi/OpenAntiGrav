//! A texture wider than the device allows is uploaded from the first level
//! that fits, or not at all - never a validation panic.
//!
//! Every GPU test skips without an adapter, the guard the rest of this crate's
//! GPU tests use.

use super::*;
use crate::mesh::BlockFormat;

/// A device whose `max_texture_dimension_2d` is exactly `limit`, below wgpu's
/// default, so a texture that fits the default 8,192 still has to be refused
/// or trimmed here. `None` without an adapter.
fn small_device(limit: u32, blocks: bool) -> Option<(wgpu::Device, wgpu::Queue)> {
    let instance = wgpu::Instance::default();
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
            .ok()?;
    let features = if blocks {
        adapter.features() & wgpu::Features::TEXTURE_COMPRESSION_BC
    } else {
        wgpu::Features::empty()
    };
    if blocks && features.is_empty() {
        return None;
    }
    pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        required_features: features,
        required_limits: wgpu::Limits {
            max_texture_dimension_2d: limit,
            ..wgpu::Limits::default()
        },
        ..Default::default()
    }))
    .ok()
}

#[test]
fn the_first_level_that_fits_is_the_first_whose_sides_both_fit() {
    // 16384 x 16384 against 8192: level 1 is 8192 square.
    assert_eq!(first_fitting_level(16384, 16384, 15, 8192, 1), Some(1));
    // Both sides count: a wide strip waits for its long side.
    assert_eq!(first_fitting_level(4096, 8, 13, 1024, 1), Some(2));
    assert_eq!(first_fitting_level(512, 512, 10, 8192, 1), Some(0));
    // A one-level chain has nothing to fall back to.
    assert_eq!(first_fitting_level(16384, 16384, 1, 8192, 1), None);
    // The chain runs out before a level fits.
    assert_eq!(first_fitting_level(16384, 16384, 2, 4096, 1), None);
    // BC wants a whole number of blocks at the chosen base: 12 -> 6 is not.
    assert_eq!(first_fitting_level(12, 12, 4, 8, 4), None);
    assert_eq!(first_fitting_level(24, 24, 5, 16, 4), Some(1));
}

#[test]
fn a_chain_over_the_limit_goes_up_from_the_level_that_fits() {
    let Some((device, queue)) = small_device(1024, false) else {
        return;
    };
    // 2048 x 4 and its levels down to 1 x 1, one byte per channel.
    let levels: Vec<Vec<u8>> = (0..12)
        .map(|level| {
            let (w, h) = ((2048u32 >> level).max(1), (4u32 >> level).max(1));
            vec![200; (w * h * 4) as usize]
        })
        .collect();
    let texture = ModelTexture::chain("over".into(), 2048, 4, levels);
    let placed = upload(&device, &queue, &texture, false).expect("level 1 fits");
    assert_eq!(placed.dropped_levels, 1);
    assert_eq!(placed.view.texture().width(), 1024);
    assert_eq!(placed.view.texture().mip_level_count(), 11);
    assert_eq!(
        plan(&texture, false, 1024),
        Some((1, placed.gpu_bytes)),
        "the census mirror says what the upload did"
    );
}

#[test]
fn a_single_level_over_the_limit_is_refused_not_panicked() {
    let Some((device, queue)) = small_device(1024, false) else {
        return;
    };
    let texture = ModelTexture::chain("one level".into(), 2048, 4, vec![vec![0; 2048 * 4 * 4]]);
    assert!(upload(&device, &queue, &texture, false).is_none());
    assert_eq!(plan(&texture, false, 1024), None);
}

#[test]
fn rgba_over_the_limit_keeps_its_synthesised_chain_from_the_level_that_fits() {
    let Some((device, queue)) = small_device(1024, false) else {
        return;
    };
    let texture = ModelTexture::rgba8("rgba".into(), 2048, 2, vec![90; 2048 * 2 * 4], None);
    let placed = upload(&device, &queue, &texture, false).expect("level 1 fits");
    assert_eq!(placed.dropped_levels, 1);
    assert_eq!(placed.view.texture().width(), 1024);
}

#[test]
fn blocks_over_the_limit_go_up_as_blocks_from_the_level_that_fits() {
    let Some((device, queue)) = small_device(1024, true) else {
        return;
    };
    // BC7, 2048 x 8: 16 bytes a 4x4 block, levels 2048x8 .. 1x1 (12 of them).
    let levels: Vec<Vec<u8>> = (0..12)
        .map(|level| {
            let (w, h) = ((2048u32 >> level).max(1), (8u32 >> level).max(1));
            vec![0; (w.div_ceil(4) * h.div_ceil(4) * 16) as usize]
        })
        .collect();
    let texture = ModelTexture {
        label: "bc7".into(),
        width: 2048,
        height: 8,
        texels: Texels::Blocks {
            format: BlockFormat::Bc7,
            levels,
        },
        mip_count: None,
    };
    let placed = upload(&device, &queue, &texture, true).expect("level 1 fits");
    assert_eq!(placed.dropped_levels, 1);
    assert_eq!(placed.view.texture().width(), 1024);
    assert_eq!(placed.view.texture().mip_level_count(), 11);
    assert_eq!(plan(&texture, true, 1024), Some((1, placed.gpu_bytes)));
}

#[test]
fn the_device_descriptor_asks_for_the_adapters_own_texture_limit() {
    let instance = wgpu::Instance::default();
    let Ok(adapter) =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
    else {
        return;
    };
    let descriptor = crate::mesh_render::device_descriptor("limits", &adapter);
    assert_eq!(
        descriptor.required_limits.max_texture_dimension_2d,
        adapter.limits().max_texture_dimension_2d
    );
    let Ok((device, _queue)) = pollster::block_on(adapter.request_device(&descriptor)) else {
        return;
    };
    assert_eq!(
        device.limits().max_texture_dimension_2d,
        adapter.limits().max_texture_dimension_2d,
        "the device holds what the adapter offered"
    );
    // The point of it: wgpu's default would have been 8,192 on every adapter
    // that offers more.
    assert!(
        adapter.limits().max_texture_dimension_2d <= 8192
            || device.limits().max_texture_dimension_2d > 8192
    );
}
