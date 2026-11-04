//! The bloom chain's law, and the chain on a real device. The device tests skip when
//! there is no adapter, so a green CI run is not evidence that they ran.

use super::law::{Curve, scanout_encode};
use super::tests::{params, read_pixel};
use super::*;

/// Vineta K's block, the disc's numbers.
const VINETA: Bloom = Bloom {
    threshold: 0.784,
    offset: 0.019,
    scale: 0.918,
    power: 3.619,
    alpha_scale: 0.237,
    blend_factors: [1.23, 0.95, 0.9, 0.85],
};

/// Keeps a scene of ones and more above every threshold whatever the curve does to it.
const GENEROUS: Bloom = Bloom {
    threshold: 0.02,
    offset: 0.1,
    scale: 1.0,
    power: 1.0,
    alpha_scale: 0.0,
    blend_factors: [2.0, 2.0, 2.0, 2.0],
};

/// The weights are what the immediates claim to be: a blur that keeps a flat image flat,
/// the same-size one being the twelfth row of Pascal's triangle over 4096.
#[test]
fn the_blurs_keep_a_flat_image_flat() {
    let sum = |weights: &[f32]| weights[0] + 2.0 * weights[1..].iter().sum::<f32>();
    assert!((sum(&law::BLUR_WEIGHTS) - 1.0).abs() < 1e-6);
    assert!((sum(&law::DOWN_WEIGHTS) - 1.0).abs() < 1e-6);
    assert_eq!(law::BLUR_WEIGHTS[0], 924.0 / 4096.0);
    assert_eq!(law::BLUR_WEIGHTS[1], (792.0 + 495.0) / 4096.0);
    assert_eq!(law::BLUR_WEIGHTS[2], (220.0 + 66.0) / 4096.0);
    assert_eq!(law::BLUR_WEIGHTS[3], (12.0 + 1.0) / 4096.0);
    let folded = 1.0 + 495.0 / (792.0 + 495.0);
    assert!((law::BLUR_OFFSETS[1] - folded).abs() < 1e-5);
}

/// Each level is half the last, rounding down, from the executable's 960x540, and a frame
/// of another shape keeps level 0's height.
#[test]
fn the_levels_halve_from_nine_sixty_by_five_forty() {
    assert_eq!(law::bloom_base((1920, 1080)), (960, 540));
    assert_eq!(law::bloom_base((3840, 2160)), (960, 540));
    assert_eq!(law::bloom_base((1280, 800)), (864, 540));
    let sizes: Vec<_> = (0..law::BLOOM_LEVELS)
        .map(|level| law::bloom_level_size(law::BLOOM_BASE, level))
        .collect();
    assert_eq!(sizes, [(960, 540), (480, 270), (240, 135), (120, 67)]);
}

/// `Scale * p / (Offset + p) + AlphaScale * mask` over the threshold, on the mean of the
/// channels and not Rec.601; a colour at or under the threshold is untouched by the first
/// term, and Sol 2's threshold above one never fires on the curve's `[0, 1]`.
#[test]
fn the_bright_pass_is_the_programs_law() {
    let l: f32 = 1.0;
    let p = (l - 0.784).powf(3.619);
    let want = 0.918 * p / (0.019 + p);
    assert!((VINETA.bright_weight(1.0, 0.0) - want).abs() < 1e-6);
    assert!((VINETA.bright_weight(1.0, 1.0) - (want + 0.237)).abs() < 1e-6);
    assert_eq!(VINETA.bright_weight(0.784, 0.0), 0.0);
    assert_eq!(VINETA.bright_pass([1.0, 0.8, 0.0]), [0.0; 3]);
    let sol = Bloom {
        threshold: 1.252,
        ..VINETA
    };
    assert_eq!(sol.bright_pass([1.0, 1.0, 1.0]), [0.0; 3]);
}

/// The levels add coarse to fine, each carrying the sum below it.
#[test]
fn the_levels_add_up_nested() {
    let sum = VINETA.blend_up([1.0, 2.0, 4.0, 8.0]);
    let want = 1.0 + 0.95 * (2.0 + 0.9 * (4.0 + 0.85 * 8.0));
    assert!((sum - want).abs() < 1e-5);
}

fn device() -> Option<(wgpu::Device, wgpu::Queue)> {
    let instance = wgpu::Instance::default();
    let Some(adapter) = oag_gpu::test_adapter::request(&instance) else {
        eprintln!("no GPU adapter: skipping");
        return None;
    };
    Some(
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("requesting the device"),
    )
}

fn output(device: &wgpu::Device, size: (u32, u32)) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("out"),
        size: wgpu::Extent3d {
            width: size.0,
            height: size.1,
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

fn chain(device: &wgpu::Device, size: (u32, u32), bloom: Option<Bloom>) -> Chain {
    let p = Params {
        composite: Composite {
            bloom,
            ..Composite::NONE
        },
        ..params()
    };
    Chain::new(device, wgpu::TextureFormat::Rgba8Unorm, size, p).expect("the chain builds")
}

/// Fills the scene, `bright` for columns under `split` and `dark` for the rest.
fn paint(queue: &wgpu::Queue, chain: &Chain, split: u32, bright: f32, dark: f32) {
    let texture = chain.scene_texture();
    let size = texture.size();
    let half = |v: f32| half_float(v).to_le_bytes();
    let mut bytes = Vec::new();
    for _ in 0..size.height {
        for x in 0..size.width {
            let v = if x < split { bright } else { dark };
            for channel in [v, v, v, 1.0] {
                bytes.extend_from_slice(&half(channel));
            }
        }
    }
    queue.write_texture(
        texture.as_image_copy(),
        &bytes,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(size.width * 8),
            rows_per_image: None,
        },
        size,
    );
}

/// An `f32` in `[0, 2)` or zero as the half float the scene target stores.
fn half_float(v: f32) -> u16 {
    if v == 0.0 {
        return 0;
    }
    let bits = v.to_bits();
    let exponent = ((bits >> 23) & 0xff) as i32 - 127 + 15;
    ((exponent as u16) << 10) | ((bits >> 13) & 0x3ff) as u16
}

/// A flat scene stays flat through every blur, so the composite's sum is the law's exactly:
/// the curve, plus `BloomScale` times the nested sum of the bright pass on the curved
/// colour. Drop the bloom term from `fs_apply`, or a level's add, and this fails.
#[test]
fn a_flat_frame_blooms_by_the_nested_sum() {
    let Some((device, queue)) = device() else {
        return;
    };
    let size = (16, 9);
    let chain = chain(&device, size, Some(GENEROUS));
    let out = output(&device, size);
    let view = out.create_view(&Default::default());
    let colour = 0.3_f32;
    paint(&queue, &chain, size.0, colour, colour);
    let mut encoder = device.create_command_encoder(&Default::default());
    chain.run(&device, &queue, &mut encoder, &view, (0.0, 0.0), size, None);
    queue.submit([encoder.finish()]);

    let curve = Curve::fit(&chain.params(), colour);
    let curved = curve.map(colour);
    let bright = GENEROUS.bright_pass([curved; 3])[0];
    assert!(bright > 0.01, "the fixture must clear the threshold");
    let bloom = GENEROUS.blend_up([bright; 4]);
    let want = chain
        .params()
        .composite
        .apply_with_bloom(0, curved, bloom, [0.5, 0.5]);
    let want = (scanout_encode(want) * 255.0).round() as i32;
    let encoder = device.create_command_encoder(&Default::default());
    let got = i32::from(read_pixel(&device, &queue, encoder, &out, (8, 4))[0]);
    assert!((got - want).abs() <= 2, "drew {got}, the law says {want}");
    let plain = (scanout_encode(curved) * 255.0).round() as i32;
    assert!(
        want > plain + 8,
        "the fixture must show the term: {want} against {plain}"
    );
}

/// A resize to another shape rebuilds the levels and the composite reads the new ones: a
/// flat frame still blooms by the same sum.
#[test]
fn a_resize_to_another_shape_keeps_the_bloom() {
    let Some((device, queue)) = device() else {
        return;
    };
    let mut chain = chain(&device, (16, 9), Some(GENEROUS));
    let size = (16, 10);
    chain.resize(&device, size);
    assert!(chain.levels.as_ref().is_some_and(|l| l.fits(size)));
    let out = output(&device, size);
    let view = out.create_view(&Default::default());
    let colour = 0.3_f32;
    paint(&queue, &chain, size.0, colour, colour);
    let mut encoder = device.create_command_encoder(&Default::default());
    chain.run(&device, &queue, &mut encoder, &view, (0.0, 0.0), size, None);
    queue.submit([encoder.finish()]);
    let curved = Curve::fit(&chain.params(), colour).map(colour);
    let bloom = GENEROUS.blend_up([GENEROUS.bright_pass([curved; 3])[0]; 4]);
    let want = chain
        .params()
        .composite
        .apply_with_bloom(0, curved, bloom, [0.5, 0.5]);
    let want = (scanout_encode(want) * 255.0).round() as i32;
    let encoder = device.create_command_encoder(&Default::default());
    let got = i32::from(read_pixel(&device, &queue, encoder, &out, (8, 5))[0]);
    assert!((got - want).abs() <= 2, "drew {got}, the law says {want}");
}

/// Bright pixels bleed over an edge into black, falling away from it; with no `Bloom`
/// block the black stays black, as it did before the chain existed.
#[test]
fn bloom_bleeds_past_an_edge_and_only_with_a_block() {
    let Some((device, queue)) = device() else {
        return;
    };
    let size = (64, 36);
    let seam = 32;
    let read_across = |bloom: Option<Bloom>| {
        let chain = chain(&device, size, bloom);
        let out = output(&device, size);
        let view = out.create_view(&Default::default());
        paint(&queue, &chain, seam, 1.5, 0.0);
        let mut encoder = device.create_command_encoder(&Default::default());
        chain.run(&device, &queue, &mut encoder, &view, (0.0, 0.0), size, None);
        queue.submit([encoder.finish()]);
        let mut row = Vec::new();
        for x in [seam + 1, seam + 3, seam + 8, seam + 30] {
            let encoder = device.create_command_encoder(&Default::default());
            row.push(read_pixel(&device, &queue, encoder, &out, (x, 18))[0]);
        }
        row
    };
    let plain = read_across(None);
    assert_eq!(plain, [0, 0, 0, 0], "no block, no bloom");
    let bloomed = read_across(Some(GENEROUS));
    assert!(bloomed[0] > 30, "the edge glows: {bloomed:?}");
    assert!(
        bloomed.windows(2).all(|pair| pair[0] >= pair[1]) && bloomed[0] > bloomed[3],
        "the glow falls away from the edge: {bloomed:?}"
    );
}

/// A split view's bloom is its own: the dark view next to a bright one draws no glow,
/// though the two share the level targets, and the bright view blooms as a whole frame does.
#[test]
fn a_split_view_blooms_alone() {
    let Some((device, queue)) = device() else {
        return;
    };
    let size = (64, 36);
    let half = size.0 / 2;
    let chain = chain(&device, size, Some(GENEROUS));
    let out = output(&device, size);
    let view = out.create_view(&Default::default());
    paint(&queue, &chain, half, 1.5, 0.0);
    for (seat, rect, first) in [
        (0, (0, 0, half, size.1), true),
        (1, (half, 0, half, size.1), false),
    ] {
        let mut encoder = device.create_command_encoder(&Default::default());
        chain.run_view(
            &device,
            &queue,
            &mut encoder,
            &view,
            View { seat, rect, first },
        );
        queue.submit([encoder.finish()]);
    }
    let read = |x| {
        let encoder = device.create_command_encoder(&Default::default());
        read_pixel(&device, &queue, encoder, &out, (x, 18))[0]
    };
    assert_eq!(
        read(half + 1),
        0,
        "the dark view reads none of its neighbour"
    );
    assert!(read(half / 2) > 0, "the bright view draws");
}
