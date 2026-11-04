//! A split screen drawn while the frame loop has a split frame open (`oag_gpu::split_finish`,
//! on by default on a machine with uniform cores) is the picture it is without one: each
//! view's scene pass is split off the view's encoder, and what follows it in the view's
//! submission (the blur over the finished picture) must still run behind it.

use super::scene_build::test_gpu;
use super::split_blur::{AREA, drawn, hull_scene, pixels, rows, target};
use super::*;
use oag_display::display::{Fov, MotionBlur, Msaa, Shadows};

fn draw(
    (device, queue): (&wgpu::Device, &wgpu::Queue),
    scene: &Scene,
    race: &Race,
    target: &wgpu::Texture,
    blur: MotionBlur,
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
            motion_blur: blur,
            ..Default::default()
        },
        &mut [],
    );
}

/// Craft 0 travels for three frames; each frame is drawn with the blur on, once as the frame
/// loop's open frame would have it and once without. The two pictures are the same, and the
/// blur is in them (the first view differs from the picture with the blur off). Fails when a
/// view's post work is submitted ahead of its own scene pass: the blur then reads the
/// previous frame's picture and the framed picture lags the plain one.
#[test]
fn a_split_drawn_in_an_open_frame_is_the_picture_it_is_without_one() {
    let Some((device, queue)) = test_gpu() else {
        return;
    };
    let gpu = (&device, &queue);
    let mut race = race_with_a_grid();
    assert_eq!(race.set_viewport_count(2), 2);
    let scene = hull_scene(&device, &queue, &race, Msaa::Off, None, None, None);
    let (off, plain, framed) = (target(&device), target(&device), target(&device));
    for _ in 0..90 {
        race.tick(&oag_gameplay::PlayerInputs::none());
    }
    for _ in 0..3 {
        let forward = race.sim.world.ships[0].physics.body.forward();
        let ahead = race.sim.world.ships[0].physics.body.position + forward * 6.0;
        race.sim.world.ships[0].physics.body.position = ahead;
        race.tick(&oag_gameplay::PlayerInputs::none());
        draw(gpu, &scene, &race, &off, MotionBlur::Off);
        draw(gpu, &scene, &race, &plain, MotionBlur::High);
        let frame = oag_gpu::split_finish::Frame::open();
        draw(gpu, &scene, &race, &framed, MotionBlur::High);
        drop(frame);
    }
    let rects = race.view_rects(AREA);
    let [off, plain, framed] =
        [&off, &plain, &framed].map(|texture| pixels(&device, &queue, texture));
    for (view, rect) in rects.iter().enumerate() {
        assert!(drawn(rows(&plain, *rect)), "view {view} shows a craft");
        assert_eq!(
            rows(&framed, *rect),
            rows(&plain, *rect),
            "view {view}: an open frame changes the picture"
        );
    }
    assert!(
        rows(&plain, rects[0]) != rows(&off, rects[0]),
        "the moving view is blurred"
    );
}
