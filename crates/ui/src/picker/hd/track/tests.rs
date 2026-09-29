//! `oag_ui::picker::hd::track` against a hand-written miniature of HD's
//! `Track_Selection_Definition.xml`. `selection_screens_ground_truth.rs`
//! reads the real one.

use super::*;
use crate::picker::pointer::{What, targets};
use crate::picker::{Entry, Event, Kind};
use oag_gameplay::input::{Button, Input};

const XML: &str = r#"<Variable global="Grey" name="Grey"><Values String="0xff969696"></Values></Variable>
<Screen name="TrackSelectionTopLevel">
  <Model name="TrackModel"><Values OriginX="1308" OriginY="440" z="-180.0"></Values></Model>
  <Item OffsetX="160" OffsetY="170">
    <MiniText><Values y="-30" idstring="RB_CHOOSE_CIRCUIT" color="FEGlobals->Grey"></Values></MiniText>
    <Item OffsetY="0">
      <Bracket><Values corner="true" X="0" Y="0" Width="680" Height="242"></Values></Bracket>
      <Image name="Emblem"><Values x="110" y="8" width="192" height="192"></Values></Image>
      <Text name="TrackName"><Values idstring="RB_TRACK_SEL" x="340" y="200"></Values></Text>
    </Item>
    <Bracket><Values corner="true" X="0" Y="282" Width="680" Height="230"></Values></Bracket>
    <TrackHexSelection name="TrackHexSelector" OffsetX="50" OffsetY="330"><Values columns="9" rows="2"></Values></TrackHexSelection>
  </Item>
  <Bracket><Values corner="true" middle="true" X="856" Y="170" Width="900" Height="565"></Values></Bracket>
  <Screen type="TrackSelection" name="Track Creation">
    <Text><Values idstring="RB_TRACK_SEL" font="Title" x="0" y="0"></Values></Text>
    <Image name="RecordParent" OffsetX="160" OffsetY="744">
      <Item OffsetY="25">
        <Block name="Record.1.1"><Values string="16" width="367" y="50"></Values></Block>
      </Item>
    </Image>
  </Screen>
  <Screen type="TournamentSelection" name="Tournament C">
    <Text name="TrackNumber0"><Values string="1"></Values></Text>
  </Screen>
</Screen>"#;

fn strings() -> StringTable {
    StringTable::from_xml(
        r#"<Screen name="Top">
        <Entry ID="RB_TRACK_SEL" String="TRACK SELECT"/>
        <Entry ID="RB_CHOOSE_CIRCUIT" String="CHOOSE CIRCUIT"/>
        <Entry ID="FE_PERSONAL" String="PERSONAL"/>
        <Entry ID="FE_FRIENDS" String="FRIENDS"/>
        <Entry ID="FE_GLOBAL" String="GLOBAL"/>
        </Screen>"#,
    )
}

fn layout() -> Layout {
    let screens = Screens::from_xml_with_fallback_globals(XML, &[]);
    read(
        XML,
        &screens,
        &strings(),
        FaceScales::default(),
        [1920.0, 1080.0],
    )
    .expect("both screens are there")
}

fn circuit(id: &str, reversed: bool) -> Entry {
    Entry {
        id: id.to_string(),
        label: id.to_uppercase(),
        details: Details::Track {
            info: ["4.4KM".into(), "13.2KM".into(), "-".into()],
            emblem: Some(emblem_src(&format!(r"Data\Environments\{id}"))),
            reversed,
        },
    }
}

#[test]
fn the_child_screen_carries_the_parents_widgets_and_none_of_tournament_cs() {
    let layout = layout();
    assert_eq!(layout.title, "TRACK SELECT");
    assert_eq!(layout.screen.name, SCREEN);
    assert!(layout.hd.is_none() && layout.is_hd());
    assert!(
        layout
            .screen
            .texts
            .iter()
            .all(|text| text.name.as_deref() != Some("TrackNumber0")),
        "the sibling Tournament C screen is not merged in"
    );
    let extra = layout.hd_track.as_deref().expect("an HD track layout");
    assert_eq!(extra.common.labels.len(), 1);
    assert_eq!(extra.common.labels[0].y, 140.0);
    assert_eq!(extra.emblem, Some([270.0, 178.0, 192.0, 192.0]));
    assert_eq!(extra.model.map(|m| m.origin), Some([1308.0, 440.0]));
    let grid = extra.hex_grid.expect("the hex selector");
    assert_eq!(
        (grid.origin, grid.columns, grid.rows),
        ([210.0, 500.0], 9, 2)
    );
    assert_eq!(extra.row_labels, ["PERSONAL", "FRIENDS", "GLOBAL"]);
    assert_eq!(layout.panel, [160.0, 170.0, 680.0, 242.0]);
    assert_eq!(layout.preview, [856.0, 170.0, 900.0, 565.0]);
}

#[test]
fn a_file_missing_either_screen_reads_as_none() {
    let xml = r#"<Screen name="TrackSelectionTopLevel"></Screen>"#;
    let screens = Screens::from_xml_with_fallback_globals(xml, &[]);
    assert!(
        read(
            xml,
            &screens,
            &strings(),
            FaceScales::default(),
            [1920.0, 1080.0]
        )
        .is_none()
    );
}

#[test]
fn the_emblem_is_the_environments_own_fe_texture() {
    assert_eq!(
        emblem_src(r"Data\Environments\01_Vineta_K"),
        r"Data\Environments\01_Vineta_K\FE\TrackSelectEmblem_Fury.gtf"
    );
}

#[test]
fn lengths_print_as_kilometres_and_a_lapless_race_has_no_distance() {
    assert_eq!(length_rows(4400.0, Some(3)), ["4.4KM", "13.2KM"]);
    assert_eq!(length_rows(4400.0, None), ["4.4KM", ""]);
}

fn press(picker: &mut Picker, button: Button) -> Vec<Event> {
    let mut input = Input::default();
    input.begin_frame(0);
    input.begin_frame(1 << button as u32);
    picker.update(&mut input)
}

fn grid() -> Picker {
    let entries = ["a", "b", "c"]
        .iter()
        .map(|id| circuit(id, false))
        .chain(["a", "b", "c"].iter().map(|id| circuit(id, true)))
        .collect();
    Picker::new(Kind::Track, entries, Some("a"), None).with_rows(3)
}

#[test]
fn right_wraps_along_the_row_and_up_down_switch_direction() {
    let mut picker = grid();
    for expected in [1, 2, 0] {
        assert_eq!(press(&mut picker, Button::Right), vec![Event::Moved]);
        assert_eq!(
            picker.index(),
            expected,
            "twelve presses in the original, three here"
        );
    }
    assert_eq!(press(&mut picker, Button::Left), vec![Event::Moved]);
    assert_eq!(picker.index(), 2, "left wraps the other way");
    assert_eq!(press(&mut picker, Button::Down), vec![Event::Moved]);
    assert_eq!(picker.index(), 5, "same circuit, the reverse row");
    assert_eq!(press(&mut picker, Button::Right), vec![Event::Moved]);
    assert_eq!(picker.index(), 3, "still on the reverse row");
    assert_eq!(press(&mut picker, Button::Up), vec![Event::Moved]);
    assert_eq!(picker.index(), 0);
}

#[test]
fn a_single_row_has_nothing_for_up_and_down_to_do() {
    let entries = ["a", "b"].iter().map(|id| circuit(id, false)).collect();
    let mut picker = Picker::new(Kind::Track, entries, None, None).with_rows(2);
    assert!(press(&mut picker, Button::Down).is_empty());
    assert_eq!(press(&mut picker, Button::Right), vec![Event::Moved]);
    assert_eq!(picker.index(), 1);
}

#[test]
fn pointer_halves_step_and_the_model_frame_confirms() {
    let layout = layout();
    let picker = grid();
    let skin = crate::menu::Skin::new(
        oag_pulse::FRONT_END.menu.unwrap(),
        oag_display::space::Space::PSP,
        22.0,
    );
    let found = targets(&picker, &layout, &skin, &|_| None);
    let at = |x, y| crate::picker::pointer::hit(&found, (x, y)).map(|t| t.what);
    assert_eq!(at(200.0, 300.0), Some(What::Previous));
    assert_eq!(at(800.0, 300.0), Some(What::Next));
    assert_eq!(at(1200.0, 400.0), Some(What::Confirm));
    // The hex frame: top half is the row above, bottom half the row below.
    assert_eq!(at(400.0, 500.0), Some(What::PreviousVariant));
    assert_eq!(at(400.0, 640.0), Some(What::NextVariant));
    // And a click there moves the cursor to the other direction's row.
    let mut picker = grid();
    let pointer = crate::pointer::Pointer {
        at: Some((400.0, 640.0)),
        moved: true,
        clicked: true,
        ..crate::pointer::Pointer::default()
    };
    assert_eq!(picker.pointer(&pointer, &found), vec![Event::Moved]);
    assert_eq!(picker.index(), 3);
    // A single row has no such target.
    let single = Picker::new(Kind::Track, vec![circuit("a", false)], None, None).with_rows(1);
    let found = targets(&single, &layout, &skin, &|_| None);
    assert!(found.iter().all(|t| t.what != What::NextVariant));
}
