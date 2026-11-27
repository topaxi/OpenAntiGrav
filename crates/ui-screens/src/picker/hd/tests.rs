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
<Variable global="HD_Blue" name="HD_Blue"><Values String="0xffac0717"></Values></Variable>
<Screen name="Team Selection Top Level">
  <Model name="ShipModel"><Values OriginX="1220" OriginY="412" z="-24.0" RotX="0.4" RotY="-0.5"></Values></Model>
  <Item OffsetX="160" OffsetY="170">
    <MiniText><Values y="-30" idstring="RC_CHOOSE_TEAM" color="FEGlobals->Grey"></Values></MiniText>
    <Bracket><Values corner="true" Y="0" Width="506" Height="156"></Values></Bracket>
    <Bracket><Values corner="true" Y="196" Width="506" Height="600"></Values></Bracket>
    <Image name="Logo"><Values transitiontype="vwipe" delay="0.5" enebletransition="0.5" x="0" y="15" width="512" height="128"></Values></Image>
    <Bracket><Values corner="true" middle="true" X="545" Width="1052" Height="650"></Values></Bracket>
    <HexSelection name="hexselection" OffsetX="112" OffsetY="225"><Values columns="5" rows="7" SelectedLockCol="0xFF808080" SelectedLockColFade="0xFF242424" HexCol="0x64808080" HexColFade="0x32808080" SelectedColumnCol="0x64ff0000"></Values></HexSelection>
  </Item>
  <Item name="shipStats" OffsetX="705" OffsetY="833">
    <Block name="Slide_0"><Values idstring="RC_SPEED" width="512" shaped="true" unselectable="true" arrowcolor="0xffffffff" color="FEGlobals->Grey" AlwaysSolidColor="0xff646464"></Values></Block>
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
    let screens = Screens::from_xml_with_fallback_globals(xml, &[("HD_Blue", "0xffac0717")]);
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
            logo: None,
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
        extra.logo_wipe,
        Some(oag_ui::screen::Vwipe {
            seconds: 0.7,
            delay: 0.5
        }),
        "`enebletransition` matches no key, so the class default stands"
    );
    assert_eq!(
        extra.ship_model.map(|model| model.origin),
        Some([1220.0, 412.0])
    );
    assert_eq!(extra.solid, vec![("Slide_0".to_string(), 0xff64_6464)]);
    let grid = extra.hex.expect("the HexSelection widget reads");
    assert_eq!((grid.columns, grid.rows), (5, 7));
    assert_eq!(grid.origin, [272.0, 395.0], "the item offsets fold in");
    assert_eq!(grid.hex, 0x6480_8080);
    assert_eq!(
        grid.selected_column, 0xffac_0717,
        "`SelectedColumnCol` is not a key the widget reads; its fill is `HD_Blue`"
    );
    assert_eq!(grid.ring_peak, Some(0xffac_0717));
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
            [1920.0, 1080.0],
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
        length_milli: None,
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
    assert_eq!(
        logo_src(r"Data\Ships\Feisar"),
        r"Data\Ships\Feisar\FE\Logo.gtf"
    );
}

mod cells;
mod slide_draw;
use super::hex::{HEX_SRC, LOCK_SRC, ModelCell, THICK_OUT_SRC};
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
                thumb: Some(format!(r"Data\Ships\{id}\FE\thumb{row}.gtf")),
                open: variant.is_some(),
                variant,
            }
        })
        .collect();
    entry
}

/// Each of the widget's own images at its own place on the sheet, so a draw's `uv` names it.
fn sheet(name: &str) -> Option<Placed> {
    let known = [HEX_SRC, LOCK_SRC, THICK_OUT_SRC];
    let at = known.iter().position(|src| *src == name);
    (at.is_some() || name.contains("thumb")).then_some(Placed {
        x: 128 * at.map_or(known.len() as u32, |at| at as u32),
        y: 0,
        width: 128,
        height: 64,
        quad_extent: None,
        quad_centre: None,
        blend: None,
        scale: [1.0, 1.0],
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
    let red = oag_ui::screen::argb_to_rgba(0xffac_0717);
    // Seven selected-column cells, each in `HD_Blue` once.
    assert_eq!(sprites.iter().filter(|(_, c)| *c == red).count(), 7);
    // 5 columns x 7 rows of cells in all: 7 flat and 28 shaded across. The four other columns
    // shade every picture (28) and their five locked rows' padlocks (20) too.
    let shaded = draws
        .iter()
        .filter(|draw| matches!(draw, Draw::GradientSprite { .. }))
        .count();
    assert_eq!(shaded, 28 + 28 + 20);
    // Five of the selected column's seven rows are padlocked, in the authored colour exactly.
    let lock = oag_ui::screen::argb_to_rgba(0xff80_8080);
    assert_eq!(sprites.iter().filter(|(_, c)| *c == lock).count(), 5);
}

/// A cell outside the selected column runs from `HexColFade` to `HexCol` across its quad: whole at
/// the selected column's edges, none at the grid's outer edges. The quad is the widget's 92x80 over
/// the art's 74x64 texels.
#[test]
fn a_cell_fades_across_its_quad_toward_the_selected_column() {
    let layout = layout();
    let extra = layout.hd.as_deref().unwrap();
    let picker = picker_of(&["A", "B", "C", "D", "E", "F"], "C");
    let draws = body(&picker, &layout, extra, &Frame::default(), &|n| sheet(n));
    // Each fill: (left edge of its quad, alpha at the left, alpha at the right), in 255ths.
    let mut cells: Vec<(f32, f32, f32)> = draws
        .iter()
        .filter_map(|draw| match draw {
            Draw::GradientSprite {
                rect,
                uv,
                left,
                right,
            } if *uv == sheet(HEX_SRC).unwrap().sheet_rect([0.0, 0.0, 74.0, 64.0]) => {
                assert_eq!([rect[2], rect[3]], [92.0, 80.0]);
                Some((rect[0], left[3] * 255.0, right[3] * 255.0))
            }
            _ => None,
        })
        .collect();
    assert_eq!(cells.len(), 28, "four shaded columns of seven");
    cells.sort_by(|a, b| a.0.total_cmp(&b.0));
    // `HexColFade`'s alpha is 50, `HexCol`'s 100, so half way is 75.
    let want = [[50.0, 75.0], [75.0, 100.0], [100.0, 75.0], [75.0, 50.0]];
    for (column, [left, right]) in cells.chunks(7).zip(want) {
        for cell in column {
            assert!((cell.1 - left).abs() < 0.01, "{cell:?} left {left}");
            assert!((cell.2 - right).abs() < 0.01, "{cell:?} right {right}");
        }
    }
}

/// The share of `HexCol` at a point, in columns from the middle of the selected column: Track
/// Select's nine columns fade over four, and the spare column of a slide has none.
#[test]
fn the_edge_share_reaches_zero_at_the_outer_edge_and_on_the_spare_column() {
    use super::hex::edge_share;
    assert_eq!(edge_share(4, 0.5), 1.0);
    assert_eq!(edge_share(4, -1.5), 0.75);
    assert_eq!(edge_share(4, 4.5), 0.0);
    assert_eq!(edge_share(2, 3.5), 0.0);
    assert_eq!(edge_share(2, -2.5), 0.0);
    assert_eq!(edge_share(0, 1.5), 0.0, "a one column grid divides by one");
}

/// The cursor ring: one quad of `Hexagon_HD_THICK_OUT.gtf`, 92x80 units over a 74x64 texel window,
/// on the chosen livery's cell of the centre column only, following the chosen livery as the
/// selection moves between variants and teams, and drawn after every cell.
#[test]
fn the_cursor_ring_is_one_quad_on_the_chosen_liveries_cell() {
    let layout = layout();
    let extra = layout.hd.as_deref().unwrap();
    let grid = extra.hex.unwrap();
    let ring = |picker: &Picker| -> Vec<[f32; 4]> {
        let draws = body(picker, &layout, extra, &Frame::default(), &|n| sheet(n));
        let ring: Vec<usize> = draws
            .iter()
            .enumerate()
            .filter_map(|(at, draw)| match draw {
                Draw::Sprite { uv, .. }
                    if *uv
                        == sheet(THICK_OUT_SRC)
                            .unwrap()
                            .sheet_rect([0.0, 0.0, 74.0, 64.0]) =>
                {
                    Some(at)
                }
                _ => None,
            })
            .collect();
        let last_cell = draws
            .iter()
            .rposition(|draw| matches!(draw, Draw::GradientSprite { .. } | Draw::Sprite { .. }))
            .unwrap();
        assert!(
            ring.iter().all(|at| *at == last_cell),
            "the ring is the last sprite"
        );
        ring.iter()
            .map(|at| match draws[*at] {
                Draw::Sprite { rect, .. } => rect,
                _ => unreachable!(),
            })
            .collect()
    };
    let cell = |picker: &Picker, row: usize| {
        grid.cells(picker.index(), picker.entries().len(), 0.0)[2 * 7 + row].rect
    };
    let row_of = |picker: &Picker| {
        let Details::Ship { models, .. } = &picker.entries()[picker.index()].details else {
            unreachable!()
        };
        models
            .iter()
            .position(|model| model.variant == Some(picker.variant_index()))
            .expect("the chosen livery has a row")
    };
    let mut picker = picker_of(&["A", "B", "C"], "B");
    picker.tick(1.0);
    let mut input = Input::default();
    let mut step = |picker: &mut Picker, button: Button| {
        input.begin_frame(0);
        input.begin_frame(1 << button as u32);
        picker.update(&mut input);
        // Long enough for the slide to end and the ring to reach its row.
        for _ in 0..60 {
            picker.tick(1.0 / 60.0);
        }
    };
    for (button, name) in [
        (None, "the opening livery"),
        (Some(Button::Down), "the second variant"),
        (Some(Button::Right), "the next team, same variant"),
        (Some(Button::Up), "back to the first variant"),
    ] {
        if let Some(button) = button {
            step(&mut picker, button);
        }
        let drawn = ring(&picker);
        assert_eq!(drawn.len(), 1, "{name}: one quad");
        let [x, y, w, h] = drawn[0];
        assert_eq!((w, h), (92.0, 80.0), "{name}");
        let at = cell(&picker, row_of(&picker));
        assert!(
            ((x + w * 0.5) - (at[0] + at[2] * 0.5)).abs() < 1e-3
                && ((y + h * 0.5) - (at[1] + at[3] * 0.5)).abs() < 1e-3,
            "{name}: centred on the chosen cell"
        );
    }
}

/// The ring pulses with the widget's own clock from the grid's `ring_peak`: white at phase 0,
/// the peak a third of a second in, black at two thirds, then a cut to white; opaque throughout.
#[test]
fn the_cursor_ring_pulses_white_to_the_peak_to_black() {
    use super::hex::{pulse_argb, pulse_phase};
    let peak = 0xffac_0717;
    assert_eq!(pulse_argb(peak, pulse_phase(0.0)), 0xffff_ffff);
    assert_eq!(pulse_argb(peak, pulse_phase(1.0 / 3.0)), peak);
    assert_eq!(pulse_argb(peak, 2.0), 0xff00_0000);
    assert!(
        pulse_phase(2.0 / 3.0 + 1.0 / 60.0) < 0.1,
        "wraps back to white"
    );
    // Per byte, truncated: a sixth of a second in is half way from white to the peak.
    assert_eq!(pulse_argb(peak, 0.5), 0xff_d5_83_8b);
    // The drawn ring takes the colour from the picker's clock.
    let layout = layout();
    let extra = layout.hd.as_deref().unwrap();
    let ring_colour = |seconds: f32| {
        let mut picker = picker_of(&["A", "B", "C"], "B");
        picker.tick(seconds);
        body(&picker, &layout, extra, &Frame::default(), &|n| sheet(n))
            .into_iter()
            .rev()
            .find_map(|draw| match draw {
                Draw::Sprite { color, .. } => Some(color),
                _ => None,
            })
            .unwrap()
    };
    assert_eq!(ring_colour(0.0), [1.0; 4]);
    assert_eq!(ring_colour(1.0 / 3.0), oag_ui::screen::argb_to_rgba(peak));
}

/// A title's own peak replaces `HD_Blue` (Omega computes its red inline), and a grid with no peak
/// draws no ring rather than a stand-in.
#[test]
fn the_cursor_ring_takes_the_grids_peak() {
    let mut layout = layout();
    let red = 0xffee_0707;
    layout
        .hd
        .as_deref_mut()
        .unwrap()
        .hex
        .as_mut()
        .unwrap()
        .ring_peak = Some(red);
    let extra = layout.hd.as_deref().unwrap();
    let mut picker = picker_of(&["A", "B", "C"], "B");
    picker.tick(1.0 / 3.0);
    let quads = |layout: &Layout| {
        let extra = layout.hd.as_deref().unwrap();
        body(&picker, layout, extra, &Frame::default(), &|n| sheet(n))
            .into_iter()
            .filter(|draw| matches!(draw, Draw::Sprite { color, .. } if *color == oag_ui::screen::argb_to_rgba(red)))
            .count()
    };
    assert!(extra.hex.is_some());
    assert!(quads(&layout) >= 1);
    layout
        .hd
        .as_deref_mut()
        .unwrap()
        .hex
        .as_mut()
        .unwrap()
        .ring_peak = None;
    assert_eq!(quads(&layout), 0);
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
    let cells = grid.cells(0, 3, 0.0);
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
    let cell = grid.cells(0, 3, 0.0)[3 * 7 + 2];
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
    // Once the slide is over, the same spot is the centre column's `concept1`: already chosen.
    for _ in 0..8 {
        picker.tick(1.0 / 60.0);
    }
    let centre = grid.cells(1, 3, 0.0)[2 * 7 + 2];
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

const RED: [f32; 4] = [172.0 / 255.0, 7.0 / 255.0, 23.0 / 255.0, 1.0];

/// The `Fill`s of the stat bar's red, with the block art a served archive
/// resolves.
fn red_fills(picker: &Picker, extra: &TeamScreen, layout: &Layout) -> Vec<[f32; 4]> {
    let placed = |x: u32, width: u32, height: u32| oag_ui::frontend::Placed {
        x,
        y: 0,
        width,
        height,
        quad_extent: None,
        quad_centre: None,
        blend: None,
        scale: [1.0, 1.0],
    };
    let frame = Frame {
        blocks: Some(oag_ui::menu::block::BlockArt {
            frame: placed(0, 64, 64),
            fill_alpha: 110.0 / 255.0,
            solid_alpha: 1.0,
            cursor: None,
            arrow: None,
            marker: None,
            fury: true,
        }),
        ..Frame::default()
    };
    body(picker, layout, extra, &frame, &|_| None)
        .into_iter()
        .filter_map(|draw| match draw {
            Draw::Fill { rect, color } if color == RED => Some(rect),
            _ => None,
        })
        .collect()
}

fn variant_picker(variant: &str) -> Picker {
    let stats = |speed| Stats {
        speed,
        thrust: 80,
        handling: 100,
        shield: 80,
        length_milli: Some(3800),
    };
    Picker::new(
        Kind::Ship,
        vec![team("Feisar", vec![Some(stats(70)), Some(stats(80))])],
        None,
        Some(variant),
    )
}

#[test]
fn the_bonus_colour_is_the_fronts_hd_blue() {
    let layout = layout();
    assert_eq!(layout.hd.as_deref().unwrap().bonus, Some(0xffac_0717));
}

/// Fury Concept's 8.0 speed over the classic hull's 7.0: the block's
/// `0.70..0.80` is red, and only that.
#[test]
fn a_variants_added_rating_is_drawn_in_the_bonus_colour() {
    let layout = layout();
    let extra = layout.hd.as_deref().unwrap();
    let fills = red_fills(&variant_picker("_c1"), extra, &layout);
    assert_eq!(fills.len(), 1, "{fills:?}");
    let block = find_block(&layout.screen, "Slide_0").unwrap();
    let [x, _, width, _] = fills[0];
    assert!((x - (block.x + block.width * 0.70)).abs() < 0.01, "{x}");
    assert!((width - block.width * 0.10).abs() < 0.01, "{width}");
}

#[test]
fn the_classic_hull_and_a_missing_colour_draw_no_red() {
    let layout = layout();
    let extra = layout.hd.as_deref().unwrap();
    assert!(red_fills(&variant_picker(""), extra, &layout).is_empty());
    let mut bare = extra.clone();
    bare.bonus = None;
    assert!(red_fills(&variant_picker("_c1"), &bare, &layout).is_empty());
}

/// A drag of `dx` units of *pan* (grid units the columns move); the finger
/// travels `dx / DRAG_GAIN`, since the hex grids follow only part of it.
fn drag_by(dx: f32) -> oag_ui::pointer::Pointer {
    oag_ui::pointer::Pointer {
        drag: (dx / crate::picker::pointer::DRAG_GAIN, 0.0),
        ..oag_ui::pointer::Pointer::default()
    }
}

/// A finger landing on a tile, as a touch screen reports it.
fn touch_down(found: &[crate::picker::pointer::Target]) -> oag_ui::pointer::Pointer {
    press_at(tile_and_backdrop(found).0)
}

fn lift() -> oag_ui::pointer::Pointer {
    oag_ui::pointer::Pointer {
        released: true,
        ..oag_ui::pointer::Pointer::default()
    }
}

/// Runs `ticks` ticks the way the front end does - the clock, then the pad
/// with nothing pressed - and returns what the coast stepped.
fn settle(picker: &mut Picker, ticks: usize) -> Vec<Event> {
    let mut out = Vec::new();
    let mut input = Input::default();
    for _ in 0..ticks {
        picker.tick(1.0 / 60.0);
        input.begin_frame(0);
        out.extend(picker.update(&mut input));
    }
    out
}

fn pan_targets(picker: &Picker) -> Vec<crate::picker::pointer::Target> {
    let skin = oag_ui::menu::Skin::new(
        oag_pulse::FRONT_END.menu.unwrap(),
        oag_display::space::Space::PSP,
        22.0,
    );
    targets(picker, &layout(), &skin, &|_| None)
}

const PITCH_X: f32 = hex::PITCH[0];

#[test]
fn a_drag_past_a_column_steps_the_team_and_less_does_not() {
    let mut picker = picker_of(&["A", "B", "C"], "B");
    let found = pan_targets(&picker);
    assert!(picker.pointer(&drag_by(PITCH_X * 0.6), &found).is_empty());
    assert_eq!(picker.index(), 1);
    assert!((picker.pan() - 0.6).abs() < 1e-4);
    // Content follows the finger: dragging right brings the left column,
    // the previous team, to the centre.
    assert_eq!(
        picker.pointer(&drag_by(PITCH_X * 0.6), &found),
        vec![Event::Moved]
    );
    assert_eq!(picker.index(), 0);
    assert!((picker.pan() - 0.2).abs() < 1e-4);
    assert_eq!(
        picker.pointer(&drag_by(-PITCH_X * 1.5), &found),
        vec![Event::Moved]
    );
    assert_eq!(picker.index(), 1);
}

#[test]
fn a_long_drag_steps_several_columns_and_wraps_at_both_ends() {
    let mut picker = picker_of(&["A", "B", "C"], "A");
    let found = pan_targets(&picker);
    let events = picker.pointer(&drag_by(PITCH_X * 1.1), &found);
    assert_eq!((events, picker.index()), (vec![Event::Moved], 2));
    let events = picker.pointer(&drag_by(-PITCH_X * 2.3), &found);
    assert_eq!(events.len(), 2);
    assert_eq!(
        picker.index(),
        1,
        "left past the last team wraps to the first"
    );
}

#[test]
fn a_drag_never_clicks_and_the_pad_and_vertical_drags_are_untouched() {
    let mut picker = picker_of(&["A", "B", "C"], "B");
    let found = pan_targets(&picker);
    let vertical = oag_ui::pointer::Pointer {
        drag: (0.0, 90.0),
        ..oag_ui::pointer::Pointer::default()
    };
    assert!(picker.pointer(&vertical, &found).is_empty());
    assert_eq!((picker.index(), picker.pan()), (1, 0.0));
    assert!(
        !picker
            .pointer(&drag_by(PITCH_X * 2.0), &found)
            .contains(&Event::Confirmed)
    );
    let mut input = Input::default();
    input.begin_frame(0);
    input.begin_frame(1 << Button::Right as u32);
    assert_eq!(picker.update(&mut input), vec![Event::Moved]);
}

#[test]
fn the_grid_follows_the_finger_and_settles_on_the_nearest_column() {
    let layout = layout();
    let grid = layout.hd.as_deref().unwrap().hex.unwrap();
    let rest = grid.cells(1, 3, 0.0);
    let moved = grid.cells(1, 3, 0.4);
    let (a, b) = (
        rest.iter().find(|c| c.distance == 0 && c.row == 2).unwrap(),
        moved
            .iter()
            .find(|c| c.distance == 0 && c.row == 2)
            .unwrap(),
    );
    assert!((b.rect[0] - a.rect[0] - 0.4 * PITCH_X).abs() < 1e-3);
    assert!(moved.len() > rest.len(), "a column slides in at the left");
    assert!(moved.iter().any(|c| c.weight < 1.0));
    assert!(rest.iter().all(|c| c.weight == 1.0));
    // Held, the grid stays where the finger put it however long it waits.
    let mut picker = picker_of(&["A", "B", "C"], "B");
    let found = pan_targets(&picker);
    picker.pointer(&touch_down(&found), &found);
    picker.pointer(&drag_by(PITCH_X * 0.4), &found);
    settle(&mut picker, 60);
    assert!((picker.pan() - 0.4).abs() < 1e-4, "{}", picker.pan());
    // Lifted short of half a column, it settles back onto its own.
    picker.pointer(&lift(), &found);
    assert!(settle(&mut picker, 60).is_empty());
    assert_eq!((picker.pan(), picker.index()), (0.0, 1));
    // Lifted past half a column, it settles onto the neighbour, which is
    // then the selection - the pad's own step.
    picker.pointer(&touch_down(&found), &found);
    picker.pointer(&drag_by(PITCH_X * 0.55), &found);
    settle(&mut picker, 30);
    picker.pointer(&lift(), &found);
    assert_eq!(settle(&mut picker, 60), vec![Event::Moved]);
    assert_eq!((picker.pan(), picker.index()), (0.0, 0));
}

#[test]
fn a_flick_coasts_across_columns_stepping_each_and_rests_on_one() {
    let mut picker = picker_of(&["A", "B", "C", "D", "E", "F", "G", "H"], "A");
    let found = pan_targets(&picker);
    picker.pointer(&touch_down(&found), &found);
    // A quarter column a tick leftward: 15 columns a second.
    let mut stepped = Vec::new();
    for _ in 0..6 {
        stepped.extend(picker.pointer(&drag_by(-PITCH_X * 0.25), &found));
        stepped.extend(settle(&mut picker, 1));
    }
    picker.pointer(&lift(), &found);
    let coasted = settle(&mut picker, 240);
    assert_eq!(picker.pan(), 0.0, "at rest on a column");
    assert!(
        coasted.len() >= 4,
        "it kept going after the lift: {coasted:?}"
    );
    assert!(coasted.iter().all(|e| *e == Event::Moved));
    let total = stepped.len() + coasted.len();
    assert_eq!(picker.index(), total % 8, "one step per column passed");
}

#[test]
fn a_tap_on_a_coasting_grid_stops_it_and_selects_nothing() {
    let mut picker = picker_of(&["A", "B", "C", "D", "E", "F", "G", "H"], "A");
    let found = pan_targets(&picker);
    picker.pointer(&touch_down(&found), &found);
    for _ in 0..6 {
        picker.pointer(&drag_by(-PITCH_X * 0.25), &found);
        settle(&mut picker, 1);
    }
    picker.pointer(&lift(), &found);
    settle(&mut picker, 4);
    let index = picker.index();
    // A tap, landing on a cell, while it moves.
    let found = pan_targets(&picker);
    let cell = found
        .iter()
        .find(|t| matches!(t.what, What::Cell { .. }))
        .unwrap();
    let at = (
        cell.rect[0] + cell.rect[2] * 0.5,
        cell.rect[1] + cell.rect[3] * 0.5,
    );
    let tap = oag_ui::pointer::Pointer {
        at: Some(at),
        moved: true,
        clicked: true,
        pressed: true,
        released: true,
        ..oag_ui::pointer::Pointer::default()
    };
    let events = picker.pointer(&tap, &found);
    assert!(events.is_empty(), "{events:?}");
    let more = settle(&mut picker, 60);
    assert!(more.len() <= 1, "it stopped, then settled: {more:?}");
    assert_eq!(picker.pan(), 0.0);
    assert!(picker.index().abs_diff(index) <= 1);
}

fn press_at(at: (f32, f32)) -> oag_ui::pointer::Pointer {
    oag_ui::pointer::Pointer {
        pressed: true,
        press_at: Some(at),
        ..oag_ui::pointer::Pointer::default()
    }
}

/// A point inside a hex tile, and one on no tile at all.
fn tile_and_backdrop(found: &[crate::picker::pointer::Target]) -> ((f32, f32), (f32, f32)) {
    use crate::picker::pointer::What;
    let cell = found
        .iter()
        .find(|target| matches!(target.what, What::Cell { .. }))
        .unwrap();
    let [x, y, w, h] = cell.rect;
    let tile = (x + w / 2.0, y + h / 2.0);
    let backdrop = (0.0, 0.0);
    assert!(oag_ui::pointer::hex_contains(cell.rect, tile));
    assert!(
        !found
            .iter()
            .any(|target| matches!(target.what, What::Cell { .. })
                && oag_ui::pointer::hex_contains(target.rect, backdrop))
    );
    (tile, backdrop)
}

#[test]
fn a_drag_that_starts_off_the_tiles_does_not_pan_or_step() {
    let mut picker = picker_of(&["A", "B", "C"], "B");
    let found = pan_targets(&picker);
    let (_, backdrop) = tile_and_backdrop(&found);
    assert!(picker.pointer(&press_at(backdrop), &found).is_empty());
    assert!(picker.pointer(&drag_by(PITCH_X * 2.0), &found).is_empty());
    assert!(picker.pointer(&drag_by(-PITCH_X * 3.0), &found).is_empty());
    assert!(picker.pointer(&lift(), &found).is_empty());
    assert_eq!((picker.index(), picker.pan()), (1, 0.0));
    assert!(settle(&mut picker, 120).is_empty());
    assert_eq!(picker.index(), 1);
}

#[test]
fn a_drag_that_starts_on_a_tile_pans_and_steps_as_before() {
    let mut picker = picker_of(&["A", "B", "C"], "B");
    let found = pan_targets(&picker);
    let (tile, _) = tile_and_backdrop(&found);
    picker.pointer(&press_at(tile), &found);
    assert!(picker.pointer(&drag_by(PITCH_X * 0.4), &found).is_empty());
    assert!((picker.pan() - 0.4).abs() < 1e-4);
    assert_eq!(
        picker.pointer(&drag_by(PITCH_X * 0.4), &found),
        vec![Event::Moved]
    );
    assert_eq!(picker.index(), 0);
}

#[test]
fn a_drag_that_started_on_a_tile_keeps_panning_off_the_grid_and_the_next_press_decides_again() {
    let mut picker = picker_of(&["A", "B", "C"], "B");
    let found = pan_targets(&picker);
    let (tile, backdrop) = tile_and_backdrop(&found);
    picker.pointer(&press_at(tile), &found);
    // The finger has left the grid: no pointer position, only travel.
    for _ in 0..3 {
        picker.pointer(&drag_by(PITCH_X * 0.5), &found);
    }
    assert_ne!(picker.index(), 1, "1.5 columns right left the first column");
    picker.pointer(&lift(), &found);
    settle(&mut picker, 120);
    let at_rest = picker.index();
    picker.pointer(&press_at(backdrop), &found);
    assert!(picker.pointer(&drag_by(PITCH_X * 2.0), &found).is_empty());
    assert_eq!(picker.index(), at_rest);
}

/// Runs one finger swipe of `per_tick` finger units a tick for `ticks` ticks,
/// lifts, and returns the columns the grid travelled in all.
fn swipe_travel(per_tick: f32, ticks: usize) -> usize {
    let mut picker = picker_of(&["A", "B", "C", "D", "E", "F", "G", "H"], "A");
    let found = pan_targets(&picker);
    picker.pointer(&touch_down(&found), &found);
    let mut steps = 0;
    for _ in 0..ticks {
        let drag = oag_ui::pointer::Pointer {
            drag: (-per_tick, 0.0),
            ..oag_ui::pointer::Pointer::default()
        };
        steps += picker.pointer(&drag, &found).len();
        steps += settle(&mut picker, 1).len();
    }
    picker.pointer(&lift(), &found);
    steps + settle(&mut picker, 600).len()
}

#[test]
fn the_same_finger_travel_moves_fewer_columns_than_it_did_at_one_to_one() {
    use crate::picker::pointer::DRAG_GAIN;
    // Two columns of finger at the old 1:1 mapping, held (no flick: it
    // stops moving before the lift).
    let mut picker = picker_of(&["A", "B", "C", "D", "E", "F", "G", "H"], "A");
    let found = pan_targets(&picker);
    let events = picker.pointer(
        &oag_ui::pointer::Pointer {
            drag: (PITCH_X * 2.0, 0.0),
            ..oag_ui::pointer::Pointer::default()
        },
        &found,
    );
    // 2 columns of finger is 1.2 of pan: one step with 0.2 left over, where
    // 1:1 stepped twice and rested on the column.
    assert_eq!(events.len(), 1, "{events:?}");
    assert!((picker.pan() - 2.0 * DRAG_GAIN + 1.0).abs() < 1e-4);
}

#[test]
fn a_given_flick_coasts_fewer_columns_than_it_did() {
    // 6 columns a second of finger at 1:1 (0.1 column a tick): the old
    // model passed 3 more columns after the lift (6 / FRICTION 2.0).
    let per_tick = PITCH_X * 0.1;
    let total = swipe_travel(per_tick, 10);
    let old_total = 1 + 3; // 10 ticks x 0.1 = 1 column held, then 3 coasted
    assert!(
        total < old_total,
        "{total} columns against the old {old_total}"
    );
    // A hard swipe is capped well under the shared 24 columns a second:
    // at most max_fling / friction past the lift.
    let hard = swipe_travel(PITCH_X * 2.0, 8);
    let held = (8.0 * 2.0 * crate::picker::pointer::DRAG_GAIN).round() as usize;
    let cap = (crate::picker::pointer::HEX_MAX_FLING / crate::picker::pointer::HEX_FRICTION).ceil()
        as usize;
    assert!(hard <= held + cap + 1, "{hard} > {held} + {cap}");
}

#[test]
fn the_shared_kinetic_defaults_are_unchanged() {
    use oag_ui::kinetic::{Extent, FRICTION, MAX_FLING};
    let (rows, wrap) = (Extent::rows(10.0), Extent::wrapping());
    for extent in [rows, wrap] {
        assert_eq!((extent.max_fling, extent.friction), (MAX_FLING, FRICTION));
    }
    assert_eq!((MAX_FLING, FRICTION), (24.0, 2.0));
    let hex = crate::picker::pointer::hex_extent();
    assert!(hex.max_fling < MAX_FLING && hex.friction > FRICTION);
}

/// A honeycomb authored at `Scale="0.8"` (Omega's, nine rows) shrinks pitch and cell alike,
/// and a grid with no scale is HD's, unchanged.
#[test]
fn a_scaled_honeycomb_shrinks_its_pitch_and_its_cells() {
    let full = hex::HexGrid::unauthored([100.0, 200.0], 5, 7);
    let small = hex::HexGrid { scale: 0.8, ..full };
    let (a, b) = (full.cells(0, 5, 0.0), small.cells(0, 5, 0.0));
    assert_eq!(a.len(), b.len());
    // The second row of the first column: one pitch below the first.
    let pitch = |cells: &[hex::Cell]| cells[1].rect[1] - cells[0].rect[1];
    assert!((pitch(&b) - 0.8 * pitch(&a)).abs() < 1e-3);
    assert!((b[0].rect[2] - 0.8 * a[0].rect[2]).abs() < 1e-3);
}

/// `Team Selection`'s columns alternate their half-row shift with the team's index, not with
/// their slot in the window: the centre column of an odd team sits half a row (42 units) lower
/// than an even team's, and a grid built by hand keeps the window's slots (`track::read` turns the
/// index phase on for `TrackHexSelection`'s).
#[test]
fn the_team_grids_half_row_follows_the_team_index() {
    let layout = layout();
    let grid = layout
        .hd
        .as_deref()
        .unwrap()
        .hex
        .expect("the team screen authors a honeycomb");
    assert!(grid.phase_by_entry);
    let centre_y = |grid: &hex::HexGrid, selected: usize| {
        let cells = grid.cells(selected, 12, 0.0);
        cells
            .iter()
            .find(|c| c.offset == 0 && c.row == 0)
            .unwrap()
            .rect[1]
    };
    let half = hex::HALF_ROW * grid.scale;
    assert_eq!(hex::HALF_ROW, 42.0);
    assert_eq!(hex::PITCH, [72.0, 83.0]);
    assert!((centre_y(&grid, 1) - centre_y(&grid, 0) - half).abs() < 1e-3);
    assert!((centre_y(&grid, 2) - centre_y(&grid, 0)).abs() < 1e-3);
    // A neighbour keeps its team's phase: team 1's column is lowered next to team 0 selected.
    let right = |g: &hex::HexGrid| {
        g.cells(0, 12, 0.0)
            .iter()
            .find(|c| c.offset == 1 && c.row == 0)
            .unwrap()
            .rect[1]
    };
    assert!((right(&grid) - centre_y(&grid, 0) - half).abs() < 1e-3);
    let track = hex::HexGrid::unauthored([100.0, 200.0], 5, 7);
    assert!(!track.phase_by_entry);
    assert!((centre_y(&track, 1) - centre_y(&track, 0)).abs() < 1e-3);
}

/// A forward step slides every column 72 units left, linearly over six ticks, each column keeping
/// the half-row shift of the window it came from, with the spare column bringing the incoming team
/// in from the right; at the sixth tick the columns are back in their slots with the new window.
/// Frame 0 is the tick the step lands on.
#[test]
fn a_forward_step_slides_the_lattice_a_pitch_and_then_snaps() {
    let layout = layout();
    let grid = layout.hd.as_deref().unwrap().hex.unwrap();
    let mut picker = picker_of(&["A", "B", "C", "D", "E", "F"], "C");
    picker.tick(1.0);
    let mut input = Input::default();
    input.begin_frame(0);
    input.begin_frame(1 << Button::Right as u32);
    picker.update(&mut input);
    let frame = |picker: &Picker| {
        grid.cells_at(
            picker.index(),
            picker.entries().len(),
            0.0,
            Some(picker.slide()),
        )
    };
    let column = |cells: &[hex::Cell], entry: usize| {
        let cell = cells
            .iter()
            .find(|c| c.entry == entry && c.row == 0)
            .unwrap();
        [
            cell.rect[0] + cell.rect[2] * 0.5,
            cell.rect[1] + cell.rect[3] * 0.5,
        ]
    };
    picker.tick(1.0 / 60.0);
    let at_rest = grid.cells(2, 6, 0.0);
    let start = frame(&picker);
    for entry in [0, 1, 2, 3, 4] {
        assert_eq!(
            column(&start, entry),
            column(&at_rest, entry),
            "frame 0, team {entry}"
        );
    }
    assert_eq!(
        start.len(),
        6 * 7,
        "the spare column is there from the first frame"
    );
    for _ in 0..3 {
        picker.tick(1.0 / 60.0);
    }
    let mid = frame(&picker);
    for entry in [0, 1, 2, 3, 4, 5] {
        let (before, now) = (column(&start, entry), column(&mid, entry));
        assert!(
            (before[0] - now[0] - 36.0).abs() < 1e-2,
            "frame 3, team {entry}: half a pitch left"
        );
        assert_eq!(
            before[1], now[1],
            "frame 3, team {entry}: no change in height"
        );
    }
    let spare = column(&mid, 5);
    assert!((spare[0] - (grid.origin[0] + 5.0 * hex::PITCH[0] * grid.scale - 36.0)).abs() < 1e-2);
    for _ in 0..3 {
        picker.tick(1.0 / 60.0);
    }
    let snapped = frame(&picker);
    assert_eq!(snapped.len(), 5 * 7, "frame 6: the spare column is gone");
    let settled = grid.cells(3, 6, 0.0);
    for entry in [1, 2, 3, 4, 5] {
        assert_eq!(
            column(&snapped, entry),
            column(&settled, entry),
            "frame 6, team {entry}"
        );
    }
}

/// The ring stays on the centre column and only its height eases: a step onto an odd team (whose
/// column sits 42 units lower) leaves it 42 units above the new row on the frame of the step, and
/// half of what is left is gone with every tick after.
#[test]
fn the_ring_eases_half_a_row_and_never_moves_sideways() {
    let layout = layout();
    let extra = layout.hd.as_deref().unwrap();
    let mut picker = picker_of(&["A", "B", "C", "D", "E", "F"], "C");
    picker.tick(1.0);
    let ring = |picker: &Picker| {
        body(picker, &layout, extra, &Frame::default(), &|n| sheet(n))
            .into_iter()
            .rev()
            .find_map(|draw| match draw {
                Draw::Sprite { rect, .. } if rect[2] == 92.0 => Some(rect),
                _ => None,
            })
            .unwrap()
    };
    let rest = ring(&picker);
    let mut input = Input::default();
    input.begin_frame(0);
    input.begin_frame(1 << Button::Right as u32);
    picker.update(&mut input);
    picker.tick(1.0 / 60.0);
    // Team D (index 3) is odd: its column is 42 lower than C's, and the ring starts 42 above it.
    let settled_y = rest[1] + 42.0;
    for (frame, left) in [(0, 42.0), (1, 21.0), (2, 10.5), (3, 5.25)] {
        let at = ring(&picker);
        assert_eq!(at[0], rest[0], "frame {frame}: x never moves");
        assert!(
            (settled_y - at[1] - left).abs() < 1e-3,
            "frame {frame}: {} above",
            settled_y - at[1]
        );
        picker.tick(1.0 / 60.0);
    }
}

fn art(src: &str) -> Option<Placed> {
    let (x, size) = match src {
        crate::campaign::hd::cell_brackets::SRC => (100, 16),
        BLOCK_ARROW_SRC => (200, 12),
        _ => return None,
    };
    Some(Placed {
        x,
        y: 0,
        width: size,
        height: size,
        quad_extent: None,
        quad_centre: None,
        blend: None,
        scale: [1.0, 1.0],
    })
}

#[test]
fn the_authored_corner_marks_and_block_arrow_are_drawn() {
    let layout = layout();
    let picker = Picker::new(Kind::Ship, vec![team("Feisar", Vec::new())], None, None);
    let extra = layout.hd.as_deref().unwrap();
    let draws = body(&picker, &layout, extra, &Frame::default(), &art);
    let from = |x: f32| {
        draws
            .iter()
            .filter(|draw| matches!(draw, Draw::Sprite { uv, .. } if uv[0].round() == x || (uv[0] - 16.0).round() == x))
            .count()
    };
    // Three `corner="true"` brackets, four marks each; one arrow.
    assert_eq!(from(100.0), 12, "{draws:?}");
    assert_eq!(from(200.0), 1, "{draws:?}");
    let none = body(&picker, &layout, extra, &Frame::default(), &|_| None);
    assert!(none.iter().all(|draw| !matches!(draw, Draw::Sprite { .. })));
}

const LOGO: &str = "Data/Ships/A/FE/Logo.gtf";

fn logo_sprites(name: &str) -> Option<Placed> {
    (name == LOGO).then_some(Placed {
        x: 1000,
        y: 0,
        width: 512,
        height: 128,
        quad_extent: None,
        quad_centre: None,
        blend: None,
        scale: [1.0, 1.0],
    })
}

fn logo_picker() -> Picker {
    let mut entries = vec![hex_team("A"), hex_team("B")];
    for entry in &mut entries {
        let Details::Ship { logo, .. } = &mut entry.details else {
            unreachable!()
        };
        *logo = Some(LOGO.to_string());
    }
    Picker::new(Kind::Ship, entries, Some("A"), None).with_entries_across()
}

/// The Logo's rect, `None` when the page draws no logo quad.
fn logo_rect_of(picker: &Picker, layout: &Layout) -> Option<[f32; 4]> {
    let extra = layout.hd.as_deref().unwrap();
    let draws = body(picker, layout, extra, &Frame::default(), &logo_sprites);
    let rects: Vec<[f32; 4]> = draws
        .iter()
        .filter_map(|draw| match draw {
            Draw::Sprite { rect, uv, .. } if uv[0] >= 1000.0 => Some(*rect),
            _ => None,
        })
        .collect();
    assert!(rects.len() <= 1, "{draws:?}");
    rects.first().copied()
}

/// A page opened draws no logo for the authored `delay` (0.5 s, 30 frames: `Item 315`'s
/// countdown in RAM crosses 0 on its delay's frame), then plays the 0.7 s ramp
/// (`image-widget.md`, "Page entry"). A still that states the page's wipes finished draws it
/// whole. This fails if the delay or the entry clock is dropped.
#[test]
fn a_fresh_screen_waits_the_logos_delay_then_wipes() {
    let layout = layout();
    let mut picker = logo_picker();
    let at = |picker: &Picker| logo_rect_of(picker, &layout);
    assert_eq!(at(&picker), None, "frame 0 of the page");
    for _ in 0..29 {
        picker.tick(1.0 / 60.0);
    }
    assert_eq!(at(&picker), None, "frame 29, the last of the delay");
    picker.tick(1.0 / 60.0);
    let first = at(&picker).expect("frame 30 is the first ramp frame");
    assert!(
        (first[3] - 12.8).abs() < 1e-3,
        "{first:?}: a slab a tenth tall"
    );
    assert!(first[1] < 185.0 + 128.0 * 0.1, "{first:?}: near the top");
    for _ in 0..42 {
        picker.tick(1.0 / 60.0);
    }
    assert_eq!(
        at(&picker),
        Some([160.0, 185.0, 512.0, 128.0]),
        "the ramp is over"
    );

    let mut settled = logo_picker();
    settled.set_entry(oag_ui::screen::EntryClock::finished());
    assert_eq!(at(&settled), Some([160.0, 185.0, 512.0, 128.0]));
}

/// A team step replays the `Logo`'s `vwipe` from the press
/// (`docs/ghidra/functions/ps3-hdfury-eu/image-widget.md`, "On a step"): a slab a tenth of the
/// quad tall at the top, then lower, back up, and the finished logo after 0.7 s. The step does
/// not wait the delay again. The screen's opening and a settled screen draw it whole.
#[test]
fn a_team_step_replays_the_logos_vwipe() {
    let layout = layout();
    let extra = layout.hd.as_deref().unwrap();
    let sprites = logo_sprites;
    let mut picker = logo_picker();
    let logo_rect = |picker: &Picker| {
        let draws = body(picker, &layout, extra, &Frame::default(), &sprites);
        let rects: Vec<[f32; 4]> = draws
            .iter()
            .filter_map(|draw| match draw {
                Draw::Sprite { rect, uv, .. } if uv[0] >= 1000.0 => Some(*rect),
                _ => None,
            })
            .collect();
        assert_eq!(rects.len(), 1, "{draws:?}");
        rects[0]
    };
    let whole = [160.0, 185.0, 512.0, 128.0];
    picker.tick(1.3);
    assert_eq!(logo_rect(&picker), whole, "no step yet");

    let mut input = Input::default();
    input.begin_frame(0);
    input.begin_frame(1 << Button::Right as u32);
    assert_eq!(picker.update(&mut input), vec![Event::Moved]);
    let close = |got: [f32; 4], want: [f32; 4]| {
        assert!(
            got.iter().zip(want).all(|(a, b)| (a - b).abs() < 1e-3),
            "{got:?} vs {want:?}"
        );
    };
    close(logo_rect(&picker), [160.0, 185.0, 512.0, 12.8]);
    picker.tick(0.35);
    close(
        logo_rect(&picker),
        [160.0, 185.0 + 128.0 * 0.675, 512.0, 12.8],
    );
    picker.tick(0.175);
    close(
        logo_rect(&picker),
        [160.0, 185.0 + 128.0 * 0.1125, 512.0, 12.8],
    );
    picker.tick(0.105);
    close(logo_rect(&picker), [160.0, 185.0, 512.0, 128.0 * 0.55]);
    picker.tick(0.35);
    assert_eq!(logo_rect(&picker), whole, "the ramp is over after 0.7 s");
}

/// The corner marks the page has out, as the top-left corners of the 16 by 16 sprites, with the
/// three brackets' `delay`s authored as the disc does (0.2, 0.2, 0.9 s).
fn corner_marks(picker: &Picker) -> Vec<[f32; 2]> {
    let mut delays = ["0.2", "0.2", "0.90"].into_iter();
    let xml = XML
        .split("<Bracket>")
        .enumerate()
        .map(|(index, part)| match index {
            0 => part.to_string(),
            _ => format!(
                "<Bracket delay=\"{}\">{part}",
                delays.next().expect("three brackets")
            ),
        })
        .collect::<String>();
    let layout = layout_of(&xml);
    let extra = layout.hd.as_deref().unwrap();
    let corners = |name: &str| {
        name.ends_with("corner2.gtf").then_some(Placed {
            x: 0,
            y: 0,
            width: 16,
            height: 16,
            quad_extent: None,
            quad_centre: None,
            blend: None,
            scale: [1.0, 1.0],
        })
    };
    body(picker, &layout, extra, &Frame::default(), &corners)
        .iter()
        .filter_map(|draw| match draw {
            Draw::Sprite { rect, .. } if rect[2] == 16.0 && rect[3] == 16.0 => {
                Some([rect[0], rect[1]])
            }
            _ => None,
        })
        .collect()
}

/// The three `Bracket`s count their own `delay` from the page's entry: none at first, the two panel
/// frames (0.2 s) and then the ship frame (0.9 s), each opening out over 0.1 s. A team step
/// re-arms only the `Logo`, so the marks stay out. This fails if the entry clock is not passed to
/// the brackets (they would all be out at frame 0).
#[test]
fn the_brackets_wait_their_delays_from_the_pages_entry() {
    let mut picker = picker_of(&["A", "B", "C"], "B");
    assert!(corner_marks(&picker).is_empty(), "frame 0");
    picker.tick(0.19);
    assert!(corner_marks(&picker).is_empty(), "inside the 0.2 s delay");
    picker.tick(0.05);
    assert_eq!(corner_marks(&picker).len(), 8, "the two panel frames open");
    picker.tick(0.5);
    assert_eq!(
        corner_marks(&picker).len(),
        8,
        "the ship frame is still waiting"
    );
    picker.tick(0.2);
    assert_eq!(
        corner_marks(&picker).len(),
        12,
        "the ship frame (0.9 s) is out"
    );
    let mut input = Input::default();
    input.begin_frame(0);
    input.begin_frame(1 << Button::Right as u32);
    picker.update(&mut input);
    assert_eq!(
        corner_marks(&picker).len(),
        12,
        "a team step re-arms no bracket"
    );
}
