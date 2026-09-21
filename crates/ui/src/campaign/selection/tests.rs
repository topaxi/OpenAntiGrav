//! `CampaignSelection`'s own stepping/confirm/pointer behaviour, on the same
//! synthetic-fixture terms `crates/ui/src/campaign/tests.rs` already gives
//! `GridSelection`/`CellSelection`. See the module doc for the RPCS3
//! measurements this model is built from.

use super::*;
use crate::pointer::Pointer;
use oag_gameplay::input::{Button, Input};

fn press(input: &mut Input, button: Button) {
    input.begin_frame(0);
    input.begin_frame(1 << button as u32);
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

#[test]
fn the_default_selection_is_fury_matching_the_measured_default() {
    let model = CampaignSelection::new();
    assert_eq!(model.selected(), Campaign::Fury);
}

#[test]
fn right_moves_from_fury_to_the_base_hd_campaign() {
    let mut model = CampaignSelection::new();
    let mut input = Input::default();
    press(&mut input, Button::Right);
    let events = model.update(&mut input);
    assert_eq!(events, vec![Event::Moved]);
    assert_eq!(model.selected(), Campaign::Hd);
}

#[test]
fn left_at_fury_is_a_no_op_clamped_not_wrapping() {
    let mut model = CampaignSelection::new();
    let mut input = Input::default();
    press(&mut input, Button::Left);
    model.update(&mut input);
    assert_eq!(
        model.selected(),
        Campaign::Fury,
        "left at the list's own first entry must not wrap to Hd - measured on RPCS3"
    );
}

#[test]
fn right_at_hd_is_also_clamped() {
    let mut model = CampaignSelection::new();
    let mut input = Input::default();
    press(&mut input, Button::Right);
    model.update(&mut input);
    assert_eq!(model.selected(), Campaign::Hd);
    let mut input = Input::default();
    press(&mut input, Button::Right);
    model.update(&mut input);
    assert_eq!(model.selected(), Campaign::Hd, "no third entry to move to");
}

#[test]
fn cross_confirms_and_circle_backs_out() {
    let mut model = CampaignSelection::new();
    let mut input = Input::default();
    press(&mut input, Button::Cross);
    assert_eq!(model.update(&mut input), vec![Event::Confirmed]);

    let mut input = Input::default();
    press(&mut input, Button::Circle);
    assert_eq!(model.update(&mut input), vec![Event::Back]);
}

#[test]
fn each_campaign_grid_range_and_screen_name_matches_the_disc() {
    assert_eq!(
        Campaign::Fury.grid_range(),
        oag_hd::campaign::FURY_GRID_RANGE
    );
    assert_eq!(Campaign::Fury.grid_screen_name(), "Grid Selection Fury");
    assert_eq!(Campaign::Hd.grid_range(), oag_hd::campaign::HD_GRID_RANGE);
    assert_eq!(Campaign::Hd.grid_screen_name(), "Grid Selection");
}

#[test]
fn a_click_on_the_right_half_selects_hd_then_confirms_on_a_second_click() {
    let mut model = CampaignSelection::new();
    let right_half = (1500.0, 500.0);
    assert_eq!(model.pointer(&hover(right_half)), vec![Event::Moved]);
    assert_eq!(model.selected(), Campaign::Hd);
    assert_eq!(model.pointer(&click(right_half)), vec![Event::Confirmed]);
}

#[test]
fn a_click_on_the_already_selected_left_half_confirms_fury_directly() {
    let mut model = CampaignSelection::new();
    let left_half = (400.0, 500.0);
    assert_eq!(model.pointer(&click(left_half)), vec![Event::Confirmed]);
}

#[test]
fn pointer_back_fires_regardless_of_where_the_pointer_is() {
    let mut model = CampaignSelection::new();
    let back = Pointer {
        back: true,
        ..Pointer::default()
    };
    assert_eq!(model.pointer(&back), vec![Event::Back]);
}
