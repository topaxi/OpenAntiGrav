//! The backdrop's passes on a real device: what a blended pass adds, what the
//! field's alpha holds, and that the picture ends in the bloom chain.

use oag_rcs::points2::Point;

use super::*;

const SIZE: u32 = 64;
const CENTRE: usize = 33;

/// `count` points stacked at the view axis, five units down it.
fn cloud(count: usize) -> PointCloud {
    let point = Point {
        position: [0.0, 0.0, -5.0],
        rand: [0; 4],
        normal: [0.0, 0.0, 1.0],
    };
    PointCloud {
        min: [0.0; 3],
        max: [0.0; 3],
        points: vec![point; count],
    }
}

/// A frame with no ramp, no fog, no trail and no glow: the sprites alone.
fn frame() -> Frame {
    let identity = [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ];
    let flat = [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 0.0, 0.0],
        [0.0, 0.0, 0.5, 1.0],
    ];
    Frame {
        cloud: 0,
        world_view: identity,
        proj: flat,
        sprite_size: 0.25,
        music_multiplier: 0.0,
        particle_colour: [1.0, 1.0, 1.0],
        colour_ramp: [0.0; 3],
        colour_ramp_factors: [0.0, 0.08, 0.5, 0.0],
        colour_ramp_factors2: [4.0, -3.0, 6.0, 0.0],
        depth_fade_factors: [0.5, -0.5],
        dof_factors: [-100.0, -1.0, 24.0, 1.0],
        fog_factors: [-100.0, -1.0, -200.0, 1.0],
        feedback: [0.0; 3],
        glow_gain: 0.0,
        tint: [1.0; 4],
        pivot_offset: [0.0; 2],
        band_top: 9.0,
        clock: 0,
    }
}

/// An exposure of one and no bloom, unless a test asks for otherwise.
fn plain() -> hd_bloom::Params {
    hd_bloom::Params {
        alpha_contribution: 0.0,
        frame_contribution: 0.0,
        frame_exponent: 2.0,
        horizontal_size: 0.5,
        vertical_size: 0.5,
        adaption_rate: 0.025,
        adaption_boost: 8.75,
        tone_adaption_boost: 0.0,
        tone_darkening_clamp: 0.0,
        tone_maximum_brightness: 1.0,
        feedback: 0.0,
        final_half_texel: false,
        eight_bit: false,
        zoom: None,
        store: oag_gpu::formats::SceneStore::Computed,
    }
}

fn read(device: &wgpu::Device, queue: &wgpu::Queue, texture: &wgpu::Texture) -> Vec<u8> {
    let row = (SIZE * 4).next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("fury test readback"),
        size: u64::from(row * SIZE),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(row),
                rows_per_image: Some(SIZE),
            },
        },
        wgpu::Extent3d {
            width: SIZE,
            height: SIZE,
            depth_or_array_layers: 1,
        },
    );
    queue.submit(Some(encoder.finish()));
    let slice = readback.slice(..);
    slice.map_async(wgpu::MapMode::Read, |_| {});
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("the readback");
    let bytes = slice.get_mapped_range().expect("the mapped range");
    bytes
        .chunks(row as usize)
        .flat_map(|line| line[..(SIZE * 4) as usize].iter().copied())
        .collect()
}

/// `(finished field, what the chain wrote)` after one frame of `count` stacked
/// points; `None` on a machine with no adapter.
fn run(count: usize, bloom: hd_bloom::Params) -> Option<(Vec<u8>, Vec<u8>)> {
    run_with(count, (bloom, None), None)
}

/// [`run`] with the block a screen with models loads and what such a screen draws.
fn run_with(
    count: usize,
    bloom: (hd_bloom::Params, Option<hd_bloom::Params>),
    layer: Option<DrawSceneLayer<'_>>,
) -> Option<(Vec<u8>, Vec<u8>)> {
    let instance = wgpu::Instance::default();
    let adapter = oag_gpu::test_adapter::request(&instance)?;
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).ok()?;
    let mut fury = FuryBackdrop::new(
        &device,
        &queue,
        wgpu::TextureFormat::Rgba8Unorm,
        &[cloud(count)],
        bloom,
    )
    .expect("the backdrop");
    let mut encoder = device.create_command_encoder(&Default::default());
    fury.prepare(&device, &queue, &mut encoder, &frame(), (SIZE, SIZE), layer);
    queue.submit(Some(encoder.finish()));
    let targets = fury.targets.as_ref().expect("built by prepare");
    Some((
        read(&device, &queue, &targets.finished.texture),
        read(&device, &queue, &targets.field.texture),
    ))
}

fn red(pixels: &[u8], x: usize, y: usize) -> u8 {
    pixels[(y * SIZE as usize + x) * 4]
}

/// The original packs a fragment before the blend unit: two stacked sprites
/// store twice one sprite's encoded value. Blending in linear light would
/// store `2 ^ (1 / 2.4)` of it, 1.33.
#[test]
fn stacked_sprites_add_as_encoded_values() {
    let Some((one, _)) = run(1, plain()) else {
        return;
    };
    let (two, _) = run(2, plain()).expect("an adapter a moment ago");
    let (one, two) = (red(&one, CENTRE, CENTRE), red(&two, CENTRE, CENTRE));
    assert!(one > 20, "one sprite stored {one}");
    let ratio = f32::from(two) / f32::from(one);
    assert!(
        (1.8..=2.2).contains(&ratio),
        "{one} to {two}: ratio {ratio}"
    );
}

/// The point pass keeps the cleared destination alpha, and nothing after it
/// adds any, so the gate's `rgb * a` term is nothing on the menu.
#[test]
fn the_field_holds_no_alpha() {
    let Some((finished, _)) = run(4, plain()) else {
        return;
    };
    assert!(
        finished.chunks(4).all(|pixel| pixel[3] == 0),
        "the finished field has alpha"
    );
}

/// A picture is pushed up by the adapted exposure, `4 - min(20 * adapted, 3)`.
#[test]
fn a_picture_is_exposed_by_the_bloom_chain() {
    let adapted = hd_bloom::Params {
        tone_adaption_boost: 20.0,
        tone_darkening_clamp: 3.0,
        tone_maximum_brightness: 4.0,
        feedback: 0.0,
        ..plain()
    };
    let Some((_, flat)) = run(1, plain()) else {
        return;
    };
    let (_, exposed) = run(1, adapted).expect("an adapter a moment ago");
    let (flat, exposed) = (red(&flat, CENTRE, CENTRE), red(&exposed, CENTRE, CENTRE));
    assert!(
        f32::from(exposed) > f32::from(flat) * 1.3,
        "{flat} to {exposed}"
    );
}

/// A card filling the field with one encoded red, as a page's scene layer.
fn flat_card(stored: u8) -> impl FnMut(SceneLayer<'_>) {
    move |layer: SceneLayer<'_>| {
        layer
            .encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("fury test card"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: layer.view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: f64::from(stored) / 255.0,
                            g: 0.0,
                            b: 0.0,
                            a: 0.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
    }
}

/// The byte a flat encoded red `stored` leaves the chain at under `block`, by the chain's
/// own law: the first frame's adapted luma is `0.3` of the red read through a `1 / 2.2`
/// power, the exposure and the gate's fade follow it, and a flat field's bloom is the
/// gate's own value.
fn flat_card_resolved(stored: u8, block: &hd_bloom::Params) -> f32 {
    let store = oag_gpu::formats::SceneStore::Encoded;
    let light = store.texel_decode(f32::from(stored) / 255.0);
    let adapted = 0.3 * light.powf(1.0 / 2.2);
    let exposure = hd_bloom::exposure(
        adapted,
        block.tone_adaption_boost,
        block.tone_darkening_clamp,
        block.tone_maximum_brightness,
    );
    let fade = 1.0 - (adapted * block.adaption_boost * hd_bloom::ADAPTION_FADE_SCALE).min(1.0);
    let gate = (0.9 * light).powf(block.frame_exponent) * block.frame_contribution * fade;
    store.texel_encode(light * (exposure + gate)) * 255.0
}

/// A page's models are drawn into the finished field before the chain reads it, and that
/// frame runs under the block a screen with models loads: a flat card of the campaign
/// card's red (`63`) leaves the chain at that block's exposure and frame bloom, which is
/// neither the backdrop's own block's level nor the card's texel.
#[test]
fn a_scene_layer_is_exposed_under_the_block_a_model_screen_loads() {
    // `fe.fury.track.envsettings` and `fe.fury.EnvSettings`, the keys the chain reads.
    let track = hd_bloom::Params {
        alpha_contribution: 0.75,
        frame_contribution: 1.0,
        frame_exponent: 2.0,
        adaption_rate: 0.025,
        adaption_boost: 8.75,
        tone_adaption_boost: 20.0,
        tone_darkening_clamp: 3.0,
        tone_maximum_brightness: 4.0,
        feedback: 0.0,
        ..plain()
    };
    let models = hd_bloom::Params {
        frame_contribution: 0.5,
        frame_exponent: 0.0,
        adaption_rate: 0.015,
        adaption_boost: 38.75,
        tone_adaption_boost: 35.0,
        ..track
    };
    const CARD: u8 = 63;
    let mut card = flat_card(CARD);
    let Some((field, resolved)) = run_with(1, (track, Some(models)), Some(&mut card)) else {
        return;
    };
    assert_eq!(
        red(&field, CENTRE, CENTRE),
        CARD,
        "the card is in the field"
    );
    let got = f32::from(red(&resolved, CENTRE, CENTRE));
    let (want, under_track) = (
        flat_card_resolved(CARD, &models),
        flat_card_resolved(CARD, &track),
    );
    assert!(
        (got - want).abs() <= 2.0,
        "the card left the chain at {got}; the model screens' block gives {want:.1}, the backdrop's {under_track:.1}"
    );
    assert!(
        (want - under_track).abs() > 10.0 && (want - f32::from(CARD)).abs() > 5.0,
        "the two blocks and the texel are told apart: {want:.1}, {under_track:.1}, {CARD}"
    );
    // With no block for it, the layer is still drawn and the backdrop's own block stands.
    let mut card = flat_card(CARD);
    let (_, resolved) =
        run_with(1, (track, None), Some(&mut card)).expect("an adapter a moment ago");
    let got = f32::from(red(&resolved, CENTRE, CENTRE));
    assert!(
        (got - under_track).abs() <= 2.0,
        "with no model block the card left the chain at {got}, the backdrop's block gives {under_track:.1}"
    );
}
