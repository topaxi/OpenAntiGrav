use super::*;
use oag_mod::campaign::Progression;

fn choices() -> Choices {
    let slots = (0..7)
        .flat_map(|x| (0..if x % 2 == 0 { 5 } else { 4 }).map(move |y| (x, y)))
        .collect();
    Choices {
        tracks: vec![Pick::plain("01_Track"), Pick::plain("02_Track")],
        teams: vec![Pick::plain("Feisar"), Pick::plain("Qirex")],
        classes: ["Venom", "Flash", "Rapier", "Phantom"]
            .map(Pick::plain)
            .to_vec(),
        slots,
        title: "wipeout-pulse".to_string(),
        authored: vec![Authored {
            mode: "time_trial".to_string(),
            track: "02_Track".to_string(),
            class: "venom".to_string(),
            targets: [150.0, 165.0, 180.0],
        }],
    }
}

#[test]
fn a_placed_event_is_written_and_reads_back() {
    let mut editor = Editor::new(choices(), "Night Shift");
    editor.select((2, 1));
    editor.confirm();
    editor.select_field(Field::Circuit);
    editor.cycle(1);
    editor.select_field(Field::Team);
    editor.cycle(2);
    editor.select_field(Field::Ai);
    editor.cycle(1);
    let def = editor.to_def().unwrap();
    let event = &def.grids[0].events[0];
    assert_eq!(event.slot, [2, 1]);
    assert_eq!(event.track.as_deref(), Some("02_Track"));
    assert_eq!(event.craft.as_ref().unwrap().team, "Qirex");
    assert_eq!(event.field.as_ref().unwrap().ai.as_deref(), Some("ours"));
    assert_eq!(def.campaign.id, "night-shift");
    let text = def.write().unwrap();
    assert_eq!(oag_mod::campaign::CampaignDef::parse(&text).unwrap(), def);
}

#[test]
fn a_zone_event_writes_no_class() {
    let mut editor = Editor::new(choices(), "Zones");
    editor.select((0, 0));
    editor.confirm();
    editor.select_field(Field::Mode);
    editor.cycle(3);
    let def = editor.to_def().unwrap();
    assert_eq!(def.grids[0].events[0].mode, "zone");
    assert_eq!(def.grids[0].events[0].class, None);
    assert!(def.write().is_ok());
}

#[test]
fn nothing_placed_or_no_name_is_not_a_campaign() {
    assert!(Editor::new(choices(), "Empty").to_def().is_err());
    let mut editor = Editor::new(choices(), "  ");
    editor.confirm();
    assert!(editor.to_def().is_err());
}

#[test]
fn the_cursor_walks_the_staggered_grid_and_stops_at_its_edge() {
    let mut editor = Editor::new(choices(), "Walk");
    editor.select((2, 2));
    editor.step(0, 1);
    assert_eq!(editor.cursor(), (2, 3));
    editor.step(1, 0);
    assert_eq!(editor.cursor().0, 3);
    editor.select((0, 0));
    editor.step(-1, 0);
    editor.step(0, -1);
    assert_eq!(editor.cursor(), (0, 0));
    assert!(
        !editor.select((9, 9)),
        "a slot the title draws no hex at is refused"
    );
}

#[test]
fn an_unnamed_start_still_makes_an_id() {
    assert_eq!(Editor::new(choices(), "123 go!").id(), "campaign-123-go");
}

#[test]
fn clearing_a_slot_empties_it() {
    let mut editor = Editor::new(choices(), "Clear");
    editor.confirm();
    assert_eq!(editor.count(), 1);
    editor.clear();
    assert_eq!(editor.count(), 0);
    assert!(!editor.editing());
}

#[test]
fn a_timed_event_takes_the_discs_own_targets_for_its_circuit_and_none_otherwise() {
    let mut editor = Editor::new(choices(), "Timed");
    editor.select((0, 0));
    editor.confirm();
    editor.select_field(Field::Mode);
    editor.cycle(1);
    let none = editor.to_def().unwrap();
    assert_eq!(none.grids[0].events[0].medals, None, "no cell on 01_Track");
    editor.select_field(Field::Circuit);
    editor.cycle(1);
    let def = editor.to_def().unwrap();
    let medals = def.grids[0].events[0].medals.as_ref().unwrap();
    assert_eq!(
        (medals.gold, medals.silver, medals.bronze),
        (Some(150.0), Some(165.0), Some(180.0))
    );
}

#[test]
fn a_race_without_an_authored_cell_is_awarded_by_finishing_place() {
    let mut editor = Editor::new(choices(), "Places");
    editor.confirm();
    let def = editor.to_def().unwrap();
    let medals = def.grids[0].events[0].medals.as_ref().unwrap();
    assert_eq!(
        (medals.gold, medals.silver, medals.bronze),
        (Some(1.0), Some(2.0), Some(3.0))
    );
}

#[test]
fn the_screens_words_are_all_in_the_english_strings() {
    let english = include_str!("../../../../../assets/ui/strings/english.toml");
    for (id, text) in WORDS
        .iter()
        .copied()
        .chain(MODES.iter().map(|m| (m.1, m.2)))
        .chain(AIS.iter().map(|a| (a.1, a.2)))
        .chain(DIFFICULTIES.iter().map(|d| (d.1, d.2)))
        .chain(PROGRESSIONS.iter().map(|p| (p.1, p.2)))
    {
        assert!(
            english.contains(&format!("{id} = {{ text = \"{text}\"")),
            "{id}"
        );
    }
}

/// The title's seven-class-wide ladder, so a bound has room to matter.
fn wide() -> Choices {
    let mut choices = choices();
    choices.tracks = (1..=9)
        .map(|n| Pick::plain(&format!("{n:02}_Track")))
        .collect();
    choices.classes = ["Venom", "Flash", "Rapier", "Phantom", "Spectre"]
        .map(Pick::plain)
        .to_vec();
    choices
}

fn filled(seed: u32) -> Editor {
    let mut editor = Editor::new(wide(), "Filled");
    editor.set_seed(seed);
    editor.fill();
    editor
}

#[test]
fn the_same_seed_fills_the_same_grid_and_another_seed_a_different_one() {
    let a = filled(7).to_def().unwrap();
    assert_eq!(a, filled(7).to_def().unwrap());
    assert_ne!(a, filled(8).to_def().unwrap());
    assert_eq!(
        a.grids[0].events.len(),
        wide().slots.len(),
        "every slot is filled"
    );
}

#[test]
fn a_fill_does_not_depend_on_what_was_placed_or_where_the_cursor_stood() {
    let clean = filled(11).to_def().unwrap();
    let mut editor = Editor::new(wide(), "Filled");
    editor.select((3, 2));
    editor.confirm();
    editor.select_field(Field::Mode);
    editor.cycle(2);
    editor.set_seed(11);
    editor.fill();
    assert_eq!(editor.to_def().unwrap().grids, clean.grids);
}

#[test]
fn every_filled_cell_draws_its_own_class_inside_the_bounds() {
    let mut editor = Editor::new(wide(), "Bounded");
    editor.set_class_bounds(1, 3);
    let mut seen = std::collections::BTreeSet::new();
    for seed in 1..=4 {
        editor.set_seed(seed);
        editor.fill();
        for event in &editor.to_def().unwrap().grids[0].events {
            if event.mode == "zone" {
                assert_eq!(event.class, None);
                continue;
            }
            let class = event.class.clone().unwrap();
            assert!(
                ["Flash", "Rapier", "Phantom"].contains(&class.as_str()),
                "{class} is outside Flash..=Phantom"
            );
            seen.insert(class);
        }
    }
    assert_eq!(
        seen.len(),
        3,
        "each cell draws its own class, not one for all: {seen:?}"
    );
}

#[test]
fn a_one_class_range_gives_every_cell_that_class() {
    let mut editor = Editor::new(wide(), "Single");
    editor.set_class_bounds(2, 2);
    editor.fill();
    assert!(
        editor.to_def().unwrap().grids[0]
            .events
            .iter()
            .all(|e| e.class.as_deref().is_none_or(|c| c == "Rapier"))
    );
}

#[test]
fn a_filled_cell_is_edited_like_any_other() {
    let mut editor = filled(5);
    let before = editor.to_def().unwrap();
    editor.select((2, 1));
    editor.confirm();
    editor.select_field(Field::Circuit);
    editor.cycle(1);
    editor.clear();
    editor.select((0, 0));
    editor.confirm();
    editor.select_field(Field::Team);
    editor.cycle(1);
    let after = editor.to_def().unwrap();
    assert_eq!(
        after.grids[0].events.len(),
        before.grids[0].events.len() - 1
    );
    assert!(after.grids[0].events.iter().all(|e| e.slot != [2, 1]));
    let first = &after.grids[0].events[0];
    assert_eq!(first.craft.as_ref().unwrap().team, "Feisar");
    assert_eq!(
        first.track, before.grids[0].events[0].track,
        "only the team changed"
    );
}

#[test]
fn a_bound_that_passes_the_other_drags_it_along() {
    let mut editor = Editor::new(wide(), "Bounds");
    assert_eq!(
        editor.class_bounds(),
        (0, 4),
        "the whole ladder to start with"
    );
    editor.select_setting(Setting::ClassTo);
    editor.cycle_setting(-1);
    editor.cycle_setting(-1);
    assert_eq!(editor.class_bounds(), (0, 2));
    editor.select_setting(Setting::ClassFrom);
    for _ in 0..3 {
        editor.cycle_setting(1);
    }
    assert_eq!(editor.class_bounds(), (3, 3));
    editor.select_setting(Setting::ClassTo);
    editor.cycle_setting(-1);
    assert_eq!(editor.class_bounds(), (2, 2));
    assert_eq!(editor.setting_label(Setting::ClassFrom), "RAPIER");
}

#[test]
fn the_campaign_page_edits_the_setting_the_file_carries() {
    let mut editor = Editor::new(wide(), "Settings");
    editor.confirm();
    let first = editor.to_def().unwrap();
    assert_eq!(first.campaign.difficulty.as_deref(), Some("medium"));
    assert_eq!(first.campaign.progression, Progression::Flat);
    editor.toggle_page();
    assert_eq!(editor.page(), Page::Campaign);
    editor.select_setting(Setting::Difficulty);
    editor.cycle_setting(1);
    editor.select_setting(Setting::Progression);
    editor.cycle_setting(1);
    editor.select_setting(Setting::Seed);
    editor.cycle_setting(-1);
    assert_eq!(editor.seed(), 0);
    let def = editor.to_def().unwrap();
    assert_eq!(def.campaign.difficulty.as_deref(), Some("hard"));
    assert_eq!(def.campaign.progression, Progression::Ramp);
    let text = def.write().unwrap();
    assert_eq!(oag_mod::campaign::CampaignDef::parse(&text).unwrap(), def);
    oag_schema::validate::accepts(
        oag_mod::campaign::SCHEMA_KIND,
        oag_mod::campaign::SCHEMA_MAJOR,
        &text,
    )
    .unwrap_or_else(|e| panic!("{e:?}\n{text}"));
    editor.select((0, 0));
    assert_eq!(
        editor.page(),
        Page::Event,
        "a click on a hex leaves the settings"
    );
}

#[test]
fn a_cell_flown_by_our_driver_names_no_difficulty_of_its_own() {
    let mut editor = Editor::new(wide(), "Ours");
    editor.confirm();
    editor.select_field(Field::Ai);
    editor.cycle(1);
    let event = &editor.to_def().unwrap().grids[0].events[0];
    let field = event.field.as_ref().unwrap();
    assert_eq!(field.ai.as_deref(), Some("ours"));
    assert_eq!(field.difficulty, None, "the campaign's setting governs it");
}

#[test]
fn the_actions_of_each_page_end_in_save_and_start_with_the_page_switch() {
    let mut editor = Editor::new(wide(), "Actions");
    assert_eq!(editor.actions(), [Action::Page, Action::Name, Action::Save]);
    editor.toggle_page();
    assert_eq!(editor.actions(), [Action::Page, Action::Fill, Action::Save]);
}
