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
        cache: oag_testdata::scratch::cache("hd-fury-ship-select-ground-truth"),
        audio_cache: oag_source::cache::default_audio_cache_dir(),
        extent: oag_game::movie::Extent::Frames(oag_game::INTRO_FRAMES_NEEDED),
        no_video: true,
        refresh_video: false,
        prefer_av1_cache: false,
        find_texture_packs: None,
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
    draw_to_music(assets, backdrop, seconds, tint, &oag_ui::backdrop::SILENCE)
}

/// [`draw`] with `bands` as the equaliser on every frame.
fn draw_to_music(
    assets: &boot::fury::FuryAssets,
    backdrop: &std::sync::Arc<boot::backdrop::MenuBackdrop>,
    seconds: f32,
    tint: impl Fn(u32) -> [f32; 4],
    bands: &[f32; oag_ui::backdrop::BANDS],
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
        model.tick(bands);
    }
    let aspect = width as f32 / height as f32;
    let leading_up = (0..settle).map(|at| {
        let frame = model.frame(height as f32, aspect, tint(settle - at));
        model.tick(bands);
        frame
    });
    let frames: Vec<_> = leading_up.collect();
    renderer.settle_fury_backdrop(&device, &queue, frames.into_iter(), (width, height), None);
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
        texture_binding_view_dimension: None,
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

/// The music reaches the picture: the same clock at rising bands, through the
/// whole chain. `musicPulse` is the law's `0.23` at silence and `1.33` at full.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso and a GPU adapter"]
fn the_cloud_brightens_with_the_music() {
    let Some(image) = image() else {
        return;
    };
    let assets = fury(&image);
    let backdrop = std::sync::Arc::new(boot::backdrop::MenuBackdrop::Fury(assets.clone()));
    let mut last = 0.0;
    for level in [0.0, 0.35, 0.7, 1.0] {
        let mut bands = oag_ui::backdrop::SILENCE;
        bands[10..13].fill(level);
        let Some(pixels) = draw_to_music(&assets, &backdrop, 38.0, |_| [1.0; 4], &bands) else {
            eprintln!("no GPU adapter: skipping");
            return;
        };
        let red = mean_red(&pixels);
        eprintln!("bands {level}: mean red {red}");
        assert!(red > last, "{red} at bands {level}, {last} below it");
        last = red;
    }
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
    // The table was read with music playing. Silent, the composite's `1/257`
    // takes more off a dimmer picture, so the ratios sit under the table's;
    // what cannot move is that they stay well above the literal scale.
    let mut last = 1.0;
    for (tint, measured) in [(0.5, 0.80), (0.25, 0.62), (0.125, 0.40), (0.0625, 0.23)] {
        let dim = draw(&assets, &backdrop, 38.0, |_| [tint, tint, tint, 1.0]).expect("adapter");
        let ratio = mean_red(&dim) / full;
        assert!(
            ratio > f64::from(tint) * 1.2 && ratio < measured + 0.05 && ratio < last,
            "tint {tint}: {ratio:.2} of tint 1, the original's {measured}, a literal scale's {tint}"
        );
        last = ratio;
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
    // The field is read half a texel off, so a scanline is the two rows either
    // side of the fourth's edge: the mean of one and `0.05` in linear light,
    // about `0.7` once encoded. RPCS3, eight frames: `0.68` to `0.77`.
    let similar = |a: f64, b: f64| (a / b - 1.0).abs() < 0.05;
    assert!(similar(by_phase[0], by_phase[3]), "{by_phase:?}");
    assert!(similar(by_phase[1], by_phase[2]), "{by_phase:?}");
    let dip = by_phase[0] / by_phase[1];
    assert!((0.6..0.8).contains(&dip), "{dip}: {by_phase:?}");
}

/// A 1280x720 capture of `page` through the real binary, with `flags` after the page.
fn capture_of(image: &Path, page: &str, flags: &[&str]) -> oag_texture::replacer::Image {
    let dir = oag_testdata::scratch::dir("hd-fury-tint-ground-truth");
    let out = dir.join("page.png");
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_oag-game"))
        .arg(image)
        .args([
            "--no-audio",
            "--no-soundtrack",
            "--no-effects",
            "--size",
            "1280x720",
            "--menu-page",
            page,
        ])
        .args(flags)
        .arg("--screenshot")
        .arg(&out)
        .env("XDG_CONFIG_HOME", dir.join("config"))
        .env("XDG_STATE_HOME", dir.join("state"))
        .env("XDG_DATA_HOME", dir.join("data"))
        .status()
        .expect("running oag-game");
    assert!(status.success(), "oag-game --menu-page {page}");
    let still = oag_texture::replacer::decode_png(&std::fs::read(&out).expect("the screenshot"))
        .expect("a PNG");
    let _ = std::fs::remove_dir_all(&dir);
    still
}

/// How many pixels of `page`'s right-hand margin are lit, and the brightest
/// channel there, on a 1280x720 capture through the real binary. The layout
/// ends at nine tenths of the width, so only the backdrop draws in the strip.
fn margin_of_a_capture(image: &Path, page: &str, tint: Option<&str>) -> (usize, u8) {
    let flags: Vec<&str> = tint
        .into_iter()
        .flat_map(|tint| ["--fury-tint", tint])
        .collect();
    let still = capture_of(image, page, &flags);
    let strip = (0..still.height).flat_map(|y| (1180..still.width).map(move |x| (x, y)));
    let brightest = |(x, y): (u32, u32)| {
        let at = ((y * still.width + x) * 4) as usize;
        still.rgba[at..at + 3].iter().copied().max().unwrap_or(0)
    };
    let levels: Vec<u8> = strip.map(brightest).collect();
    (
        levels.iter().filter(|&&level| level > 16).count(),
        levels.iter().copied().max().unwrap_or(0),
    )
}

/// `--fury-tint 0` is the hidden cloud of a compare shot: the state a pair
/// script writes over the original's `default` row before its grab. On RPCS3,
/// `Track Creation` and `Team Selection` with that row at zero are a black
/// page, the same on two boots, where two boots with the cloud showing differ
/// by a mean of 1.7 grey levels outside the model
/// (docs/ghidra/functions/ps3-hdfury-eu/menu-backdrop.md, "Hidden for a
/// compare shot"). The capture's page has to lose its cloud the same way, and
/// keep it without the flag.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso and a GPU adapter"]
fn a_stated_tint_of_nothing_hides_the_cloud_on_a_captured_page() {
    let Some(image) = image() else {
        return;
    };
    let (lit, _) = margin_of_a_capture(&image, "track-select", None);
    assert!(
        lit > 500,
        "the cloud reaches the margin under the row's own tint: {lit} pixels"
    );
    for page in ["track-select", "ship-select"] {
        let (lit, brightest) = margin_of_a_capture(&image, page, Some("0"));
        assert_eq!(
            (lit, brightest),
            (0, 0),
            "{page}: the margin is black with the tint stated as nothing"
        );
    }
}

/// The sum of the red, green and blue of the pixels over 60 in the middle of a 1280x720 Ship Select
/// capture, where the craft stands.
fn hull_light(still: &oag_texture::replacer::Image) -> u64 {
    let mut sum = 0;
    for y in 100..560 {
        for x in 480..1180 {
            let at = ((y * still.width + x) * 4) as usize;
            let [r, g, b] = [0, 1, 2].map(|c| u64::from(still.rgba[at + c]));
            if r.max(g).max(b) > 60 {
                sum += r + g + b;
            }
        }
    }
    sum
}

/// `--fury-adapted-luma` states the luma the craft's exposure is taken at. The exposure is
/// `4 - min(35 * luma, 3)`: at the title's `0.0163` it is 3.43, at `0.0` it is 4, and at `0.1`
/// it is the clamp's 1. The hull follows it in both directions, so a flag dropped on the way to
/// the craft's light leaves all three frames equal.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso and a GPU adapter"]
fn a_stated_adapted_luma_sets_the_exposure_the_craft_is_lit_at() {
    let Some(image) = image() else {
        return;
    };
    let hull = |luma: Option<&str>| {
        let flags: Vec<&str> = ["--fury-tint", "0"]
            .into_iter()
            .chain(
                luma.into_iter()
                    .flat_map(|luma| ["--fury-adapted-luma", luma]),
            )
            .collect();
        hull_light(&capture_of(&image, "ship-select", &flags))
    };
    let (dark, own, bright) = (hull(Some("0.1")), hull(None), hull(Some("0.0")));
    assert!(dark > 0, "a hull is drawn");
    assert!(
        own as f64 > dark as f64 * 1.15 && bright as f64 > own as f64 * 1.02,
        "exposure 1, 3.43 and 4 draw the hull at {dark}, {own}, {bright}"
    );
    assert_eq!(
        own,
        hull(Some("0.0163")),
        "the stated value of the title's read is the title's read"
    );
}

/// The sum of red, green and blue over a whole capture.
fn frame_light(still: &oag_texture::replacer::Image) -> u64 {
    still
        .rgba
        .chunks(4)
        .map(|texel| texel[..3].iter().map(|&c| u64::from(c)).sum::<u64>())
        .sum()
}

/// `--fury-adapted-luma` also pins the backdrop chain's adapted luma, which is what the exposure of
/// a page's card follows (`Cell Selection`'s card is in the scene the chain reads). Pinned at the
/// clamp (0.1, exposure 1) the card is darker than where ours' own scene lands, and pinned at 0
/// (exposure 4) it is brighter. A pin dropped on the way to the chain leaves all three equal.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso and a GPU adapter"]
fn a_stated_adapted_luma_pins_the_exposure_a_card_is_drawn_at() {
    let Some(image) = image() else {
        return;
    };
    let card = |luma: Option<&str>| {
        let flags: Vec<&str> = ["--fury-tint", "0"]
            .into_iter()
            .chain(
                luma.into_iter()
                    .flat_map(|luma| ["--fury-adapted-luma", luma]),
            )
            .collect();
        frame_light(&capture_of(&image, "cell-select", &flags))
    };
    let (own, dark, bright) = (card(None), card(Some("0.1")), card(Some("0.0")));
    assert!(own > 0, "a card is drawn");
    assert!(
        dark < own && own < bright,
        "exposure 1, the scene's own and 4 draw the card at {dark}, {own}, {bright}"
    );
}
