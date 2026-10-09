//! `oag_ui_screens::picker::hd::track` against a hand-written miniature of HD's
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
            icon: Some(grid_icon_src(&format!(r"Data\Environments\{id}"))),
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
    // HD authors no `x`/`y` and no `orthoScale`: the placement is `z` alone.
    let model = extra.model.expect("the TrackModel widget");
    assert_eq!((model.offset, model.ortho_scale), ([0.0; 2], [1.0; 3]));
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
    let skin = oag_ui::menu::Skin::new(
        oag_pulse::FRONT_END.menu.unwrap(),
        oag_display::space::Space::PSP,
        22.0,
    );
    let found = targets(&picker, &layout, &skin, &|_| None);
    let at = |x, y| crate::picker::pointer::hit(&found, (x, y)).map(|t| t.what);
    assert_eq!(at(200.0, 300.0), Some(What::Previous));
    assert_eq!(at(800.0, 300.0), Some(What::Next));
    assert_eq!(at(1200.0, 400.0), Some(What::Confirm));
    // The grid draws, so its frame's halves are not targets: a hexagon is.
    // Circuit `a` is the centre column (origin 210 + 4 * 72.2), its reverse
    // twin the cell below.
    let (cx, top, bottom) = (210.0 + 4.0 * 72.2, 500.0, 500.0 + 82.8);
    assert_eq!(
        at(cx, top),
        Some(What::Cell {
            entry: 0,
            variant: 0
        })
    );
    assert_eq!(
        at(cx, bottom),
        Some(What::Cell {
            entry: 3,
            variant: 0
        })
    );
    // The next column is the next circuit, `b`, on its half-row shift.
    assert_eq!(
        at(cx + 72.2, top + 41.4),
        Some(What::Cell {
            entry: 1,
            variant: 0
        })
    );
    // A click on the other direction's hexagon moves the cursor there, and
    // a second click on it confirms.
    let mut picker = grid();
    let pointer = oag_ui::pointer::Pointer {
        at: Some((cx, bottom)),
        moved: true,
        clicked: true,
        ..oag_ui::pointer::Pointer::default()
    };
    assert_eq!(picker.pointer(&pointer, &found), vec![Event::Moved]);
    assert_eq!(picker.index(), 3);
    let found = targets(&picker, &layout, &skin, &|_| None);
    assert_eq!(picker.pointer(&pointer, &found), vec![Event::Confirmed]);
    // Without the grid the frame's halves switch the direction row.
    let mut bare = layout.clone();
    bare.hd_track.as_mut().expect("hd").hex_grid = None;
    let found = targets(&grid(), &bare, &skin, &|_| None);
    let at = |x, y| crate::picker::pointer::hit(&found, (x, y)).map(|t| t.what);
    assert_eq!(at(400.0, 500.0), Some(What::PreviousVariant));
    assert_eq!(at(400.0, 640.0), Some(What::NextVariant));
    // A single row has no such target.
    let single = Picker::new(Kind::Track, vec![circuit("a", false)], None, None).with_rows(1);
    let found = targets(&single, &bare, &skin, &|_| None);
    assert!(found.iter().all(|t| t.what != What::NextVariant));
}

fn sheet(name: &str) -> Option<oag_ui::frontend::Placed> {
    (name.contains("Hexagon") || name.to_ascii_lowercase().contains("trackselectemblem_bw"))
        .then_some(oag_ui::frontend::Placed {
            x: 0,
            y: 0,
            width: 128,
            height: 64,
            quad_extent: None,
            blend: None,
        })
}

fn grid_draws(picker: &Picker) -> Vec<Draw> {
    let layout = layout();
    let extra = layout.hd_track.as_deref().expect("hd");
    let mut out = Vec::new();
    draw_grid(
        extra.hex_grid.as_ref().expect("grid"),
        picker,
        &sheet,
        &mut out,
    );
    out
}

#[test]
fn the_grid_puts_a_circuit_in_each_hexagon_and_red_on_the_chosen_column() {
    let picker = grid();
    let draws = grid_draws(&picker);
    // 9 columns x 2 rows: a hexagon and an icon each, the chosen column's
    // two hexagons drawn twice, and the ring on the forward row.
    let sprites = draws
        .iter()
        .filter(|d| matches!(d, Draw::Sprite { .. }))
        .count();
    assert_eq!(sprites, 18 + 2 + 18 + 1);
    let red = draws
        .iter()
        .filter(|d| matches!(d, Draw::Sprite { color, .. } if color[0] == 1.0 && color[1] == 0.0))
        .count();
    assert_eq!(red, 4, "both chosen-column hexagons, twice each");
}

#[test]
fn the_ring_follows_the_direction_row_and_dropping_the_grid_draws_nothing() {
    let mut picker = grid();
    let ring_y = |draws: &[Draw]| {
        draws.iter().rev().find_map(|d| match d {
            Draw::Sprite { rect, color, .. } if (color[0] - RING[0]).abs() < 1e-6 => Some(rect[1]),
            _ => None,
        })
    };
    let forward = ring_y(&grid_draws(&picker)).expect("ring on the forward row");
    press(&mut picker, Button::Down);
    let reverse = ring_y(&grid_draws(&picker)).expect("ring on the reverse row");
    assert!((reverse - forward - 82.8).abs() < 0.01);
    let layout = layout();
    let extra = layout.hd_track.as_deref().expect("hd");
    let mut bare = extra.clone();
    bare.hex_grid = None;
    let with = body(&picker, &layout, extra, &Frame::default(), &sheet);
    let without = body(&picker, &layout, &bare, &Frame::default(), &sheet);
    assert!(with.len() > without.len());
}
