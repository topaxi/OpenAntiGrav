//! Multi-window: the views are dealt across OS windows, each cut up inside its window like a
//! split screen, so the layout sees the window's own views, and a frame's camera-independent
//! work (the shadow maps, the scenery clocks, the advert cards) runs once however many windows
//! draw.

use super::scene_build::{bare_scene, test_gpu};
use super::*;
use oag_display::display::{Fov, Shadows};

fn two_and_two() -> Vec<WindowViews> {
    vec![
        WindowViews {
            seats: vec![0, 1],
            cut: oag_render::viewport::Cut::Vertical,
        },
        WindowViews {
            seats: vec![2, 3],
            cut: oag_render::viewport::Cut::Horizontal,
        },
    ]
}

/// Each window cuts its own seats by its own cut; the cameras are the race's four. Fails if
/// the layout reads the race-wide count or cut instead of the window's.
#[test]
fn each_window_cuts_its_own_seats_by_its_own_cut() {
    let mut race = race_with_a_grid();
    assert_eq!(race.set_viewport_count(4), 4);
    let area = (0.0, 0.0, 1280.0, 720.0);
    assert_eq!(race.view_rects(area).len(), 4, "a split cuts the area");
    assert!(!race.windowed());

    race.set_windows(two_and_two());
    assert_eq!(race.viewport_count(), 4, "four cameras, four seats");
    assert_eq!(race.window_drawn(0), vec![0, 1]);
    assert_eq!(race.window_drawn(1), vec![2, 3]);
    let left_right = race.window_rects(0, area);
    assert_eq!(left_right.len(), 2);
    assert!(
        left_right[0].0 < left_right[1].0,
        "window 1 is cut left and right"
    );
    let top_bottom = race.window_rects(1, area);
    assert!(
        top_bottom[0].1 < top_bottom[1].1,
        "window 2 is cut top and bottom"
    );
    // What the track panel and the end screens are laid out against is window 1's.
    assert_eq!(race.views_drawn(), 2);
    assert_eq!(race.view_rects(area), left_right);
    assert!(race.window_drawn(2).is_empty(), "no such window");

    race.set_windows(Vec::new());
    assert_eq!(race.view_rects(area).len(), 4);
}

/// A window of one seat is the whole window, and a window may hold seats the race has no
/// camera for: they are skipped, not drawn from the wrong craft.
#[test]
fn a_one_seat_window_is_whole_and_a_seat_without_a_camera_is_skipped() {
    let mut race = race_with_a_grid();
    assert_eq!(race.set_viewport_count(3), 3);
    race.set_windows(vec![
        WindowViews {
            seats: vec![0],
            cut: Default::default(),
        },
        WindowViews {
            seats: vec![1, 2, 3],
            cut: Default::default(),
        },
    ]);
    let area = (0.0, 0.0, 640.0, 360.0);
    assert_eq!(race.window_rects(0, area), vec![area]);
    assert_eq!(race.window_drawn(1), vec![1, 2], "slot 3 has no camera");
}

/// Once every person has finished, each window that holds a seat draws the spectator view of the
/// player alone (the end screens sit over it, at the window's own size), and a window with no
/// seat still draws nothing. Fails on the build where only the window holding the player
/// collapsed and the others kept their chase cameras.
#[test]
fn once_the_race_is_over_every_window_with_a_seat_draws_the_one_end_view() {
    let mut race = race_with_a_grid();
    race.sim.world.controllers[1] = oag_gameplay::Controller::Local;
    race.sim.world.controllers[2] = oag_gameplay::Controller::Local;
    assert_eq!(race.set_viewport_count(3), 3);
    race.set_windows(vec![
        WindowViews {
            seats: vec![0],
            cut: Default::default(),
        },
        WindowViews {
            seats: vec![1, 2],
            cut: oag_render::viewport::Cut::Vertical,
        },
        WindowViews {
            seats: Vec::new(),
            cut: Default::default(),
        },
    ]);
    assert_eq!(race.window_drawn(1), vec![1, 2], "racing: its own seats");
    for slot in 0..3 {
        race.sim.world.race[slot].finished = true;
    }
    assert!(race.finished());
    let player = race.player_slot();
    assert_eq!(race.window_drawn(0), vec![player]);
    assert_eq!(
        race.window_drawn(1),
        vec![player],
        "window 2 shows the end too"
    );
    assert_eq!(
        race.window_cut(1),
        Default::default(),
        "one view is not cut"
    );
    assert!(race.window_drawn(2).is_empty(), "a window with no seat");
}

/// Closing a window takes it out of the layout and the windows after it keep their place.
#[test]
fn removing_a_window_keeps_the_others_and_never_removes_window_one() {
    let mut race = race_with_a_grid();
    race.set_viewport_count(4);
    race.set_windows(two_and_two());
    race.remove_window(0);
    assert_eq!(race.windows().len(), 2, "window 1 is never removed");
    race.remove_window(1);
    assert_eq!(race.windows().len(), 1);
    assert_eq!(race.window_drawn(0), vec![0, 1]);
    assert_eq!(
        race.viewport_count(),
        4,
        "the closed window's seats are still in the race"
    );
}

fn four_windows() -> Vec<WindowViews> {
    (0..4)
        .map(|slot| WindowViews {
            seats: vec![slot],
            cut: Default::default(),
        })
        .collect()
}

fn target(device: &wgpu::Device) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("windowed test target"),
            size: wgpu::Extent3d {
                width: 64,
                height: 64,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        })
        .create_view(&Default::default())
}

/// Window `window` of `race` drawn into `target` and submitted, the way a window's frame loop
/// does it. `shared` says an earlier window of the frame has drawn.
fn draw_window(
    (device, queue): (&wgpu::Device, &wgpu::Queue),
    scene: &Scene,
    race: &Race,
    target: &wgpu::TextureView,
    window: usize,
    shadows: Shadows,
    shared: bool,
) {
    draw_window_with(
        (device, queue),
        scene,
        race,
        target,
        window,
        shadows,
        shared,
        WindowEffects::default(),
    );
}

#[allow(clippy::too_many_arguments)]
fn draw_window_with(
    (device, queue): (&wgpu::Device, &wgpu::Queue),
    scene: &Scene,
    race: &Race,
    target: &wgpu::TextureView,
    window: usize,
    shadows: Shadows,
    shared: bool,
    effects: WindowEffects<'_>,
) {
    let mut encoder = device.create_command_encoder(&Default::default());
    scene.render_window(
        device,
        queue,
        &mut encoder,
        target,
        race,
        window,
        (0.0, 0.0, 64.0, 64.0),
        Fov::AUTHORED,
        true,
        true,
        None,
        shadows,
        &[],
        shared,
        effects,
        &mut [],
    );
    queue.submit(Some(encoder.finish()));
    assert_eq!(
        race.extra_viewpoint(),
        None,
        "window {window} puts the player's view back"
    );
}

/// Four one-seat windows are one frame: the first runs the shadow maps, the scenery clocks and the
/// advert cards and the other three read what it left, and the next frame runs them again, once.
/// Fails if a window does the work itself (the old behaviour: four runs a frame), or if the
/// work is never done again after the first frame.
#[test]
fn a_frame_does_the_shared_work_once_across_its_windows() {
    let Some((device, queue)) = test_gpu() else {
        return;
    };
    let scene = bare_scene(&device, &queue);
    let mut race = race_with_a_grid();
    assert_eq!(race.set_viewport_count(4), 4);
    race.set_windows(four_windows());
    let target = target(&device);
    for frame in 1..=3_u32 {
        for window in 0..4 {
            draw_window(
                (&device, &queue),
                &scene,
                &race,
                &target,
                window,
                Shadows::Original,
                window > 0,
            );
            let work = scene.frame_work();
            assert_eq!(work.scenery_clocks, frame, "frame {frame}, window {window}");
            assert_eq!(work.shadow_maps, frame, "frame {frame}, window {window}");
            assert_eq!(work.advert_cards, frame, "frame {frame}, window {window}");
            assert_eq!(work.shine_maps, frame, "frame {frame}, window {window}");
        }
    }
}

/// `shared` is read: a window told an earlier one drew does none of it, so a caller that says it
/// wrongly is a stale shadow, not a double run.
#[test]
fn a_window_told_the_work_is_done_does_none_of_it() {
    let Some((device, queue)) = test_gpu() else {
        return;
    };
    let scene = bare_scene(&device, &queue);
    let mut race = race_with_a_grid();
    assert_eq!(race.set_viewport_count(4), 4);
    race.set_windows(four_windows());
    let target = target(&device);
    draw_window(
        (&device, &queue),
        &scene,
        &race,
        &target,
        0,
        Shadows::Original,
        true,
    );
    let work = scene.frame_work();
    assert_eq!(
        (
            work.scenery_clocks,
            work.shadow_maps,
            work.advert_cards,
            work.shine_maps
        ),
        (0, 0, 0, 0)
    );
}

/// The `mapped` tier's depth map is fitted on the craft the view follows, so every window
/// redraws it for its own seat even when the rest of the work is shared.
#[test]
fn a_shared_window_still_fits_the_mapped_depth_map_on_its_own_craft() {
    let Some((device, queue)) = test_gpu() else {
        return;
    };
    let scene = bare_scene(&device, &queue);
    let mut race = race_with_a_grid();
    assert_eq!(race.set_viewport_count(4), 4);
    race.set_windows(four_windows());
    let target = target(&device);
    draw_window(
        (&device, &queue),
        &scene,
        &race,
        &target,
        0,
        Shadows::Mapped,
        false,
    );
    let first = scene.mapped_matrix();
    draw_window(
        (&device, &queue),
        &scene,
        &race,
        &target,
        3,
        Shadows::Mapped,
        true,
    );
    assert_ne!(
        scene.mapped_matrix(),
        first,
        "window 4's map is fitted on its own craft"
    );
    assert_eq!(
        scene.frame_work().shadow_maps,
        1,
        "the coverage map ran once"
    );
}

/// A window of several seats draws them as a split does - the first view does the frame's shared
/// work, the later ones read it - and a second window after it does none: two windows of two seats
/// run the scenery clocks once.
#[test]
fn windows_of_several_seats_do_the_shared_work_once_and_leave_the_viewpoint() {
    let Some((device, queue)) = test_gpu() else {
        return;
    };
    let scene = bare_scene(&device, &queue);
    let mut race = race_with_a_grid();
    assert_eq!(race.set_viewport_count(4), 4);
    race.set_windows(two_and_two());
    let target = target(&device);
    for window in 0..2 {
        draw_window(
            (&device, &queue),
            &scene,
            &race,
            &target,
            window,
            Shadows::Original,
            window > 0,
        );
        assert_eq!(
            scene.frame_work().scenery_clocks,
            1,
            "window {window}: the clocks ran once for the frame"
        );
    }
}

/// Each window keeps the history of its own camera: window A's previous view-projection is A's
/// last one however many other windows drew between, and the camera-jitter phase advances once
/// per window frame. Fails with one shared history, where window 2 reads window 1's previous
/// view (the same tick and draw alpha are "the same frame") and takes every other jitter phase.
#[test]
fn each_window_keeps_the_history_of_its_own_camera() {
    let Some((device, queue)) = test_gpu() else {
        return;
    };
    let gpu = (&device, &queue);
    let scene = bare_scene(&device, &queue);
    let mut race = race_with_a_grid();
    assert_eq!(race.set_viewport_count(2), 2);
    race.set_windows(vec![
        WindowViews {
            seats: vec![0],
            cut: Default::default(),
        },
        WindowViews {
            seats: vec![1],
            cut: Default::default(),
        },
    ]);
    let target = target(&device);
    let phases = 8;
    let mut last = [None; 2];
    for frame in 0..4_u32 {
        race.tick(&oag_gameplay::PlayerInputs::none());
        #[allow(clippy::needless_range_loop)]
        for window in 0..2 {
            draw_window_with(
                gpu,
                &scene,
                &race,
                &target,
                window,
                Shadows::Original,
                window > 0,
                WindowEffects {
                    motion_blur: oag_display::display::MotionBlur::Medium,
                    camera_jitter: Some(phases),
                    ..Default::default()
                },
            );
            let record = scene.temporal(window).expect("a frame was recorded").frames[0];
            assert_eq!(
                record.jitter,
                oag_post::jitter::offset_pixels(frame, phases),
                "window {window}, frame {frame}: the jitter phase is the window's own frame count"
            );
            assert_eq!(
                record.reset,
                frame == 0,
                "window {window}, frame {frame}: only a window's first frame resets its history"
            );
            let (previous, current) = scene.motion_probe(window).expect("a history");
            if let Some(before) = last[window] {
                assert_eq!(
                    previous, before,
                    "window {window}, frame {frame}: its previous camera is its own last one"
                );
            }
            last[window] = Some(current);
        }
        assert_ne!(
            last[0], last[1],
            "the two windows look through different cameras"
        );
    }
}
