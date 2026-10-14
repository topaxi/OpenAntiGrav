//! Multi-window: every view fills a window of its own, so the layout sees one view
//! and each window does a whole frame's work.

use super::scene_build::{bare_scene, test_gpu};
use super::*;
use oag_display::display::{Fov, Shadows};

#[test]
fn a_windowed_race_keeps_its_cameras_and_draws_one_view_per_window() {
    let mut race = race_with_a_grid();
    assert_eq!(race.set_viewport_count(4), 4);
    let area = (0.0, 0.0, 1280.0, 720.0);
    assert_eq!(race.view_rects(area).len(), 4, "a split cuts the area");
    assert!(!race.windowed());

    race.set_windowed(true);
    assert_eq!(race.viewport_count(), 4, "four cameras, four seats");
    assert_eq!(race.views_drawn(), 1, "each window draws one");
    assert_eq!(
        race.view_rects(area),
        vec![area],
        "the view is the whole window"
    );

    race.set_windowed(false);
    assert_eq!(race.view_rects(area).len(), 4);
}

#[test]
fn each_window_does_the_frames_shared_work_itself_and_leaves_the_viewpoint() {
    let Some((device, queue)) = test_gpu() else {
        return;
    };
    let scene = bare_scene(&device, &queue);
    let mut race = race_with_a_grid();
    assert_eq!(race.set_viewport_count(4), 4);
    race.set_windowed(true);
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
