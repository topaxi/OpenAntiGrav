//! Multi-window: the views are dealt across OS windows, each cut up inside its window like a
//! split screen, so the layout sees the window's own views and each window does a whole
//! frame's work.

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

#[test]
fn each_window_does_the_frames_shared_work_itself_and_leaves_the_viewpoint() {
    let Some((device, queue)) = test_gpu() else {
        return;
    };
    let scene = bare_scene(&device, &queue);
    let mut race = race_with_a_grid();
    assert_eq!(race.set_viewport_count(4), 4);
    race.set_windows(two_and_two());
    for slot in 0..4 {
        let target = device
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
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            })
            .create_view(&Default::default());
        let mut encoder = device.create_command_encoder(&Default::default());
        scene.render_view(
            &device,
            &queue,
            &mut encoder,
            &target,
            &race,
            slot,
            (0.0, 0.0, 64.0, 64.0),
            Fov::AUTHORED,
            true,
            true,
            None,
            Shadows::Original,
            &[],
            None,
        );
        queue.submit(Some(encoder.finish()));
        assert_eq!(
            race.extra_viewpoint(),
            None,
            "window {slot} puts the player's view back"
        );
        // A window is a whole frame: it is not a later view of a split, which would
        // read what an earlier one left.
        assert_eq!(
            scene.frame_work().scenery_clocks,
            slot as u32 + 1,
            "window {slot} runs the scenery clocks"
        );
    }
}

/// A window of several seats draws them as a split does - the first view does the frame's
/// shared work, the later ones read it - and each window is a frame of its own: two windows
/// of two seats run the scenery clocks twice, not four times and not once.
#[test]
fn a_window_of_several_seats_does_the_shared_work_once_and_leaves_the_viewpoint() {
    let Some((device, queue)) = test_gpu() else {
        return;
    };
    let scene = bare_scene(&device, &queue);
    let mut race = race_with_a_grid();
    assert_eq!(race.set_viewport_count(4), 4);
    race.set_windows(two_and_two());
    let target = device
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
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
        .create_view(&Default::default());
    for window in 0..2 {
        scene.render_window_views(
            &device,
            &queue,
            &target,
            &race,
            window,
            (0.0, 0.0, 64.0, 64.0),
            Fov::AUTHORED,
            true,
            true,
            None,
            Shadows::Original,
            &[],
        );
        assert_eq!(race.extra_viewpoint(), None, "the player's view is back");
        assert_eq!(
            scene.frame_work().scenery_clocks,
            window as u32 + 1,
            "window {window}: one run of the clocks for its two views"
        );
    }
}
