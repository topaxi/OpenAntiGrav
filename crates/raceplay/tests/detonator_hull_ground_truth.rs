//! A Detonator race flies Detonator's own hull, livery and handling file, and draws its own mine,
//! bomb and pad models.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this project does not
//! ship.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-raceplay --run-ignored all \
//!     -E 'test(/^detonator_hull_ground_truth::/)'
//! ```
//!
//! The evidence is `docs/ghidra/functions/ps3-hdfury-eu/detonator-hull.md`. Zone's own numbers
//! are the control: the same load in `Mode::Zone` must keep flying Zone's hull and file.

use oag_raceplay as race;

const HD: &str = "data/images/hdfury-ps3-eu-dec.iso";

fn load(mode: oag_race::Mode, team: &str) -> Option<race::Loaded> {
    let path = oag_testdata::image(HD)?;
    Some(
        race::load(&race::Options {
            source: path.display().to_string(),
            class: "VENOM".to_string(),
            team: Some(team.to_string()),
            mode,
            ..race::Options::default()
        })
        .expect("loading an HD race"),
    )
}

/// The hull's width (`Misc.width`), which differs between Zone's file (5.0) and Detonator's (4.0).
/// [`load`] on a named circuit.
fn load_on(mode: oag_race::Mode, track: &str) -> Option<race::Loaded> {
    let path = oag_testdata::image(HD)?;
    Some(
        race::load(&race::Options {
            source: path.display().to_string(),
            class: "VENOM".to_string(),
            team: Some("Qirex".to_string()),
            mode,
            track: Some(track.to_string()),
            ..race::Options::default()
        })
        .expect("loading an HD race"),
    )
}

fn width(loaded: race::Loaded) -> f32 {
    let race = race::Race::start(loaded.setup, loaded.view, loaded.modules);
    race.core.sim.world.ships[0].handling.dimensions.width
}

/// Every craft flies `/data/ships/detonator/ship.vex`, in the team's own `zoneship_<team>` livery,
/// and the hull's locators and flare are that folder's.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn an_hd_detonator_craft_flies_the_detonator_hull_in_its_teams_livery() {
    let Some(loaded) = load(oag_race::Mode::Detonator, "Qirex") else {
        return;
    };
    let livery = &loaded.liveries[0];
    assert!(
        livery
            .hull
            .label
            .to_ascii_lowercase()
            .contains("ships\\detonator\\ship.vex"),
        "the hull is {}",
        livery.hull.label
    );
    assert!(!livery.hull.indices.is_empty(), "the hull draws");
    assert!(
        loaded.cannon_muzzles(0).iter().all(Option::is_some),
        "the Detonator folder's locators.vex authors both cannon_flash nodes"
    );
    assert_eq!(loaded.cannon_muzzles(0), livery.cannon_flash);
    assert!(
        loaded
            .report
            .iter()
            .any(|line| line.contains("Detonator livery zoneship_quirex replaces zoneship_zone")),
        "the team's livery is swapped in: {:#?}",
        loaded.report
    );
    assert!(
        loaded
            .report
            .iter()
            .any(|line| line.contains("a Detonator craft raises no shield")),
        "the shield is skipped and the report says so"
    );
}

/// The race flies `/data/ships/detonator/handlingstats.xml` (`Misc.width` 4.0), where a Zone race
/// on the same team flies Zone's (5.0).
#[test]
#[ignore = "needs a disc image in data/images/"]
fn an_hd_detonator_race_flies_its_own_handling_file_and_zone_keeps_its_own() {
    let Some(loaded) = load(oag_race::Mode::Detonator, "Qirex") else {
        return;
    };
    assert!(
        loaded.report.iter().any(|line| line.starts_with(
            "detonator: flying /data/ships/detonator/handlingstats.xml's classless block"
        )),
        "no Detonator handling line in the report"
    );
    let detonator_triangles = loaded.track_model.indices.len() / 3;
    assert!(
        loaded.pad_model.is_none() && loaded.weapon_pad_model.is_none(),
        "no separate circuit pad model under the Detonator pad"
    );
    // Zone's default circuit draws its own speed pads in the world-space pass (its pad nodes
    // name no chunk, 2093 triangles); a Detonator race there leaves them out under its own pad
    // (`detonator-emp.md`, "Do the circuit's own pads still draw?", 60).
    let zone = load(oag_race::Mode::Zone, "Qirex").expect("the same disc");
    let on_zone_circuit = load_on(oag_race::Mode::Detonator, &zone.circuit).expect("loads");
    let (zone_triangles, without) = (
        zone.track_model.indices.len() / 3,
        on_zone_circuit.track_model.indices.len() / 3,
    );
    println!(
        "track triangles on {}: Zone {zone_triangles}, Detonator {without}; Detonator's own circuit {detonator_triangles}",
        zone.circuit
    );
    assert_eq!(
        zone_triangles - without,
        2093,
        "Detonator leaves exactly the circuit's pad chunks out of the track"
    );
    assert_eq!(width(loaded), 4.0);
    assert!(
        zone.report
            .iter()
            .any(|line| line.starts_with("zone: flying /data/ships/zone/handlingstats.xml")),
        "Zone keeps its own file"
    );
    assert_eq!(width(zone), 5.0);
}

/// A Detonator mine is the football and its spikes, a bomb its body, equalisers and glow edges,
/// and every speed pad and weapon pad carries the Detonator pad. A mine also carries its
/// `electric_visualiser` ring, drawn through the electricity program's two glow-table entries.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn an_hd_detonator_race_draws_its_own_mine_bomb_and_pad_models() {
    let Some(loaded) = load(oag_race::Mode::Detonator, "Qirex") else {
        return;
    };
    let label = |model: &Option<oag_mesh::mesh::Model>| {
        model
            .as_ref()
            .map(|model| model.label.to_ascii_lowercase())
            .unwrap_or_default()
    };
    assert!(label(&loaded.mine_model).contains("mine_rita_football"));
    assert!(label(&loaded.bomb_model).contains("detonator_bomb_final.vex"));
    let parts = &loaded.bomb_blast_models.detonator;
    assert!(label(&parts.mine_spikes).contains("mine_rita_spikes"));
    assert!(label(&parts.bomb_equalisers).contains("final_equalisers"));
    assert!(label(&parts.bomb_glow_edges).contains("glowedges"));
    assert!(label(&parts.pad).contains("detonator_pad"));
    assert!(label(&parts.cannon_bolt).contains("detonator_cannonbolt"));
    assert!(label(&parts.muzzle_flash).contains("hd_muzzleflash_detonator"));
    assert_eq!(
        parts.pads.len(),
        loaded.setup.speedup_pads.len() + loaded.setup.weapon_pads.len(),
        "a pad model on every speed pad and every weapon pad"
    );
    assert!(!parts.pads.is_empty(), "the circuit has pads");
    assert!(
        loaded.pad_model.is_none() && loaded.weapon_pad_model.is_none(),
        "the circuit's own pads are not drawn under the Detonator pad"
    );

    assert!(label(&parts.mine_visualiser).contains("electric_visualiser"));
    let ring = parts
        .mine_visualiser
        .as_ref()
        .expect("the visualiser loaded");
    let programs: Vec<f32> = ring.emissive.iter().map(|entry| entry.program[0]).collect();
    assert!(
        programs.contains(&oag_mesh::mesh::PROGRAM_ELECTRICITY)
            && programs.contains(&oag_mesh::mesh::PROGRAM_ELECTRICITY_AUDIO),
        "the ring is drawn by its own program, not the generic path: {programs:?}"
    );
    // The control: a Zone race on the same circuit keeps the ordinary bodies.
    let Some(zone) = load(oag_race::Mode::Zone, "Qirex") else {
        return;
    };
    assert!(label(&zone.mine_model).contains("hd_mine"));
    assert!(zone.bomb_blast_models.detonator.pads.is_empty());
}

/// Slot 0's pad with Square held.
fn square() -> oag_gameplay::PlayerInputs {
    let mut snapshot = oag_gameplay::InputSnapshot::new();
    snapshot
        .buttons
        .begin_frame(oag_gameplay::input::Button::Square.bit());
    oag_gameplay::PlayerInputs::single(snapshot)
}

/// **The cannon's muzzles are the hull's `cannon_flash` locators.** The simulation reads the
/// pair itself (`load::muzzle::cannon_flash`), the livery reads the same pair for the drawn
/// flash: the first volley's two rounds leave from the livery's locators carried by the craft,
/// the left one on the craft's left, and where the drawn hull puts those locators is the same
/// place.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn an_hd_detonator_volley_leaves_from_the_hulls_cannon_flash_locators() {
    let Some(loaded) = load(oag_race::Mode::Detonator, "Qirex") else {
        return;
    };
    let locators = loaded.cannon_muzzles(0).map(|m| m.expect("both locators"));
    for locator in &locators {
        assert_eq!(
            locator.z_axis.truncate(),
            oag_core::math::Vec3::Z,
            "the locators carry no rotation, so a round leaves along the craft's own forward"
        );
    }
    let mut race = race::Race::start(loaded.setup, loaded.view, loaded.modules);
    while oag_race::RaceState::thrust_gated(race.core.sim.world.tick) {
        race.tick(&oag_gameplay::PlayerInputs::none());
    }
    race.core.sim.drain_cues();
    race.tick(&square());
    let fired = race
        .core
        .sim
        .drain_cues()
        .iter()
        .filter(|event| event.cue == oag_gameplay::cue::Cue::Cannon)
        .count();
    assert_eq!(fired, 2, "each round of the volley plays CANNON");
    let cannon = &race.core.sim.world.detonator.cannon;
    let mut rounds = cannon.rounds.iter().filter(|r| r.live);
    let (first, second) = (rounds.next().expect("a round"), rounds.next().expect("two"));
    assert!(rounds.next().is_none(), "one volley is two rounds");

    let body = race.core.sim.world.ships[0].physics.body;
    let drawn = race.ship_model_matrix();
    for (round, local, side) in [(first, locators[0], -1.0), (second, locators[1], 1.0)] {
        let sim = oag_weapons::projectile::plasma::hull_point(&body, local.w_axis.truncate());
        assert!(
            (round.prev - sim).length() < 1e-3,
            "a round leaves from its locator: {:?} against {sim:?}",
            round.prev
        );
        let at = drawn * local.w_axis.truncate().extend(1.0);
        assert!(
            (round.prev - at.truncate()).length() < 0.5,
            "the drawn hull puts the locator within half a unit of where the round left: {:?} \
             against {:?}",
            round.prev,
            at.truncate()
        );
        assert!(
            (round.prev - body.position).dot(body.right()) * side > 0.0,
            "the left locator is on the craft's left"
        );
    }
}

/// The scale the player's hull is drawn at: the length of its model matrix's columns.
fn drawn_hull_scale(loaded: race::Loaded) -> f32 {
    let race = race::Race::start(loaded.setup, loaded.view, loaded.modules);
    race.ship_model_matrices()[0].x_axis.truncate().length()
}

/// A live Detonator frame's hull world matrix carries rows of length `0.630` (`0.7 * 0.9`), where
/// every other HD mode carries `0.75` (`docs/ghidra/functions/ps3-hdfury-eu/camera.md`, "The drawn
/// hull"). Zone's race on the same disc is the control.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn an_hd_detonator_hull_is_drawn_at_the_detonator_scale_and_zones_at_the_ordinary_one() {
    let (Some(detonator), Some(zone)) = (
        load(oag_race::Mode::Detonator, "Feisar"),
        load(oag_race::Mode::Zone, "Feisar"),
    ) else {
        return;
    };
    let detonator = drawn_hull_scale(detonator);
    let zone = drawn_hull_scale(zone);
    assert!(
        (detonator - 0.63).abs() < 1e-4,
        "Detonator draws at {detonator}"
    );
    assert!((zone - 0.75).abs() < 1e-4, "Zone draws at {zone}");
}
