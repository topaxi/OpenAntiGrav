//! A resume countdown on a title with no countdown art is text in that title's own
//! `HUD` face, one digit per view of a split, clear of the craft.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this project
//! does not ship; run it with `just test-data` (and a GPU adapter, else it skips).
//!
//! What only the real face can say: that `3`, `2`, `1` and the language's `GO` are all
//! glyphs of the title's loaded `HUD` face (a missing glyph draws nothing, silently),
//! and that the word lands in the HUD's own frame (a second pass would be overwritten
//! by the HUD's write) inside each view. Placement is **chosen, not measured**: the
//! original has no resume countdown.

use oag_hud::Readout;
use oag_render::viewport::{Cut, layout};
use oag_title::hud::ResumeCountdown;

const SIZE: (u32, u32) = (1440, 816);
const VIEWPORT: (f32, f32, f32, f32) = (0.0, 0.0, 1440.0, 816.0);

/// A text title's source, and what it is called in a failure message.
const TEXT_TITLES: [&str; 4] = [
    "data/images/pure-psp-eu.chd",
    "data/images/hdfury-ps3-eu-dec.iso",
    "data/extracted/vita/PCSF00007",
    "data/extracted/ps4",
];

fn hud(image: &str) -> Option<oag_hud::Assets> {
    let source = oag_testdata::image(image)?;
    let loaded = oag_raceplay::load(&oag_raceplay::Options {
        source: source.display().to_string(),
        views: 2,
        ..oag_raceplay::Options::default()
    })
    .unwrap_or_else(|e| panic!("{image}: loading a race: {e:#}"));
    Some(loaded.hud)
}

fn device() -> Option<(wgpu::Device, wgpu::Queue)> {
    let instance = wgpu::Instance::default();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).ok()?;
    pollster::block_on(adapter.request_device(&Default::default())).ok()
}

/// The HUD of two blank views, with `countdown` as the word, over transparent black.
fn pixels(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    overlay: &mut oag_game::hud_overlay::Overlay,
    countdown: Option<&str>,
    cut: Cut,
) -> Vec<u8> {
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let (width, height) = SIZE;
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("resume countdown"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = target.create_view(&Default::default());
    let mut encoder = device.create_command_encoder(&Default::default());
    drop(encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: None,
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: &view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                store: wgpu::StoreOp::Store,
            },
        })],
        ..Default::default()
    }));
    overlay.draw_views(
        device,
        queue,
        &mut encoder,
        &view,
        vec![Readout::blank(), Readout::blank()],
        countdown,
        cut,
        VIEWPORT,
    );
    let padded = (width as usize * 4).div_ceil(256) * 256;
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: (padded * height as usize) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        target.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded as u32),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    queue.submit(Some(encoder.finish()));
    buffer.slice(..).map_async(wgpu::MapMode::Read, |_| {});
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("poll");
    let data = buffer.slice(..).get_mapped_range().expect("mapped");
    let mut out = Vec::new();
    for row in 0..height as usize {
        out.extend_from_slice(&data[row * padded..row * padded + width as usize * 4]);
    }
    out
}

/// The bounding box `(x0, y0, x1, y1)` of the pixels that differ between two frames
/// inside `rect`, or `None` when none does.
fn changed(
    with: &[u8],
    without: &[u8],
    rect: (f32, f32, f32, f32),
) -> Option<(usize, usize, usize, usize)> {
    let (x0, y0) = (rect.0 as usize, rect.1 as usize);
    let (x1, y1) = ((rect.0 + rect.2) as usize, (rect.1 + rect.3) as usize);
    let mut bounds: Option<(usize, usize, usize, usize)> = None;
    for y in y0..y1.min(SIZE.1 as usize) {
        for x in x0..x1.min(SIZE.0 as usize) {
            let at = (y * SIZE.0 as usize + x) * 4;
            if with[at..at + 4] != without[at..at + 4] {
                let (a, b, c, d) = bounds.unwrap_or((x, y, x, y));
                bounds = Some((a.min(x), b.min(y), c.max(x), d.max(y)));
            }
        }
    }
    bounds
}

#[test]
#[ignore = "needs the disc images in data/ and a GPU adapter"]
fn a_title_with_no_art_says_the_count_in_its_hud_face_in_every_view() {
    let Some((device, queue)) = device() else {
        println!("skipping: no GPU adapter");
        return;
    };
    for image in TEXT_TITLES {
        let Some(assets) = hud(image) else {
            continue;
        };
        assert_eq!(
            assets.art.resume_countdown,
            ResumeCountdown::Text,
            "{image}: authors no countdown art"
        );
        let mut overlay = oag_game::hud_overlay::Overlay::new(
            &device,
            &queue,
            wgpu::TextureFormat::Rgba8Unorm,
            &assets,
            oag_display::display::HudScale::Linear,
        )
        .expect("the overlay")
        .expect("a layout");
        assert_eq!(overlay.resume_countdown(), ResumeCountdown::Text);
        for cut in [Cut::Horizontal, Cut::Vertical] {
            let views = layout(2, VIEWPORT, cut);
            let without = pixels(&device, &queue, &mut overlay, None, cut);
            // `GO` is the longest word and each digit is a glyph of its own: every
            // one has to reach the pixels, which a glyph the face lacks would not.
            // The vertical cut is the placement check; the glyph census is the
            // horizontal one's, so the software rasteriser draws each word once.
            let words: &[&str] = match cut {
                Cut::Horizontal => &["3", "2", "1", "GO"],
                Cut::Vertical => &["GO"],
            };
            for &word in words {
                let with = pixels(&device, &queue, &mut overlay, Some(word), cut);
                for (i, view) in views.iter().enumerate() {
                    let (x0, y0, x1, y1) = changed(&with, &without, *view).unwrap_or_else(|| {
                        panic!("{image} {cut:?}: {word:?} drew nothing in view {i}")
                    });
                    let (centre_x, top, bottom) = (
                        (x0 + x1) as f32 / 2.0,
                        (y0 as f32 - view.1) / view.3,
                        (y1 as f32 - view.1) / view.3,
                    );
                    assert!(
                        (centre_x - (view.0 + view.2 / 2.0)).abs() < view.2 * 0.05,
                        "{image} {cut:?} {word:?} view {i}: not centred across the view ({x0}..{x1})"
                    );
                    // Above the craft, which sits in the lower half of a view: the
                    // whole word is in the view's upper 55%.
                    assert!(
                        top >= 0.0 && bottom < 0.55,
                        "{image} {cut:?} {word:?} view {i}: {top:.2}..{bottom:.2} of the view"
                    );
                }
                // Nothing leaks into the gap or the other view: the changed pixels
                // are in the views' own rectangles.
                let outside = changed(&with, &without, (0.0, 0.0, 1440.0, 816.0)).expect("drawn");
                assert!(
                    outside.0 >= views[0].0 as usize
                        && outside.2 <= (views[1].0 + views[1].2) as usize
                );
            }
        }
    }
}

#[test]
#[ignore = "needs data/images/pulse-psp-eu.chd"]
fn pulse_plays_its_authored_art_and_not_the_font() {
    let Some(assets) = hud("data/images/pulse-psp-eu.chd") else {
        return;
    };
    assert_eq!(assets.art.resume_countdown, ResumeCountdown::Authored);
}
