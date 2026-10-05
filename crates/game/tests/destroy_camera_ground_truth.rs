//! The camera a wrecked player is cut to, on a real circuit.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --test destroy_camera_ground_truth --run-ignored all
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
    let mut race = race::Race::start(loaded.setup);
    for _ in 0..30 {
        race.tick(&PlayerInputs::none());
    }
    let chase_eye = race.camera_position();

    race.force_destroy(0);
    for _ in 0..200 {
        race.tick(&PlayerInputs::none());
    }
    let craft = race.sim.world.ships[0].physics.body.position;
    let eye = race.camera_position();
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
        .vertical_fov(480.0 / 272.0, oag_display::display::Fov::AUTHORED)
        .to_degrees();
    let want = destroy::framing_fov_degrees(destroy::FRAME_SIZE, eye.distance(craft));
    assert!((fov - want).abs() < 0.02, "field {fov}, framing {want}");
}
