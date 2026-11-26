//! A viewport squeezed into fewer columns (`Scene::set_squeeze`) is drawn with the camera of
//! the full picture, so the host's stretch gives back the same framing rather than a crop.

use super::craft_cull::target;
use super::scene_build::{bare_scene, test_gpu};
use super::*;
use oag_core::math::Mat4;
use oag_display::display::{Fov, MotionBlur, Shadows};

/// The projection a frame of `width` by 64 pixels at `squeeze` is drawn with.
fn projection_at(width: f32, squeeze: f32) -> Option<Mat4> {
    let (device, queue) = test_gpu()?;
    let scene = bare_scene(&device, &queue);
    let view = target(&device);
    let mut race = race_with_a_grid();
    race.tick(&oag_gameplay::PlayerInputs::none());
    scene.set_squeeze(squeeze);
    let mut encoder = device.create_command_encoder(&Default::default());
    scene.render(
        &device,
        &queue,
        &mut encoder,
        &view,
        &race,
        (0.0, 0.0, width, 64.0),
        Fov::AUTHORED,
        true,
        true,
        None,
        MotionBlur::Off,
        Shadows::Off,
        None,
        &[],
        None,
        None,
        None,
    );
    queue.submit([encoder.finish()]);
    scene.drawn_projection()
}

/// Half the columns at a squeeze of a half is the whole picture's camera, to the bit. Drop the
/// squeeze from `Scene::view_aspect` and the half width is drawn as a narrower picture.
#[test]
fn a_squeezed_viewport_keeps_the_whole_pictures_camera() {
    let (Some(whole), Some(squeezed), Some(cropped)) = (
        projection_at(64.0, 1.0),
        projection_at(32.0, 0.5),
        projection_at(32.0, 1.0),
    ) else {
        return;
    };
    assert_eq!(squeezed, whole);
    assert_ne!(cropped, whole);
}
