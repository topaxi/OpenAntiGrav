//! A run of frames draws the same however a split records them: a single frame shows a wrong
//! order of writes but not a stale buffer, which takes a value that moved between frames, a
//! view that wrote nothing this frame, or a write its dedupe skipped because the buffer already
//! held that value.

use super::scene_build::{glowing_scene, test_gpu};
use super::split_blur::{AREA, pixels, target};
use super::split_exposure::{CHAINS, scene_of};
use super::*;
use oag_display::display::{Fov, MotionBlur, Shadows};

const RECORDINGS: [Recording; 3] = [Recording::Serial, Recording::Reversed, Recording::Threaded];

fn draw(
    (device, queue): (&wgpu::Device, &wgpu::Queue),
    scene: &Scene,
    race: &Race,
    target: &wgpu::Texture,
    blur: MotionBlur,
) -> Vec<u8> {
    // What follows the views in the frame's own encoder, as the session's frame has it: an open
    // frame is what lets a split's commands wait for that one submission.
    let frame = oag_gpu::split_finish::Frame::open();
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
        Shadows::Original,
        &[],
        WindowEffects {
            motion_blur: blur,
            ..WindowEffects::default()
        },
        &mut [],
    );
    let mut encoder = device.create_command_encoder(&Default::default());
    scene.composite_bloom(&mut encoder, &target.create_view(&Default::default()), AREA);
    frame.submit(queue, encoder);
    pixels(device, queue, target)
}

/// Each scene drawn the same race frame by frame, the first interleaved and each other with
/// its own recording, through frames where the count of views changes: every frame's pixels
/// are the interleaved scene's.
fn same_frames(
    (device, queue): (&wgpu::Device, &wgpu::Queue),
    build: impl Fn(&Race) -> Scene,
    blur: MotionBlur,
    name: &str,
) {
    for recording in RECORDINGS {
        let mut race = race_with_a_grid();
        assert_eq!(race.set_viewport_count(2), 2);
        let interleaved = build(&race);
        interleaved.set_split_recording(Recording::Interleaved);
        let batched = build(&race);
        batched.set_split_recording(recording);
        let (left, right) = (target(device), target(device));
        for frame in 0..12 {
            for _ in 0..20 {
                race.tick(&oag_gameplay::PlayerInputs::none());
            }
            // A view that goes away and comes back: its buffers hold what they held then.
            match frame {
                5 => assert_eq!(race.set_viewport_count(1), 1),
                6 => assert_eq!(race.set_viewport_count(2), 2),
                8 => assert_eq!(race.set_viewport_count(4), 4),
                _ => {}
            }
            let want = draw((device, queue), &interleaved, &race, &left, blur);
            let got = draw((device, queue), &batched, &race, &right, blur);
            assert!(
                want == got,
                "{name}: {recording:?} leaves frame {frame} different from interleaved"
            );
        }
    }
}

/// HD's, Omega's and 2048's chains hold constants written once per seat and skip a write of
/// the value the buffer already holds.
#[test]
fn a_run_of_frames_through_each_post_chain_draws_the_same_however_it_is_recorded() {
    let Some((device, queue)) = test_gpu() else {
        return;
    };
    for chain in CHAINS {
        same_frames(
            (&device, &queue),
            |race| scene_of(&device, &queue, race, chain),
            MotionBlur::Off,
            &format!("{chain:?}"),
        );
    }
}

/// Pulse's glow and the blur run after the last view, over one rectangle at a time.
#[test]
fn a_run_of_frames_with_glow_and_blur_draws_the_same_however_it_is_recorded() {
    let Some((device, queue)) = test_gpu() else {
        return;
    };
    same_frames(
        (&device, &queue),
        |_| glowing_scene(&device, &queue),
        MotionBlur::Off,
        "glow",
    );
    same_frames(
        (&device, &queue),
        |race| scene_of(&device, &queue, race, CHAINS[0]),
        MotionBlur::High,
        "blur",
    );
}
