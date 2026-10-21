//! A captured field and HUD state are what a race starts with, out of a real disc.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this project does not
//! ship. Run it with `just test-data`.
//!
//! `Options::field` puts every opponent of a Single Race grid on a captured pose and keeps the
//! grid's opponents alive beside a posed player (a pose alone drops them), and
//! `Options::hud` imposes the original's place, lap, clocks, speed, shield and best lap on the
//! primary readout. Both are what `psp-pulse-pair.py --field` feeds `oag-game` from the
//! original's RAM (`docs/tools/compare-page.md`, "Race shots").

use oag_core::math::{Quat, Vec3};
use oag_gameplay::spawn::Pose;
use oag_raceplay as race;

fn pose_at(x: f32, z: f32) -> Pose {
    Pose {
        position: Vec3::new(x, -50.0, z),
        orientation: Quat::IDENTITY,
    }
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_captured_field_and_hud_state_are_what_the_race_starts_with() {
    let Some(image) = oag_testdata::image("data/images/pulse-psp-eu.chd") else {
        return;
    };
    let player = pose_at(-100.0, -170.0);
    let field: Vec<Pose> = (1..=7u8)
        .map(|n| pose_at(f32::from(n) * 11.0, f32::from(n) * 5.0))
        .collect();
    let hud = race::CaptureHud {
        place: 5,
        lap: 2,
        lap_ticks: 1_740,
        race_ticks: 2_333,
        speed_kmh: Some(463.0),
        shield_percent: Some(99.0),
        best_lap_ticks: Some(3_380),
        zone: None,
    };
    let start = |field: Vec<Pose>, hud: Option<race::CaptureHud>| {
        let loaded = race::load(&race::Options {
            source: image.display().to_string(),
            class: "VENOM".to_string(),
            mode: oag_race::Mode::SingleRace,
            pose: Some(race::PoseRequest::Exact(player)),
            field,
            hud,
            ..race::Options::default()
        })
        .expect("loading the race");
        race::Race::start(loaded.setup)
    };

    let race = start(field.clone(), Some(hud));
    let world = &race.sim.world;
    assert_eq!(world.ship_count, 8, "the grid's seven opponents stay alive");
    assert_eq!(world.ships[0].physics.body.position, player.position);
    for (n, pose) in field.iter().enumerate() {
        assert_eq!(
            world.ships[n + 1].physics.body.position,
            pose.position,
            "opponent {} is on its captured pose",
            n + 1
        );
    }
    let readout = race.readout();
    assert_eq!((readout.place, readout.lap), (5, 2));
    assert_eq!((readout.lap_ticks, readout.race_ticks), (1_740, 2_333));
    assert_eq!(readout.best_lap_ticks, Some(3_380));
    assert!((readout.speed_kmh - 463.0).abs() < 1e-3);
    assert!((readout.shield - readout.shield_max * 0.99).abs() < 1e-3);

    // Neither is a default: with no field a posed player races alone, and with no
    // HUD state the readout is the race's own.
    let alone = start(Vec::new(), None);
    assert_eq!(alone.sim.world.ship_count, 1, "a pose alone drops the grid");
    assert_ne!(alone.readout().race_ticks, 2_333);
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_field_warm_up_gives_every_craft_a_trail_and_a_plume_and_leaves_the_poses_alone() {
    let Some(image) = oag_testdata::image("data/images/pulse-psp-eu.chd") else {
        return;
    };
    let field: Vec<Pose> = (1..=7u8)
        .map(|n| pose_at(f32::from(n) * 11.0, f32::from(n) * 5.0))
        .collect();
    let start = || {
        let loaded = race::load(&race::Options {
            source: image.display().to_string(),
            class: "VENOM".to_string(),
            mode: oag_race::Mode::SingleRace,
            pose: Some(race::PoseRequest::Exact(pose_at(-100.0, -170.0))),
            field: field.clone(),
            ..race::Options::default()
        })
        .expect("loading the race");
        race::Race::start(loaded.setup)
    };
    let velocities = vec![Vec3::new(0.0, 0.0, 100.0); 8];

    // The control: a posed race has no exhaust history at all.
    let cold = start();
    for slot in 0..8 {
        assert!(
            !cold.exhaust_of(slot).trail_ready(),
            "slot {slot} starts cold"
        );
        assert_eq!(cold.exhaust_of(slot).intensity(), 0.0);
    }

    let mut warm = start();
    let poses: Vec<(Vec3, Quat)> = (0..8)
        .map(|slot| {
            let body = &warm.sim.world.ships[slot].physics.body;
            (body.position, body.orientation)
        })
        .collect();
    let ticks = warm.field_warm_up_ticks();
    warm.warm_up_field(&velocities, ticks);
    for (slot, (position, orientation)) in poses.into_iter().enumerate() {
        let body = &warm.sim.world.ships[slot].physics.body;
        assert_eq!(
            body.position, position,
            "slot {slot} ends on its captured pose"
        );
        assert_eq!(body.orientation, orientation);
        let exhaust = warm.exhaust_of(slot);
        assert!(exhaust.trail_ready(), "slot {slot} has a full trail ring");
        assert!(
            exhaust.intensity() > 0.99,
            "slot {slot}'s plume is at the level a racing craft holds"
        );
        assert!(warm.nozzle_of(slot).is_some(), "slot {slot} has a nozzle");
        assert!(
            warm.trail_head_of(slot).is_none(),
            "slot {slot} is drawn at its tick, so its newest sample is already at the nozzle"
        );
    }
    assert_eq!(warm.sim.world.tick, 0, "no simulation tick ran");
}
