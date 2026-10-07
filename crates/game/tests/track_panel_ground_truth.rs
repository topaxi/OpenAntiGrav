//! The track-description panel over the pre-race flyby, against the disc that authors it.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this project does not
//! ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --test track_panel_ground_truth --run-ignored all
//! ```
//!
//! # What only real data can say here
//!
//! `InGameTrackDescriptionScreen` is authored in `InGame_Definition.xml`, and the circuit's name
//! and paragraph are string-table entries the race constructor looks up by `PI_Track` id
//! (`docs/gameplay/race-intro.md`). A fixture proves none of: that the screen parses to the ten
//! widgets, that the id resolves from a bare `.vex` entry name (forward and reversed), that both
//! strings are in the table, that the tile is reachable, or that a `Race` carries the panel from
//! the flyby's first tick to 0.7 s after its end. Nothing here writes a disc string down: the
//! checks are against the table's own answers.

use oag_gameplay::PlayerInputs;
use oag_gameplay::input::{Button, Input, InputSnapshot};
use oag_raceplay as race;
use oag_raceplay::Race;
use oag_ui::frontend::Draw;
use oag_ui_screens::track_panel::{FADE_SECONDS, Progress, WIPE_WIDTH};

fn load(track: &str) -> Option<race::Loaded> {
    let image = oag_testdata::image("data/images/pulse-psp-usa.chd")?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::TimeTrial,
        track: Some(track.to_string()),
        ..race::Options::default()
    })
    .expect("loading the race");
    for line in loaded
        .report
        .iter()
        .filter(|l| l.starts_with("track panel"))
    {
        println!("{line}");
    }
    Some(loaded)
}

fn texts(list: &[Draw]) -> Vec<&str> {
    list.iter()
        .filter_map(|draw| match draw {
            Draw::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect()
}

fn cross() -> PlayerInputs {
    let mut buttons = Input::EMPTY;
    buttons.begin_frame(Button::Cross.bit());
    PlayerInputs::single(InputSnapshot {
        buttons,
        ..InputSnapshot::EMPTY
    })
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn a_circuit_and_its_reverse_each_get_their_own_name_and_paragraph() {
    let (Some(forward), Some(reversed)) = (
        load(r"Data\Environments\16_Track\track.vex"),
        load(r"Data\Environments\16_Track\track_reversed.vex"),
    ) else {
        return;
    };
    let forward = forward
        .track_panel
        .expect("the forward circuit has a panel");
    let reversed = reversed
        .track_panel
        .expect("the reversed circuit has a panel");
    for panel in [&forward, &reversed] {
        assert!(!panel.name().is_empty() && !panel.description().is_empty());
        assert!(
            !panel.name().ends_with("_Track") && !panel.description().starts_with("MSC_TRACK"),
            "a string that did not resolve falls back to its own id"
        );
    }
    assert_ne!(
        forward.name(),
        reversed.name(),
        "16_Track and 32_Track are different races on one directory"
    );
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn the_panel_draws_both_strings_inside_the_wipe_and_nothing_once_gone() {
    let Some(loaded) = load(r"Data\Environments\16_Track\track.vex") else {
        return;
    };
    let panel = loaded.track_panel.expect("a panel");
    let settled = panel.draw_lists(Progress {
        entered: 5.0,
        left: None,
    });
    assert!(texts(&settled.frame).contains(&panel.name()));
    assert!(texts(&settled.body).contains(&panel.description()));
    assert_eq!(settled.right, WIPE_WIDTH);
    assert!(
        settled
            .frame
            .iter()
            .any(|d| matches!(d, Draw::TiledSprite { .. })),
        "the hexagon tile reached the sheet"
    );
    assert!(
        settled
            .frame
            .iter()
            .any(|d| matches!(d, Draw::GradientFill { .. })),
        "the gradient rules draw"
    );
    let half = panel.draw_lists(Progress {
        entered: 1.0,
        left: None,
    });
    assert_eq!(
        half.right,
        WIPE_WIDTH / 2.0,
        "a second in, the wipe is half open"
    );
    let faded = Progress {
        entered: 5.0,
        left: Some(FADE_SECONDS),
    };
    assert!(!faded.visible());
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn a_race_carries_the_panel_from_the_flyby_to_seven_tenths_after_it() {
    let Some(loaded) = load(r"Data\Environments\16_Track\track.vex") else {
        return;
    };
    assert!(loaded.track_panel.is_some());
    let mut race = Race::start(loaded.setup);
    assert!(
        race.track_panel_progress().is_none(),
        "before the flyby the race draws as always"
    );
    assert!(race.begin_intro());
    race.tick_intro(&PlayerInputs::none());
    let first = race.track_panel_progress().expect("up from the first tick");
    assert!(first.left.is_none() && first.alpha() < 0.1);
    while race.in_intro() {
        race.tick_intro(&cross());
    }
    // The tick the flyby ends on shows the chase view with the panel still up, fading.
    let leaving = race.track_panel_progress().expect("still fading");
    assert_eq!(leaving.left, Some(0.0));
    assert!(leaving.alpha() > 0.99, "a skipped flyby had faded in fully");
    let mut ticks = 0;
    while race.track_panel_progress().is_some() {
        race.tick(&PlayerInputs::none());
        ticks += 1;
        assert!(ticks < 100, "the panel never left");
    }
    assert_eq!(ticks, 42, "0.7 s at 60 Hz");
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn a_pointer_press_skips_the_flyby_when_the_lock_lifts_and_not_before() {
    let Some(loaded) = load(r"Data\Environments\16_Track\track.vex") else {
        return;
    };
    let mut race = Race::start(loaded.setup);
    assert!(race.begin_intro());
    race.skip_intro();
    let mut ticks = 0u32;
    while race.in_intro() {
        race.tick_intro(&PlayerInputs::none());
        ticks += 1;
        assert!(ticks < 3000, "a press did not end the flyby");
    }
    assert_eq!(ticks, oag_pulse::pre_race::PRE_RACE.lock_ticks.value + 1);
}

/// What `progress` draws over a flat mid-grey 480x272 target: RGBA, or `None` with no adapter.
fn pixels(panel: oag_raceplay::track_panel::Assets, progress: Progress) -> Option<Vec<u8>> {
    let instance = wgpu::Instance::default();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).ok()?;
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).ok()?;
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let (width, height) = (480u32, 272u32);
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("track panel"),
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
    let mut overlay =
        oag_game::track_panel::Overlay::new(&device, &queue, format, panel).expect("the panel");
    let mut encoder = device.create_command_encoder(&Default::default());
    drop(encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: None,
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: &view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color {
                    r: 0.5,
                    g: 0.5,
                    b: 0.5,
                    a: 1.0,
                }),
                store: wgpu::StoreOp::Store,
            },
        })],
        ..Default::default()
    }));
    overlay.draw(
        &device,
        &queue,
        &mut encoder,
        &view,
        progress,
        (0.0, 0.0, width as f32, height as f32),
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
    device.poll(wgpu::PollType::wait_indefinitely()).ok()?;
    let data = buffer.slice(..).get_mapped_range().ok()?;
    let mut out = Vec::with_capacity(width as usize * height as usize * 4);
    for row in 0..height as usize {
        out.extend_from_slice(&data[row * padded..row * padded + width as usize * 4]);
    }
    Some(out)
}

/// How many pixels in the same window are near white: ink, which the bars and tile (at most 0.69
/// over the grey) never reach.
fn ink(rgba: &[u8], x0: usize, x1: usize, y0: usize, y1: usize) -> usize {
    (y0..y1)
        .flat_map(|y| (x0..x1).map(move |x| (x, y)))
        .filter(|(x, y)| rgba[(y * 480 + x) * 4..][..3].iter().all(|c| *c > 215))
        .count()
}

/// How many pixels in columns `x0..x1` and rows `y0..y1` are not the 0.5 grey the target was
/// cleared to.
fn touched(rgba: &[u8], x0: usize, x1: usize, y0: usize, y1: usize) -> usize {
    (y0..y1)
        .flat_map(|y| (x0..x1).map(move |x| (x, y)))
        .filter(|(x, y)| {
            let at = (y * 480 + x) * 4;
            rgba[at..at + 3].iter().any(|c| c.abs_diff(128) > 3)
        })
        .count()
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd and a GPU adapter"]
fn the_panel_reaches_the_pixels_wipes_open_and_leaves() {
    let Some(loaded) = load(r"Data\Environments\16_Track\track.vex") else {
        return;
    };
    let panel = loaded.track_panel.expect("a panel");
    let Some(open) = pixels(
        panel.clone(),
        Progress {
            entered: 5.0,
            left: None,
        },
    ) else {
        return;
    };
    assert!(touched(&open, 0, 480, 5, 36) > 500, "the title bar draws");
    assert!(
        touched(&open, 0, 480, 205, 272) > 3000,
        "the paragraph's bar draws"
    );
    assert_eq!(
        touched(&open, 0, 480, 60, 190),
        0,
        "the picture between is left alone"
    );
    let half = pixels(
        panel.clone(),
        Progress {
            entered: 1.0,
            left: None,
        },
    )
    .expect("the same adapter");
    // The bars are whole by a second; the texts are clipped to the wipe's half.
    assert!(
        ink(&half, 12, 235, 6, 34) > 50,
        "the title is inked left of the edge"
    );
    assert_eq!(
        ink(&half, 240, 480, 6, 34),
        0,
        "no title ink past the wipe's edge"
    );
    assert!(
        ink(&open, 240, 330, 6, 34) > 50,
        "settled, the title runs past it"
    );
    let gone = pixels(
        panel,
        Progress {
            entered: 5.0,
            left: Some(FADE_SECONDS),
        },
    )
    .expect("the same adapter");
    assert_eq!(
        touched(&gone, 0, 480, 0, 272),
        0,
        "nothing is left once it has faded"
    );
}
