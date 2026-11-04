//! `portal-view` drawn: how many scenes it costs, counted in passes. A race that asks for nothing
//! draws none, a continuous view draws one a frame while the portal is in the view, a snapshot
//! one, and a split screen one however many seats ask. What it draws is
//! `mod_shader_frame_ground_truth`'s.

use oag_core::math::Vec3;
use oag_display::display::{Fov, MotionBlur, Msaa, Shadows};
use oag_ext_abi::{Command, Slot, ViewUpdate};
use oag_gameplay::PlayerInputs;

use super::extension_faults::scripted;
use super::extensions::grid;
use super::extensions::on::prepared;
use super::scene_build::{livery, scene_over_post, test_gpu, triangle};
use super::split_blur::{AREA, target};
use super::*;
use crate::extensions::Source;
use crate::{Scene, WindowEffects};

type Gpu = (wgpu::Device, wgpu::Queue);

const GOOD: &str =
    "fn mod_fragment(input: ModInput) -> vec4<f32> { return engine_texture(input.screen_uv); }\n";

/// Lap progress of the course point nearest a place.
fn progress_near(race: &Race, at: Vec3) -> f32 {
    let course = race.sim.course.as_ref().unwrap();
    let index = course.locate(at, None).unwrap().index;
    course.progress_at(index).unwrap()
}

/// The fixture's camera-side and far-side points: the entrance a little ahead of the player's
/// camera, so its face looks at it, and the exit further on.
fn pair_ahead(race: &Race) -> (f32, f32) {
    let camera = race.view().inverse();
    let eye = camera.w_axis.truncate();
    let forward = -camera.z_axis.truncate();
    (
        progress_near(race, eye + forward * 20.0),
        progress_near(race, eye + forward * 45.0),
    )
}

fn portal_view(
    slot: usize,
    (entrance, exit): (f32, f32),
    update: ViewUpdate,
    generation: u8,
) -> Command {
    Command::PortalView {
        slot: Slot::new(slot).unwrap(),
        entrance,
        exit,
        radius: 8,
        update,
        generation,
    }
}

fn shaded(commands: &[Command]) -> Source {
    let mut source = scripted("test.portal", "", commands, &[]);
    source.shaders = vec![("a-window".to_string(), "a.wgsl".to_string())];
    source.files = std::sync::Arc::new(|_| Ok(GOOD.as_bytes().to_vec()));
    source
}

/// A race with the mod's shader, sending `commands` every tick, its chase camera settled.
fn race(commands: &[Command]) -> Race {
    let source = shaded(commands);
    let mut race = grid(prepared(std::slice::from_ref(&source)));
    let shaders = crate::extension_shaders::load(
        std::slice::from_ref(&source),
        "wipeout-pulse",
        None,
        &mut Vec::new(),
    );
    race.view.handles = std::mem::take(&mut race.view.handles).with_shaders(shaders);
    for _ in 0..90 {
        race.tick(&PlayerInputs::none());
    }
    race
}

/// The far plane comes off the track's radius, and the fixture's portals are tens of units on.
fn bare_scene(gpu: &Gpu, race: &Race) -> Scene {
    let mut track = triangle();
    track.radius = 500.0;
    let liveries: Vec<_> = (0..race.ship_count()).map(|_| livery()).collect();
    scene_over_post(
        &gpu.0,
        &gpu.1,
        track,
        &liveries,
        Msaa::Off,
        None,
        None,
        None,
    )
}

fn scene_for(gpu: &Gpu, race: &Race) -> Scene {
    let scene = bare_scene(gpu, race);
    scene.attach_mod_shaders(&gpu.0, &gpu.1, race.mod_shaders());
    scene
}

fn draw(gpu: &Gpu, scene: &Scene, race: &Race, view: &wgpu::TextureView) {
    let mut encoder = gpu.0.create_command_encoder(&Default::default());
    scene.render(
        &gpu.0,
        &gpu.1,
        &mut encoder,
        view,
        race,
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
    gpu.1.submit(Some(encoder.finish()));
}

fn frames(gpu: &Gpu, scene: &Scene, race: &mut Race, count: u32) -> u32 {
    let texture = target(&gpu.0);
    let view = texture.create_view(&Default::default());
    let before = scene.frame_work().portal_views;
    for _ in 0..count {
        race.tick(&PlayerInputs::none());
        draw(gpu, scene, race, &view);
    }
    scene.frame_work().portal_views - before
}

/// Where a view of this fixture puts its portal, off a race that sends nothing.
fn pair() -> (f32, f32) {
    pair_ahead(&race(&[]))
}

/// The camera through the pair, against the road's own numbers: the fixture's straight runs along
/// +x, so the view stands as far behind the exit as the player's stands behind the entrance,
/// looks the same way and keeps its offset. A view facing back from past the exit fails here,
/// and only here: the frame ground truth takes its reference camera from the view's own log.
#[test]
fn the_view_stands_behind_the_exit_and_looks_down_the_road() {
    let (entrance, exit) = pair();
    let race = race(&[]);
    let request = crate::extensions::PortalRequest {
        entrance,
        exit,
        radius: 8.0,
        update: ViewUpdate::Continuous,
        generation: 0,
    };
    let projection = race.projection(1.0, 2000.0, Fov::AUTHORED);
    let shot = race.portal_shot(&request, projection).expect("in the view");
    let course = race.sim.course.as_ref().unwrap();
    let at = |progress| {
        let index = course.index_at_progress(progress).unwrap();
        course.position(index).unwrap()
    };
    let main = race.view().inverse();
    let camera = shot.view.inverse();
    let moved = at(exit) - at(entrance);
    assert!(
        moved.x > 10.0,
        "the fixture's pair is along the straight: {moved}"
    );
    let eye = camera.w_axis.truncate();
    let expected = main.w_axis.truncate() + moved;
    assert!(eye.distance(expected) < 0.01, "{eye} against {expected}");
    let forward = -camera.z_axis.truncate();
    assert!(
        forward.distance(-main.z_axis.truncate()) < 1e-3,
        "{forward}"
    );
    assert!(camera.y_axis.truncate().distance(main.y_axis.truncate()) < 1e-3);
    assert!(
        shot.exit_plane.w < 0.0,
        "the eye is behind the exit's plane"
    );
    let main_clip = projection * race.view();
    let engine = shot.engine_view_projection.to_cols_array();
    assert!(
        engine
            .iter()
            .zip(main_clip.to_cols_array())
            .all(|(a, b)| (a - b).abs() < 1e-3),
        "a point of the surface reads the texel it covers on the screen"
    );
}

/// Acceptance: a race with a loaded shader that asks for no view renders no extra scene and
/// builds no target.
#[test]
fn a_guest_asking_for_no_view_costs_no_extra_scene() {
    let Some(gpu) = test_gpu() else { return };
    let mut race = race(&[]);
    let scene = scene_for(&gpu, &race);
    assert_eq!(frames(&gpu, &scene, &mut race, 5), 0);
    assert!(!scene.has_portal_target(), "no target either");
}

#[test]
fn a_continuous_view_draws_one_scene_a_frame() {
    let Some(gpu) = test_gpu() else { return };
    let mut race = race(&[portal_view(0, pair(), ViewUpdate::Continuous, 0)]);
    let scene = scene_for(&gpu, &race);
    assert_eq!(frames(&gpu, &scene, &mut race, 5), 5);
    assert!(scene.has_portal_target());
}

/// Medium and High are the same scene at a different rate: one snapshot, held.
#[test]
fn a_snapshot_draws_one_scene_and_holds_it() {
    let Some(gpu) = test_gpu() else { return };
    let mut race = race(&[portal_view(0, pair(), ViewUpdate::Once, 3)]);
    let scene = scene_for(&gpu, &race);
    assert_eq!(frames(&gpu, &scene, &mut race, 6), 1);
    assert!(scene.portal_engine_of(&race).is_some(), "still shown");
}

/// A new generation draws it again.
#[test]
fn a_new_generation_draws_the_snapshot_again() {
    let Some(gpu) = test_gpu() else { return };
    let portals = pair();
    let mut race = race(&[portal_view(0, portals, ViewUpdate::Once, 3)]);
    let scene = scene_for(&gpu, &race);
    assert_eq!(frames(&gpu, &scene, &mut race, 3), 1);
    let again = crate::extensions::PortalRequest {
        entrance: portals.0,
        exit: portals.1,
        radius: 8.0,
        update: ViewUpdate::Once,
        generation: 4,
    };
    race.sim.extensions.set_portal_view(0, again);
    let texture = target(&gpu.0);
    draw(
        &gpu,
        &scene,
        &race,
        &texture.create_view(&Default::default()),
    );
    assert_eq!(scene.frame_work().portal_views, 2);
}

/// The player past the entrance, its face away from the camera: nothing of it can be seen.
#[test]
fn a_portal_behind_the_camera_draws_nothing() {
    let Some(gpu) = test_gpu() else { return };
    let mut race = race(&[portal_view(0, pair(), ViewUpdate::Continuous, 0)]);
    race.sim.world.ships[0].physics.body.position.x = 60.0;
    for _ in 0..90 {
        race.tick(&PlayerInputs::none());
    }
    let eye = race.camera_position();
    assert!(eye.x > 30.0, "the camera followed: {eye}");
    let scene = scene_for(&gpu, &race);
    assert_eq!(frames(&gpu, &scene, &mut race, 3), 0);
}

/// Off the lap: a request for nothing, not a fault.
#[test]
fn a_progress_off_the_lap_draws_nothing() {
    let Some(gpu) = test_gpu() else { return };
    let mut race = race(&[portal_view(0, (1.0e6, 2.0e6), ViewUpdate::Continuous, 0)]);
    let scene = scene_for(&gpu, &race);
    assert_eq!(frames(&gpu, &scene, &mut race, 3), 0);
}

/// A shaderless race has nothing to read the view: the request costs nothing.
#[test]
fn a_race_with_no_shader_draws_nothing() {
    let Some(gpu) = test_gpu() else { return };
    let portals = pair();
    let source = scripted(
        "test.portal",
        "",
        &[portal_view(0, portals, ViewUpdate::Continuous, 0)],
        &[],
    );
    let mut race = grid(prepared(std::slice::from_ref(&source)));
    let scene = bare_scene(&gpu, &race);
    assert_eq!(frames(&gpu, &scene, &mut race, 3), 0);
}

/// One extra scene a frame, however many seats ask: the first seat with the portal in its
/// view has the texture, and the other's surfaces read `engine_texture_bound()` as false.
#[test]
fn a_split_screen_draws_one_extra_scene_a_frame_for_all_its_seats() {
    let Some(gpu) = test_gpu() else { return };
    let portals = pair();
    let mut race = race(&[
        portal_view(0, portals, ViewUpdate::Continuous, 0),
        portal_view(1, portals, ViewUpdate::Continuous, 0),
    ]);
    assert_eq!(race.set_viewport_count(2), 2);
    let scene = scene_for(&gpu, &race);
    let texture = target(&gpu.0);
    let view = texture.create_view(&Default::default());
    for frame in 1..=4 {
        race.tick(&PlayerInputs::none());
        scene.render_viewports(
            &gpu.0,
            &gpu.1,
            &view,
            &race,
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
        assert_eq!(scene.frame_work().portal_views, frame);
    }
    race.set_viewpoint(1);
    assert!(
        scene.portal_engine_of(&race).is_none(),
        "the second seat falls back"
    );
    race.reset_viewpoint();
    assert!(scene.portal_engine_of(&race).is_some());
}
