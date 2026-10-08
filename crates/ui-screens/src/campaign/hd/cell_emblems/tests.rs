use super::*;

fn cell(mode: Mode, class: &str, weapons: bool) -> Cell {
    Cell {
        name: "grid8_0_0".to_string(),
        track: Some("17_Track".to_string()),
        mode,
        class: class.to_string(),
        weapons,
        damage: true,
        locked: None,
        status: None,
        ai_count: None,
        skill: None,
        skill_easy: None,
        skill_hard: None,
        laps: Some(3),
        ship: None,
        ship_choice: None,
        gold: 1,
        silver: 2,
        bronze: 3,
        tournament_tracks: Vec::new(),
        difficulty_targets: None,
        nitro_elimination_targets: None,
    }
}

fn emblem(id: &str) -> Option<String> {
    (id == "17_Track").then(|| track_emblem_src(r"Data\Environments\Talons_Junction"))
}

#[test]
fn a_single_race_on_venom_with_weapons_names_four_pictures() {
    let cell = cell(Mode::Race, "Venom", true);
    let named: Vec<Option<String>> = [
        "Event Emblem",
        "Track Emblem",
        "Speed Class Emblem",
        "Weapons Emblem",
    ]
    .iter()
    .map(|slot| source(slot, &cell, &emblem))
    .collect();
    assert_eq!(
        named,
        [
            Some(r"Data\FE\Images\singlerace_bw.gtf".to_string()),
            Some(r"Data\Environments\Talons_Junction\FE\TrackSelectEmblem_bw.gtf".to_string()),
            Some(r"Data\FE\Images\venom_bw.gtf".to_string()),
            Some(r"Data\FE\Images\weaponson_bw.gtf".to_string()),
        ]
    );
}

#[test]
fn what_the_disc_has_no_icon_for_draws_nothing() {
    let nitro = cell(Mode::Other("NitroBattle".to_string()), "Venom", false);
    assert_eq!(source("Event Emblem", &nitro, &emblem), None);
    let zone = cell(Mode::Zone, "Zone", true);
    assert_eq!(source("Speed Class Emblem", &zone, &emblem), None);
    let off = cell(Mode::Race, "Venom", false);
    assert_eq!(
        source("Weapons Emblem", &off, &emblem).as_deref(),
        Some(r"Data\FE\Images\weaponsoff_bw.gtf")
    );
}

#[test]
fn the_sheet_wants_every_icon_the_screen_can_name() {
    let wanted = sheet_sources();
    assert_eq!(wanted.len(), 14);
    for mode in [Mode::Race, Mode::TimeTrial, Mode::SpeedLap, Mode::Zone] {
        let cell = cell(mode, "Venom", true);
        let src = source("Event Emblem", &cell, &emblem).expect("a stem");
        assert!(wanted.contains(&src), "{src}");
    }
}
