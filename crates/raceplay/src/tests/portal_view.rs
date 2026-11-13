//! `portal-view` drawn: how many scenes it costs, counted in passes. A race that asks for nothing
//! draws none, a continuous view draws one a frame while the portal is in the view, a snapshot
//! one, and a split screen one however many seats ask. What it draws is
//! `mod_shader_frame_ground_truth`'s.

use oag_core::math::Vec3;
use oag_display::display::{Fov, MotionBlur, Msaa, Shadows};
use oag_ext_abi::{Command, Slot, ViewUpdate};
use oag_gameplay::PlayerInputs;
use oag_mesh::mesh::{Bounds, DrawCall, Model};

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
    pair_ahead_by(race, 20.0)
}

/// [`pair_ahead`] with the entrance `distance` units ahead of the camera and the exit 25 on.
fn pair_ahead_by(race: &Race, distance: f32) -> (f32, f32) {
    let camera = race.view().inverse();
    let eye = camera.w_axis.truncate();
    let forward = -camera.z_axis.truncate();
    (
        progress_near(race, eye + forward * distance),
        progress_near(race, eye + forward * (distance + 25.0)),
    )
}

/// The sectioned fixture's pair, far enough ahead that the entrance's sphere (radius 8) stands
/// in sections other than the craft's and the camera's.
fn pair_in_other_sections(race: &Race) -> (f32, f32) {
    pair_ahead_by(race, 40.0)
}

/// The ring exit a progress names.
fn ring_exit(race: &Race, progress: f32) -> Option<crate::transit::Exit> {
    let course = race.sim.course.as_ref()?;
    course
        .index_at_progress(progress)
        .map(crate::transit::Exit::ring)
}

fn portal_view(slot: usize, pair: (f32, f32), update: ViewUpdate, generation: u8) -> Command {
    portal_view_reaching(slot, pair, update, generation, 8)
}

/// [`portal_view`] whose surface reaches `radius` world units from the entrance.
fn portal_view_reaching(
    slot: usize,
    (entrance, exit): (f32, f32),
    update: ViewUpdate,
    generation: u8,
    radius: u8,
) -> Command {
    Command::PortalView {
        slot: Slot::new(slot).unwrap(),
        entrance,
        exit: oag_ext_abi::Distance::Progress(exit),
        radius,
        update,
        generation,
    }
}

fn shaded(commands: &[Command], text: &'static str) -> Source {
    let mut source = scripted("test.portal", "", commands, &[]);
    source.shaders = vec![("a-window".to_string(), "a.wgsl".to_string())];
    source.files = std::sync::Arc::new(move |_| Ok(text.as_bytes().to_vec()));
    source
}

/// A race with the mod's shader, sending `commands` every tick, its chase camera settled.
fn race(commands: &[Command]) -> Race {
    race_with(commands, GOOD)
}

/// [`race`] with `text` for the mod's shader.
fn race_with(commands: &[Command], text: &'static str) -> Race {
    let source = shaded(commands, text);
    settled(grid(prepared(std::slice::from_ref(&source))), &source)
}

/// `race` with `source`'s shaders loaded and its chase camera settled.
fn settled(mut race: Race, source: &Source) -> Race {
    let shaders = crate::extension_shaders::load(
        std::slice::from_ref(source),
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

/// [`race`] on a straight whose control points each author their own section (1, 2, ...), so
/// the entrance ahead of the camera is in another section than the craft and the camera.
fn sectioned_race(commands: &[Command]) -> Race {
    let source = shaded(commands, GOOD);
    let mut setup = setup(hulled_handling());
    let ai = straight_track_in_sections(12.0, 12.0, |k| u8::try_from(k + 1).unwrap_or(u8::MAX));
    setup.spline = Spline::from_track(&ai);
    setup.course = Course::from_track(&ai, None);
    setup.ai = ai;
    setup.mode = Mode::SingleRace;
    setup.extensions = prepared(std::slice::from_ref(&source));
    setup.start_position = Some(oag_vex::track::StartPosition {
        position: [0.0, 0.0, 0.0],
        left: [0.0, 0.0, -1.0],
        up: [0.0, 1.0, 0.0],
        forward: [1.0, 0.0, 0.0],
    });
    settled(Race::start(setup), &source)
}

/// Sections 1 to 8, each seeing only itself: a wall between any two of them.
fn walled_sections() -> crate::visibility::TrackVisibility {
    let pvs = oag_vex::pvs::TrackPvs::from_sections((1u8..=8).map(|index| oag_vex::pvs::Section {
        index,
        visible: 1u64 << index,
        bounds: None,
    }));
    crate::visibility::TrackVisibility::declaring(pvs)
}

/// Sections 1 to 8, each seeing all the others: no wall anywhere.
fn open_sections() -> crate::visibility::TrackVisibility {
    let pvs = oag_vex::pvs::TrackPvs::from_sections((1u8..=8).map(|index| oag_vex::pvs::Section {
        index,
        visible: u64::MAX,
        bounds: None,
    }));
    crate::visibility::TrackVisibility::declaring(pvs)
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

/// A fill through a shader whose header says `fill = "backdrop"` stands behind the view's own
/// craft: in a transit that craft is out of the world and still drawn for its own view, where
/// its camera follows it, and for no other; its camera flies on at the speed it went in at,
/// and its pair is still seen from its seat. A fill that is no backdrop draws no such craft.
#[test]
fn a_craft_in_a_transit_is_flown_and_drawn_for_its_own_view_under_a_backdrop() {
    const BEHIND: &str = "//! fill = \"backdrop\"\nfn mod_fragment(input: ModInput) -> vec4<f32> { return input.colour; }\n";
    let fill = Command::ScreenFill {
        slot: Slot::new(0).unwrap(),
        rgba: oag_ext_abi::Rgba::new(255, 255, 255, 255),
        shader: oag_ext_abi::ModName::new("a-window"),
    };
    let flown = |text: &'static str| {
        let mut race = race_with(&[fill], text);
        race.sim.world.ships[0].physics.body.linear_velocity = Vec3::new(30.0, 0.0, 0.0);
        let before = race.view().inverse().w_axis.truncate();
        race.begin_transit(0, 40, crate::transit::Distance::Units(25.0), 0.9)
            .expect("the player is racing on the course");
        for _ in 0..20 {
            race.tick(&PlayerInputs::none());
        }
        (race, before)
    };
    let (mut race, before) = flown(BEHIND);
    assert!(!race.ship_active(0), "out of the world");
    assert!(race.ship_drawn(0) && race.ship_flown_in_view(0));
    // A second seat's view of the same race.
    assert_eq!(race.set_viewport_count(2), 2);
    race.set_viewpoint(1);
    assert_eq!(race.viewed_slot(), 1);
    assert!(!race.ship_drawn(0), "no other view draws it");
    race.reset_viewpoint();
    // 30 units a second, and 19 ticks since the tick the seat was opened on: the camera and
    // the hull are 9.5 units on from the seat, and the pair is seen from where the camera
    // would stand had it stayed on it.
    let eye = race.view().inverse().w_axis.truncate();
    assert!(
        eye.x - before.x > 8.0,
        "the camera flew on: {before} to {eye}"
    );
    let hull = race.ship_model_matrix_of(0).w_axis.truncate();
    let frozen = race.sim.world.ships[0].physics.body.position;
    assert!(
        hull.x - frozen.x > 8.0,
        "the hull flew on: {frozen} to {hull}"
    );
    let pair = race.pair_view().inverse().w_axis.truncate();
    assert!((eye.x - pair.x - 9.5).abs() < 0.01, "{pair} against {eye}");
    let frame = race.craft_frame();
    assert!(
        frame.z_axis.x < -0.99,
        "the frame's back is against the flight"
    );
    assert!(frame.w_axis.truncate().distance(hull) < 0.5);

    let (race, _) = flown(GOOD);
    assert!(
        !race.ship_drawn(0),
        "a fill over the picture hides the craft with the rest"
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

/// A scene over `visibility` on the sectioned fixture.
fn sectioned_scene(
    gpu: &Gpu,
    race: &Race,
    visibility: crate::visibility::TrackVisibility,
) -> Scene {
    scene_for(gpu, race).with_visibility(visibility)
}

/// Whether the main view's visible set takes any section the portal's surface reaches in.
fn entrance_is_in_the_pvs(
    race: &Race,
    visibility: &crate::visibility::TrackVisibility,
    request: &crate::extensions::PortalRequest,
) -> bool {
    let projection = race.projection(1.0, 500.0, Fov::AUTHORED);
    let shot = race.portal_shot(request, projection).expect("in the view");
    let set = race.visible_set(visibility, &(projection * race.view()));
    set.allows(shot.reach.sections)
}

/// A wall between the camera and the portal: the entrance's section is outside the main view's
/// PVS, so the view is not drawn (no pass, no target, no texture for the shader to read).
#[test]
fn a_portal_behind_a_wall_draws_nothing() {
    let Some(gpu) = test_gpu() else { return };
    let portals = pair_in_other_sections(&sectioned_race(&[]));
    let commands = [portal_view(0, portals, ViewUpdate::Continuous, 0)];
    let mut race = sectioned_race(&commands);
    let request = race.extension_portal_view(0).expect("the guest asked");
    assert!(
        !entrance_is_in_the_pvs(&race, &walled_sections(), &request),
        "the fixture puts a wall between them"
    );
    let scene = sectioned_scene(&gpu, &race, walled_sections());
    assert_eq!(frames(&gpu, &scene, &mut race, 5), 0);
    assert!(!scene.has_portal_target(), "no target either");
    assert!(scene.portal_engine_of(&race).is_none(), "no texture bound");
}

/// The same portal with the entrance's section in the main view's PVS draws as before.
#[test]
fn a_portal_in_the_main_views_pvs_still_draws() {
    let Some(gpu) = test_gpu() else { return };
    let portals = pair_in_other_sections(&sectioned_race(&[]));
    let commands = [portal_view(0, portals, ViewUpdate::Continuous, 0)];
    let mut race = sectioned_race(&commands);
    let request = race.extension_portal_view(0).expect("the guest asked");
    assert!(entrance_is_in_the_pvs(&race, &open_sections(), &request));
    let scene = sectioned_scene(&gpu, &race, open_sections());
    assert_eq!(frames(&gpu, &scene, &mut race, 5), 5);
    assert!(
        scene.portal_engine_of(&race).is_some(),
        "the texture is bound"
    );
}

/// A snapshot is taken on the first frame its entrance is in view, not spent on a frame where a
/// wall hides it: the skipped frames leave the one snapshot for later.
#[test]
fn a_snapshot_behind_a_wall_is_not_spent_until_the_entrance_is_in_view() {
    let Some(gpu) = test_gpu() else { return };
    let portals = pair_in_other_sections(&sectioned_race(&[]));
    let commands = [portal_view(0, portals, ViewUpdate::Once, 1)];
    let mut race = sectioned_race(&commands);
    let scene = sectioned_scene(&gpu, &race, walled_sections());
    assert_eq!(frames(&gpu, &scene, &mut race, 4), 0, "behind the wall");
    assert!(!scene.has_portal_target());
    let request = race.extension_portal_view(0).expect("the guest asked");
    let at = race
        .portal_frame(
            ring_exit(&race, request.entrance).expect("on the lap"),
            oag_render::camera::portal::Face::Oncoming,
        )
        .expect("a frame")
        .w_axis
        .truncate();
    race.sim.world.ships[0].physics.body.position.x = at.x - 1.0;
    for _ in 0..90 {
        race.tick(&PlayerInputs::none());
    }
    assert!(
        entrance_is_in_the_pvs(&race, &walled_sections(), &request),
        "the craft stands in the entrance's section"
    );
    assert_eq!(frames(&gpu, &scene, &mut race, 4), 1, "taken once, held");
}

/// The entrance's section is hidden, but the surface's radius reaches the craft's own section,
/// which the main view draws: the portal is on the screen, so the view is drawn.
#[test]
fn a_portal_whose_radius_reaches_a_visible_section_still_draws() {
    let Some(gpu) = test_gpu() else { return };
    let portals = pair_in_other_sections(&sectioned_race(&[]));
    let commands = [portal_view_reaching(
        0,
        portals,
        ViewUpdate::Continuous,
        0,
        40,
    )];
    let mut race = sectioned_race(&commands);
    let request = race.extension_portal_view(0).expect("the guest asked");
    let projection = race.projection(1.0, 500.0, Fov::AUTHORED);
    let shot = race.portal_shot(&request, projection).expect("in the view");
    let set = race.visible_set(&walled_sections(), &(projection * race.view()));
    assert!(
        !set.allows(1u64 << sample_section_at(&race, request.entrance)),
        "the entrance's own section is behind a wall"
    );
    assert!(set.allows(shot.reach.sections), "its reach is not");
    let scene = sectioned_scene(&gpu, &race, walled_sections());
    assert_eq!(frames(&gpu, &scene, &mut race, 5), 5);
    assert!(scene.portal_engine_of(&race).is_some());
}

/// The section of the road sample under a progress.
fn sample_section_at(race: &Race, progress: f32) -> u8 {
    race.road_sample(ring_exit(race, progress).expect("on the lap"))
        .expect("a road sample")
        .section_id
}

/// A `.pvs` of `chunks` chunks over cells at `positions`, every cell seeing the `sees` chunks.
fn chunk_partition(positions: &[[f32; 3]], chunks: usize, sees: &[usize]) -> oag_rcs::hd_pvs::Pvs {
    let width = chunks / 8 + 1;
    let mut blob = Vec::new();
    for word in [positions.len() as u32, chunks as u32, 0x10, 0] {
        blob.extend_from_slice(&word.to_be_bytes());
    }
    for position in positions {
        for axis in position {
            blob.extend_from_slice(&axis.to_be_bytes());
        }
        blob.extend_from_slice(&position[2].to_be_bytes());
    }
    for _ in positions {
        let mut map = vec![0u8; width];
        for chunk in sees {
            map[chunk >> 3] |= 1 << (chunk & 7);
        }
        blob.extend_from_slice(&map);
    }
    oag_rcs::hd_pvs::Pvs::parse(&blob).expect("a partition")
}

/// A track of one tiny draw per `(chunk, place)`.
fn chunked_track(draws: &[(u32, Vec3)]) -> Model {
    let mut track = triangle();
    track.radius = 500.0;
    let template = track.draws[0].clone();
    track.draws = draws
        .iter()
        .map(|&(chunk, at)| DrawCall {
            chunk: Some(chunk),
            bounds: Bounds {
                centre: at.to_array(),
                radius: 2.0,
            },
            ..template.clone()
        })
        .collect();
    track
}

/// A scene over `track`, the chunk partition a cell at the craft sees `sees` of.
fn chunked_scene(gpu: &Gpu, race: &Race, track: Model, sees: &[usize]) -> Scene {
    let liveries: Vec<_> = (0..race.ship_count()).map(|_| livery()).collect();
    let scene = scene_over_post(
        &gpu.0,
        &gpu.1,
        track,
        &liveries,
        Msaa::Off,
        None,
        None,
        None,
    );
    scene.attach_mod_shaders(&gpu.0, &gpu.1, race.mod_shaders());
    let cell = race.sim.world.ships[0].physics.body.position.to_array();
    scene.with_visibility(crate::visibility::TrackVisibility::declaring_chunks(
        chunk_partition(&[cell], 16, sees),
    ))
}

/// Where the fixture's entrance stands.
fn entrance_centre(race: &Race, request: &crate::extensions::PortalRequest) -> Vec3 {
    race.portal_frame(
        ring_exit(race, request.entrance).expect("on the lap"),
        oag_render::camera::portal::Face::Oncoming,
    )
    .expect("a frame")
    .w_axis
    .truncate()
}

/// What `frames` counts of a race over a chunk partition, with the portal's draws as given
/// relative to its entrance: `(chunk, offset from the entrance)`.
fn chunked_frames(near: &[(u32, Vec3)], far: &[(u32, Vec3)], sees: &[usize]) -> (u32, bool) {
    let Some(gpu) = test_gpu() else {
        return (u32::MAX, false);
    };
    let portals = pair_ahead(&race(&[]));
    let mut race = race(&[portal_view(0, portals, ViewUpdate::Continuous, 0)]);
    let request = race.extension_portal_view(0).expect("the guest asked");
    let entrance = entrance_centre(&race, &request);
    let draws: Vec<_> = near
        .iter()
        .map(|&(chunk, offset)| (chunk, entrance + offset))
        .chain(far.iter().copied())
        .collect();
    let scene = chunked_scene(&gpu, &race, chunked_track(&draws), sees);
    let drawn = frames(&gpu, &scene, &mut race, 5);
    (drawn, scene.portal_engine_of(&race).is_some())
}

/// A chunk partition where everything the surface reaches is hidden: no pass, no texture. A
/// visible chunk far from the surface does not bring the view back.
#[test]
fn a_portal_among_hidden_chunks_draws_nothing() {
    let far = (0, Vec3::new(-900.0, 0.0, 0.0));
    let (drawn, bound) = chunked_frames(&[(1, Vec3::ZERO), (1, Vec3::X * 3.0)], &[far], &[0]);
    assert_eq!(drawn, 0);
    assert!(!bound, "no texture bound");
}

/// The entrance's own chunk is hidden but a neighbour the radius reaches is visible.
#[test]
fn a_portal_whose_radius_reaches_a_visible_chunk_still_draws() {
    let (drawn, bound) = chunked_frames(&[(1, Vec3::ZERO), (0, Vec3::X * 6.0)], &[], &[0]);
    assert_eq!(drawn, 5);
    assert!(bound);
}

/// The control for the two above: the entrance's chunk is visible.
#[test]
fn a_portal_in_a_visible_chunk_draws() {
    let (drawn, bound) = chunked_frames(&[(1, Vec3::ZERO)], &[], &[1]);
    assert_eq!(drawn, 5);
    assert!(bound);
}

/// A draw with no chunk index is never excluded, so the surface reaching one keeps drawing.
#[test]
fn a_portal_reaching_an_unplaced_draw_still_draws() {
    let Some(gpu) = test_gpu() else { return };
    let portals = pair_ahead(&race(&[]));
    let mut race = race(&[portal_view(0, portals, ViewUpdate::Continuous, 0)]);
    let request = race.extension_portal_view(0).expect("the guest asked");
    let at = entrance_centre(&race, &request);
    let mut track = chunked_track(&[(1, at), (1, at)]);
    track.draws[1].chunk = None;
    let scene = chunked_scene(&gpu, &race, track, &[0]);
    assert_eq!(frames(&gpu, &scene, &mut race, 5), 5);
}

/// A track with no draw in reach has nothing to hide: the view is drawn.
#[test]
fn a_portal_reaching_no_draw_still_draws() {
    let (drawn, _) = chunked_frames(&[], &[(1, Vec3::new(-900.0, 0.0, 0.0))], &[0]);
    assert_eq!(drawn, 5);
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
    let source = shaded(&[command], GOOD);
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
