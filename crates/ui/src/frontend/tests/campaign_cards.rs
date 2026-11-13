//! The season cards on the 2048 campaign map: which are shown, what they hold
//! in the canvas's priority order, and that each draws as its baked picture.

use super::wipeout2048::{XML, boot_on, reach_the_shell, three_events};
use super::*;
use crate::frontend::{canvas_cells, canvas_lattice};

const LABELS: &str = r#"<TouchScroll name="FeShellTouchScroll"><FE3DCanvas>
<CanvasLabel name="2048SeasonText"><Values x="51" y="29" u="0.0" v="0.478515625" width="14" height="3"></Values></CanvasLabel>
<CanvasLabel name="2049SeasonText"><Values x="96" y="39" u="0.0" v="0.59814453125 " width="14" height="3" linkedevent="2049 - Event 1"></Values></CanvasLabel>
<CanvasLabel name="Trophy-2048-2-3"><Values x="57" y="40" u=" 0.0" v="0.4384765625" width="2" height="1" linkedevent="2048 - Event 2"></Values></CanvasLabel>
</FE3DCanvas></TouchScroll>"#;

fn on_the_map() -> Frontend {
    let xml = XML.replacen(
        "<Screen name=\"GameModeChoice\"",
        &format!("{LABELS}<Screen name=\"GameModeChoice\""),
        1,
    );
    let mut frontend = boot_on(&xml, 0);
    frontend.set_campaign(three_events());
    let mut input = Input::new();
    reach_the_shell(&mut frontend, &mut input);
    frontend
}

fn names(frontend: &Frontend) -> Vec<String> {
    frontend
        .season_cards()
        .into_iter()
        .map(|card| card.name)
        .collect()
}

#[test]
fn a_card_with_no_linked_event_shows_and_one_waits_for_its_event_to_be_open() {
    let mut frontend = on_the_map();
    assert_eq!(
        names(&frontend),
        ["2048SeasonText"],
        "2049's first event is locked; a trophy is not a season card"
    );
    frontend.refresh_campaign_progress(|name| {
        matches!(name, "2048 - Event 1" | "2048 - Event 2").then_some(EarnedTier::Pass)
    });
    assert_eq!(
        names(&frontend),
        ["2048SeasonText", "2049SeasonText"],
        "2049's first event is open once 2048's is passed"
    );
}

#[test]
fn a_shown_card_draws_as_its_baked_picture_and_holds_cells_above_the_wave() {
    let mut frontend = on_the_map();
    let card = frontend.season_cards().remove(0);
    let size = canvas_cells::card_size(&card);
    frontend.placements.push((
        canvas_cells::card_name(&card),
        Placed {
            x: 5000,
            y: 0,
            width: size.0,
            height: size.1,
            quad_extent: None,
            quad_centre: None,
            blend: None,
            scale: [1.0, 1.0],
        },
    ));
    frontend.placements.push((
        canvas_lattice::TILE.to_string(),
        Placed {
            x: 0,
            y: 0,
            width: 960,
            height: 544,
            quad_extent: None,
            quad_centre: None,
            blend: None,
            scale: [1.0, 1.0],
        },
    ));
    let drawn = |frontend: &Frontend| {
        frontend
            .draw_list()
            .iter()
            .filter(|draw| matches!(draw, Draw::Sprite { uv, .. } if uv[0] == 5001.0))
            .count()
    };
    assert_eq!(drawn(&frontend), 1, "one sprite for the whole card");
    let priorities = canvas_cells::priorities(&frontend.canvas_buttons(), &frontend.season_cards());
    assert_eq!(
        priorities.get(&(60, 30)),
        Some(&canvas_cells::CARD_PRIORITY),
        "a card cell is not below the wave's threshold"
    );
}
