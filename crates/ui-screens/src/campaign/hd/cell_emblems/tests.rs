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
    shown_since_armed(seconds - ENTRY_HOLD_SECONDS)
}

fn shown_since_armed(since_armed: f32) -> Vec<f32> {
    draws(
        &slots(),
        &cell(Mode::Race, "Venom", true),
        &emblem(),
        &sprites,
        since_armed,
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

fn clock_at(seconds: f32) -> EntryClock {
    let mut clock = EntryClock::default();
    clock.tick(seconds);
    clock
}

/// Nothing re-arms the emblems inside the entry hold or when no change has happened, and a change
/// after the hold counts the `delay` from the change alone. This fails if the change is ignored
/// or the hold is applied again on top of it.
#[test]
fn a_change_after_the_hold_rearms_the_delay_without_the_hold() {
    let since = |entry: f32, refreshed_at: Option<f32>| since_armed(clock_at(entry), refreshed_at);
    assert!((since(1.0, None) - (1.0 - ENTRY_HOLD_SECONDS)).abs() < 1e-5);
    assert!(
        (since(1.0, Some(0.2)) - (1.0 - ENTRY_HOLD_SECONDS)).abs() < 1e-5,
        "a change inside the hold is covered by the hold's own re-arming"
    );
    assert!(
        (since(1.5, Some(1.0)) - 0.5).abs() < 1e-5,
        "a change at 1.0 s starts the delay there"
    );
    assert_eq!(
        since_armed(EntryClock::finished(), None),
        f32::INFINITY,
        "a still with no clock stays finished"
    );
}

/// After a change every emblem is gone again and then returns in order, each at its own delay.
#[test]
fn a_rearmed_page_hides_the_emblems_and_brings_them_back_in_order() {
    let arm = |entry: f32, at: f32| shown_since_armed(since_armed(clock_at(entry), Some(at)));
    assert_eq!(arm(1.0, 1.0), Vec::<f32>::new(), "all four wait again");
    assert_eq!(arm(1.06, 1.0).len(), 1, "the event emblem first, 0.05 s on");
    assert_eq!(arm(1.16, 1.0).len(), 2, "then the track emblem");
    assert_eq!(arm(1.40, 1.0).len(), 4, "all four back");
    assert_eq!(arm(2.0, 1.0), [96.0; 4], "and whole once the ramps are out");
}

/// The widgets the cell refresh re-arms hold their brackets' clock too; the target rows do not.
#[test]
fn the_refresh_rearms_the_emblems_and_the_right_column_but_not_the_target_rows() {
    for name in [
        "Event Emblem",
        "Track Emblem",
        "Speed Class Emblem",
        "Weapons Emblem",
        "RightColumnText",
    ] {
        assert!(rearms_brackets(Some(name)), "{name}");
    }
    assert!(!rearms_brackets(Some("Target Title")));
    assert!(!rearms_brackets(None));
}
