//! A split screen runs HD's post chain per view, each with an adaptation of its own
//! ([`Scene::run_split_hd`]).

use super::scene_build::test_gpu;
use super::split_blur::{AREA, drawn, hull_scene, pixels, rows, target};
use super::*;
use oag_display::display::{Fov, MotionBlur, Msaa, Shadows};

/// A circuit's `HDR and Bloom` block with an adaptation the picture can move, and no glow
/// (the exposure is what is under test).
fn params() -> oag_post::hd_bloom::Params {
    oag_post::hd_bloom::Params {
        alpha_contribution: 0.0,
        frame_contribution: 0.0,
        frame_exponent: 1.0,
        horizontal_size: 1.0,
        vertical_size: 1.0,
        adaption_rate: 0.5,
        adaption_boost: 1.0,
        tone_adaption_boost: 1.0,
        tone_darkening_clamp: 1.0,
        tone_maximum_brightness: 1.0,
        zoom: None,
    }
}

fn draw_split(
    (device, queue): (&wgpu::Device, &wgpu::Queue),
    scene: &Scene,
    race: &Race,
    target: &wgpu::Texture,
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
        WindowEffects::default(),
        &mut [],
    );
}

/// Every view of a split is resolved into its own rectangle by its own run of the chain, and
/// each seat keeps an adaptation of its own: two lanes after a two-view frame. Fails when the
/// chain is not run per view (no lanes, or a black view: the second view's resolve would be
/// the first's), and when a later view clears the frame (the first view goes black).
#[test]
fn every_view_resolves_into_its_own_rectangle_with_a_lane_of_its_own() {
    let Some((device, queue)) = test_gpu() else {
        return;
    };
    let mut race = race_with_a_grid();
    assert_eq!(race.set_viewport_count(2), 2);
    let scene = hull_scene(&device, &queue, &race, Msaa::Off, Some(params()));
    let target = target(&device);
    for _ in 0..90 {
        race.tick(&oag_gameplay::PlayerInputs::none());
    }
    draw_split((&device, &queue), &scene, &race, &target);
    let picture = pixels(&device, &queue, &target);
    let rects = race.view_rects(AREA);
    assert!(drawn(rows(&picture, rects[0])), "the first view survives");
    assert!(drawn(rows(&picture, rects[1])), "the second view is drawn");
    assert_eq!(scene.split_exposure_seats(), 2, "a lane per seat");
}

/// The first single-view frame after a split forgets the seats' lanes, as it forgets their
/// camera histories: a seat that comes back settles from its own first frame again.
#[test]
fn a_single_view_frame_forgets_the_seats_adaptation() {
    let Some((device, queue)) = test_gpu() else {
        return;
    };
    let mut race = race_with_a_grid();
    assert_eq!(race.set_viewport_count(2), 2);
    let scene = hull_scene(&device, &queue, &race, Msaa::Off, Some(params()));
    let target = target(&device);
    draw_split((&device, &queue), &scene, &race, &target);
    assert_eq!(scene.split_exposure_seats(), 2);
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
        None,
        &[],
        None,
        None,
        None,
    );
    queue.submit(Some(encoder.finish()));
    assert_eq!(scene.split_exposure_seats(), 0);
}
