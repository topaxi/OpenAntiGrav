use super::*;
use crate::language::StringTable;
use crate::menu::{Frame, Skin};
use oag_gameplay::input::Input;
use oag_tables::race_campaign::Cell;

/// `CellMode_Definition.xml` in miniature: the same container nesting and
/// widget names the real file authors, trimmed to two grid tiers and four
/// hex slots per screen rather than four/thirty-two.
const XML: &str = r#"
<Screen name="Top">
<Screen type="GridSelection" name="Grid Selection">
<LeftLayer transition="0.5" OffsetX="256" OffsetY="80">
<Item OffsetX="14">
<Text name="Title" String="GRID 1" font="Menu" y="13" color="0xffffffff"></Text>
<Item OffsetY="51">
<Text name="Medals" String="00/16" font="default" x="120" color="0xffffffff"></Text>
</Item>
<Item OffsetY="66">
<Text name="Points" String="000/110" font="default" x="120" color="0xffffffff"></Text>
</Item>
<Item OffsetY="81">
<Text name="Required" String="20" font="default" x="120" color="0xffffffff"></Text>
</Item>
</Item>
</LeftLayer>
<LeftLayer transition="0.5">
<Text name="honey" String="1/1" x="135" y="167" color="0xffffffff"></Text>
<Item OffsetX="35" OffsetY="50">
<GridController name="Grid" focus="true">
<Values MaxX="4" MaxY="1"></Values>
<Item OffsetX="30" OffsetY="19">
<Image name="Medal_0_0" x="0" y="76" src="Data\FE\Images\hex_filled.mip"></Image>
</Item>
<Item OffsetX="60" OffsetY="0">
<Image name="Medal_1_0" x="0" y="76" src="Data\FE\Images\hex_filled.mip"></Image>
</Item>
<Item OffsetX="90" OffsetY="19">
<Image name="Medal_2_0" x="0" y="38" src="Data\FE\Images\hex_filled.mip"></Image>
</Item>
<Item OffsetX="120" OffsetY="0">
<Image name="Medal_3_0" x="0" y="38" src="Data\FE\Images\hex_filled.mip"></Image>
</Item>
<Item OffsetX="34" OffsetY="23">
<Image name="Lock_0_0" x="0" y="76" src="Data\FE\Images\pulse_assets.mip"></Image>
</Item>
<Image name="Selector" x="30" y="95" width="42" height="43" src="Data\FE\Images\pulse_assets.mip"></Image>
</GridController>
</Item>
</LeftLayer>
</Screen>
<Screen type="CellSelection" name="Cell Selection">
<LeftLayer transition="0.5">
<GridController name="Grid" OffsetX="35" OffsetY="40" focus="true">
<Values MaxX="7" MaxY="5"></Values>
<Item OffsetX="0" OffsetY="0">
<Image name="Medal_0_0" x="0" y="0" src="Data\FE\Images\hex_filled.mip"></Image>
<Image name="Medal_0_1" x="0" y="38" src="Data\FE\Images\hex_filled.mip"></Image>
</Item>
<Item OffsetX="30" OffsetY="19">
<Image name="Medal_1_0" x="0" y="0" src="Data\FE\Images\hex_filled.mip"></Image>
</Item>
<Item OffsetX="0" OffsetY="0">
<Image name="Outline_0_0" x="0" y="0" src="Data\FE\Images\hex_outline.mip"></Image>
<Image name="Outline_0_1" x="0" y="38" src="Data\FE\Images\hex_outline.mip"></Image>
</Item>
<Item OffsetX="30" OffsetY="19">
<Image name="Outline_1_0" x="0" y="0" src="Data\FE\Images\hex_outline.mip"></Image>
</Item>
<Item OffsetX="4" OffsetY="4">
<Image name="Lock_0_0" x="0" y="0" src="Data\FE\Images\pulse_assets.mip"></Image>
</Item>
<Image name="Selector" x="30" y="95" width="42" height="43" src="Data\FE\Images\pulse_assets.mip"></Image>
</GridController>
</LeftLayer>
<LeftLayer transition="0.5">
<Text name="Title" String="SINGLE RACE 01" font="Menu" x="270" y="38" color="0xffffffff"></Text>
<Item OffsetX="260" OffsetY="60">
<Text name="Track Line" font="default" x="10" y="0" color="0xffffffff"></Text>
</Item>
<Item OffsetX="260" OffsetY="82">
<Text name="Line1 Title" String="l1 title" font="default" x="10" y="0" color="0xff34ACC2"></Text>
<Text name="Line1" String="l1" font="default" x="110" y="0" color="0xffffffff"></Text>
</Item>
<Item OffsetX="260" OffsetY="97">
<Text name="Line2" String="l2" font="default" x="110" y="0" color="0xffffffff"></Text>
</Item>
<Item OffsetX="260" OffsetY="112">
<Text name="Line3" String="l3" font="default" x="110" y="0" color="0xffffffff"></Text>
</Item>
<Item OffsetX="260" OffsetY="150">
<Text name="Line6" String="--5" font="default" x="110" y="0" color="0xffffffff"></Text>
</Item>
<Item OffsetX="260" OffsetY="165">
<Text name="Line7" String="--6" font="default" x="110" y="0" color="0xffffffff"></Text>
</Item>
<Item OffsetX="260" OffsetY="180">
<Image name="Target0 Image" x="10" y="1" width="14" height="14" color="0xfffaeb38" src="Data\FE\Images\hex_filled.mip"></Image>
<Text name="Target0 Title" idstring="IG_HUD_TARGET" font="default" x="30" y="0" color="0xff34ACC2"></Text>
<Text name="Target0" String="value" font="default" x="110" y="0" color="0xffffffff"></Text>
</Item>
</LeftLayer>
</Screen>
</Screen>
"#;

fn strings() -> StringTable {
    StringTable::from_xml(
        r#"<Strings>
<Entry ID="FE_RACE_CAM" String="RACE CAMPAIGN"></Entry>
<Entry ID="FE_NA" String="N/A"></Entry>
<Entry ID="FE_ON" String="ON"></Entry>
<Entry ID="FE_OFF" String="OFF"></Entry>
<Entry ID="RC_INF" String="INF"></Entry>
<Entry ID="MSC_NONE" String="NONE"></Entry>
<Entry ID="IG_HUD_TARGET" String="Target"></Entry>
</Strings>"#,
    )
}

fn skin() -> Skin {
    Skin::new(
        oag_pulse::FRONT_END.menu,
        oag_display::space::Space::PSP,
        22.0,
    )
}

fn press(input: &mut Input, button: Button) {
    input.begin_frame(0);
    input.begin_frame(1 << button as u32);
}

fn race_cell(name: &str, track: &str) -> Cell {
    Cell {
        name: name.to_string(),
        track: Some(track.to_string()),
        mode: Mode::Race,
        class: "Venom".to_string(),
        weapons: true,
        damage: true,
        locked: None,
        status: None,
        ai_count: Some(7),
        skill: Some(1.75),
        skill_easy: Some(1.1),
        skill_hard: Some(2.5),
        laps: Some(3),
        ship: Some("None".to_string()),
        ship_choice: Some(true),
        gold: 1,
        silver: 2,
        bronze: 3,
        tournament_tracks: Vec::new(),
        // Neither is Pulse's own shape - see `race_campaign`'s HD section.
        difficulty_targets: None,
        nitro_elimination_targets: None,
    }
}

fn time_trial_cell(name: &str) -> Cell {
    Cell {
        mode: Mode::TimeTrial,
        weapons: false,
        ai_count: None,
        skill: None,
        skill_easy: None,
        skill_hard: None,
        gold: 6600,
        silver: 6800,
        bronze: 7000,
        ..race_cell(name, "16_Track")
    }
}

fn zone_cell(name: &str) -> Cell {
    Cell {
        mode: Mode::Zone,
        class: "Zone".to_string(),
        laps: Some(0),
        gold: 20,
        silver: 17,
        bronze: 15,
        ..race_cell(name, "16_Track")
    }
}

#[test]
fn grid_selection_pages_four_at_a_time_and_wraps() {
    let grids: Vec<GridSummary> = (0..6)
        .map(|n| GridSummary {
            name: format!("grid{n}"),
            cell_count: 8,
            max_points: 24,
            required_points: 12,
        })
        .collect();
    let mut model = GridSelection::new(grids);
    assert_eq!(model.page(), 0);
    assert_eq!(model.counter(), "1-4 / 6");
    let mut input = Input::new();
    for _ in 0..4 {
        press(&mut input, Button::Down);
        model.update(&mut input);
    }
    assert_eq!(model.index(), 4);
    assert_eq!(model.page(), 1);
    assert_eq!(model.counter(), "5-6 / 6");
    press(&mut input, Button::Down);
    model.update(&mut input);
    assert_eq!(model.index(), 5);
    press(&mut input, Button::Down);
    model.update(&mut input);
    assert_eq!(
        model.index(),
        0,
        "down off the last tier wraps to the first"
    );
    press(&mut input, Button::Cross);
    assert_eq!(model.update(&mut input), vec![Event::Confirmed]);
    press(&mut input, Button::Circle);
    assert_eq!(model.update(&mut input), vec![Event::Back]);
}

#[test]
fn a_fresh_grids_title_is_its_own_raw_name_and_required_falls_back_to_na() {
    let screens = Screens::from_xml(XML);
    let layout = Layout::read(
        &screens,
        "Grid Selection",
        &strings(),
        crate::picker::FaceScales::default(),
        PSP_GRID,
    )
    .unwrap();
    let model = GridSelection::new(vec![GridSummary {
        name: "grid0".to_string(),
        cell_count: 8,
        max_points: 24,
        required_points: 0,
    }]);
    let layers = grid_draw_list(
        &model,
        &layout,
        &skin(),
        &Frame::default(),
        &strings(),
        None,
        false,
        &|_| None,
    );
    let texts: Vec<(String, f32, f32)> = layers
        .body
        .iter()
        .filter_map(|draw| match draw {
            Draw::Text { text, x, y, .. } => Some((text.clone(), *x, *y)),
            _ => None,
        })
        .collect();
    assert!(
        texts.contains(&("grid0".to_string(), 270.0, 93.0)),
        "{texts:?}"
    );
    assert!(
        texts.contains(&("00/08".to_string(), 390.0, 131.0)),
        "{texts:?}"
    );
    assert!(
        texts.contains(&("N/A".to_string(), 390.0, 161.0)),
        "{texts:?}"
    );
}

#[test]
fn cell_selection_moves_toward_the_pressed_direction_and_skips_absent_slots() {
    let cells = vec![
        race_cell("g_0_0", "16_Track"),
        race_cell("g_1_0", "16_Track"),
    ];
    let mut model = CellSelection::new(cells);
    assert_eq!(model.selected().unwrap().name, "g_0_0");
    let mut input = Input::new();
    press(&mut input, Button::Right);
    assert_eq!(model.update(&mut input), vec![Event::Moved]);
    assert_eq!(model.selected().unwrap().name, "g_1_0");
    // No cell further right - the press is consumed, nothing moves.
    press(&mut input, Button::Right);
    assert_eq!(model.update(&mut input), Vec::new());
    assert_eq!(model.selected().unwrap().name, "g_1_0");
}

#[test]
fn triangle_opens_help_and_suspends_movement() {
    let cells = vec![
        race_cell("g_0_0", "16_Track"),
        race_cell("g_1_0", "16_Track"),
    ];
    let mut model = CellSelection::new(cells);
    let mut input = Input::new();
    press(&mut input, Button::Triangle);
    assert_eq!(model.update(&mut input), vec![Event::Help]);
    assert!(model.help_open());
    press(&mut input, Button::Right);
    assert_eq!(
        model.update(&mut input),
        Vec::new(),
        "movement is inert while Cell Help is open"
    );
    assert_eq!(model.selected().unwrap().name, "g_0_0");
    press(&mut input, Button::Circle);
    assert_eq!(
        model.update(&mut input),
        vec![Event::Help],
        "circle closes the overlay rather than backing out"
    );
    assert!(!model.help_open());
}

#[test]
fn targets_are_hidden_for_race_and_shown_as_a_time_for_time_trial() {
    let screens = Screens::from_xml(XML);
    let layout = Layout::read(
        &screens,
        "Cell Selection",
        &strings(),
        crate::picker::FaceScales::default(),
        PSP_GRID,
    )
    .unwrap();

    let race = CellSelection::new(vec![race_cell("grid0_0_0", "16_Track")]);
    let layers = cell_draw_list(
        &race,
        &layout,
        &skin(),
        &Frame::default(),
        &strings(),
        None,
        false,
        &|_| None,
    );
    assert!(
        !layers
            .body
            .iter()
            .any(|draw| matches!(draw, Draw::Text { text, .. } if text == "1")),
        "Race hides its target column: {:?}",
        layers.body
    );

    let tt = CellSelection::new(vec![time_trial_cell("grid0_0_0")]);
    let layers = cell_draw_list(
        &tt,
        &layout,
        &skin(),
        &Frame::default(),
        &strings(),
        None,
        false,
        &|_| None,
    );
    let texts: Vec<&String> = layers
        .body
        .iter()
        .filter_map(|draw| match draw {
            Draw::Text { text, .. } => Some(text),
            _ => None,
        })
        .collect();
    assert!(texts.iter().any(|t| t.as_str() == "1:06.00"), "{texts:?}");
    assert!(texts.iter().any(|t| t.as_str() == "Target"), "{texts:?}");
}

/// A real sprite lookup - `&|_| None` in the tests above hides a widget that
/// draws unconditionally in the *general* image loop as readily as one that
/// is correctly gated, since neither ever reaches `Draw::Sprite` at all. This
/// is the regression test for exactly that: the `Target{n} Image` swatches
/// used to draw whether or not `targets_visible` said so, because nothing
/// excluded them from the screen's own ungated `for image in &screen.images`
/// pass - only the *second*, gated pass that draws them properly.
#[test]
fn race_draws_no_target_swatch_even_though_the_sprite_resolves() {
    let screens = Screens::from_xml(XML);
    let layout = Layout::read(
        &screens,
        "Cell Selection",
        &strings(),
        crate::picker::FaceScales::default(),
        PSP_GRID,
    )
    .unwrap();
    let race = CellSelection::new(vec![race_cell("grid0_0_0", "16_Track")]);
    let placed = crate::frontend::Placed {
        x: 0,
        y: 0,
        width: 14,
        height: 14,
        quad_extent: None,
        blend: None,
    };
    let layers = cell_draw_list(
        &race,
        &layout,
        &skin(),
        &Frame::default(),
        &strings(),
        None,
        false,
        &|_| Some(placed),
    );
    let sprites: Vec<&[f32; 4]> = layers
        .body
        .iter()
        .filter_map(|draw| match draw {
            Draw::Sprite { rect, .. } | Draw::TiledSprite { rect, .. } => Some(rect),
            _ => None,
        })
        .collect();
    // Exactly the occupied cell's own `Medal_0_0`/`Outline_0_0` plus the
    // `Selector` cursor - `Medal_0_1`/`Medal_1_0`/`Outline_0_1`/`Outline_1_0`
    // are unoccupied slots, `Lock_0_0` is never drawn, and `Target0 Image`
    // is this test's own regression: it must not appear for a `Race` cell.
    assert_eq!(sprites.len(), 3, "{:?}", layers.body);
}

#[test]
fn zone_blanks_the_class_line_and_counts_up() {
    let cell = zone_cell("grid0_4_2");
    assert_eq!(target_value(cell.gold, &cell.mode), "20");
    assert_eq!(laps_line(&cell, &strings()), "INF");
}

#[test]
fn centiseconds_format_as_minutes_seconds_hundredths() {
    assert_eq!(format_centiseconds(6600), "1:06.00");
    assert_eq!(format_centiseconds(59), "0:00.59");
}
