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
        Some(oag_ui::screen::Vwipe { seconds: 0.7 }),
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

/// With a fade the hexagon fills are shaded across each cell and ease with the column's
/// distance from the selected one: brighter toward it, on both sides, and dimmer the further
/// out. Without one (the layout as read) they stay the one authored colour.
#[test]
fn a_faded_grid_shades_each_fill_toward_the_selected_column() {
    let mut layout = layout();
    let picker = picker_of(&["A", "B", "C", "D", "E", "F"], "C");
    let flat = |layout: &Layout| {
        let extra = layout.hd.as_deref().unwrap();
        body(&picker, layout, extra, &Frame::default(), &|n| sheet(n))
            .iter()
            .any(|draw| matches!(draw, Draw::GradientSprite { .. }))
    };
    assert!(!flat(&layout), "a grid read off the disc fades nothing");
    layout.hd.as_deref_mut().unwrap().hex.as_mut().unwrap().fade = Some(oag_title::HexFade {
        columns: 2.22,
        span: Some(109.0),
    });
    let extra = layout.hd.as_deref().unwrap();
    let draws = body(&picker, &layout, extra, &Frame::default(), &|n| sheet(n));
    // Each cell's fill: (column centre, left alpha, right alpha), over the five columns.
    let mut cells: Vec<(f32, f32, f32)> = draws
        .iter()
        .filter_map(|draw| match draw {
            Draw::GradientSprite {
                rect, left, right, ..
            } => Some((rect[0], left[3], right[3])),
            _ => None,
        })
        .collect();
    assert_eq!(cells.len(), 28, "four fading columns of seven");
    cells.sort_by(|a, b| a.0.total_cmp(&b.0));
    let by_column: Vec<&[(f32, f32, f32)]> = cells.chunks(7).collect();
    let (outer_left, inner_left, inner_right, outer_right) = (
        by_column[0][0],
        by_column[1][0],
        by_column[2][0],
        by_column[3][0],
    );
    assert!(
        outer_left.2 > outer_left.1,
        "left of the selection, brighter toward it"
    );
    assert!(inner_left.2 > inner_left.1);
    assert!(
        inner_right.1 > inner_right.2,
        "right of it, brighter on its left"
    );
    assert!(outer_right.1 > outer_right.2);
    assert!(
        inner_left.1 > outer_left.1 && inner_right.1 > outer_right.1,
        "dimmer further out"
    );
    let authored = oag_ui::screen::argb_to_rgba(0x6480_8080)[3];
    assert!(
        inner_left.2 < authored,
        "the grid's own colour is the selected column's"
    );
}

/// The cursor ring: a band of the outline art, one copy per unit of its width, in `TEAM_RING`,
/// on the chosen livery's cell of the selected team's column only, and following the chosen
/// livery as the selection moves between variants and teams.
#[test]
fn the_cursor_ring_is_a_band_on_the_chosen_liveries_cell() {
    let layout = layout();
    let extra = layout.hd.as_deref().unwrap();
    let grid = extra.hex.unwrap();
    // The ring's copies, outermost first, in one frame's draws.
    let ring = |picker: &Picker| -> Vec<[f32; 4]> {
        body(picker, &layout, extra, &Frame::default(), &|n| sheet(n))
            .into_iter()
            .filter_map(|draw| match draw {
                Draw::Sprite { rect, color, .. } if color == super::hex::TEAM_RING => Some(rect),
                _ => None,
            })
            .collect()
    };
    // `row`'s cell in the selected column, which is the middle of five.
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
    let mut input = Input::default();
    let mut step = |picker: &mut Picker, button: Button| {
        input.begin_frame(0);
        input.begin_frame(1 << button as u32);
        picker.update(&mut input);
    };
    let (mut down, mut across) = (None, None);
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
        assert_eq!(
            drawn.len(),
            12,
            "{name}: six copies of the outline, two passes each"
        );
        assert!(
            drawn.chunks(2).all(|pass| pass[0] == pass[1]),
            "{name}: the passes agree"
        );
        let copies: Vec<[f32; 4]> = drawn.chunks(2).map(|pass| pass[0]).collect();
        // The outermost copy is on the chosen row of the selected column: its offset from that
        // row's cell is one constant for every row and team.
        let row = row_of(&picker);
        let (dy, dx) = (
            copies[0][1] - cell(&picker, row)[1],
            copies[0][0] - cell(&picker, row)[0],
        );
        assert_eq!(*down.get_or_insert(dy), dy, "{name}: on row {row}");
        assert_eq!(*across.get_or_insert(dx), dx, "{name}: in the column");
        // Each copy is the one before scaled down about the hexagon's centre by a unit, and the
        // six of them span the band's six units.
        let band = |copy: &[f32; 4]| (copies[0][3] - copy[3]) * 0.5 * 62.0 / 64.0;
        for (n, copy) in copies.iter().enumerate() {
            assert!((band(copy) - n as f32).abs() < 0.05, "{name}: copy {n}");
        }
    }
    // A cell's ring never reaches another row or column: one band in the frame, not one per cell.
    assert_eq!(ring(&picker).len(), 12);
}

/// The ring is drawn in the grid's own tint, which a title's data may change (Omega's is red).
#[test]
fn the_cursor_ring_takes_the_grids_tint() {
    let mut layout = layout();
    let red = [0.84, 0.0, 0.0, 1.0];
    layout.hd.as_deref_mut().unwrap().hex.as_mut().unwrap().ring = red;
    let extra = layout.hd.as_deref().unwrap();
    let picker = picker_of(&["A", "B", "C"], "B");
    let rings = body(&picker, &layout, extra, &Frame::default(), &|n| sheet(n))
        .into_iter()
        .filter(|draw| matches!(draw, Draw::Sprite { color, .. } if *color == red))
        .count();
    assert_eq!(rings, 12);
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
    // The same spot is now the centre column's `concept1`: already chosen.
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
/// their slot in the window: the centre column of an odd team sits half a row lower than an
/// even team's, and the grid `TrackHexSelection` builds keeps the window's slots.
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
    let half = hex::PITCH[1] * grid.scale * 0.5;
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
    // Through the first frames of a step the columns keep the previous team's phase.
    let held = |since: f32| {
        grid.cells_at(1, 12, 0.0, since)
            .iter()
            .find(|c| c.offset == 0 && c.row == 0)
            .unwrap()
            .rect[1]
    };
    assert!((held(0.0) - centre_y(&grid, 0)).abs() < 1e-3);
    assert!((held(hex::PHASE_HOLD_SECONDS) - centre_y(&grid, 1)).abs() < 1e-3);
    // The drawn grid takes the hold: a picker a frame after a step draws other rects than a settled one.
    let extra = layout.hd.as_deref().unwrap();
    let mut picker = picker_of(&["A", "B", "C"], "B");
    picker.tick(1.0);
    let settled = body(&picker, &layout, extra, &Frame::default(), &|n| sheet(n));
    picker.set_since_selection(0.0);
    let stepped = body(&picker, &layout, extra, &Frame::default(), &|n| sheet(n));
    assert_ne!(settled, stepped);
    picker.set_since_selection(1.0);
    assert_eq!(
        settled,
        body(&picker, &layout, extra, &Frame::default(), &|n| sheet(n))
    );
    let track = hex::HexGrid::unauthored([100.0, 200.0], 5, 7);
    assert!(!track.phase_by_entry);
    assert!((centre_y(&track, 1) - centre_y(&track, 0)).abs() < 1e-3);
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

/// A team step replays the `Logo`'s `vwipe` from the press
/// (`docs/ghidra/functions/ps3-hdfury-eu/image-widget.md`, "On a step"): a slab a tenth of the
/// quad tall at the top, then lower, back up, and the finished logo after 0.7 s. The screen's
/// opening and a settled screen draw it whole.
#[test]
fn a_team_step_replays_the_logos_vwipe() {
    const LOGO: &str = "Data/Ships/A/FE/Logo.gtf";
    let layout = layout();
    let extra = layout.hd.as_deref().unwrap();
    let sprites = |name: &str| {
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
    };
    let mut entries = vec![hex_team("A"), hex_team("B")];
    for entry in &mut entries {
        let Details::Ship { logo, .. } = &mut entry.details else {
            unreachable!()
        };
        *logo = Some(LOGO.to_string());
    }
    let mut picker = Picker::new(Kind::Ship, entries, Some("A"), None).with_entries_across();
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
    picker.tick(1.0);
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
