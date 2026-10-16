//! A multi-window race where one window holds a single seat and another a split: the split's
//! seats keep their camera histories and HD's and Omega's adaptation from frame to frame
//! whichever window draws first, and a seat missing for a frame is forgotten alone
//! ([`Scene::keep_split_seat`]).

use super::scene_build::test_gpu;
use super::split_blur::{AREA, target};
use super::split_exposure::{CHAINS, Chain, scene_of};
use super::*;
use oag_display::display::{Fov, Shadows};

fn window(seats: &[usize]) -> WindowViews {
    WindowViews {
        seats: seats.to_vec(),
        cut: Default::default(),
    }
}

/// Moves every craft forward and ticks, so each camera has travelled since the last frame.
fn step(race: &mut Race) {
    for slot in 0..3 {
        let forward = race.sim.world.ships[slot].physics.body.forward();
        race.sim.world.ships[slot].physics.body.position += forward * (3.0 + slot as f32);
    }
    race.tick(&oag_gameplay::PlayerInputs::none());
}

/// One frame: every window of the layout, the first drawn with `shared` false.
fn frame((device, queue): (&wgpu::Device, &wgpu::Queue), scene: &Scene, race: &Race) {
    let target = target(device);
    let view = target.create_view(&Default::default());
    for index in 0..race.windows().len() {
        let mut encoder = device.create_command_encoder(&Default::default());
        scene.render_window(
            device,
            queue,
            &mut encoder,
            &view,
            race,
            index,
            AREA,
            Fov::AUTHORED,
            true,
            true,
            None,
            Shadows::Off,
            &[],
            index > 0,
            WindowEffects::default(),
            &mut [],
        );
        queue.submit(Some(encoder.finish()));
    }
}

/// The split window's two seats keep their own camera's history and their adaptation lane
/// across four frames, beside a window of one seat drawn before or after it. Fails when a
/// one-seat window's frame forgets the split's seats (the old rule): each seat's previous
/// camera is then its current one, and a lane is built every frame.
#[test]
fn a_one_seat_window_leaves_the_split_windows_seats_alone() {
    let Some((device, queue)) = test_gpu() else {
        return;
    };
    let gpu = (&device, &queue);
    for chain in CHAINS {
        for windows in [
            vec![window(&[0]), window(&[1, 2])],
            vec![window(&[1, 2]), window(&[0])],
        ] {
            let mut race = race_with_a_grid();
            assert_eq!(race.set_viewport_count(3), 3);
            race.set_windows(windows);
            let scene = scene_of(&device, &queue, &race, chain);
            let mut last = [None; 2];
            for number in 0..4 {
                step(&mut race);
                frame(gpu, &scene, &race);
                for (seat, last) in [1, 2].into_iter().zip(&mut last) {
                    let (previous, current) = scene.split_motion_probe(seat).expect("a history");
                    if let Some(before) = *last {
                        assert_eq!(
                            previous, before,
                            "{chain:?}, frame {number}, seat {seat}: its previous camera is its own last one"
                        );
                        assert_ne!(previous, current, "{chain:?}: seat {seat} moved");
                    }
                    *last = Some(current);
                }
            }
            assert_eq!(
                scene.split_exposure_builds(),
                2,
                "{chain:?}: one lane per seat, built once"
            );
        }
    }
}

/// A seat that misses a frame of the split is forgotten alone: seat 2 drawn by a window of its
/// own for a frame comes back measuring its first frame against itself with a lane built again,
/// while seat 1, a split view throughout, keeps its history and its lane.
#[test]
fn a_seat_back_after_a_gap_is_forgotten_and_the_others_are_not() {
    let Some((device, queue)) = test_gpu() else {
        return;
    };
    let gpu = (&device, &queue);
    let chain = Chain::Hd;
    let mut race = race_with_a_grid();
    assert_eq!(race.set_viewport_count(3), 3);
    race.set_windows(vec![window(&[0]), window(&[1, 2])]);
    let scene = scene_of(&device, &queue, &race, chain);
    for _ in 0..2 {
        step(&mut race);
        frame(gpu, &scene, &race);
    }
    assert_eq!(scene.split_exposure_builds(), 2);
    race.set_windows(vec![window(&[0, 1]), window(&[2])]);
    step(&mut race);
    frame(gpu, &scene, &race);
    // Seat 0 joins the split: one new lane. Seat 1 is a split view still: kept.
    assert_eq!(scene.split_exposure_builds(), 3);
    let (_, seat_one) = scene.split_motion_probe(1).expect("a history");
    race.set_windows(vec![window(&[0]), window(&[1, 2])]);
    step(&mut race);
    frame(gpu, &scene, &race);
    assert_eq!(
        scene.split_exposure_builds(),
        4,
        "seat 2 back after a gap builds its lane again, seat 1 does not"
    );
    let (previous, current) = scene.split_motion_probe(2).expect("a history");
    assert_eq!(
        previous, current,
        "seat 2 measures its first frame back against itself"
    );
    let (previous, _) = scene.split_motion_probe(1).expect("a history");
    assert_eq!(previous, seat_one, "seat 1 keeps its own last camera");
}
