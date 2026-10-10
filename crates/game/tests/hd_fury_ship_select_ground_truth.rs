//! What tint Fury's selection screens carry, and how that tint reads on screen.
//!
//! RPCS3, 2026-10-10: on `Team Selection` the widget's current row
//! (`BackgroundAnimFury` item `+0xc4`) is the **`default`** row, `0.12549` per
//! channel, and so it is on `Single Player` and `Track Creation`; only
//! `Main Menu`, `Additional`, `Controls Menu` and `Extras` are white. Editing
//! that row's tint live gave, as the mean excess brightness over 28 groups of
//! five frames taken a quarter second apart, `1 : 0.80 : 0.62 : 0.40 : 0.23`
//! for tints `1, 1/2, 1/4, 1/8, 1/16` - not the `1 : .5 : .25 : .125 : .0625` a
//! literal scale gives. The tint is applied in linear light and the buffer
//! encodes it (this build leaves a tint of 1 as it was). See docs/ghidra/functions/ps3-hdfury-eu/menu-backdrop.md,
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

/// `Draw::FuryBackdrop` on a black page, read back: the RGBA bytes, or `None`
/// where there is no GPU adapter. At 1280x720: below about 414 lines the
/// original's `resScale` is zero and no cloud is drawn at all.
fn draw(
    assets: &boot::fury::FuryAssets,
    backdrop: &std::sync::Arc<boot::backdrop::MenuBackdrop>,
    seconds: f32,
    tint: [f32; 4],
) -> Option<Vec<u8>> {
    let (width, height) = (1280u32, 720u32);
    let instance = wgpu::Instance::default();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).ok()?;
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
    for _ in 0..(seconds * oag_ui::backdrop::FRAMES_PER_SECOND) as u32 {
        model.tick();
    }
    let frame = model.frame(height as f32, width as f32 / height as f32, tint);
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

fn mean_red(rgba: &[u8]) -> f64 {
    rgba.chunks(4).map(|p| f64::from(p[0])).sum::<f64>() / (rgba.len() / 4) as f64
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

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso and a GPU adapter"]
fn a_tint_of_an_eighth_reads_as_about_two_fifths_on_screen() {
    let Some(image) = image() else {
        return;
    };
    let assets = fury(&image);
    let backdrop = std::sync::Arc::new(boot::backdrop::MenuBackdrop::Fury(assets.clone()));
    let eighth = [0.125, 0.125, 0.125, 1.0];
    // The clip passes through emptier stretches; the densest of a few clocks.
    let mut best = None;
    for seconds in [6.0f32, 14.0, 26.0, 38.0] {
        let Some(full) = draw(&assets, &backdrop, seconds, [1.0; 4]) else {
            eprintln!("no GPU adapter: skipping");
            return;
        };
        let level = mean_red(&full);
        if best.as_ref().is_none_or(|(_, l, _)| level > *l) {
            best = Some((seconds, level, full));
        }
    }
    let (seconds, full_level, _) = best.expect("four clocks");
    assert!(full_level > 4.0, "the clip is empty at every clock tried");
    let dim = mean_red(&draw(&assets, &backdrop, seconds, eighth).expect("adapter"));
    let ratio = dim / full_level;
    assert!(
        (0.3..0.55).contains(&ratio),
        "{dim} of {full_level} is {ratio}: a literal scale gives 0.125, the original reads 0.40"
    );
}
