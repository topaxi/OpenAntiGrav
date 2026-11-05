//! What tint Fury's selection screens carry, and how that tint reads on screen.
//!
//! RPCS3, 2026-10-10: on `Team Selection` the widget's current row
//! (`BackgroundAnimFury` item `+0xc4`) is the **`default`** row, `0.12549` per
//! channel, and so it is on `Single Player` and `Track Creation`; only
//! `Main Menu`, `Additional`, `Controls Menu` and `Extras` are white. Editing
//! that row's tint live gave, as the mean excess brightness over 28 groups of
//! five frames taken a quarter second apart, `1 : 0.80 : 0.62 : 0.40 : 0.23`
//! for tints `1, 1/2, 1/4, 1/8, 1/16` - not the `1 : .5 : .25 : .125 : .0625` a
//! literal scale gives. The tint scales the fresh particles in the trail blend,
//! and that blend runs in linear light. See docs/ghidra/functions/ps3-hdfury-eu/menu-backdrop.md,
//! "The tint is applied in linear light".

use std::path::{Path, PathBuf};

use oag_display::space::Space;
use oag_game::boot;
use oag_game::render::Renderer;
use oag_ui::frontend::{self, Draw};

fn image() -> Option<PathBuf> {
    oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso")
}

fn fury(image: &Path) -> std::sync::Arc<boot::fury::FuryAssets> {
    let options = boot::Options {
        language: None,
        source: image.display().to_string(),
        dlc: Vec::new(),
        leg: frontend::Leg::LogoFmv,
        movie: None,
        cache: std::env::temp_dir().join("oag-hd-fury-ship-select-ground-truth"),
        audio_cache: oag_source::cache::default_audio_cache_dir(),
        extent: oag_game::movie::Extent::Frames(oag_game::INTRO_FRAMES_NEEDED),
        no_video: true,
        refresh_video: false,
        prefer_av1_cache: false,
    };
    let (shell, _archives, _title) =
        boot::load_shell(&options).expect("HD's front end is wired; see ADR-0025");
    match &*shell
        .fury_backdrop
        .expect("the Fury style authors the widget")
    {
        boot::backdrop::MenuBackdrop::Fury(assets) => std::sync::Arc::clone(assets),
        boot::backdrop::MenuBackdrop::Scene(_) => panic!("the Fury style draws the Fury widget"),
    }
}

/// `Draw::FuryBackdrop` on a black page after a settled run of frames, read
/// back: the RGBA bytes, or `None` where there is no GPU adapter. At 1280x720:
/// below about 414 lines the original's `resScale` is zero and no cloud is
/// drawn at all.
///
/// `tint` gets how many frames are left, so a run can change it part way;
/// the last frame is the one read back, with `0` left.
fn draw(
    assets: &boot::fury::FuryAssets,
    backdrop: &std::sync::Arc<boot::backdrop::MenuBackdrop>,
    seconds: f32,
    tint: impl Fn(u32) -> [f32; 4],
) -> Option<Vec<u8>> {
    let (width, height) = (1280u32, 720u32);
    let instance = wgpu::Instance::default();
    let adapter = oag_gpu::test_adapter::request(&instance)?;
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).ok()?;
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let mut renderer = Renderer::new(
        &device,
        &queue,
        format,
        None,
        oag_ui::font::Atlas::build(),
        &oag_hud::sprite::Sheet::default(),
    )
    .expect("the ui pipeline");
    renderer.set_space(Space {
        size: (width as f32, height as f32),
        display_aspect: width as f32 / height as f32,
    });
    backdrop.install(&mut renderer, &device, &queue);
    let mut model =
        oag_ui::backdrop::Fury::new(assets.settings.clone(), assets.first, boot::fury::SEED)?;
    let settle = oag_ui::backdrop::FRAMES_TO_SETTLE;
    for _ in 0..(seconds * oag_ui::backdrop::FRAMES_PER_SECOND) as u32 - settle {
        model.tick();
    }
    let aspect = width as f32 / height as f32;
    let leading_up = (0..settle).map(|at| {
        let frame = model.frame(height as f32, aspect, tint(settle - at));
        model.tick();
        frame
    });
    let frames: Vec<_> = leading_up.collect();
    renderer.settle_fury_backdrop(&device, &queue, frames.into_iter(), (width, height));
    let frame = model.frame(height as f32, aspect, tint(0));
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("fury ship select test"),
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
    renderer.render(
        &device,
        &queue,
        &mut encoder,
        &view,
        &[Draw::FuryBackdrop(Box::new(frame))],
        (0.0, 0.0, width as f32, height as f32),
        None,
    );
    let padded = (width * 4).div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)
        * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("fury ship select readback"),
        size: u64::from(padded * height),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        target.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded),
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
    let slice = readback.slice(..);
    slice.map_async(wgpu::MapMode::Read, |_| {});
    device.poll(wgpu::PollType::wait_indefinitely()).ok()?;
    let mapped = slice.get_mapped_range().expect("mapped");
    let mut pixels = Vec::new();
    for row in mapped.chunks(padded as usize).take(height as usize) {
        pixels.extend_from_slice(&row[..(width * 4) as usize]);
    }
    Some(pixels)
}

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn the_selection_screens_carry_the_default_rows_tint() {
    let Some(image) = image() else {
        return;
    };
    let assets = fury(&image);
    let grey = 0x20 as f32 / 255.0;
    for screen in [
        "Team Selection",
        "TrackHexSelection",
        "Single Player",
        "Track Creation",
    ] {
        assert_eq!(
            assets.tints.for_screen(screen),
            [grey, grey, grey, 1.0],
            "{screen} authors no row of its own"
        );
    }
    assert_eq!(assets.tints.for_screen("Main Menu"), [1.0; 4]);
}

/// The mean of the red channel over the two rows in four the scanlines leave
/// alone.
fn mean_red(pixels: &[u8]) -> f64 {
    let rows = pixels.chunks(1280 * 4).enumerate();
    let red: Vec<u8> = rows
        .filter(|(row, _)| row % 4 == 1 || row % 4 == 2)
        .flat_map(|(_, row)| row.chunks(4).map(|texel| texel[0]))
        .collect();
    red.iter().map(|&v| f64::from(v)).sum::<f64>() / red.len() as f64
}

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso and a GPU adapter"]
fn the_tint_reads_as_it_does_because_it_is_applied_in_linear_light() {
    let Some(image) = image() else {
        return;
    };
    let assets = fury(&image);
    let backdrop = std::sync::Arc::new(boot::backdrop::MenuBackdrop::Fury(assets.clone()));
    // The same clock at every tint: only the tint differs, and the ratios are
    // the table's on menu-backdrop.md, "The tint is applied in linear light".
    let Some(full) = draw(&assets, &backdrop, 38.0, |_| [1.0; 4]) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let full = mean_red(&full);
    assert!(full > 1.0, "the backdrop drew: {full}");
    for (tint, measured) in [(0.5, 0.80), (0.25, 0.62), (0.125, 0.40), (0.0625, 0.23)] {
        let dim = draw(&assets, &backdrop, 38.0, |_| [tint, tint, tint, 1.0]).expect("adapter");
        let ratio = mean_red(&dim) / full;
        assert!(
            ratio > measured - 0.16 && ratio < measured + 0.1,
            "tint {tint}: {ratio:.2} of tint 1, the original's {measured}, a literal scale's {tint}"
        );
    }
}

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso and a GPU adapter"]
fn a_tint_of_nothing_leaves_the_trail_to_decay() {
    let Some(image) = image() else {
        return;
    };
    let assets = fury(&image);
    let backdrop = std::sync::Arc::new(boot::backdrop::MenuBackdrop::Fury(assets.clone()));
    // The tint goes to zero three frames before the end: it stops new light
    // entering the accumulation, and what is there keeps decaying on screen.
    // A tint on the composite would turn the picture black at once.
    let Some(fading) = draw(&assets, &backdrop, 38.0, |left| {
        if left > 3 {
            [1.0; 4]
        } else {
            [0.0, 0.0, 0.0, 1.0]
        }
    }) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let full = draw(&assets, &backdrop, 38.0, |_| [1.0; 4]).expect("adapter");
    let dark = draw(&assets, &backdrop, 38.0, |_| [0.0, 0.0, 0.0, 1.0]).expect("adapter");
    let (fading, full, dark) = (mean_red(&fading), mean_red(&full), mean_red(&dark));
    assert!(dark < 0.5, "no tint from the start draws nothing: {dark}");
    assert!(
        fading > full * 0.2 && fading < full * 0.9,
        "the trail decays through the frames after the tint goes: {fading} against {full}"
    );
}

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso and a GPU adapter"]
fn every_fourth_row_from_the_top_is_a_scanline() {
    let Some(image) = image() else {
        return;
    };
    let assets = fury(&image);
    let backdrop = std::sync::Arc::new(boot::backdrop::MenuBackdrop::Fury(assets.clone()));
    let Some(pixels) = draw(&assets, &backdrop, 38.0, |_| [1.0; 4]) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let by_phase: [f64; 4] = std::array::from_fn(|phase| {
        let rows = pixels.chunks(1280 * 4).skip(phase).step_by(4);
        rows.flat_map(|row| row.chunks(4).map(|texel| f64::from(texel[0])))
            .sum::<f64>()
    });
    // The field is read half a row low, so a scanline is the two rows either
    // side of the fourth's edge: the mean of one and `0.05` in linear light,
    // about `0.7` once encoded. RPCS3, eight frames: `0.68` to `0.74`.
    let similar = |a: f64, b: f64| (a / b - 1.0).abs() < 0.05;
    assert!(similar(by_phase[0], by_phase[3]), "{by_phase:?}");
    assert!(similar(by_phase[1], by_phase[2]), "{by_phase:?}");
    let dip = by_phase[0] / by_phase[1];
    assert!((0.6..0.8).contains(&dip), "{dip}: {by_phase:?}");
}
