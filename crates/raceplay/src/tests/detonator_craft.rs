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
    assert_eq!(own_craft(pulse, Mode::Detonator), None);
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
