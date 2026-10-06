//! Omega's HD-style menu backdrop: the scene `skin.xml`'s `<BackgroundAnim>`
//! names loads off the disc, its authored animation and camera move, and the
//! renderer draws a grey picture that changes with the clock.
//!
//! The widget is `BackgroundAnim_Item.cpp` in Wipeout HD's executable, read in
//! `docs/ghidra/functions/ps3-hdfury-eu/menu-backdrop-scene.md`. Omega ships no
//! Fury point clouds, so this is the only backdrop its menus can have.
//!
//! Measured here off the disc, not assumed: the scene is
//! `FrontEndScene_HD_ATG.vex` (9,184 triangles, 56 meshes), seven of its
//! nodes carry an `Anim Transform` and loop at 60 seconds, and one of them is
//! the scene's own camera.

use std::path::PathBuf;

use oag_display::space::Space;
use oag_game::boot::{self, backdrop::MenuBackdrop};
use oag_game::render::Renderer;
use oag_ui::frontend::{Draw, Leg};
use oag_ui::scene_backdrop::Scene;

fn omega() -> Option<PathBuf> {
    oag_testdata::exact("data/extracted/ps4")
}

fn assets(source: &std::path::Path) -> std::sync::Arc<MenuBackdrop> {
    let options = boot::Options {
        language: None,
        source: source.display().to_string(),
        dlc: Vec::new(),
        leg: Leg::LogoFmv,
        movie: None,
        cache: std::env::temp_dir().join("oag-omega-menu-backdrop-ground-truth"),
        audio_cache: oag_source::cache::default_audio_cache_dir(),
        extent: oag_game::movie::Extent::Frames(oag_game::INTRO_FRAMES_NEEDED),
        no_video: true,
        refresh_video: false,
        prefer_av1_cache: false,
    };
    let (shell, _archives, _title) = boot::load_shell(&options)
        .expect("Omega's front end boots; see docs/formats/omega-status.md");
    shell
        .fury_backdrop
        .expect("Omega's skin authors a BackgroundAnim widget and its title draws it")
}

fn scene(backdrop: &MenuBackdrop) -> &boot::scene::SceneAssets {
    match backdrop {
        MenuBackdrop::Scene(assets) => assets,
        MenuBackdrop::Fury(_) => panic!("Omega has no Fury clouds, so its backdrop is the scene"),
    }
}

#[test]
#[ignore = "needs the decrypted PS4 package pair in data/extracted/ps4/"]
fn the_scene_the_skin_names_loads_and_its_camera_and_ships_move() {
    let Some(source) = omega() else {
        return;
    };
    let backdrop = assets(&source);
    let assets = scene(&backdrop);
    assert!(
        assets
            .widget
            .src
            .to_ascii_lowercase()
            .ends_with("frontendscene_hd_atg.vex"),
        "{}",
        assets.widget.src
    );
    assert_eq!(assets.widget.anim_length, 60.0);
    assert!(assets.widget.model_camera);
    assert_eq!(assets.model.indices.len() / 3, 9184);
    // The `.vex` authors seven `Anim Transform` nodes (two ships, two track
    // surfaces, the camera and its group, and the world joint), and the
    // model's 51 node-bound meshes under them take slots of the shader's table.
    assert_eq!(assets.model.anim_nodes.len(), 7);

    let at = |seconds: f32| assets.view_projection(seconds, 16.0 / 9.0);
    assert_ne!(at(0.0), at(10.0), "the camera does not move");
    // A loop of 60 seconds: a minute on, the camera is where it was (to the
    // few microseconds the `LoopEnd` of 3,600 frames at 1/60 rounds to).
    for (a, b) in at(3.0).iter().flatten().zip(at(63.0).iter().flatten()) {
        assert!(
            (a - b).abs() < 0.05,
            "{a} against {b}: the loop is not 60 seconds"
        );
    }
    let ships_at = |seconds: f32| assets.model.sample_anim_nodes(seconds);
    assert_ne!(ships_at(0.0), ships_at(30.0), "nothing in the scene moves");
}

/// What the renderer draws for `seconds` into a `width` x `height` target: the
/// RGBA bytes, or `None` where there is no GPU adapter.
fn draw(
    backdrop: &std::sync::Arc<MenuBackdrop>,
    seconds: f32,
    size: (u32, u32),
) -> Option<Vec<u8>> {
    let instance = wgpu::Instance::default();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).ok()?;
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).ok()?;
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let (width, height) = size;
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

    let assets = scene(backdrop);
    let frame = Scene::new(assets.widget.clone()).settled(
        seconds,
        true,
        assets.view_projection(seconds, width as f32 / height as f32),
    );
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("omega backdrop test"),
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
        &[Draw::SceneBackdrop(Box::new(frame))],
        (0.0, 0.0, width as f32, height as f32),
        None,
    );
    let padded = (width * 4).div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)
        * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("omega backdrop readback"),
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

/// Dark enough to be drawing and not the white the target is cleared to.
fn grey_pixels(rgba: &[u8]) -> usize {
    rgba.chunks(4).filter(|p| p[0] < 240).count()
}

#[test]
#[ignore = "needs the decrypted PS4 package pair in data/extracted/ps4/ and a GPU adapter"]
fn the_renderer_draws_a_grey_drawing_that_changes_with_the_clock() {
    let Some(source) = omega() else {
        return;
    };
    let backdrop = assets(&source);
    let size = (320, 180);
    let Some(early) = draw(&backdrop, 0.0, size) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let late = draw(&backdrop, 15.0, size).expect("the adapter was there a moment ago");
    let total = (size.0 * size.1) as usize;
    for (name, picture) in [("0 s", &early), ("15 s", &late)] {
        let grey = grey_pixels(picture);
        assert!(
            grey > total / 50 && grey < total,
            "{name}: {grey} of {total} pixels are drawn on, so the scene is missing or fills the screen"
        );
        // Opaque, and a grey: the filter writes one value to all three.
        assert!(
            picture
                .chunks(4)
                .all(|p| p[3] == 255 && p[0] == p[1] && p[1] == p[2])
        );
    }
    assert_ne!(early, late, "the picture does not move with the clock");
    assert_eq!(
        early,
        draw(&backdrop, 0.0, size).unwrap(),
        "the same clock draws the same picture"
    );
}

/// The `oag_texture::png` writer's own output back to RGBA: stored deflate
/// blocks, filter 0 on every row.
fn decode_stored_png(bytes: &[u8]) -> (usize, usize, Vec<u8>) {
    let (mut width, mut height, mut zlib) = (0usize, 0usize, Vec::new());
    let mut at = 8;
    while at + 8 <= bytes.len() {
        let len = u32::from_be_bytes(bytes[at..at + 4].try_into().unwrap()) as usize;
        let kind = &bytes[at + 4..at + 8];
        let data = &bytes[at + 8..at + 8 + len];
        match kind {
            b"IHDR" => {
                width = u32::from_be_bytes(data[0..4].try_into().unwrap()) as usize;
                height = u32::from_be_bytes(data[4..8].try_into().unwrap()) as usize;
            }
            b"IDAT" => zlib.extend_from_slice(data),
            _ => {}
        }
        at += 12 + len;
    }
    let mut raw = Vec::new();
    let mut at = 2;
    loop {
        let last = zlib[at] & 1 == 1;
        let len = u16::from_le_bytes(zlib[at + 1..at + 3].try_into().unwrap()) as usize;
        raw.extend_from_slice(&zlib[at + 5..at + 5 + len]);
        at += 5 + len;
        if last {
            break;
        }
    }
    let mut rgba = Vec::with_capacity(width * height * 4);
    for row in raw.chunks(width * 4 + 1) {
        assert_eq!(
            row[0], 0,
            "a filtered row: this decoder reads the writer's own"
        );
        rgba.extend_from_slice(&row[1..]);
    }
    (width, height, rgba)
}

/// `--menu-page main` for `anim_seconds`, through the real binary: the path a
/// player's window takes, which the tests above go around.
fn menu_page(source: &std::path::Path, anim_seconds: &str, name: &str) -> (usize, usize, Vec<u8>) {
    let out = std::env::temp_dir().join(format!("oag-omega-backdrop-{name}.png"));
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_oag-game"))
        .arg(source)
        .args(["--no-audio", "--size", "640x360", "--menu-page", "main"])
        .args(["--anim-seconds", anim_seconds, "--screenshot"])
        .arg(&out)
        .env(
            "XDG_CONFIG_HOME",
            std::env::temp_dir().join("oag-omega-backdrop-config"),
        )
        .env(
            "XDG_STATE_HOME",
            std::env::temp_dir().join("oag-omega-backdrop-state"),
        )
        .status()
        .expect("running oag-game");
    assert!(
        status.success(),
        "oag-game --menu-page main --anim-seconds {anim_seconds}"
    );
    decode_stored_png(&std::fs::read(&out).expect("the screenshot"))
}

/// The wiring end to end: Omega's `--menu-page main` draws the scene where the
/// rows are not, and draws a different picture a few seconds on. Dropping the
/// backdrop from the boot, the stage or the capture leaves a white page and
/// fails both.
#[test]
#[ignore = "needs the decrypted PS4 package pair in data/extracted/ps4/ and a GPU adapter"]
fn omega_menu_page_main_shows_the_scene_and_it_moves() {
    let Some(source) = omega() else {
        return;
    };
    let (width, height, early) = menu_page(&source, "0", "early");
    let (_, _, late) = menu_page(&source, "20", "late");
    // The middle of the page, clear of the title and tab rows and the footer.
    let mut grey = 0;
    for y in height * 3 / 10..height * 85 / 100 {
        for x in width * 3 / 10..width * 7 / 10 {
            grey += usize::from(early[(y * width + x) * 4] < 240);
        }
    }
    assert!(
        grey > 200,
        "{grey} grey pixels in the middle of the page: the backdrop is not drawn"
    );
    assert_ne!(early, late, "the page does not change with the clock");
}
