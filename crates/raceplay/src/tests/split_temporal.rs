//! A split screen feeds a temporal upscaler per view: each view jitters on its own seat's
//! sequence and records its frame in its seat's history, and [`Scene::temporal`] answers one
//! frame per view with the view's seat and rectangle.

use super::scene_build::test_gpu;
use super::split_blur::{AREA, hull_scene, target};
use super::*;
use oag_display::display::{Fov, MotionBlur, Msaa, Shadows};

fn draw_split(
    (device, queue): (&wgpu::Device, &wgpu::Queue),
    scene: &Scene,
    race: &Race,
    target: &wgpu::Texture,
    phases: Option<u32>,
) {
    scene.render_viewports(
        device,
        queue,
        &target.create_view(&Default::default()),
        race,
        AREA,
        Fov::AUTHORED,
        true,
        true,
        None,
        Shadows::Off,
        &[],
        WindowEffects {
            camera_jitter: phases,
            ..Default::default()
        },
        &mut [],
    );
}

/// Three split frames with jitter on: one frame per view, each naming its seat and its own
/// rectangle, each jittered at its seat's own frame count (the first frame resets, the later
/// ones do not), and the two views' cameras apart. Then a single-view frame answers one whole
/// picture again. Fails when a split's views draw no jitter (no record: `None`), when the
/// views share one jitter sequence (view 1 a phase ahead of view 0), and when a split is
/// answered as one picture.
#[test]
fn each_view_of_a_split_records_its_own_jittered_frame() {
    let Some((device, queue)) = test_gpu() else {
        return;
    };
    let gpu = (&device, &queue);
    let mut race = race_with_a_grid();
    assert_eq!(race.set_viewport_count(2), 2);
    let scene = hull_scene(&device, &queue, &race, Msaa::Off, None, None);
    let target = target(&device);
    let phases = 8;
    draw_split(gpu, &scene, &race, &target, None);
    assert!(
        scene.temporal(0).is_none(),
        "a split with no jitter feeds no upscaler"
    );
    let rects = race.view_rects(AREA);
    for frame in 0..3_u32 {
        race.tick(&oag_gameplay::PlayerInputs::none());
        draw_split(gpu, &scene, &race, &target, Some(phases));
        let temporal = scene.temporal(0).expect("each view recorded a frame");
        assert_eq!(temporal.frames.len(), 2, "one frame per view");
        for (index, view) in temporal.frames.iter().enumerate() {
            let rect = rects[index];
            assert_eq!(
                view.view,
                Some((
                    index,
                    (rect.0 as u32, rect.1 as u32, rect.2 as u32, rect.3 as u32)
                )),
                "frame {frame}: view {index} names its seat and rectangle"
            );
            // The no-jitter frame above counted a phase too (the counter runs whether or not
            // jitter is on), so the sequence is one ahead of `frame`.
            assert_eq!(
                view.jitter,
                oag_post::jitter::offset_pixels(frame + 1, phases),
                "frame {frame}: view {index} jitters at its own seat's frame count"
            );
            assert!(!view.reset, "frame {frame}: view {index} keeps its history");
        }
        assert_ne!(
            temporal.frames[0].camera, temporal.frames[1].camera,
            "the views' cameras are their own"
        );
    }
    assert_eq!(race.set_viewport_count(1), 1);
    let mut encoder = device.create_command_encoder(&Default::default());
    scene.render(
        &device,
        &queue,
        &mut encoder,
        &target.create_view(&Default::default()),
        &race,
        AREA,
        Fov::AUTHORED,
        true,
        true,
        None,
        MotionBlur::Off,
        Shadows::Off,
        Some(phases),
        &[],
        None,
        None,
        None,
    );
    queue.submit(Some(encoder.finish()));
    let temporal = scene.temporal(0).expect("the single view recorded a frame");
    assert_eq!(temporal.frames.len(), 1);
    assert_eq!(temporal.frames[0].view, None, "a whole picture again");
}
