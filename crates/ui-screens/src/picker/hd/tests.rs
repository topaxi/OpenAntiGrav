//! `oag_ui_screens::picker::hd` against a hand-written miniature of HD's
//! `Team_Selection_Definition.xml`: the same element shapes and nesting,
//! not the disc's file. `selection_screens_ground_truth.rs` reads the real
//! one.

use super::*;
use crate::picker::pointer::{What, targets};
use crate::picker::{Entry, Event, Kind};
use oag_gameplay::input::{Button, Input};
use oag_ui::frontend::Draw;

const XML: &str = r#"<Variable global="Grey" name="Grey"><Values String="0xff969696"></Values></Variable>
<Screen name="Team Selection Top Level">
  <Model name="ShipModel"><Values OriginX="1220" OriginY="412" z="-24.0" RotX="0.4" RotY="-0.5"></Values></Model>
  <Item OffsetX="160" OffsetY="170">
    <MiniText><Values y="-30" idstring="RC_CHOOSE_TEAM" color="FEGlobals->Grey"></Values></MiniText>
    <Bracket><Values corner="true" Y="0" Width="506" Height="156"></Values></Bracket>
    <Bracket><Values corner="true" Y="196" Width="506" Height="600"></Values></Bracket>
    <Image name="Logo"><Values x="0" y="15" width="512" height="128"></Values></Image>
    <Bracket><Values corner="true" middle="true" X="545" Width="1052" Height="650"></Values></Bracket>
  </Item>
  <Item name="shipStats" OffsetX="705" OffsetY="833">
    <Block name="Slide_0"><Values idstring="RC_SPEED" width="512" shaped="true" color="FEGlobals->Grey" AlwaysSolidColor="0xff646464"></Values></Block>
  </Item>
  <Text name="netLobbyStatus"><Values string="" font="Title" x="0" y="0"></Values></Text>
  <Screen type="TeamSelection" name="Team Selection">
    <Text><Values idstring="RC_SHIPSEL" font="Title" x="0" y="0"></Values></Text>
  </Screen>
</Screen>"#;

fn strings() -> StringTable {
    StringTable::from_xml(
        r#"<Screen name="Top">
        <Entry ID="RC_SHIPSEL" String="SHIP SELECT"/>
        <Entry ID="RC_CHOOSE_TEAM" String="CHOOSE TEAM"/>
        <Entry ID="RC_SPEED" String="SPEED"/>
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

fn team(id: &str, stats: Vec<Option<Stats>>) -> Entry {
    Entry {
        id: id.to_string(),
        label: id.to_uppercase(),
        details: Details::Ship {
            loyalty: None,
            rating: None,
            variants: vec![
                (String::new(), "HD".into()),
                ("_c1".into(), "Fury Concept".into()),
            ],
            stats,
        },
    }
}

#[test]
fn the_child_screen_carries_the_parents_widgets_and_its_own_title() {
    let layout = layout();
    // The child's `RC_SHIPSEL`, not the parent's empty `netLobbyStatus`,
    // which is in the same face and comes first.
    assert_eq!(layout.title, "SHIP SELECT");
    assert_eq!(layout.screen.name, SCREEN);
    assert!(
        layout
            .screen
            .blocks
            .iter()
            .any(|block| block.name.as_deref() == Some("Slide_0")),
        "the parent's blocks come across"
    );
    let extra = layout.hd.as_deref().expect("an HD layout");
    // Every `Item` offset folded in.
    assert_eq!(
        extra.labels,
        vec![MiniText {
            x: 160.0,
            y: 140.0,
            text: "CHOOSE TEAM".into(),
            color: 0xff96_9696,
        }]
    );
    assert_eq!(extra.brackets.len(), 3);
    assert_eq!(layout.panel, [160.0, 170.0, 506.0, 156.0]);
    assert_eq!(layout.preview, [705.0, 170.0, 1052.0, 650.0]);
    assert_eq!(extra.logo, Some([160.0, 185.0, 512.0, 128.0]));
    assert_eq!(
        extra.ship_model.map(|model| model.origin),
        Some([1220.0, 412.0])
    );
    assert_eq!(extra.solid, vec![("Slide_0".to_string(), 0xff64_6464)]);
}

#[test]
fn a_file_missing_either_screen_reads_as_none() {
    let xml = r#"<Screen name="Team Selection Top Level"></Screen>"#;
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
fn teams_move_across_and_liveries_up_and_down() {
    let mut picker = Picker::new(
        Kind::Ship,
        vec![team("Feisar", Vec::new()), team("Qirex", Vec::new())],
        Some("Feisar"),
        None,
    )
    .with_entries_across();
    let mut input = Input::default();
    let mut press = |picker: &mut Picker, button: Button| {
        input.begin_frame(0);
        input.begin_frame(1 << button as u32);
        picker.update(&mut input)
    };
    assert_eq!(press(&mut picker, Button::Right), vec![Event::Moved]);
    assert_eq!(picker.index(), 1);
    assert_eq!(
        press(&mut picker, Button::Down),
        vec![Event::VariantChanged]
    );
    assert_eq!(picker.variant().map(|(id, _)| id.as_str()), Some("_c1"));
    assert_eq!(press(&mut picker, Button::Left), vec![Event::Moved]);
    assert_eq!(picker.index(), 0);
}

#[test]
fn the_team_frame_steps_the_team_and_the_ship_frame_confirms() {
    let layout = layout();
    let picker = Picker::new(Kind::Ship, vec![team("Feisar", Vec::new())], None, None);
    let skin = oag_ui::menu::Skin::new(
        oag_pulse::FRONT_END.menu.unwrap(),
        oag_display::space::Space::PSP,
        22.0,
    );
    let targets = targets(&picker, &layout, &skin, &|_| None);
    let what = |at: (f32, f32)| crate::picker::pointer::hit(&targets, at).map(|t| t.what);
    assert_eq!(what((200.0, 250.0)), Some(What::Previous));
    assert_eq!(what((600.0, 250.0)), Some(What::Next));
    assert_eq!(what((300.0, 400.0)), Some(What::PreviousVariant));
    assert_eq!(what((300.0, 700.0)), Some(What::NextVariant));
    assert_eq!(what((1200.0, 500.0)), Some(What::Confirm));
    assert_eq!(what((50.0, 1000.0)), None);
}

#[test]
fn the_stat_value_prints_three_digits_at_its_split() {
    let layout = layout();
    let stats = Stats {
        speed: 70,
        thrust: 80,
        handling: 100,
        shield: 85,
    };
    let picker = Picker::new(
        Kind::Ship,
        vec![team("Feisar", vec![Some(stats), None])],
        None,
        None,
    );
    let extra = layout.hd.as_deref().unwrap();
    let draws = body(&picker, &layout, extra, &Frame::default(), &|_| None);
    let texts: Vec<&str> = draws
        .iter()
        .filter_map(|draw| match draw {
            Draw::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert!(texts.contains(&"SPEED"), "{texts:?}");
    // No `Label_0` in the miniature, so no value text - but the stats are
    // the selected livery's, and a livery with none prints none.
    assert_eq!(selected_stats(&picker), Some(stats));
}

#[test]
fn a_livery_with_no_model_of_its_own_shows_no_stats() {
    let mut picker = Picker::new(
        Kind::Ship,
        vec![team("Feisar", vec![None, None])],
        None,
        Some("_c1"),
    );
    assert_eq!(selected_stats(&picker), None);
    picker = Picker::new(Kind::Ship, vec![team("Feisar", Vec::new())], None, None);
    assert_eq!(selected_stats(&picker), None);
}

#[test]
fn the_logo_is_the_teams_own_front_end_texture() {
    assert_eq!(logo_src("Feisar"), r"Data\Ships\Feisar\FE\Logo.gtf");
}
