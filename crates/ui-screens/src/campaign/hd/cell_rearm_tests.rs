//! `Cell Selection`'s four emblems replay their wipe when the cell or the rung changes, drawn from
//! the whole page's draw list.

use super::*;
use oag_gameplay::input::{Button, Input};
use oag_tables::race_campaign::Mode;
use oag_ui::language::StringTable;
use oag_ui::menu::{Frame, Skin};
use oag_ui::screen::{Screen, Slot, Vwipe};

fn cell(name: &str) -> Cell {
    Cell {
        name: name.to_string(),
        track: Some("16_Track".to_string()),
        mode: Mode::Race,
        class: "Venom".to_string(),
        weapons: true,
        damage: true,
        locked: None,
        status: None,
        ai_count: None,
        skill: None,
        skill_easy: None,
        skill_hard: None,
        laps: Some(3),
        ship: None,
        ship_choice: None,
        gold: 1,
        silver: 2,
        bronze: 3,
        tournament_tracks: Vec::new(),
        difficulty_targets: None,
        nitro_elimination_targets: None,
    }
}

/// How many emblems the page draws, by their 96 by 96 slots' icon sprites.
fn emblems_drawn(model: &CellSelection) -> usize {
    let slots = [
        ("Event Emblem", 0.05),
        ("Track Emblem", 0.125),
        ("Speed Class Emblem", 0.2),
        ("Weapons Emblem", 0.275),
    ]
    .into_iter()
    .map(|(name, delay)| Slot {
        name: Some(name.to_string()),
        x: 10.0,
        y: 20.0,
        width: Some(96.0),
        height: Some(96.0),
        wipe: Some(Vwipe {
            seconds: 0.3,
            delay,
        }),
    })
    .collect();
    page_body(
        model,
        Screen {
            slots,
            ..Screen::default()
        },
    )
    .iter()
    .filter(|draw| matches!(draw, Draw::Sprite { rect, .. } if rect[0] == 10.0 && rect[2] == 96.0))
    .count()
}

/// The page's body draws for `screen`, every sprite the sheet is asked for placed 96 by 96.
fn page_body(model: &CellSelection, screen: Screen) -> Vec<Draw> {
    let layout = Layout {
        screen,
        scale: [1.0, 1.0],
        faces: crate::picker::FaceScales::default(),
        labels: Vec::new(),
    };
    let skin = Skin::new(
        oag_hd::frontend::MENU_SKIN,
        oag_display::space::Space::HD,
        33.0,
    );
    let sprites = |src: &str| {
        (src.ends_with("_bw.gtf") || src.ends_with("corner2.gtf")).then_some(Placed {
            x: 0,
            y: 0,
            width: 96,
            height: 96,
            quad_extent: None,
            quad_centre: None,
            blend: None,
            scale: [1.0, 1.0],
        })
    };
    let layers = hd_cell_draw_list(
        model,
        &layout,
        &skin,
        &Frame::default(),
        &StringTable::default(),
        &CircuitNames::default(),
        0,
        8,
        &GridSummary::empty(),
        &CellArt {
            next_flyer: None,
            fury: true,
            medal_tier_rows: [0, 1, 2],
            track_emblems: &std::collections::HashMap::from([(
                "16_track".to_string(),
                r"Data\FE\Images\trackselectemblem_bw.gtf".to_string(),
            )]),
        },
        None,
        false,
        &sprites,
        &[],
    );
    layers.body
}

fn page_up_for(seconds: f32) -> CellSelection {
    let mut model = CellSelection::new(vec![cell("grid8_0_0"), cell("grid8_1_0")]);
    model.tick(seconds);
    model
}

fn press(model: &mut CellSelection, button: Button) {
    let mut input = Input::default();
    input.begin_frame(0);
    input.begin_frame(1 << button as u32);
    model.update(&mut input);
}

/// A page up for 2 s shows no emblem at the same instant a step moves the cursor, and brings
/// them back in order from there. This fails if the draw ignores [`CellSelection::refreshed_at`].
#[test]
fn a_cell_step_replays_the_emblems_wipes() {
    let mut model = page_up_for(2.0);
    assert_eq!(emblems_drawn(&model), 4);
    press(&mut model, Button::Right);
    assert_eq!(emblems_drawn(&model), 0, "all four wait their delay again");
    model.tick(0.1);
    assert_eq!(
        emblems_drawn(&model),
        1,
        "the event emblem is the first back"
    );
    model.tick(0.5);
    assert_eq!(emblems_drawn(&model), 4);
}

/// The `Square` rung step runs the same refresh.
#[test]
fn a_rung_step_replays_the_emblems_wipes_too() {
    let mut model = page_up_for(2.0);
    model.cycle_difficulty();
    assert_eq!(emblems_drawn(&model), 0);
    model.tick(0.7);
    assert_eq!(emblems_drawn(&model), 4);
}

/// A still draws the finished page, and a step on it does not wipe anything.
#[test]
fn a_finished_still_does_not_replay() {
    let mut model = CellSelection::new(vec![cell("grid8_0_0"), cell("grid8_1_0")]);
    *model.entry_mut() = oag_ui::screen::EntryClock::finished();
    press(&mut model, Button::Right);
    assert_eq!(emblems_drawn(&model), 4);
}

const BRACKET_FIXTURE: &str = r#"
<Screen>
  <Screen name="Cell Selection">
    <Image name="Event Emblem" OffsetX="750" OffsetY="315" EnableTransition="0.3" delay="0.05">
      <Values transitiontype="vwipe" x="4" y="0" width="96" height="96"></Values>
      <Bracket delay="0.1"><Values corner="true" x="0" y="0" Width="100" Height="96" Colour="0xFF646464"></Values></Bracket>
    </Image>
    <Text name="Target Title" OffsetX="750" OffsetY="810" transition="0" delay="0.6">
      <Values x="30" y="13" string="TARGET"></Values>
      <Item OffsetX="0" OffsetY="50">
        <Bracket delay="0.625"><Values corner="true" x="0" y="0" Width="100" Height="60" Colour="0xFF646464"></Values></Bracket>
      </Item>
    </Text>
    <Item name="RightColumnText" OffsetX="1286" OffsetY="315">
      <Bracket delay="0.3"><Values corner="true" x="-16" y="0" Width="415" Height="486" Colour="0xFF646464"></Values></Bracket>
    </Item>
  </Screen>
</Screen>
"#;

/// The top-left corner of each 16 by 16 mark the page draws, by the bracket's own left edge.
fn bracket_marks(model: &CellSelection) -> Vec<f32> {
    let screens = oag_ui::screen::Screens::from_xml(BRACKET_FIXTURE);
    let screen = screens
        .by_name("Cell Selection")
        .expect("the screen")
        .clone();
    page_body(model, screen)
        .iter()
        .filter_map(|draw| match draw {
            Draw::Sprite { rect, .. } if rect[2] == 16.0 && rect[3] == 16.0 => Some(rect[0]),
            _ => None,
        })
        .collect()
}

/// A bracket inside a re-armed widget counts its `delay` from the arming, the entry hold and each
/// change included; one inside no re-armed widget counts from the page's entry. Dropping the
/// `group` split, or the delay, fails this: the page would draw every mark from frame 0, or the
/// emblem's bracket would not replay at a step.
#[test]
fn a_bracket_in_a_rearmed_widget_replays_with_it() {
    let at = |seconds: f32| {
        let mut model = CellSelection::new(vec![cell("grid8_0_0"), cell("grid8_1_0")]);
        model.tick(seconds);
        model
    };
    assert!(bracket_marks(&at(0.0)).is_empty(), "nothing at frame 0");
    // The hold ends at 0.37 s; the emblem's bracket (0.1 s) is out from 0.47 s, the column's
    // (0.3 s) from 0.67 s. The target row's (0.625 s) counts from the entry, with no hold.
    assert!(
        bracket_marks(&at(0.4)).is_empty(),
        "the hold is over, the delays not"
    );
    assert_eq!(
        bracket_marks(&at(0.5)).len(),
        4,
        "the emblem's marks are out"
    );
    assert_eq!(bracket_marks(&at(0.6)).len(), 4);
    assert_eq!(
        bracket_marks(&at(0.65)).len(),
        8,
        "the target row's, 0.625 s in"
    );
    assert_eq!(bracket_marks(&at(0.7)).len(), 12, "and the column's");
    // Past the opening, a cell step restarts the emblem's and the column's, and the row's stays.
    let mut model = at(2.0);
    assert_eq!(bracket_marks(&model).len(), 12);
    press(&mut model, Button::Right);
    assert_eq!(
        bracket_marks(&model).len(),
        4,
        "only the target row's stays"
    );
    model.tick(0.15);
    assert_eq!(bracket_marks(&model).len(), 8, "the emblem's is back");
    model.tick(0.3);
    assert_eq!(bracket_marks(&model).len(), 12);
    // A still with no stated time is the finished page.
    let mut still = CellSelection::new(vec![cell("grid8_0_0")]);
    *still.entry_mut() = oag_ui::screen::EntryClock::finished();
    assert_eq!(bracket_marks(&still).len(), 12);
}
