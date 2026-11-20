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

fn emblem() -> HashMap<String, String> {
    HashMap::from([(
        "17_track".to_string(),
        track_emblem_src(r"Data\Environments\Talons_Junction"),
    )])
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
    .map(|slot| source(slot, &cell, &emblem()))
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
    assert_eq!(source("Event Emblem", &nitro, &emblem()), None);
    let zone = cell(Mode::Zone, "Zone", true);
    assert_eq!(source("Speed Class Emblem", &zone, &emblem()), None);
    let off = cell(Mode::Race, "Venom", false);
    assert_eq!(
        source("Weapons Emblem", &off, &emblem()).as_deref(),
        Some(r"Data\FE\Images\weaponsoff_bw.gtf")
    );
}

#[test]
fn the_sheet_wants_every_icon_the_screen_can_name() {
    let wanted = sheet_sources();
    assert_eq!(wanted.len(), 14);
    for mode in [Mode::Race, Mode::TimeTrial, Mode::SpeedLap, Mode::Zone] {
        let cell = cell(mode, "Venom", true);
        let src = source("Event Emblem", &cell, &emblem()).expect("a stem");
        assert!(wanted.contains(&src), "{src}");
    }
}

fn slots() -> Vec<Slot> {
    // `CellMode_Definition.xml`'s four emblems: 96 square, `vwipe` in 0.3 s, the delays RAM read as
    // 3, 8, 12 and 17 frames after the hold.
    [
        ("Event Emblem", 0.05),
        ("Track Emblem", 0.125),
        ("Speed Class Emblem", 0.2),
        ("Weapons Emblem", 0.275),
    ]
    .into_iter()
    .map(|(name, delay)| Slot {
        name: Some(name.to_string()),
        x: 10.0,
        y: 20.0,
        width: Some(96.0),
        height: Some(96.0),
        wipe: Some(oag_ui::screen::Vwipe {
            seconds: 0.3,
            delay,
        }),
    })
    .collect()
}

fn sprites(src: &str) -> Option<Placed> {
    src.ends_with("_bw.gtf").then_some(Placed {
        x: 0,
        y: 0,
        width: 96,
        height: 96,
        quad_extent: None,
        quad_centre: None,
        blend: None,
        scale: [1.0, 1.0],
    })
}

fn shown_at(seconds: f32) -> Vec<f32> {
    let mut clock = EntryClock::default();
    clock.tick(seconds);
    draws(
        &slots(),
        &cell(Mode::Race, "Venom", true),
        &emblem(),
        &sprites,
        clock,
    )
    .iter()
    .map(|draw| match draw {
        Draw::Sprite { rect, .. } => rect[3],
        other => panic!("{other:?}"),
    })
    .collect()
}

/// The page holds its four emblems back for [`ENTRY_HOLD_SECONDS`], then each waits its own
/// delay and ramps over 0.3 s: the first emblem is the first one drawn (0.37 + 0.05 s), the
/// last the last (0.37 + 0.275 s), and the page is whole once the ramps are out. This fails if the
/// hold or the entry clock is dropped.
#[test]
fn the_emblems_hold_then_start_in_order_and_finish() {
    assert_eq!(
        (ENTRY_HOLD_SECONDS * 60.0).round(),
        22.0,
        "22 frames of hold"
    );
    assert_eq!(shown_at(0.0), Vec::<f32>::new());
    assert_eq!(shown_at(0.40), Vec::<f32>::new(), "the hold has not ended");
    // 0.37 + 0.05 s: the first ramp frame, a slab a tenth of the 96 tall.
    let first = shown_at(0.43);
    assert_eq!(first.len(), 1, "{first:?}");
    assert!((first[0] - 9.6).abs() < 1e-3, "{first:?}");
    assert_eq!(shown_at(0.50).len(), 2, "the track emblem has started");
    assert_eq!(shown_at(0.60).len(), 3, "the speed class emblem too");
    assert_eq!(shown_at(0.70).len(), 4, "all four have started");
    assert_eq!(shown_at(1.5), [96.0; 4], "every emblem whole");
}
