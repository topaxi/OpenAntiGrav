use super::*;

fn choices() -> Choices {
    let slots = (0..7)
        .flat_map(|x| (0..if x % 2 == 0 { 5 } else { 4 }).map(move |y| (x, y)))
        .collect();
    Choices {
        tracks: vec![Pick::plain("01_Track"), Pick::plain("02_Track")],
        teams: vec![Pick::plain("Feisar"), Pick::plain("Qirex")],
        classes: ["Venom", "Flash", "Rapier", "Phantom"].map(Pick::plain).to_vec(),
        slots,
        title: "wipeout-pulse".to_string(),
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
    assert!(!editor.select((9, 9)), "a slot the title draws no hex at is refused");
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
