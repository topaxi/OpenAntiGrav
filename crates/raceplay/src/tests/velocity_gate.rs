//! The track and the hulls write the velocity buffer on a frame that reads it and
//! not on one that does not: the wiring of `scene::velocity_gate` into
//! `Scene::render`, one frame at a time.

use super::craft_cull::target;
use super::scene_build::{bare_scene, test_gpu};
use super::*;
use oag_display::display::{Fov, MotionBlur, Shadows};

/// Whether the track was told to write velocity by a frame drawn with `blur` and `jitter`.
fn gate_after(blur: MotionBlur, jitter: Option<u32>) -> Option<bool> {
    let (device, queue) = test_gpu()?;
    let scene = bare_scene(&device, &queue);
    let view = target(&device);
    let mut race = race_with_a_grid();
    race.tick(&oag_gameplay::PlayerInputs::none());
    let mut encoder = device.create_command_encoder(&Default::default());
    scene.render(
        &device,
        &queue,
        &mut encoder,
        &view,
        &race,
        (0.0, 0.0, 64.0, 64.0),
        Fov::AUTHORED,
        true,
        true,
        None,
        blur,
        Shadows::Off,
        jitter,
        &[],
        None,
        None,
        None,
    );
    queue.submit([encoder.finish()]);
    Some(scene.velocity_gate_open())
}

/// With the blur off and no jitter nothing reads velocity, so the gate is shut.
/// Drop the call in `Scene::render` and the track is never told, and this reads open.
#[test]
fn a_frame_nobody_reads_does_not_write_velocity() {
    if let Some(open) = gate_after(MotionBlur::Off, None) {
        assert!(!open);
    }
}

/// Each reader on its own opens it: the blur at any strength, and camera jitter,
/// which is how a temporal upscaler asks for its velocity.
#[test]
fn each_reader_turns_the_write_on() {
    for blur in [MotionBlur::Low, MotionBlur::Medium, MotionBlur::High] {
        if let Some(open) = gate_after(blur, None) {
            assert!(open, "blur {blur:?} reads velocity");
        }
    }
    if let Some(open) = gate_after(MotionBlur::Off, Some(8)) {
        assert!(open, "a jittered frame hands its velocity to the upscaler");
    }
}
