//! The camera a wrecked player is cut to, on a real circuit.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-raceplay destroy_camera_ground_truth:: --run-ignored all
//! ```
//!
//! # What only real data can say here
//!
//! The original puts the player at one of the circuit's own authored `Camera`
//! nodes when their craft is destroyed (`docs/ghidra/functions/psp-pulse-usa/camera.md`,
//! "The destroy camera"). Talon's Junction authors ten. This proves the loader
//! reads all ten off `track.vex` with the aim points the original keeps at
//! `+0xa0` (the payload's `+0x10`), that a destroyed player's picture comes
//! from the one whose aim is nearest the craft, and that the field of view
//! has eased to the one that frames the craft at 35 units - the numbers the
//! running original was measured at (eye `468.3436, -22.8052, -39.8696`, 488.6
//! units from a craft on the start line, 4.1018 degrees).

use oag_gameplay::PlayerInputs;
use oag_raceplay as race;
use oag_render::camera::destroy;

const TRACK: &str = r"Data\Environments\16_Track\track.vex";

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn a_wrecked_player_is_seen_from_the_circuits_own_camera() {
    let Some(image) = oag_testdata::image("data/images/pulse-psp-usa.chd") else {
        return;
    };
    let mut archives =
        oag_assets::Archives::open(&image.to_string_lossy(), oag_pulse::TITLE).expect("archives");
    let blob = archives.read_name(TRACK).expect("the circuit");
    let cameras = oag_vex::camera::cameras(&blob);
    assert_eq!(cameras.len(), 10, "Talon's Junction authors ten cameras");
    let measured = cameras
        .iter()
        .find(|camera| (camera.position()[0] - 468.3436).abs() < 1e-3)
        .expect("the camera the original was seen picking on the start line");
    assert!((measured.position()[1] + 22.8052).abs() < 1e-3);
    assert!((measured.position()[2] + 39.8696).abs() < 1e-3);
    assert!(
        (measured.aim[0] - 344.1187).abs() < 1e-3,
        "{:?}",
        measured.aim
    );
    assert!((measured.aim[1] + 43.1941).abs() < 1e-3);
    assert!((measured.aim[2] + 137.0839).abs() < 1e-3);

    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::TimeTrial,
        track: Some(TRACK.to_string()),
        ..race::Options::default()
    })
    .expect("loading the race");
    assert!(
        loaded
            .report
            .iter()
            .any(|line| line.contains("destroy camera: 10 authored Camera node(s)")),
        "the loader says how many cameras it found"
    );
    let mut race = race::Race::start(loaded.setup, loaded.view, loaded.modules);
    for _ in 0..30 {
        race.tick(&PlayerInputs::none());
    }
    let chase_eye = race.view_state().camera_position(&race.core);

    race.core.sim.force_destroy(0);
    for _ in 0..200 {
        race.tick(&PlayerInputs::none());
    }
    let craft = race.core.sim.world.ships[0].physics.body.position;
    let eye = race.view_state().camera_position(&race.core);
    assert!(
        (eye - chase_eye).length() > 10.0,
        "the chase camera was not left"
    );
    let stations: Vec<destroy::Station> = cameras
        .iter()
        .map(|camera| destroy::Station {
            eye: oag_core::math::Vec3::from_array(camera.position()),
            aim: oag_core::math::Vec3::from_array(camera.aim),
        })
        .collect();
    let picked = destroy::nearest_station(&stations, craft).expect("ten stations");
    assert!(
        (eye - stations[picked].eye).length() < 1e-2,
        "{eye:?} is not station {picked}'s eye {:?}",
        stations[picked].eye
    );

    let fov = race
        .view_state()
        .vertical_fov(
            &race.core,
            480.0 / 272.0,
            oag_display::display::Fov::AUTHORED,
        )
        .to_degrees();
    let want = destroy::framing_fov_degrees(destroy::FRAME_SIZE, eye.distance(craft));
    assert!((fov - want).abs() < 0.02, "field {fov}, framing {want}");
}

const HD_TRACK: &str = "/data/environments/talons_junction/track.vex";

/// Wipeout HD's destroyed seat (mode 8, `docs/ghidra/functions/ps3-hdfury-eu/destroy-camera.md`,
/// rows D1 and D2): the picture comes from the circuit's own `Camera` node whose aim is nearest
/// the wreck, not from the chase camera and not from a Pulse-gated nothing - HD's
/// `circuit_effects` rule is not read, so only the title's own destroy camera reads the nodes.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn an_hd_destroyed_seat_picks_a_camera_node_near_the_wreck() {
    let Some(image) = oag_testdata::image("hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let mut archives =
        oag_assets::Archives::open(&image.to_string_lossy(), oag_hd::TITLE).expect("archives");
    let blob = archives.read_name(HD_TRACK).expect("the circuit");
    let cameras = oag_vex::camera::cameras(&blob);
    assert_eq!(cameras.len(), 10, "Talon's Junction authors ten cameras");
    assert!(
        cameras.iter().all(|camera| camera.aim != [0.0; 3]),
        "every HD node has an aim (the census in destroy-camera.md, D2)"
    );
    let stations: Vec<destroy::Station> = cameras
        .iter()
        .map(|camera| destroy::Station {
            eye: oag_core::math::Vec3::from_array(camera.position()),
            aim: oag_core::math::Vec3::from_array(camera.aim),
        })
        .collect();

    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::TimeTrial,
        track: Some(HD_TRACK.to_string()),
        ..race::Options::default()
    })
    .expect("loading an HD circuit");
    assert!(
        loaded
            .report
            .iter()
            .any(|line| line.contains("destroy camera: 10 authored Camera node(s)")),
        "HD reads its stations without the circuit-effects rule"
    );
    let mut race = race::Race::start(loaded.setup, loaded.view, loaded.modules);
    for _ in 0..30 {
        race.tick(&PlayerInputs::none());
    }
    let chase_eye = race.view_state().camera_position(&race.core);

    race.core.sim.force_destroy(0);
    for _ in 0..300 {
        race.tick(&PlayerInputs::none());
    }
    let craft = race.core.sim.world.ships[0].physics.body.position;
    let eye = race.view_state().camera_position(&race.core);
    assert!(
        (eye - chase_eye).length() > 10.0,
        "the chase camera was not left"
    );
    let picked = destroy::nearest_station(&stations, craft).expect("ten stations");
    assert!(
        (eye - stations[picked].eye).length() < 1e-2,
        "{eye:?} is not the nearest-aim node {picked}'s eye {:?}",
        stations[picked].eye
    );
    let fov = race
        .view_state()
        .vertical_fov(
            &race.core,
            480.0 / 272.0,
            oag_display::display::Fov::AUTHORED,
        )
        .to_degrees();
    let want = destroy::framing_fov_degrees(destroy::FRAME_SIZE, eye.distance(craft));
    assert!((fov - want).abs() < 0.3, "field {fov}, framing {want}");
}

/// Omega reads its HD circuits' `Camera` nodes the way HD does (rows W1 to W8): `Data\environments\`
/// ships HD's circuits with their ten to thirteen nodes, and a wreck is seen from the nearest, then
/// handed to the hold view when the 8 s clock runs out.
#[test]
#[ignore = "needs the extracted Omega in data/extracted/ps4/"]
fn an_omega_destroyed_seat_is_cut_to_a_camera_node_then_held() {
    let Some(source) = oag_testdata::exact("data/extracted/ps4") else {
        return;
    };
    let track = r"Data\environments\talons_junction\track.vex";
    let mut archives = oag_omega::open(&source.display().to_string()).expect("opening Omega");
    let blob = archives.read_name(track).expect("the circuit");
    let stations: Vec<destroy::Station> = oag_vex::camera::cameras(&blob)
        .iter()
        .map(|camera| destroy::Station {
            eye: oag_core::math::Vec3::from_array(camera.position()),
            aim: oag_core::math::Vec3::from_array(camera.aim),
        })
        .collect();
    assert_eq!(stations.len(), 10, "the same ten as HD's Talon's Junction");
    let loaded = race::load(&race::Options {
        source: source.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::TimeTrial,
        track: Some(track.to_string()),
        ..race::Options::default()
    })
    .expect("loading the circuit");
    let mut race = race::Race::start(loaded.setup, loaded.view, loaded.modules);
    for _ in 0..30 {
        race.tick(&PlayerInputs::none());
    }
    race.core.sim.force_destroy(0);
    for _ in 0..60 {
        race.tick(&PlayerInputs::none());
    }
    let craft = race.core.sim.world.ships[0].physics.body.position;
    let eye = race.view_state().camera_position(&race.core);
    let picked = destroy::nearest_station(&stations, craft).expect("ten stations");
    assert!((eye - stations[picked].eye).length() < 1e-2, "{eye:?}");
    // The camera's clock was at 1.5 s at the cut, so the hand-back is inside the next 8 s.
    for _ in 0..480 {
        race.tick(&PlayerInputs::none());
    }
    let held = race.view_state().camera_position(&race.core);
    assert!(
        stations
            .iter()
            .all(|station| (held - station.eye).length() > 1.0),
        "the station was left for the hold view"
    );
    let fov = race
        .view_state()
        .vertical_fov(&race.core, 16.0 / 9.0, oag_display::display::Fov::AUTHORED)
        .to_degrees();
    assert!(
        (fov - destroy::HOLD_FOV).abs() < 0.01,
        "the hold view's field, got {fov}"
    );
    for _ in 0..60 {
        race.tick(&PlayerInputs::none());
    }
    assert!((race.view_state().camera_position(&race.core) - held).length() < 1e-3);
}

/// 2048's own circuits author no `Camera` node (a census of `altima`, `tower` and `sol`), so its
/// destroyed seat is held where the chase camera stood at the cut.
#[test]
#[ignore = "needs the decrypted Vita package in data/extracted/vita/"]
fn a_2048_destroyed_seat_holds_on_a_circuit_with_no_camera_node() {
    let Some(source) = oag_testdata::exact("data/extracted/vita/PCSF00007") else {
        return;
    };
    let mut archives = oag_2048::open(&source.display().to_string()).expect("opening 2048");
    for circuit in ["altima", "tower", "sol"] {
        let name = format!(r"Data\art\published\environments\{circuit}\track.vex");
        let blob = archives.read_name(&name).expect("the circuit");
        assert!(
            oag_vex::camera::cameras(&blob).is_empty(),
            "{circuit} authors none"
        );
    }
    let loaded = race::load(&race::Options {
        source: source.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::TimeTrial,
        track: Some(oag_2048::race::DEFAULT_TRACK.to_string()),
        ..race::Options::default()
    })
    .expect("loading the circuit");
    let mut race = race::Race::start(loaded.setup, loaded.view, loaded.modules);
    for _ in 0..30 {
        race.tick(&PlayerInputs::none());
    }
    race.core.sim.force_destroy(0);
    for _ in 0..30 {
        race.tick(&PlayerInputs::none());
    }
    let fov = race
        .view_state()
        .vertical_fov(&race.core, 16.0 / 9.0, oag_display::display::Fov::AUTHORED)
        .to_degrees();
    assert!(
        (fov - destroy::HOLD_FOV).abs() < 0.01,
        "the hold view's field, got {fov}"
    );
}
