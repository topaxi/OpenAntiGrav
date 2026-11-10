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

/// The ring exit a progress names.
fn ring_exit(race: &Race, progress: f32) -> Option<crate::transit::Exit> {
    let course = race.sim.course.as_ref()?;
    course
        .index_at_progress(progress)
        .map(crate::transit::Exit::ring)
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
        exit: oag_ext_abi::Distance::Progress(exit),
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
        exit: ring_exit(&race, exit),
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

/// A craft's own flight through a pair: frozen off the road's middle, its camera eases onto
/// its seat (`crate::transit_camera`), so the view through the pair at its own progress and its
/// exit is, on the last tick out of the world, the camera it has on the tick it is out. With
/// the camera left on the frozen craft the two stand the craft's offset apart; with the spring
/// left to cross the gap at the exit they stand most of the jump apart.
#[test]
fn a_view_through_a_transits_own_pair_ends_on_the_camera_the_craft_comes_out_with() {
    let mut race = race(&[]);
    let ship = &mut race.sim.world.ships[0];
    ship.physics.body.position.z += 2.0;
    ship.physics.body.linear_velocity = Vec3::new(30.0, 0.0, 4.0);
    race.begin_transit(0, 40, crate::transit::Distance::Units(25.0), 0.9)
        .expect("the player is racing on the course");
    let transit = race.sim.world.ships[0].transit.expect("in a transit");
    for _ in 1..40 {
        race.tick(&PlayerInputs::none());
    }
    let ship = &race.sim.world.ships[0];
    assert!(ship.transit.is_some(), "the last tick out of the world");
    let request = crate::extensions::PortalRequest {
        entrance: ship.standing.progress.expect("a fix"),
        exit: Some(crate::transit::Exit {
            to: transit.to as usize,
            route: transit.route,
        }),
        radius: 8.0,
        update: ViewUpdate::Continuous,
        generation: 0,
    };
    let projection = race.projection(1.0, 2000.0, Fov::AUTHORED);
    let shot = race.portal_shot(&request, projection).expect("in the view");
    let through = shot.view.inverse();
    race.tick(&PlayerInputs::none());
    assert_eq!(race.sim.world.ships[0].transit, None, "out");
    let out = race.view().inverse();
    let apart = through.w_axis.truncate().distance(out.w_axis.truncate());
    assert!(apart < 0.25, "the eyes stand {apart} apart");
    let turned = through.z_axis.truncate().distance(out.z_axis.truncate());
    assert!(turned < 0.02, "the two look {turned} apart");
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
        exit: ring_exit(&race, portals.1),
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

/// Pulse PSP EU's `05_Track`: a fork whose first route runs on a deck above the ring.
const FORKED: &str = "Data\\Environments\\05_Track\\track.vex";

/// The setup of a `05_Track` race, the disc's. `None` without the image.
fn forked_setup() -> Option<Setup> {
    let image = oag_testdata::image("data/images/pulse-psp-eu.chd")?;
    let loaded = crate::load(&crate::Options {
        source: image.display().to_string(),
        track: Some(FORKED.to_string()),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        no_driver_for_class: true,
        ..crate::Options::default()
    })
    .expect("loading the race");
    Some(loaded.setup)
}

/// A `05_Track` race with the mod's shader and a trigger volume standing on route 0 of the fork,
/// whose guest asks for a view from a ring point ahead of the player's camera to `exit` (a
/// progress, or trigger 0), its chase camera settled. `None` without the disc image.
fn forked_race(exit: oag_ext_abi::Distance) -> Option<(Race, crate::extensions::PortalRequest)> {
    let mut bare = Race::start(forked_setup()?);
    for _ in 0..90 {
        bare.tick(&PlayerInputs::none());
    }
    let entrance = pair_ahead(&bare).0;
    let mut setup = forked_setup()?;
    let course = setup.course.take().expect("a course");
    let route = &course.routes()[0];
    let span = (course.progress_at(route.merge).unwrap()
        - course.progress_at(route.split).unwrap())
    .rem_euclid(course.length());
    let ring_index = course.advance(route.split, span * 0.4);
    let on_route = course.route_point(0, ring_index).expect("a route point");
    let mut to_world = oag_ext_abi::track::IDENTITY;
    to_world[12..15].copy_from_slice(&on_route.to_array());
    let props = crate::extensions::TrackProps {
        triggers: vec![oag_trackfile::props::Trigger {
            name: "exit".into(),
            to_world,
            min: [-1.0; 3],
            max: [1.0; 3],
            props: Default::default(),
            emits: None,
        }],
        ..Default::default()
    };
    let command = Command::PortalView {
        slot: Slot::new(0).unwrap(),
        entrance,
        exit,
        radius: 8,
        update: ViewUpdate::Continuous,
        generation: 0,
    };
    let source = shaded(&[command]);
    let mut report = Vec::new();
    setup.extensions = crate::extensions::prepare(
        std::slice::from_ref(&source),
        "wipeout-pulse",
        None,
        crate::extensions::Track {
            name: "05_Track",
            props: &props,
            speedup_pads: &[],
            weapon_pads: &[],
            paths: &[],
            routes: course.routes(),
        },
        None,
        &mut report,
    );
    setup.course = Some(course);
    let mut race = Race::start(setup);
    for _ in 0..90 {
        race.tick(&PlayerInputs::none());
    }
    let request = race.extension_portal_view(0).expect("the guest asked");
    Some((race, request))
}

/// A portal whose exit is a trigger standing on a fork's route frames that route, not the ring.
/// Acceptance: the exit frame is on the route's deck (the ring is tens of units under it), the
/// view's eye is on the route's side of the ring point, and a ring exit's camera is the one the
/// ring formula gives, to the bit.
#[test]
#[ignore = "needs data/images/pulse-psp-eu.chd"]
fn a_portal_view_to_a_forks_route_is_framed_from_the_route() {
    use crate::transit::Exit;
    use oag_ext_abi::Distance;

    let Some((race, request)) = forked_race(Distance::Trigger(0)) else {
        return;
    };
    let course = race.sim.course.as_ref().unwrap();
    let exit = request.exit.expect("the trigger is on the course");
    assert_eq!(exit.route, 1, "the trigger resolves onto the fork's route");
    let route_point = course.route_point(0, exit.to).unwrap();
    let ring_point = course.position(exit.to).unwrap();
    assert!(
        route_point.distance(ring_point) > 30.0,
        "the route is above the ring here: {}",
        route_point.distance(ring_point)
    );

    let frame = race.portal_frame(exit, oag_render::camera::portal::Face::Departing);
    let centre = frame.expect("a frame").w_axis.truncate();
    assert!(
        centre.distance(route_point) < 5.0,
        "the exit stands on the route: {} from it",
        centre.distance(route_point)
    );

    // It faces the way the route runs, not the way the ring does there.
    let frame = frame.unwrap();
    let ahead = course
        .route_point(0, course.advance(exit.to, 30.0))
        .expect("the route runs on");
    let along = (ahead - route_point).normalize();
    assert!(
        frame.z_axis.truncate().dot(along) > 0.9,
        "the exit looks down the route: {}",
        frame.z_axis.truncate().dot(along)
    );

    let projection = race.projection(1.0, 4000.0, Fov::AUTHORED);
    let shot = race.portal_shot(&request, projection).expect("in the view");
    let eye = shot.view.inverse().w_axis.truncate();
    assert!(
        eye.distance(route_point) * 1.2 < eye.distance(ring_point),
        "the eye is {} from the route's point and {} from the ring's",
        eye.distance(route_point),
        eye.distance(ring_point)
    );

    // A ring exit at the same ring point: the camera of the ring formula, bit for bit.
    let ring_progress = course.progress_at(exit.to).unwrap();
    let (race, request) = forked_race(Distance::Progress(ring_progress)).unwrap();
    let course = race.sim.course.as_ref().unwrap();
    let ring = request.exit.unwrap();
    assert_eq!(ring, Exit::ring(exit.to));
    let shot = race.portal_shot(&request, projection).expect("in the view");
    let frame_of = |index: usize, face| {
        let (_, up, centre, _) = race.road_frame(Exit::ring(index)).unwrap();
        oag_render::camera::portal::frame(centre, course.tangent(index).unwrap(), up, face).unwrap()
    };
    let entrance = course.index_at_progress(request.entrance).unwrap();
    let through = oag_render::camera::portal::through(
        frame_of(entrance, oag_render::camera::portal::Face::Oncoming),
        frame_of(exit.to, oag_render::camera::portal::Face::Departing),
    );
    let view = oag_render::camera::portal::view(race.view(), through);
    assert_eq!(
        shot.view.to_cols_array().map(f32::to_bits),
        view.to_cols_array().map(f32::to_bits)
    );
}
