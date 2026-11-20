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
    let layout = Layout {
        screen: Screen {
            slots,
            ..Screen::default()
        },
        scale: [1.0, 1.0],
        faces: crate::picker::FaceScales::default(),
    };
    let skin = Skin::new(
        oag_hd::frontend::MENU_SKIN,
        oag_display::space::Space::HD,
        33.0,
    );
    let sprites = |src: &str| {
        src.ends_with("_bw.gtf").then_some(Placed {
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
    layers
        .body
        .iter()
        .filter(
            |draw| matches!(draw, Draw::Sprite { rect, .. } if rect[0] == 10.0 && rect[2] == 96.0),
        )
        .count()
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
