//! Which hull, livery and handling a Detonator race flies, answered from title data alone
//! (`docs/ghidra/functions/ps3-hdfury-eu/detonator-hull.md`).

use super::*;
use oag_livery::entry::{light_craft, model_directory, own_craft};

const TEAM_LIVERIES: [(&str, &str); 1] = [("Qirex", "zoneship_quirex")];

fn liveries() -> Vec<(String, String)> {
    TEAM_LIVERIES
        .iter()
        .map(|(team, livery)| (team.to_string(), livery.to_string()))
        .collect()
}

#[test]
fn a_detonator_craft_flies_its_own_handling_file_and_zone_keeps_its_own() {
    let hd = oag_hd::TITLE.race;
    assert_eq!(
        crate::load::sim::mode_handling(hd, Mode::Detonator).map(|file| file.entry.value),
        Some("/data/ships/detonator/handlingstats.xml")
    );
    assert_eq!(
        crate::load::sim::mode_handling(hd, Mode::Zone).map(|file| file.entry.value),
        Some("/data/ships/zone/handlingstats.xml")
    );
    assert!(crate::load::sim::mode_handling(hd, Mode::TimeTrial).is_none());
    for title in [oag_2048::TITLE, oag_omega::TITLE] {
        assert_eq!(
            crate::load::sim::mode_handling(title.race, Mode::Detonator)
                .map(|file| file.entry.value),
            Some(r"Data\art\published\hdships\Detonator\handlingstats.xml"),
            "{}",
            title.name
        );
        assert!(
            crate::load::sim::mode_handling(title.race, Mode::Zone).is_none(),
            "{}: Zone's file stays unread there",
            title.name
        );
    }
    assert!(crate::load::sim::mode_handling(oag_pulse::TITLE.race, Mode::TimeTrial).is_none());
}

#[test]
fn every_team_flies_the_one_detonator_hull_and_other_modes_keep_theirs() {
    let hd = oag_hd::TITLE.race.ships_for("Qirex");
    let hull = |mode| hd.hull_entry_name("Qirex", own_craft(hd, mode), None);
    assert_eq!(hull(Mode::Detonator), r"Data\Ships\detonator\Ship.vex");
    assert_eq!(hull(Mode::Zone), r"Data\Ships\zone\Ship.vex");
    assert_eq!(hull(Mode::TimeTrial), r"Data\Ships\Qirex\Ship.vex");
    assert_eq!(
        model_directory(hd, "Qirex", Mode::Detonator),
        (r"Data\Ships", "detonator"),
        "the flare, absorb shell and locators come out of the same folder"
    );
    let omega = oag_omega::TITLE.race.ships_for("Qirex");
    assert_eq!(
        omega.hull_entry_name("Qirex", own_craft(omega, Mode::Detonator), None),
        r"Data\art\published\hdships\Detonator\Ship.vex"
    );
    let pulse = oag_pulse::TITLE.race.ships_for("Feisar");
    assert_eq!(
        pulse.hull_entry_name("Feisar", own_craft(pulse, Mode::Detonator), None),
        r"Data\Ships\Feisar\Zone.vex",
        "Pulse ships no Detonator hull: the team's Zone one is flown"
    );
}

#[test]
fn the_engine_light_stays_zones_in_a_detonator_race() {
    let hd = oag_hd::TITLE.race.ships();
    assert_eq!(light_craft(hd, Mode::Detonator), Some(hd.zone));
    assert_eq!(light_craft(hd, Mode::Zone), Some(hd.zone));
    assert_eq!(light_craft(hd, Mode::TimeTrial), None);
}

#[test]
fn the_team_wears_its_detonator_livery_on_the_detonator_hull() {
    let hd = oag_hd::TITLE.race.ships();
    let rows = liveries();
    let mut report = Vec::new();
    let folder = oag_livery::zone_livery::swap(hd, Mode::Detonator, &rows, "qirex", &mut report)
        .expect("Qirex wears a Detonator livery");
    assert_eq!(
        folder
            .rewrite("data/ships/detonator/zoneship_zone/detonator_ao.gtf")
            .as_deref(),
        Some("data/ships/detonator/zoneship_quirex/detonator_ao.gtf")
    );
    assert!(report.is_empty(), "{report:?}");
    let fallback =
        oag_livery::zone_livery::swap(hd, Mode::Detonator, &rows, "Auricom", &mut report)
            .expect("a team with none wears zoneship_faisar on HD");
    assert_eq!(
        fallback
            .rewrite("data/ships/detonator/zoneship_zone/detonator_ao.gtf")
            .as_deref(),
        Some("data/ships/detonator/zoneship_faisar/detonator_ao.gtf")
    );
    let omega = oag_omega::TITLE.race.ships();
    assert!(
        oag_livery::zone_livery::swap(omega, Mode::Detonator, &rows, "Auricom", &mut report)
            .is_none()
    );
    assert_eq!(report.len(), 1, "an absent livery says so: {report:?}");
    let own = oag_livery::zone_livery::swap(omega, Mode::Detonator, &rows, "Qirex", &mut report)
        .expect("Omega takes the same rows");
    assert_eq!(own.own, "zoneship_team", "Omega's key");
}

/// The draw's per-mine constants line up with the matrices it draws: the ordinary mines first,
/// then each laid Detonator mine the survey can place, in the matrices' own order. A mine whose
/// constants went to another mine's slot would take that mine's colour and envelope.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn the_laid_mines_the_draw_colours_are_the_ones_it_places() {
    let Some(image) = oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let loaded = crate::load(&crate::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: Mode::Detonator,
        ..crate::Options::default()
    })
    .expect("loading an HD Detonator race");
    let mut race = Race::start(loaded.setup, loaded.view, loaded.modules);
    race.tick(&oag_gameplay::PlayerInputs::none());
    let (ordinary, laid) = race.detonator_mine_looks();
    assert!(!laid.is_empty(), "stage 1 laid mines");
    assert_eq!(ordinary + laid.len(), race.mine_model_matrices().len());
    assert!(
        laid.iter().all(|mine| mine.live),
        "the opening stage's mines are all live"
    );
}

/// A laid mine's size reaches its football's and its spikes' matrices as the scene recorded it
/// (`m+0xd8` and the spikes' own); a mine with none recorded is drawn at size 1.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn a_laid_mines_recorded_size_scales_its_football_and_its_spikes() {
    let Some(image) = oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let loaded = crate::load(&crate::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: Mode::Detonator,
        ..crate::Options::default()
    })
    .expect("loading an HD Detonator race");
    let mut race = Race::start(loaded.setup, loaded.view, loaded.modules);
    race.tick(&oag_gameplay::PlayerInputs::none());
    let (ordinary, laid) = race.detonator_mine_looks();
    let first = ordinary;
    let axis = |m: &oag_core::math::Mat4| m.x_axis.truncate().length();
    assert_eq!(axis(&race.mine_model_matrices()[first]), 1.0);
    race.view.mine_poses.lock().expect("not poisoned").insert(
        (laid[0].entry, laid[0].lane),
        crate::detonator_look::MinePose {
            football: 1.5,
            spikes: 2.25,
            age: 0.0,
        },
    );
    assert!((axis(&race.mine_model_matrices()[first]) - 1.5).abs() < 1e-5);
    assert!((axis(&race.mine_spike_matrices()[first]) - 2.25).abs() < 1e-5);
}

/// An HD mine's football and spikes are drawn on the world axes turned about the mine's up by
/// `4 x age` (`DetonatorMine_AnimateToMusic`), not on an upright frame: a quarter-turn age puts the
/// model's X where the original's row 0 says.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn an_hd_mine_turns_about_its_up_by_four_times_its_age() {
    let Some(image) = oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let loaded = crate::load(&crate::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: Mode::Detonator,
        ..crate::Options::default()
    })
    .expect("loading an HD Detonator race");
    let mut race = Race::start(loaded.setup, loaded.view, loaded.modules);
    race.tick(&oag_gameplay::PlayerInputs::none());
    let (ordinary, laid) = race.detonator_mine_looks();
    let age = std::f32::consts::FRAC_PI_8;
    race.view.mine_poses.lock().expect("not poisoned").insert(
        (laid[0].entry, laid[0].lane),
        crate::detonator_look::MinePose {
            football: 1.0,
            spikes: 1.5,
            age,
        },
    );
    let football = race.mine_model_matrices()[ordinary];
    let spikes = race.mine_spike_matrices()[ordinary];
    let detonator = race.core.sim.detonator.as_ref().expect("a Detonator race");
    let lane_width = detonator.layout.values.distance_offset_between_mines;
    let placed = race.core.sim.world.detonator.live_mines()[0];
    let (position, up) = detonator
        .survey
        .mine_point(&placed, lane_width)
        .expect("the first mine stands on its lane");
    let basis = crate::detonator_look::spun_basis(up, std::f32::consts::FRAC_PI_2);
    let close = |a: oag_core::math::Vec3, b: oag_core::math::Vec3| (a - b).length() < 1e-4;
    assert!(close(football.w_axis.truncate(), position));
    assert!(
        close(football.x_axis.truncate(), basis.x_axis),
        "football X {:?}, the original's row 0 {:?}",
        football.x_axis,
        basis.x_axis
    );
    assert!(close(football.z_axis.truncate(), basis.z_axis));
    assert!(close(spikes.x_axis.truncate(), basis.x_axis * 1.5));
}

/// A laid mine's visualiser ring is centred on the mine, its Z axis points at the camera, and it
/// takes `1 + 0.4 (football - 1)` of the football's size.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn a_laid_mines_visualiser_ring_faces_the_camera_and_follows_the_footballs_size() {
    let Some(image) = oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let loaded = crate::load(&crate::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: Mode::Detonator,
        ..crate::Options::default()
    })
    .expect("loading an HD Detonator race");
    let mut race = Race::start(loaded.setup, loaded.view, loaded.modules);
    race.tick(&oag_gameplay::PlayerInputs::none());
    let (ordinary, laid) = race.detonator_mine_looks();
    let eye = oag_core::math::Vec3::new(10.0, 20.0, 30.0);
    let football = race.mine_model_matrices()[ordinary];
    let ring = race.mine_visualiser_matrices(eye);
    assert_eq!(ring.len(), laid.len(), "one ring per laid mine");
    let centre = football.w_axis.truncate();
    assert!((ring[0].w_axis.truncate() - centre).length() < 1e-4);
    let toward = (eye - centre).normalize();
    assert!(ring[0].z_axis.truncate().normalize().dot(toward) > 0.9999);
    assert!((ring[0].z_axis.truncate().length() - 1.0).abs() < 1e-4);
    race.view.mine_poses.lock().expect("not poisoned").insert(
        (laid[0].entry, laid[0].lane),
        crate::detonator_look::MinePose {
            football: 2.0,
            spikes: 1.0,
            age: 0.0,
        },
    );
    let grown = race.mine_visualiser_matrices(eye);
    assert!((grown[0].z_axis.truncate().length() - 1.4).abs() < 1e-4);
}
