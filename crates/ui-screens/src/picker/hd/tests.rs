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
    <HexSelection name="hexselection" OffsetX="112" OffsetY="225"><Values columns="5" rows="7" SelectedLockCol="0xFF808080" SelectedLockColFade="0xFF242424" HexCol="0x64808080" HexColFade="0x32808080" SelectedColumnCol="0x64ff0000"></Values></HexSelection>
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
    layout_of(XML)
}

fn layout_of(xml: &str) -> Layout {
    let screens = Screens::from_xml_with_fallback_globals(xml, &[]);
    read(
        xml,
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
            models: Vec::new(),
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
    let grid = extra.hex.expect("the HexSelection widget reads");
    assert_eq!((grid.columns, grid.rows), (5, 7));
    assert_eq!(grid.origin, [272.0, 395.0], "the item offsets fold in");
    assert_eq!(grid.hex, 0x6480_8080);
    assert_eq!(grid.selected_column, 0x64ff_0000);
    assert_eq!(grid.selected_lock, 0xff80_8080);
    assert_eq!(grid.selected_lock_fade, 0xff24_2424);
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
fn without_a_hex_grid_the_team_frame_steps_the_team_and_the_ship_frame_confirms() {
    let start = XML.find("    <HexSelection").unwrap();
    let end = start + XML[start..].find("</HexSelection>").unwrap() + "</HexSelection>\n".len();
    let layout = layout_of(&format!("{}{}", &XML[..start], &XML[end..]));
    assert!(layout.hd.as_deref().unwrap().hex.is_none());
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

use super::hex::{HEX_SRC, LOCK_SRC, ModelCell, OUTLINE_SRC};
use oag_ui::frontend::Placed;

/// A team with seven models in the disc's order, of which `normal` (row 3)
/// and `concept1` (row 2) are open - Feisar's two `<Unlock>`-free models.
fn hex_team(id: &str) -> Entry {
    let mut entry = team(id, Vec::new());
    let Details::Ship { models, .. } = &mut entry.details else {
        unreachable!()
    };
    *models = (0..7)
        .map(|row| {
            let variant = match row {
                3 => Some(0),
                2 => Some(1),
                _ => None,
            };
            ModelCell {
                thumb: variant.map(|_| format!(r"Data\Ships\{id}\FE\thumb{row}.gtf")),
                open: variant.is_some(),
                variant,
            }
        })
        .collect();
    entry
}

fn sheet(name: &str) -> Option<Placed> {
    let known = [HEX_SRC, LOCK_SRC, OUTLINE_SRC];
    (known.contains(&name) || name.contains("thumb")).then_some(Placed {
        x: 0,
        y: 0,
        width: 128,
        height: 64,
        quad_extent: None,
        blend: None,
    })
}

fn picker_of(teams: &[&str], selected: &str) -> Picker {
    Picker::new(
        Kind::Ship,
        teams.iter().map(|id| hex_team(id)).collect(),
        Some(selected),
        None,
    )
    .with_entries_across()
}

#[test]
fn the_hex_column_draws_cells_padlocks_and_the_selected_team_in_red() {
    let layout = layout();
    let extra = layout.hd.as_deref().unwrap();
    let picker = picker_of(&["A", "B", "C", "D", "E", "F"], "C");
    let draws = body(&picker, &layout, extra, &Frame::default(), &|n| sheet(n));
    let sprites: Vec<([f32; 4], [f32; 4])> = draws
        .iter()
        .filter_map(|draw| match draw {
            Draw::Sprite { rect, color, .. } => Some((*rect, *color)),
            _ => None,
        })
        .collect();
    let red = oag_ui::screen::argb_to_rgba(0x64ff_0000);
    // Seven selected-column cells, each in the authored red twice.
    assert_eq!(sprites.iter().filter(|(_, c)| *c == red).count(), 14);
    // 5 columns x 7 rows of cells in all, 14 + 28 draws.
    let grey = oag_ui::screen::argb_to_rgba(0x6480_8080);
    assert_eq!(sprites.iter().filter(|(_, c)| *c == grey).count(), 28);
    // Five of seven rows are padlocked in each of the five columns.
    let lock = oag_ui::screen::argb_to_rgba(0xff80_8080);
    assert_eq!(sprites.iter().filter(|(_, c)| *c == lock).count(), 5);
    // The selected column's padlock reads the authored colour exactly; the
    // outer columns have faded toward `SelectedLockColFade`.
    let faded = oag_ui::screen::argb_to_rgba(0xff24_2424);
    assert!(
        sprites
            .iter()
            .any(|(_, c)| c[0] < lock[0] && c[0] > faded[0] && c[3] == 1.0)
    );
}

#[test]
fn the_cursor_ring_follows_the_chosen_livery_row() {
    let layout = layout();
    let extra = layout.hd.as_deref().unwrap();
    let ring = |picker: &Picker| -> Option<f32> {
        body(picker, &layout, extra, &Frame::default(), &|n| sheet(n))
            .iter()
            .filter_map(|draw| match draw {
                Draw::Sprite { rect, color, .. } if *color == [0.73, 0.14, 0.22, 1.0] => {
                    Some(rect[1])
                }
                _ => None,
            })
            .next()
    };
    let mut picker = picker_of(&["A", "B", "C"], "B");
    let first = ring(&picker).expect("a ring");
    let mut input = Input::default();
    input.begin_frame(0);
    input.begin_frame(1 << Button::Down as u32);
    picker.update(&mut input);
    let second = ring(&picker).expect("a ring");
    assert!(second < first, "row 2 sits above row 3: {first} {second}");
}

#[test]
fn a_hex_is_a_pointer_target_only_when_its_model_is_open() {
    let layout = layout();
    let picker = picker_of(&["A", "B", "C"], "A");
    let skin = oag_ui::menu::Skin::new(
        oag_pulse::FRONT_END.menu.unwrap(),
        oag_display::space::Space::PSP,
        22.0,
    );
    let targets = targets(&picker, &layout, &skin, &|_| None);
    let grid = layout.hd.as_deref().unwrap().hex.unwrap();
    let cells = grid.cells(0, 3);
    let at = |column: usize, row: usize| {
        let cell = cells[column * 7 + row];
        (
            cell.rect[0] + cell.rect[2] * 0.5,
            cell.rect[1] + cell.rect[3] * 0.5,
        )
    };
    let what = |at: (f32, f32)| crate::picker::pointer::hit(&targets, at).map(|t| t.what);
    // The column right of the selected one (team B), `concept1`'s row.
    assert_eq!(
        what(at(3, 2)),
        Some(What::Cell {
            entry: 1,
            variant: 1
        })
    );
    // A padlocked row is nothing - not the old half-bracket fallback.
    assert_eq!(what(at(3, 0)), None);
    // The selected cell itself.
    assert_eq!(
        what(at(2, 3)),
        Some(What::Cell {
            entry: 0,
            variant: 0
        })
    );
}

#[test]
fn clicking_a_hex_selects_its_team_and_livery_and_the_chosen_one_confirms() {
    let layout = layout();
    let mut picker = picker_of(&["A", "B", "C"], "A");
    let skin = oag_ui::menu::Skin::new(
        oag_pulse::FRONT_END.menu.unwrap(),
        oag_display::space::Space::PSP,
        22.0,
    );
    let grid = layout.hd.as_deref().unwrap().hex.unwrap();
    let cell = grid.cells(0, 3)[3 * 7 + 2];
    let at = (
        cell.rect[0] + cell.rect[2] * 0.5,
        cell.rect[1] + cell.rect[3] * 0.5,
    );
    let click = oag_ui::pointer::Pointer {
        at: Some(at),
        clicked: true,
        ..oag_ui::pointer::Pointer::default()
    };
    let events = picker.pointer(&click, &targets(&picker, &layout, &skin, &|_| None));
    assert_eq!(events, vec![Event::Moved]);
    assert_eq!((picker.index(), picker.variant_index()), (1, 1));
    // The same spot is now the centre column's `concept1`: already chosen.
    let centre = grid.cells(1, 3)[2 * 7 + 2];
    let at = (
        centre.rect[0] + centre.rect[2] * 0.5,
        centre.rect[1] + centre.rect[3] * 0.5,
    );
    let click = oag_ui::pointer::Pointer {
        at: Some(at),
        clicked: true,
        ..oag_ui::pointer::Pointer::default()
    };
    let events = picker.pointer(&click, &targets(&picker, &layout, &skin, &|_| None));
    assert_eq!(events, vec![Event::Confirmed]);
}
