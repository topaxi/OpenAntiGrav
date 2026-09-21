//! Draw-list tests: everything that needs a [`crate::menu::Layers`] out of
//! [`grid_draw_list`]/[`cell_draw_list`]. Split from `campaign/tests.rs`'s
//! model-level tests the same way `draw.rs` itself was, under the
//! 1,000-line rule (`scripts/check-file-size.py`).

use super::*;
use crate::campaign::{CellSelection, GridSelection, GridSummary, Layout, PSP_GRID};
use crate::frontend::Draw;
use crate::language::StringTable;
use crate::menu::{Frame, Skin};
use crate::screen::Screens;
use oag_tables::race_campaign::Cell;

/// `CellMode_Definition.xml` in miniature - see `campaign/tests.rs`'s own
/// copy of this fixture for why it is duplicated rather than shared: each
/// test file is a separate leaf module with no path to the other's private
/// items.
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
<Item OffsetX="64" OffsetY="4">
<Image name="Lock_1_0" x="0" y="76" src="Data\FE\Images\pulse_assets.mip"></Image>
</Item>
<Image name="Selector" x="30" y="95" width="42" height="43" src="Data\FE\Images\pulse_assets.mip"></Image>
</GridController>
</Item>
</LeftLayer>
</Screen>
<Screen type="CellSelection" name="Cell Selection">
<LeftLayer transition="0.5">
<Item OffsetX="256" OffsetY="83">
<Image name="linebg1l" x="0" y="0" color="0x00ffffff"></Image>
</Item>
<Item OffsetX="256" OffsetY="98">
<Image name="linebg2l" x="0" y="0" color="0x00ffffff"></Image>
</Item>
<Item OffsetX="256" OffsetY="113">
<Image name="linebg3l" x="0" y="0" color="0x00ffffff"></Image>
</Item>
<Item OffsetX="256" OffsetY="128">
<Image name="linebg4l" x="0" y="0" color="0x00ffffff"></Image>
</Item>
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
<Item OffsetX="34" OffsetY="23">
<Image name="Lock_1_0" x="0" y="0" src="Data\FE\Images\pulse_assets.mip"></Image>
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
<Text name="Line2 Title" String="l2 title" font="default" x="10" y="0" color="0xff34ACC2"></Text>
<Text name="Line2" String="l2" font="default" x="110" y="0" color="0xffffffff"></Text>
</Item>
<Item OffsetX="260" OffsetY="112">
<Text name="Line3 Title" String="l3 title" font="default" x="10" y="0" color="0xff34ACC2"></Text>
<Text name="Line3" String="l3" font="default" x="110" y="0" color="0xffffffff"></Text>
</Item>
<Item OffsetX="260" OffsetY="135">
<Text name="Line5 Title" String="--7" font="default" x="10" y="0" color="0xff34ACC2"></Text>
<Text name="Line5" String="--7" font="default" x="110" y="0" color="0xffffffff"></Text>
</Item>
<Item OffsetX="260" OffsetY="150">
<Text name="Line6 Title" String="--5" font="default" x="10" y="0" color="0xff34ACC2"></Text>
<Text name="Line6" String="--5" font="default" x="110" y="0" color="0xffffffff"></Text>
</Item>
<Item OffsetX="260" OffsetY="165">
<Text name="Line7 Title" String="--6" font="default" x="10" y="0" color="0xff34ACC2"></Text>
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
<Entry ID="RC_SC" String="Speed class"></Entry>
<Entry ID="RC_LAPS" String="Laps"></Entry>
<Entry ID="RB_WEAP" String="Weapons"></Entry>
<Entry ID="ER_POINTS" String="Points"></Entry>
<Entry ID="IG_HUD_BEST" String="Best"></Entry>
<Entry ID="MSC_NONE" String="NONE"></Entry>
<Entry ID="IG_HUD_TARGET" String="Target"></Entry>
<Entry ID="MSC_EVENT_SR" String="Single Race: take on a full grid of opponents."></Entry>
<Entry ID="MSC_EVENT_TT" String="Time Trial: beat the clock in this solo race."></Entry>
<Entry ID="MSC_EVENT_TOURN" String="Tournament: take part in a series of single races."></Entry>
<Entry ID="16_Track" String="Talon's Junction White"></Entry>
<Entry ID="Grid0" String="Grid 1"></Entry>
<Entry ID="Grid4" String="Grid 5"></Entry>
<Entry ID="Grid11" String="Grid 12"></Entry>
<Entry ID="Grid12" String="Phantom Grid 1"></Entry>
</Strings>"#,
    )
}

fn skin() -> Skin {
    Skin::new(
        oag_pulse::FRONT_END.menu.unwrap(),
        oag_display::space::Space::PSP,
        22.0,
    )
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

fn grid_layout() -> Layout {
    Layout::read(
        &Screens::from_xml(XML),
        "Grid Selection",
        &strings(),
        crate::picker::FaceScales::default(),
        PSP_GRID,
    )
    .unwrap()
}

fn cell_layout() -> Layout {
    Layout::read(
        &Screens::from_xml(XML),
        "Cell Selection",
        &strings(),
        crate::picker::FaceScales::default(),
        PSP_GRID,
    )
    .unwrap()
}

fn placed32() -> Option<crate::frontend::Placed> {
    Some(crate::frontend::Placed {
        x: 0,
        y: 0,
        width: 32,
        height: 32,
        quad_extent: None,
        blend: None,
    })
}

fn texts(layers: &crate::menu::Layers) -> Vec<&String> {
    layers
        .body
        .iter()
        .filter_map(|draw| match draw {
            // `Default`-role labels now draw as `FacedText` (`text_draw`'s
            // own `super::footer::face_role` routing) rather than the plain
            // `Text` every campaign-screen label used before lane 7's own
            // atlas made the role mean something - both are "a text draw"
            // for what these tests check.
            Draw::Text { text, .. } | Draw::FacedText { text, .. } => Some(text),
            _ => None,
        })
        .collect()
}

/// `Grid Selection`'s `Title` resolves `grid0` to `"Grid 1"`, not the raw
/// `"grid0"` - see `docs/ui/campaign-screens.md`'s "Measured against
/// PPSSPP", which found the previous "raw name" reading backwards.
#[test]
fn a_grids_title_resolves_through_the_per_grid_idstring() {
    let layout = grid_layout();
    let model = GridSelection::new(vec![GridSummary {
        name: "grid0".to_string(),
        cell_count: 8,
        max_points: 24,
        required_points: 0,
        gold_medals: 0,
        points_earned: 0,
        locked: false,
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
        &[],
    );
    let texts: Vec<(String, f32, f32)> = layers
        .body
        .iter()
        .filter_map(|draw| match draw {
            // See the module `texts()` helper's own doc on why both
            // variants: `Grid`'s `Title` (`font="default"`) now routes
            // through `Draw::FacedText`.
            Draw::Text { text, x, y, .. } | Draw::FacedText { text, x, y, .. } => {
                Some((text.clone(), *x, *y))
            }
            _ => None,
        })
        .collect();
    assert!(
        texts.contains(&("Grid 1".to_string(), 270.0, 93.0)),
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

/// **`grid12`..`grid15` are not `"Grid 13"`..`"Grid 16"`.** The disc's own
/// table answers `"Phantom Grid 1"` for `Grid12` - a real per-grid idstring,
/// not a formula this build could derive from the flat index. Pinned here
/// so nobody re-derives `"Grid {n+1}"` and silently regresses these four.
#[test]
fn the_last_four_grids_are_phantom_grid_not_grid_thirteen_through_sixteen() {
    let s = strings();
    assert_eq!(grid_title("grid11", &s), "Grid 12");
    assert_eq!(grid_title("grid12", &s), "Phantom Grid 1");
}

/// A source whose table has no entry for a grid falls back to the raw name -
/// a visible absence, the same rule every other label on this screen follows.
#[test]
fn an_unresolved_grid_title_falls_back_to_the_raw_name() {
    assert_eq!(grid_title("grid9", &StringTable::default()), "grid9");
}

/// `Cell Selection`'s `Title` is the localised mode name - `"SINGLE RACE"`
/// for `Race`, not the raw enum spelling `"Race"` - see
/// `docs/ui/campaign-screens.md`'s "Measured against PPSSPP".
#[test]
fn a_race_cells_title_is_the_localised_mode_name_not_the_raw_enum_spelling() {
    let s = strings();
    assert_eq!(
        cell_title(&race_cell("grid0_0_0", "16_Track"), &s),
        "Single Race"
    );
    assert_eq!(cell_title(&time_trial_cell("grid0_0_0"), &s), "Time Trial");
}

/// `Tournament`/`Head2Head` resolve through the identical mechanism, off
/// `MSC_EVENT_TOURN`/`MSC_EVENT_HTH` - unmeasured against a live frame (no
/// capture reaches either mode), but the same reading as the five measured
/// ones.
#[test]
fn tournament_resolves_through_the_same_mechanism_though_unmeasured_live() {
    let mut cell = race_cell("grid0_0_0", "16_Track");
    cell.mode = Mode::Tournament;
    assert_eq!(cell_title(&cell, &strings()), "Tournament");
}

/// `Custom Grid`/`AI Race` have no `MSC_EVENT_*` entry at all - the raw
/// spelling stands, per `docs/ui/campaign-screens.md`.
#[test]
fn custom_grid_falls_back_to_its_raw_spelling() {
    let mut cell = race_cell("grid0_0_0", "16_Track");
    cell.mode = Mode::CustomGrid;
    assert_eq!(cell_title(&cell, &strings()), "Custom Grid");
}

/// `Track Line` resolves the circuit's own display name off the disc's
/// English table - `"16_Track"` -> `"Talon's Junction White"` - rather than
/// drawing the raw id, per `docs/ui/campaign-screens.md`'s "Measured
/// against PPSSPP".
#[test]
fn track_line_resolves_the_circuits_display_name() {
    let layout = cell_layout();
    let model = CellSelection::new(vec![race_cell("grid0_0_0", "16_Track")]);
    let layers = cell_draw_list(
        &model,
        &layout,
        &skin(),
        &Frame::default(),
        &strings(),
        None,
        false,
        &|_| None,
        &[],
    );
    assert!(
        texts(&layers)
            .iter()
            .any(|t| t.as_str() == "Talon's Junction White"),
        "{:?}",
        texts(&layers)
    );
}

/// The panel's row labels resolve to real idstrings - `RC_SC`/`RC_LAPS`/
/// `RB_WEAP`/`ER_POINTS` - not the template junk `CellMode_Definition.xml`
/// authors as a placeholder (`"l1 title"` etc).
#[test]
fn the_panels_row_labels_resolve_to_their_own_idstrings() {
    let layout = cell_layout();
    let model = CellSelection::new(vec![race_cell("grid0_0_0", "16_Track")]);
    let layers = cell_draw_list(
        &model,
        &layout,
        &skin(),
        &Frame::default(),
        &strings(),
        None,
        false,
        &|_| None,
        &[],
    );
    let texts = texts(&layers);
    for label in ["Speed class", "Laps", "Weapons", "Points"] {
        assert!(texts.iter().any(|t| t.as_str() == label), "{texts:?}");
    }
    assert!(
        !texts.iter().any(|t| t.as_str() == "l1 title"),
        "the template placeholder must never draw: {texts:?}"
    );
}

/// `Best` (`Line7 Title`/`Line7`) sits in the same row `Target0..2` occupy,
/// not alongside them - `docs/ui/campaign-screens.md`'s "Best/Target looks
/// mutually exclusive" finding.
#[test]
fn best_is_hidden_when_the_target_rows_show_instead() {
    let layout = cell_layout();
    let model = CellSelection::new(vec![time_trial_cell("grid0_0_0")]);
    let layers = cell_draw_list(
        &model,
        &layout,
        &skin(),
        &Frame::default(),
        &strings(),
        None,
        false,
        &|_| None,
        &[],
    );
    assert!(
        !texts(&layers).iter().any(|t| t.as_str() == "Best"),
        "{:?}",
        texts(&layers)
    );

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
        &[],
    );
    assert!(texts(&layers).iter().any(|t| t.as_str() == "Best"));
}

/// `Line5` (`Cell_SavedRecord`) draws the selected cell's own saved record,
/// formatted the same `M:SS.CC` way `Target0..2` already are - and stays
/// silent for a cell with none, the same visible-absence rule every other
/// label on this screen follows.
#[test]
fn line5_draws_the_selected_cells_saved_record_when_one_exists() {
    let layout = cell_layout();
    let with_none = CellSelection::new(vec![time_trial_cell("grid0_0_0")]);
    let layers = cell_draw_list(
        &with_none,
        &layout,
        &skin(),
        &Frame::default(),
        &strings(),
        None,
        false,
        &|_| None,
        &[],
    );
    assert!(
        !texts(&layers).iter().any(|t| t.as_str() == "2:10.48"),
        "{:?}",
        texts(&layers)
    );

    let with_record = CellSelection::with_medals_and_records(
        vec![time_trial_cell("grid0_0_0")],
        &|_| None,
        &|_| Some(13_048),
    );
    let layers = cell_draw_list(
        &with_record,
        &layout,
        &skin(),
        &Frame::default(),
        &strings(),
        None,
        false,
        &|_| None,
        &[],
    );
    assert!(
        texts(&layers).iter().any(|t| t.as_str() == "2:10.48"),
        "{:?}",
        texts(&layers)
    );
}

/// The regression this pass fixes: `Selector` used to share the selected
/// hex's own top-left corner, which put a 42x43 cursor visibly down-and-right
/// of a 32x32 hex rather than around it - see `docs/ui/campaign-screens.md`.
#[test]
fn the_selector_is_centred_on_the_selected_hex_not_top_left_aligned() {
    let layout = grid_layout();
    let model = GridSelection::new(vec![GridSummary {
        name: "grid0".to_string(),
        cell_count: 8,
        max_points: 24,
        required_points: 12,
        gold_medals: 0,
        points_earned: 0,
        locked: false,
    }]);
    let layers = grid_draw_list(
        &model,
        &layout,
        &skin(),
        &Frame::default(),
        &strings(),
        None,
        false,
        &|_| placed32(),
        &[],
    );
    let selector_rect = layers
        .body
        .iter()
        .find_map(|draw| match draw {
            Draw::Sprite { rect, .. } if rect[2] == 42.0 && rect[3] == 43.0 => Some(*rect),
            _ => None,
        })
        .expect("the Selector draws as a 42x43 sprite");
    // Hex 0 sits at [65,145,32,32]. Centred: `65 + (32-42)/2 = 60`,
    // `145 + (32-43)/2 = 139.5`.
    assert_eq!(selector_rect, [60.0, 139.5, 42.0, 43.0]);
}

#[test]
fn targets_are_hidden_for_race_and_shown_as_a_time_for_time_trial() {
    let layout = cell_layout();

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
        &[],
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
        &[],
    );
    let texts = texts(&layers);
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
    let layout = cell_layout();
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
        &[],
    );
    let sprites: Vec<&[f32; 4]> = layers
        .body
        .iter()
        .filter_map(|draw| match draw {
            Draw::Sprite { rect, .. } | Draw::TiledSprite { rect, .. } => Some(rect),
            _ => None,
        })
        .collect();
    // The occupied cell's own `Outline_0_0` (its `Medal_0_0` is unmedalled,
    // so it draws no swatch), the `Lock_0_0` glyph (no `Locked` attribute on
    // this cell - defaults to true, no medal, no medalled neighbour), and
    // the `Selector` cursor. `Medal_0_1`/`Medal_1_0`/`Outline_0_1`/
    // `Outline_1_0`/`Lock_1_0` are unoccupied slots, and `Target0 Image` is
    // this test's own regression: it must not appear for a `Race` cell.
    assert_eq!(sprites.len(), 3, "{:?}", layers.body);
}

/// The hex-look regression this pass fixes: an unmedalled cell's own
/// `Medal_0_0` must not draw at all (the base `Outline_0_0` still does) -
/// this build used to draw both unconditionally, which read as a filled
/// hex everywhere rather than an outline with a swatch only where earned.
#[test]
fn an_unmedalled_cells_medal_swatch_does_not_draw() {
    let layout = cell_layout();
    // `Locked = false` on both cells, so the `Lock_0_0` glyph - which itself
    // toggles on the same "has a medal" test - cannot also change the
    // sprite count and confound what this test isolates.
    let mut unmedalled_cell = race_cell("grid0_0_0", "16_Track");
    unmedalled_cell.locked = Some(false);
    let model = CellSelection::new(vec![unmedalled_cell.clone()]);
    let placed = placed32().unwrap();
    let layers = cell_draw_list(
        &model,
        &layout,
        &skin(),
        &Frame::default(),
        &strings(),
        None,
        false,
        &|src| src.contains("hex_filled").then_some(placed).or(placed32()),
        &[],
    );
    // Every `Draw::Sprite`/`TiledSprite` this build can attribute to
    // `hex_filled.mip` specifically would need the UV/texture-size fields
    // threaded through, which `Draw` does not carry a source name for - so
    // this checks the sprite *count* against the medalled case below
    // instead, the same indirect assertion `race_draws_no_target_swatch...`
    // already uses for a different widget.
    let unmedalled_count = layers
        .body
        .iter()
        .filter(|draw| matches!(draw, Draw::Sprite { .. } | Draw::TiledSprite { .. }))
        .count();

    let medalled = CellSelection::with_medals(vec![unmedalled_cell], &|_| Some(Medal::Gold));
    let layers = cell_draw_list(
        &medalled,
        &layout,
        &skin(),
        &Frame::default(),
        &strings(),
        None,
        false,
        &|src| src.contains("hex_filled").then_some(placed).or(placed32()),
        &[],
    );
    let medalled_count = layers
        .body
        .iter()
        .filter(|draw| matches!(draw, Draw::Sprite { .. } | Draw::TiledSprite { .. }))
        .count();
    assert_eq!(
        medalled_count,
        unmedalled_count + 1,
        "a medalled cell draws exactly one more sprite - its own Medal_0_0 swatch"
    );
}

/// A locked cell's `Lock_x_y` draws; an unlocked one's does not, and the
/// glyph draws at its own authored (inset) position - no centring, unlike
/// `Selector`.
#[test]
fn a_locked_cells_lock_glyph_draws_at_its_own_authored_position() {
    let layout = cell_layout();
    let locked = CellSelection::new(vec![race_cell("grid0_0_0", "16_Track")]);
    let layers = cell_draw_list(
        &locked,
        &layout,
        &skin(),
        &Frame::default(),
        &strings(),
        None,
        false,
        &|_| placed32(),
        &[],
    );
    let lock_rect = layers.body.iter().find_map(|draw| match draw {
        // `Lock_0_0` is authored at `Item OffsetX="4" OffsetY="4"` in this
        // fixture - distinct from `Outline_0_0`'s `(0,0)` and `Selector`'s
        // 42x43, so the rect alone identifies it.
        Draw::Sprite { rect, .. } if *rect == [39.0, 44.0, 32.0, 32.0] => Some(*rect),
        _ => None,
    });
    assert!(lock_rect.is_some(), "{:?}", layers.body);

    let mut cell = race_cell("grid0_0_0", "16_Track");
    cell.locked = Some(false);
    let unlocked = CellSelection::new(vec![cell]);
    let layers = cell_draw_list(
        &unlocked,
        &layout,
        &skin(),
        &Frame::default(),
        &strings(),
        None,
        false,
        &|_| placed32(),
        &[],
    );
    assert!(
        !layers.body.iter().any(
            |draw| matches!(draw, Draw::Sprite { rect, .. } if *rect == [39.0, 44.0, 32.0, 32.0])
        ),
        "an unlocked cell draws no Lock glyph: {:?}",
        layers.body
    );
}

/// `line bg1..3` draw one stripe per *visible* detail row - three on a
/// `Race` cell (`Speed class`/`Laps`/`Weapons` all show), two on `Time
/// Trial` (`Weapons` hidden) - and `line bg4` never draws at all, since
/// nothing ever populates `Line4` to sit above it. See
/// `docs/ui/campaign-screens.md`'s "Measured against PPSSPP".
#[test]
fn the_row_divider_count_matches_the_visible_row_count() {
    let layout = cell_layout();
    let fill_count = |cell: Cell| {
        let model = CellSelection::new(vec![cell]);
        let layers = cell_draw_list(
            &model,
            &layout,
            &skin(),
            &Frame::default(),
            &strings(),
            None,
            false,
            &|_| None,
            &[],
        );
        layers
            .body
            .iter()
            .filter(|draw| matches!(draw, Draw::Fill { .. } | Draw::GradientFill { .. }))
            .count()
    };
    assert_eq!(fill_count(race_cell("grid0_0_0", "16_Track")), 3);
    assert_eq!(fill_count(time_trial_cell("grid0_0_0")), 2);
}

/// [`GridSummary::from_grid_with_medals`]'s own reason to exist, on the
/// draw side: `Line6`/`Line7` stop reading as a fresh profile's `"0/3"`/
/// `MSC_NONE` once a caller can say what a cell's own best medal was.
#[test]
fn a_cells_saved_medal_reaches_line6_and_line7() {
    let layout = cell_layout();
    let model = CellSelection::with_medals(vec![race_cell("grid0_0_0", "16_Track")], &|_| {
        Some(Medal::Silver)
    });
    let layers = cell_draw_list(
        &model,
        &layout,
        &skin(),
        &Frame::default(),
        &strings(),
        None,
        false,
        &|_| None,
        &[],
    );
    let texts = texts(&layers);
    assert!(
        texts.contains(&&format!(
            "{}/{}",
            Medal::Silver.points(),
            Medal::Gold.points()
        )),
        "{texts:?}"
    );
    assert!(
        texts.contains(&&"IG_HUD_SILVER".to_string()),
        "unresolved id falls back to itself, the same as every other label \
         this test file's own `strings()` does not carry: {texts:?}"
    );
}

/// The same fix as `the_selector_is_centred_on_the_selected_hex_not_top_left_aligned`,
/// on `Cell Selection`'s own `Selector`.
#[test]
fn cell_selections_selector_is_also_centred_on_the_selected_hex() {
    let layout = cell_layout();
    let model = CellSelection::new(vec![race_cell("grid0_1_0", "16_Track")]);
    let layers = cell_draw_list(
        &model,
        &layout,
        &skin(),
        &Frame::default(),
        &strings(),
        None,
        false,
        &|_| placed32(),
        &[],
    );
    let selector_rect = layers
        .body
        .iter()
        .find_map(|draw| match draw {
            Draw::Sprite { rect, .. } if rect[2] == 42.0 && rect[3] == 43.0 => Some(*rect),
            _ => None,
        })
        .expect("the Selector draws as a 42x43 sprite");
    // Cell (1,0) sits at [65,59,32,32].
    assert_eq!(selector_rect, [60.0, 53.5, 42.0, 43.0]);
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
