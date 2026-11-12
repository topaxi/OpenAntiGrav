//! Finding a hidden prototype event: a click or Confirm on one of a
//! revealable spot's cells finds it, the spot moves as one flat piece with the
//! music wave until then, and an unmet gate leaves ordinary ground.

use super::wipeout2048::{boot, press, reach_the_shell, three_events, tick};
use super::*;
use crate::frontend::canvas_camera::{self, Camera};
use crate::frontend::canvas_cells::HIDDEN_SHAPE;
use crate::frontend::canvas_lattice::{CELL_DX, CELL_DY};
use crate::frontend::{Reveal, canvas_cells, canvas_lattice};
use crate::pointer::Pointer;

const SPOT: &str = "2048 - P Ship Challenge";
/// The spot's first cell: a few cells right and below Event 1, inside the
/// wave's radius and clear of every ring.
const FIRST: (i32, i32) = (65, 40);

fn spot_cells() -> [(i32, i32); 4] {
    HIDDEN_SHAPE.map(|(dc, dr)| (FIRST.0 + dc, FIRST.1 + dr))
}

fn on_the_map(reveal: Reveal) -> (Frontend, Input) {
    let mut events = three_events();
    let mut spot = events[0].clone();
    spot.name = SPOT.to_string();
    spot.x = FIRST.0 as f32;
    spot.y = FIRST.1 as f32;
    spot.requires = None;
    spot.rank_gated = true;
    spot.rank_required = Some(10);
    events.push(spot);
    let mut frontend = boot(0);
    frontend.set_campaign(events);
    frontend.refresh_campaign_progress(|name| {
        (name == "2048 - Event 1").then_some(EarnedTier::Pass)
    });
    frontend.refresh_campaign_reveals(|event| {
        if event.name == SPOT { reveal } else { Reveal::Hidden }
    });
    let mut input = Input::new();
    reach_the_shell(&mut frontend, &mut input);
    let mut put = |name: &str, x: u32, size: (u32, u32)| {
        frontend.placements.push((
            name.to_string(),
            Placed {
                x,
                y: 0,
                width: size.0,
                height: size.1,
                quad_extent: None,
                quad_centre: None,
                blend: None,
                scale: [1.0, 1.0],
            },
        ));
    };
    put(canvas_lattice::TILE, 0, (960, 544));
    for fill in 0..=4_u8 {
        for up in [true, false] {
            put(
                canvas_cells::name(fill, up),
                1000 + u32::from(fill) * 200 + if up { 0 } else { 100 },
                canvas_cells::layout(up).size,
            );
        }
    }
    (frontend, input)
}

/// The grid point at the middle of `cell`, where the view is now.
fn at_cell(frontend: &Frontend, cell: (i32, i32)) -> (f32, f32) {
    let camera = Camera::scrolled(frontend.campaign.scroll());
    let on = camera.project([cell.0 as f32 * CELL_DX, cell.1 as f32 * CELL_DY, 0.0]);
    let (width, height) = frontend.space().size;
    (
        on.0 * width / canvas_camera::VIEWPORT.0,
        on.1 * height / canvas_camera::VIEWPORT.1,
    )
}

fn click(at: (f32, f32)) -> Pointer {
    Pointer {
        at: Some(at),
        moved: true,
        clicked: true,
        ..Pointer::default()
    }
}

fn spot_index(frontend: &Frontend) -> usize {
    frontend
        .campaign_events()
        .iter()
        .position(|event| event.name == SPOT)
        .expect("the spot is on the map")
}

#[test]
fn a_click_on_a_revealable_spot_finds_it_and_it_becomes_the_grey_prism() {
    let (mut frontend, mut input) = on_the_map(Reveal::Revealable);
    let index = spot_index(&frontend);
    assert!(frontend.campaign.hidden(index));
    for cell in spot_cells() {
        let (mut frontend, _) = on_the_map(Reveal::Revealable);
        let at = at_cell(&frontend, cell);
        assert!(frontend.pointer(&click(at)));
        assert_eq!(frontend.take_found_events(), vec![SPOT.to_string()], "{cell:?}");
    }
    let at = at_cell(&frontend, spot_cells()[3]);
    frontend.pointer(&click(at));
    tick(&mut frontend, &mut input, 0);
    assert_eq!(frontend.take_found_events(), vec![SPOT.to_string()]);
    assert!(!frontend.campaign.hidden(index), "found: shown as a tile");
    assert_eq!(frontend.selected_event().map(|e| e.name.as_str()), Some(SPOT));
    assert_eq!(
        frontend.campaign_event_state(SPOT),
        Some(ProgressState::Open)
    );
    assert!(!frontend.event_card_open(), "finding does not open the card");
    frontend.pointer(&click(at_cell(&frontend, spot_cells()[0])));
    assert!(frontend.take_found_events().is_empty(), "found once");
}

#[test]
fn a_click_on_an_unmet_spot_or_on_plain_ground_does_nothing() {
    let (mut frontend, mut input) = on_the_map(Reveal::Hidden);
    let index = spot_index(&frontend);
    for cell in spot_cells() {
        frontend.pointer(&click(at_cell(&frontend, cell)));
        tick(&mut frontend, &mut input, 0);
    }
    assert!(frontend.take_found_events().is_empty());
    assert!(frontend.campaign.hidden(index));

    let (mut frontend, _) = on_the_map(Reveal::Revealable);
    frontend.pointer(&click(at_cell(&frontend, (FIRST.0 + 6, FIRST.1))));
    assert!(frontend.take_found_events().is_empty(), "ground beside the spot");
}

#[test]
fn confirm_on_the_cursor_over_a_revealable_spot_finds_it() {
    let (mut frontend, mut input) = on_the_map(Reveal::Revealable);
    frontend.campaign.cursor.at = Some(at_cell(&frontend, spot_cells()[1]));
    press(&mut frontend, &mut input, Button::Cross);
    assert_eq!(frontend.take_found_events(), vec![SPOT.to_string()]);
    assert!(!frontend.event_card_open());
}

#[test]
fn the_pad_reaches_a_found_event_and_skips_an_unfound_one() {
    let (mut frontend, mut input) = on_the_map(Reveal::Revealable);
    frontend.select_campaign_event("2048 - Event 1");
    press(&mut frontend, &mut input, Button::Down);
    assert_ne!(
        frontend.selected_event().map(|e| e.name.as_str()),
        Some(SPOT),
        "unfound"
    );

    let (mut frontend, mut input) = on_the_map(Reveal::Found);
    frontend.select_campaign_event("2048 - Event 1");
    press(&mut frontend, &mut input, Button::Down);
    assert_eq!(
        frontend.selected_event().map(|e| e.name.as_str()),
        Some(SPOT),
        "found"
    );
}

fn bass(level: f32) -> Option<[f32; 12]> {
    let mut levels = [0.0; 12];
    levels[1] = level;
    Some(levels)
}

/// The largest height difference inside the spot's four cells over a run of a
/// beating bass.
fn spread_over_a_run(reveal: Reveal) -> f32 {
    let (mut frontend, mut input) = on_the_map(reveal);
    let mut worst = 0.0_f32;
    let mut lifted = 0.0_f32;
    for step in 0..240 {
        frontend.set_music_levels(bass(if (step / 15) % 2 == 0 { 0.9 } else { 0.1 }));
        tick(&mut frontend, &mut input, 0);
        let heights = spot_cells().map(|cell| frontend.campaign.wave.height(cell));
        let (low, high) = heights
            .iter()
            .fold((f32::MAX, f32::MIN), |(l, h), &v| (l.min(v), h.max(v)));
        worst = worst.max(high - low);
        lifted = lifted.max(high);
    }
    assert!(lifted > 0.05, "the wave reaches the spot ({lifted})");
    worst
}

#[test]
fn a_revealable_spot_rises_and_falls_as_one_flat_piece_and_an_unmet_one_does_not() {
    let flat = spread_over_a_run(Reveal::Revealable);
    let ground = spread_over_a_run(Reveal::Hidden);
    assert!(
        flat < ground * 0.5,
        "flattened spread {flat} against ground {ground}"
    );
}
