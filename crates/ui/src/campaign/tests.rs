//! Model-level tests: navigation, paging, pointer hit-testing, and the lock
//! predicates - everything that does not need a [`crate::menu::Layers`] out
//! of [`super::draw`]. See `campaign/draw/tests.rs` for the draw-list ones,
//! split out the same way `draw.rs` itself was, under the 1,000-line rule.

use super::*;
use crate::language::StringTable;
use crate::pointer::Pointer;
use crate::screen::Image;
use oag_gameplay::input::Input;
use oag_tables::race_campaign::{Cell, Mode};

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
<Entry ID="RC_SC" String="Speed class"></Entry>
<Entry ID="RC_LAPS" String="Laps"></Entry>
<Entry ID="RB_WEAP" String="Weapons"></Entry>
<Entry ID="ER_POINTS" String="Points"></Entry>
<Entry ID="IG_HUD_BEST" String="Best"></Entry>
<Entry ID="MSC_NONE" String="NONE"></Entry>
<Entry ID="IG_HUD_TARGET" String="Target"></Entry>
<Entry ID="MSC_EVENT_SR" String="Single Race: take on a full grid."></Entry>
<Entry ID="MSC_EVENT_TT" String="Time Trial: beat the clock."></Entry>
<Entry ID="Grid0" String="Grid 1"></Entry>
<Entry ID="Grid4" String="Grid 5"></Entry>
<Entry ID="Grid12" String="Phantom Grid 1"></Entry>
</Strings>"#,
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

#[test]
fn grid_selection_pages_four_at_a_time_and_wraps() {
    let grids: Vec<GridSummary> = (0..6)
        .map(|n| GridSummary {
            name: format!("grid{n}"),
            cell_count: 8,
            max_points: 24,
            required_points: 12,
            gold_medals: 0,
            points_earned: 0,
            locked: false,
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

/// [`GridSummary::from_grid_with_medals`]'s own reason to exist: a real
/// progress source moves `Medals`/`Points` off the fresh-profile `"00/.."`
/// this crate otherwise draws.
#[test]
fn a_grid_with_one_gold_cell_shows_it_on_medals_and_points() {
    let grid = Grid {
        name: "grid0".to_string(),
        required_points: 12,
        locked: false,
        group: None,
        unlock_grid: None,
        cells: vec![
            race_cell("grid0_2_1", "16_Track"),
            race_cell("grid0_3_1", "03_Track"),
        ],
        // HD's own grid attributes - see `race_campaign`'s HD section.
        campaign: None,
        title_color: None,
        text_color: None,
        flyer_name: None,
        billboard_name: None,
    };
    let summary = GridSummary::from_grid_with_medals(&grid, &|name| {
        (name == "grid0_2_1").then_some(Medal::Gold)
    });
    assert_eq!(summary.gold_medals, 1);
    assert_eq!(summary.points_earned, Medal::Gold.points());
    assert_eq!(summary.cell_count, 2);
    assert_eq!(summary.max_points, 6);
    assert!(!summary.locked, "grid0 authors Locked=\"false\"");
}

/// The two-term display rule, cell side: `docs/ghidra/functions/psp-pulse-usa/race-campaign.md`'s
/// "Unlock rules, cell and tier" - locked by default, cleared by the cell's
/// own medal.
#[test]
fn a_cell_with_no_locked_attribute_shows_its_lock_until_it_or_a_neighbour_earns_a_medal() {
    let unmedalled = CellSelection::new(vec![race_cell("grid0_2_1", "16_Track")]);
    assert!(
        unmedalled.cell_shows_lock(2, 1),
        "absent Locked defaults to true - confidence 72, \
         docs/ghidra/functions/psp-pulse-usa/race-campaign.md"
    );

    let own_medal = CellSelection::with_medals(vec![race_cell("grid0_2_1", "16_Track")], &|_| {
        Some(Medal::Bronze)
    });
    assert!(
        !own_medal.cell_shows_lock(2, 1),
        "the cell's own medal clears it"
    );
}

/// The cell rule's third term: a hex-adjacent cell's medal clears the lock
/// too, even though the cell itself has none - `g_anCellNeighbourOffsets`'s
/// six-neighbour check.
#[test]
fn a_medalled_neighbour_clears_an_unmedalled_cells_lock() {
    let cells = vec![
        race_cell("grid0_2_1", "16_Track"),
        race_cell("grid0_3_1", "16_Track"), // (3,1): odd x, "right" neighbour of (2,1)
    ];
    let medalled_neighbour = CellSelection::with_medals(cells, &|name| {
        (name == "grid0_3_1").then_some(Medal::Silver)
    });
    assert!(
        !medalled_neighbour.cell_shows_lock(2, 1),
        "a neighbour's own medal clears this cell's lock too"
    );
}

/// A cell that explicitly authors `Locked="false"` never shows the glyph,
/// medal or not.
#[test]
fn a_cell_authored_locked_false_never_shows_the_glyph() {
    let mut cell = race_cell("grid0_3_1", "16_Track");
    cell.locked = Some(false);
    let model = CellSelection::new(vec![cell]);
    assert!(!model.cell_shows_lock(3, 1));
}

/// The tier rule, mirrored: locked by default, cleared by this grid's own
/// points, or by the previous tile's own points meeting its own
/// `RequiredPoints`.
#[test]
fn a_tier_is_locked_until_its_own_points_or_the_previous_tiers_are_met() {
    let base = |name: &str, required: u32, earned: u32| GridSummary {
        name: name.to_string(),
        cell_count: 8,
        max_points: 24,
        required_points: required,
        gold_medals: 0,
        points_earned: earned,
        locked: true,
    };
    // grid0 never locked at all - `grid_00.xml`'s own `Locked="false"`.
    let grid0 = GridSummary {
        locked: false,
        ..base("grid0", 12, 0)
    };
    let grid1_unmet = base("grid1", 16, 0);
    let model = GridSelection::new(vec![grid0.clone(), grid1_unmet]);
    assert!(
        model.tier_shows_lock(1),
        "grid0's own points (0) have not met its own RequiredPoints (12)"
    );

    let grid0_met = GridSummary {
        points_earned: 12,
        ..grid0
    };
    let grid1_still_locked = base("grid1", 16, 0);
    let model = GridSelection::new(vec![grid0_met, grid1_still_locked]);
    assert!(
        !model.tier_shows_lock(1),
        "grid0's own points now meet its RequiredPoints, clearing grid1's glyph"
    );

    let grid1_own_points = base("grid1", 16, 3);
    let model = GridSelection::new(vec![base("grid0", 12, 0), grid1_own_points]);
    assert!(
        !model.tier_shows_lock(1),
        "grid1's own points, even short of RequiredPoints, clear its own glyph"
    );
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

/// [`grid_layout`], with the paging arrows the miniature fixture otherwise
/// leaves out - at `docs/ui/campaign-screens.md`'s own measured positions.
fn grid_layout_with_arrows() -> Layout {
    let xml = XML.replace(
        r#"<Text name="honey""#,
        r#"<Image name="up arrow" x="117" y="84" width="17" height="14" src="Data\FE\Images\pulse_assets.mip"></Image><Text name="honey""#,
    );
    Layout::read(
        &Screens::from_xml(&xml),
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

fn grid_summaries(n: usize) -> Vec<GridSummary> {
    (0..n)
        .map(|i| GridSummary {
            name: format!("grid{i}"),
            cell_count: 8,
            max_points: 24,
            required_points: 12,
            gold_medals: 0,
            points_earned: 0,
            locked: false,
        })
        .collect()
}

fn hover(at: (f32, f32)) -> Pointer {
    Pointer {
        at: Some(at),
        moved: true,
        ..Pointer::default()
    }
}

fn click(at: (f32, f32)) -> Pointer {
    Pointer {
        at: Some(at),
        moved: true,
        clicked: true,
        ..Pointer::default()
    }
}

fn hex_centre(rect: [f32; 4]) -> (f32, f32) {
    (rect[0] + rect[2] * 0.5, rect[1] + rect[3] * 0.5)
}

#[test]
fn grid_selections_hexes_are_at_the_docs_own_measured_positions() {
    let layout = grid_layout();
    let model = GridSelection::new(grid_summaries(4));
    let targets = pointer::grid_targets(&model, &layout, &|_| placed32());
    let rect_of = |index: usize| {
        targets
            .iter()
            .find(|target| target.what == pointer::What::Hex(index))
            .unwrap_or_else(|| panic!("hex {index}"))
            .rect
    };
    // `docs/ui/campaign-screens.md`'s own staggered-diagonal measurement.
    assert_eq!(rect_of(0), [65.0, 145.0, 32.0, 32.0]);
    assert_eq!(rect_of(1), [95.0, 126.0, 32.0, 32.0]);
    assert_eq!(rect_of(2), [125.0, 107.0, 32.0, 32.0]);
    assert_eq!(rect_of(3), [155.0, 88.0, 32.0, 32.0]);
}

#[test]
fn a_grid_page_offers_no_hex_for_a_slot_the_page_has_no_tier_for() {
    let layout = grid_layout();
    let model = GridSelection::new(grid_summaries(2));
    let targets = pointer::grid_targets(&model, &layout, &|_| placed32());
    assert!(targets.iter().any(|t| t.what == pointer::What::Hex(0)));
    assert!(targets.iter().any(|t| t.what == pointer::What::Hex(1)));
    assert!(
        !targets
            .iter()
            .any(|t| matches!(t.what, pointer::What::Hex(2 | 3))),
        "only two tiers exist - slots 2 and 3 draw no hex, so they are no \
         target either: {targets:?}"
    );
}

#[test]
fn hovering_a_grid_hex_selects_it_and_a_second_click_confirms() {
    let layout = grid_layout();
    let mut model = GridSelection::new(grid_summaries(4));
    let targets = pointer::grid_targets(&model, &layout, &|_| placed32());
    assert_eq!(
        model.pointer(&hover((111.0, 142.0)), &targets),
        vec![Event::Moved]
    );
    assert_eq!(model.index(), 1);
    // A tap that lands on the tier already selected confirms rather than
    // reselecting it - the two-tap idiom `oag_ui::picker::pointer` uses.
    assert_eq!(
        model.pointer(&click((111.0, 142.0)), &targets),
        vec![Event::Confirmed]
    );
}

/// The regression this pass exists for: two neighbouring hexes' bounding
/// boxes overlap by a couple of pixels at a corner (`[65,145,32,32]` and
/// `[95,126,32,32]` here), and a point in that sliver must resolve to
/// whichever hexagon actually contains it - never to whichever hex a box
/// test or list order happens to favour.
#[test]
fn a_point_in_two_hexes_overlapping_bounding_boxes_picks_the_one_that_actually_contains_it() {
    let layout = grid_layout();
    let targets = pointer::grid_targets(&GridSelection::new(grid_summaries(4)), &layout, &|_| {
        placed32()
    });
    let hex0 = targets
        .iter()
        .find(|t| t.what == pointer::What::Hex(0))
        .unwrap();
    let hex1 = targets
        .iter()
        .find(|t| t.what == pointer::What::Hex(1))
        .unwrap();
    let point = (96.85, 145.0);
    assert!(
        crate::pointer::contains(hex0.rect, point) && crate::pointer::contains(hex1.rect, point),
        "both hexes' bounding boxes must contain this point for the test to \
         mean anything: {hex0:?} {hex1:?}"
    );
    let hit = pointer::hit(&targets, point);
    assert_eq!(
        hit.map(|t| t.what),
        Some(pointer::What::Hex(1)),
        "hex 0's own hexagon does not reach this point even though its box does"
    );
}

#[test]
fn a_point_in_the_gap_between_two_grid_hexes_picks_nothing() {
    let layout = grid_layout();
    let targets = pointer::grid_targets(&GridSelection::new(grid_summaries(4)), &layout, &|_| {
        placed32()
    });
    assert_eq!(pointer::hit(&targets, (96.0, 152.0)), None);
}

#[test]
fn clicking_the_paging_arrow_steps_one_tier_and_the_secondary_button_backs_out() {
    let layout = grid_layout_with_arrows();
    let mut model = GridSelection::new(grid_summaries(4));
    let targets = pointer::grid_targets(&model, &layout, &|_| placed32());
    let up = targets
        .iter()
        .find(|t| t.what == pointer::What::Previous)
        .expect("the up arrow is a target");
    assert_eq!(
        model.pointer(&click(hex_centre(up.rect)), &targets),
        vec![Event::Moved]
    );
    assert_eq!(model.index(), 3, "up from tier 0 wraps to the last tier");
    let back = Pointer {
        back: true,
        ..Pointer::default()
    };
    assert_eq!(model.pointer(&back, &targets), vec![Event::Back]);
}

#[test]
fn cell_selections_hexes_sit_at_their_own_grid_coords_position() {
    let layout = cell_layout();
    let cells = vec![
        race_cell("grid0_0_0", "16_Track"),
        race_cell("grid0_0_1", "16_Track"),
        race_cell("grid0_1_0", "16_Track"),
    ];
    let model = CellSelection::new(cells);
    let targets = pointer::cell_targets(&model, &layout, &|_| placed32());
    let rect_of = |index: usize| {
        targets
            .iter()
            .find(|t| t.what == pointer::What::Hex(index))
            .unwrap_or_else(|| panic!("cell {index}"))
            .rect
    };
    assert_eq!(rect_of(0), [35.0, 40.0, 32.0, 32.0]);
    assert_eq!(rect_of(1), [35.0, 78.0, 32.0, 32.0]);
    assert_eq!(rect_of(2), [65.0, 59.0, 32.0, 32.0]);
}

#[test]
fn hovering_a_cell_selects_it_and_a_second_click_confirms() {
    let layout = cell_layout();
    let cells = vec![
        race_cell("grid0_0_0", "16_Track"),
        race_cell("grid0_0_1", "16_Track"),
        race_cell("grid0_1_0", "16_Track"),
    ];
    let mut model = CellSelection::new(cells);
    let targets = pointer::cell_targets(&model, &layout, &|_| placed32());
    assert_eq!(
        model.pointer(&hover((81.0, 75.0)), &targets),
        vec![Event::Moved]
    );
    assert_eq!(model.selected().unwrap().name, "grid0_1_0");
    assert_eq!(
        model.pointer(&click((81.0, 75.0)), &targets),
        vec![Event::Confirmed]
    );
}

#[test]
fn a_point_in_the_gap_between_two_cells_picks_nothing() {
    let layout = cell_layout();
    let cells = vec![
        race_cell("grid0_0_0", "16_Track"),
        race_cell("grid0_1_0", "16_Track"),
    ];
    let model = CellSelection::new(cells);
    let targets = pointer::cell_targets(&model, &layout, &|_| placed32());
    let hex0 = targets
        .iter()
        .find(|t| t.what == pointer::What::Hex(0))
        .unwrap();
    let hex1 = targets
        .iter()
        .find(|t| t.what == pointer::What::Hex(1))
        .unwrap();
    let point = (66.0, 65.0);
    assert!(
        crate::pointer::contains(hex0.rect, point) && crate::pointer::contains(hex1.rect, point),
        "both cells' bounding boxes must contain this point for the test to \
         mean anything: {hex0:?} {hex1:?}"
    );
    assert_eq!(pointer::hit(&targets, point), None);
}

#[test]
fn a_click_while_cell_help_is_open_closes_it_rather_than_confirming_the_cell_underneath() {
    let layout = cell_layout();
    let cells = vec![race_cell("grid0_0_0", "16_Track")];
    let mut model = CellSelection::new(cells);
    let mut input = Input::new();
    press(&mut input, Button::Triangle);
    assert_eq!(model.update(&mut input), vec![Event::Help]);
    assert!(model.help_open());
    let targets = pointer::cell_targets(&model, &layout, &|_| placed32());
    assert_eq!(
        model.pointer(&click((51.0, 56.0)), &targets),
        vec![Event::Help]
    );
    assert!(!model.help_open());
}

/// The regression this pass fixes: HD's `CellMode_Definition.xml` authors a
/// `Medal_{x}_{y}` widget for every grid slot, occupied or not, sourcing
/// `Hexmedal_HD.mip` - a shared multi-colour atlas, not a single hex - with
/// no `width`/`height` of its own either. `hex_rect` used to try `Medal_`
/// before `Outline_`, so it returned the whole atlas's size as "the hex's
/// rect" for every HD cell, sending `Selector`'s own centred position miles
/// from the hex it marks - the floating hex outline
/// `docs/ui/campaign-screens.md`'s "An open cell-grid artifact" section
/// found. `Outline_` is always single-hex-sized on both titles; this pins
/// `hex_rect` preferring it over an oversized `Medal_` at the same slot.
#[test]
fn hex_rect_prefers_outline_over_an_oversized_medal_atlas() {
    let outline = Image {
        name: Some("Outline_0_0".to_string()),
        src: r"Data\FE\Images\Hexagon_HD.mip".to_string(),
        x: 350.0,
        y: 332.0,
        width: None,
        height: None,
        centred: false,
        color: 0xffff_ffff,
        u: None,
        v: None,
        texture_width: None,
        texture_height: None,
        auto_load: false,
    };
    let medal = Image {
        name: Some("Medal_0_0".to_string()),
        src: r"Data\FE\Images\Hexmedal_HD.mip".to_string(),
        ..outline.clone()
    };
    let screen = Screen {
        name: "Cell Selection".to_string(),
        images: vec![medal, outline],
        ..Screen::default()
    };
    let sprites = |src: &str| -> Option<crate::frontend::Placed> {
        if src == r"Data\FE\Images\Hexmedal_HD.mip" {
            Some(crate::frontend::Placed {
                x: 0,
                y: 738,
                width: 1024,
                height: 256,
                quad_extent: None,
                blend: None,
            })
        } else {
            placed32()
        }
    };
    assert_eq!(
        hex_rect(&screen, 0, 0, &sprites),
        Some([350.0, 332.0, 32.0, 32.0]),
        "must resolve to Outline_0_0's own 32x32 hex, not Medal_0_0's 1024x256 atlas"
    );
}
