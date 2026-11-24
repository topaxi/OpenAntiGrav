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
        best_lap_centiseconds: None,
        boost_phase: None,
        autopilot_seconds: Some(5.0),
        held_pickup: None,
        shield_flash: None,
        song: None,
        head_to_head: None,
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
        race::Race::start(loaded.setup, loaded.view, loaded.modules)
    };

    let race = start(field.clone(), Some(hud));
    let world = &race.core.sim.world;
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
    // The running Autopilot's icon and `5.00`, on the readout alone: the race's own timer stays 0.
    assert_eq!(readout.pickup, Some(oag_tables::weapons::Weapon::Autopilot));
    assert_eq!(readout.sub_weapon, Some(oag_hud::SubWeapon::Autopilot(5.0)));
    assert_eq!(world.ships[0].autopilot_timer, 0.0);

    // Neither is a default: with no field a posed player races alone, and with no
    // HUD state the readout is the race's own.
    let alone = start(Vec::new(), None);
    assert_eq!(
        alone.core.sim.world.ship_count, 1,
        "a pose alone drops the grid"
    );
    assert_ne!(alone.readout().race_ticks, 2_333);
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_held_pickup_pin_puts_the_pickup_row_on_the_readout_alone() {
    use oag_tables::weapons::Weapon;
    let Some(image) = oag_testdata::image("data/images/pulse-psp-eu.chd") else {
        return;
    };
    let start = |held_pickup: Option<Weapon>| {
        let loaded = race::load(&race::Options {
            source: image.display().to_string(),
            class: "VENOM".to_string(),
            mode: oag_race::Mode::TimeTrial,
            pose: Some(race::PoseRequest::Exact(pose_at(-100.0, -170.0))),
            hud: Some(race::CaptureHud {
                place: 1,
                lap: 1,
                lap_ticks: 10_554,
                race_ticks: 10_554,
                speed_kmh: Some(165.0),
                shield_percent: Some(99.0),
                best_lap_ticks: None,
                best_lap_centiseconds: None,
                boost_phase: None,
                autopilot_seconds: None,
                held_pickup,
                shield_flash: None,
                song: None,
                head_to_head: None,
                zone: None,
            }),
            ..race::Options::default()
        })
        .expect("loading the race");
        race::Race::start(loaded.setup, loaded.view, loaded.modules)
    };
    let held = start(Some(Weapon::Turbo));
    let readout = held.readout();
    assert_eq!(readout.pickup, Some(Weapon::Turbo));
    assert_eq!(readout.sub_weapon, None, "a held pickup shows no countdown");
    assert!((readout.speed_kmh - 165.0).abs() < 1e-3);
    let ship = &held.core.sim.world.ships[0];
    assert_eq!(
        ship.pickup.weapon, None,
        "the craft holds nothing: the pin is the readout's alone"
    );
    assert_eq!(ship.autopilot_timer, 0.0);
    assert_eq!(start(None).readout().pickup, None, "no pin, no pickup row");
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_shield_flash_pin_turns_the_shield_bar_red_on_the_readout_alone() {
    let Some(image) = oag_testdata::image("data/images/pulse-psp-eu.chd") else {
        return;
    };
    let start = |shield_flash: Option<race::CaptureShieldFlash>| {
        let loaded = race::load(&race::Options {
            source: image.display().to_string(),
            class: "VENOM".to_string(),
            mode: oag_race::Mode::TimeTrial,
            pose: Some(race::PoseRequest::Exact(pose_at(-100.0, -170.0))),
            hud: Some(race::CaptureHud {
                place: 1,
                lap: 1,
                lap_ticks: 10_554,
                race_ticks: 10_554,
                speed_kmh: Some(160.0),
                shield_percent: Some(99.98),
                best_lap_ticks: None,
                best_lap_centiseconds: None,
                boost_phase: None,
                autopilot_seconds: None,
                held_pickup: None,
                shield_flash,
                song: None,
                head_to_head: None,
                zone: None,
            }),
            ..race::Options::default()
        })
        .expect("loading the race");
        race::Race::start(loaded.setup, loaded.view, loaded.modules)
    };
    let flashing = start(Some(race::CaptureShieldFlash {
        timer: 0.1335,
        blink: 0.0,
    }));
    let readout = flashing.readout();
    assert!(readout.shield_fraction() > 0.99, "the pool is not low");
    assert!(readout.shield_flashing && readout.shield_forced_red());
    assert!(
        readout.shield_blink_phase_on(),
        "the bar is drawn on the frame's blink phase"
    );
    assert!(
        !start(None).readout().shield_forced_red(),
        "no pin, the bar keeps its authored colour"
    );
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
        race::Race::start(loaded.setup, loaded.view, loaded.modules)
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
            let body = &warm.core.sim.world.ships[slot].physics.body;
            (body.position, body.orientation)
        })
        .collect();
    let motion: Vec<(Vec3, f32)> = (0..8)
        .map(|slot| {
            let ship = &warm.core.sim.world.ships[slot].physics;
            (ship.body.linear_velocity, ship.thrust)
        })
        .collect();
    let ticks = warm.core.sim.field_warm_up_ticks();
    warm.warm_up_field(&velocities, ticks, None);
    for (slot, (position, orientation)) in poses.into_iter().enumerate() {
        let body = &warm.core.sim.world.ships[slot].physics.body;
        assert_eq!(
            (
                body.linear_velocity,
                warm.core.sim.world.ships[slot].physics.thrust
            ),
            motion[slot],
            "slot {slot} is handed back at the speed and thrust it had: only its exhaust was replayed"
        );
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
        assert!(
            warm.view_state().nozzle_of(&warm.core, slot).is_some(),
            "slot {slot} has a nozzle"
        );
        assert!(
            warm.trail_head_of(slot).is_none(),
            "slot {slot} is drawn at its tick, so its newest sample is already at the nozzle"
        );
    }
    assert_eq!(warm.core.sim.world.tick, 0, "no simulation tick ran");
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_field_warm_up_arms_the_players_boost_at_its_age_and_nobody_elses() {
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
        race::Race::start(loaded.setup, loaded.view, loaded.modules)
    };
    let velocities = vec![Vec3::new(0.0, 0.0, 100.0); 8];

    let mut plain = start();
    let ticks = plain.core.sim.field_warm_up_ticks();
    plain.warm_up_field(&velocities, ticks, None);
    assert_eq!(plain.exhaust_of(0).boost_timer(), 0.0, "no age, no boost");

    // The original's flare read 0.349 s left on a boost that runs 0.8 s: an age of 0.451 s, 27 whole ticks.
    let mut boosted = start();
    boosted.warm_up_field(&velocities, ticks, Some(0.8 - 0.349_067));
    let left = boosted.exhaust_of(0).boost_timer();
    assert!(
        (left - 0.349_067).abs() <= 1.0 / 60.0,
        "the player's flare has {left} s of boost left, the original's row read 0.349"
    );
    assert!(
        boosted.exhaust_of(0).trail_ready(),
        "the boost rides the warmed trail"
    );
    for slot in 1..8 {
        assert_eq!(
            boosted.exhaust_of(slot).boost_timer(),
            0.0,
            "slot {slot} is not on the pad"
        );
    }
}
