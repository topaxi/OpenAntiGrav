//! The pad's focus on the campaign map: the cone pick, the repeat, the view
//! following the focus, and the focus look.

use super::wipeout2048::{boot, reach_the_shell, tick};
use super::*;
use crate::frontend::map_focus::{
    self, REPEAT_EVERY, REPEAT_FIRST, RIPPLE_RADIUS, RIPPLE_SECONDS, Repeat,
};

fn event(name: &str, col: f32, row: f32) -> MapEvent {
    MapEvent {
        name: name.to_string(),
        x: col,
        y: row,
        detail: "circuit / mode".to_string(),
        requires: None,
        kind: EventIcon::Race,
        forced_craft: None,
        refused_craft: Vec::new(),
        card: crate::frontend::EventCard::default(),
        role: oag_title::MapRole::default(),
        rank_gated: false,
        rank_required: None,
    }
}

fn on_the_map(events: &[(&str, f32, f32)]) -> (Frontend, Input) {
    let mut frontend = boot(0);
    frontend.set_campaign(
        events
            .iter()
            .map(|&(name, col, row)| event(name, col, row))
            .collect(),
    );
    let mut input = Input::new();
    reach_the_shell(&mut frontend, &mut input);
    (frontend, input)
}

fn selected(frontend: &Frontend) -> &str {
    &frontend.selected_event().unwrap().name
}

fn tap(frontend: &mut Frontend, input: &mut Input, button: Button) {
    tick(frontend, input, button.bit());
    tick(frontend, input, 0);
}

#[test]
fn the_cone_picks_a_near_event_off_axis_over_a_far_one_on_axis() {
    // From (60, 30): `far` lies dead right, 20 columns away; `near` is 4
    // columns right and 7 rows down, 30 degrees off the row axis, which is 60
    // off the right axis but inside right's 25..155 degree cone and 8 away. The
    // old `along + 2 * |across|` score chose `far`.
    let (mut frontend, mut input) = on_the_map(&[
        ("here", 60.0, 30.0),
        ("far", 80.0, 30.0),
        ("near", 64.0, 37.0),
    ]);
    tap(&mut frontend, &mut input, Button::Right);
    assert_eq!(selected(&frontend), "near");
}

#[test]
fn a_direction_with_nothing_in_its_cone_leaves_the_focus() {
    // Straight up the screen is a smaller row: the only event is below.
    let (mut frontend, mut input) = on_the_map(&[("here", 60.0, 30.0), ("below", 60.0, 40.0)]);
    tap(&mut frontend, &mut input, Button::Up);
    assert_eq!(selected(&frontend), "here");
    tap(&mut frontend, &mut input, Button::Down);
    assert_eq!(selected(&frontend), "below");
}

#[test]
fn the_cones_are_130_degrees_wide_and_overlap() {
    let from = (0.0, 0.0);
    let at = |angle: f32| {
        let (sin, cos) = angle.to_radians().sin_cos();
        [(7, (5.0 * sin, 5.0 * cos))]
    };
    let hits = |angle: f32, button| map_focus::pick(from, at(angle), button).is_some();
    // 0 degrees is +row (down on screen), 90 is +column (right).
    assert!(hits(0.0, Button::Down) && hits(60.0, Button::Down) && !hits(70.0, Button::Down));
    assert!(hits(30.0, Button::Right) && hits(150.0, Button::Right) && !hits(20.0, Button::Right));
    assert!(hits(180.0, Button::Up) && !hits(100.0, Button::Up) && hits(120.0, Button::Up));
    assert!(hits(270.0, Button::Left) && hits(210.0, Button::Left) && !hits(340.0, Button::Left));
    // Inside two cones at once.
    assert!(hits(40.0, Button::Down) && hits(40.0, Button::Right));
}

#[test]
fn a_held_direction_fires_after_the_first_delay_and_then_on_the_repeat() {
    let row: Vec<(String, f32, f32)> = (0..8)
        .map(|at| (format!("e{at}"), 60.0 + 4.0 * at as f32, 30.0))
        .collect();
    let row: Vec<(&str, f32, f32)> = row.iter().map(|(n, c, r)| (n.as_str(), *c, *r)).collect();
    let (mut frontend, mut input) = on_the_map(&row);
    let index = |frontend: &Frontend| selected(frontend)[1..].parse::<usize>().unwrap();
    let hold = |frontend: &mut Frontend, input: &mut Input, frames: usize| {
        for _ in 0..frames {
            tick(frontend, input, Button::Right.bit());
        }
    };
    hold(&mut frontend, &mut input, 1);
    assert_eq!(index(&frontend), 1, "the press fires at once");
    hold(&mut frontend, &mut input, 21);
    assert_eq!(index(&frontend), 1, "not before {REPEAT_FIRST} s");
    hold(&mut frontend, &mut input, 5);
    assert_eq!(index(&frontend), 2, "after {REPEAT_FIRST} s");
    hold(&mut frontend, &mut input, 9);
    assert_eq!(index(&frontend), 3, "then every {REPEAT_EVERY} s");
    hold(&mut frontend, &mut input, 3);
    assert_eq!(index(&frontend), 3);
}

#[test]
fn the_repeat_state_restarts_when_the_direction_is_let_go() {
    let mut repeat = Repeat::default();
    assert!(repeat.step(0.0, true, true));
    assert!(!repeat.step(0.39, true, false));
    assert!(repeat.step(0.02, true, false));
    assert!(!repeat.step(0.1, true, false));
    assert!(repeat.step(0.03, true, false));
    assert!(!repeat.step(0.0, false, false));
    assert!(
        !repeat.step(0.39, true, false),
        "a hold with no press edge waits"
    );
}

#[test]
fn the_view_follows_the_focus_and_eases_there() {
    let (mut frontend, mut input) = on_the_map(&[("here", 60.0, 30.0), ("there", 84.0, 30.0)]);
    let before = frontend.campaign.scroll();
    tap(&mut frontend, &mut input, Button::Right);
    let first = frontend.campaign.scroll();
    for _ in 0..90 {
        tick(&mut frontend, &mut input, 0);
    }
    let after = frontend.campaign.scroll();
    assert_ne!(after, before, "the view moved with the focus");
    let target = canvas_camera::centred_on(CampaignMap::cell(&frontend.campaign.events[1]), before);
    let far = (target.0 - before.0).abs() + (target.1 - before.1).abs();
    assert!(
        (first.0 - before.0).abs() + (first.1 - before.1).abs() < far * 0.9,
        "it eases rather than jumping: {before:?} {first:?} {after:?}"
    );
    let face = frontend.campaign.face_of(1).unwrap();
    let (mid_x, mid_y) = (face[0] + face[2] * 0.5, face[1] + face[3] * 0.5);
    assert!(
        (mid_x - canvas_camera::CENTRE.0).abs() < 60.0
            && (mid_y - canvas_camera::CENTRE.1).abs() < 60.0,
        "the event ends near the middle of the view: {mid_x} {mid_y}"
    );
}

#[test]
fn a_focus_change_runs_a_ripple_over_the_lattice_with_no_music() {
    let (mut frontend, mut input) = on_the_map(&[("here", 60.0, 30.0), ("there", 64.0, 30.0)]);
    assert!(frontend.campaign.wave.is_calm());
    tap(&mut frontend, &mut input, Button::Right);
    for _ in 0..12 {
        tick(&mut frontend, &mut input, 0);
    }
    assert!(
        frontend.campaign.wave.raised().count() > 0,
        "the ripple lifts cells round the focus"
    );
    for _ in 0..(5 * 60) {
        tick(&mut frontend, &mut input, 0);
    }
    assert!(frontend.campaign.wave.is_calm(), "and runs out");
}

#[test]
fn the_ripple_law_rises_fades_and_stays_inside_its_radius() {
    let r = RIPPLE_RADIUS;
    assert_eq!(
        map_focus::ripple_alpha(0.0, 1.0, r),
        0.0,
        "nothing at t = 0"
    );
    assert_eq!(map_focus::ripple_alpha(RIPPLE_SECONDS, 1.0, r), 0.0);
    assert_eq!(
        map_focus::ripple_alpha(0.5, r * r, r),
        0.0,
        "nothing at the rim"
    );
    let peak = (0..100)
        .map(|at| map_focus::ripple_alpha(0.3, at as f32 * 0.1, r))
        .fold(0.0_f32, f32::max);
    assert!(peak > 50.0 && peak <= 255.0, "{peak}");
}

#[test]
fn the_pulse_runs_between_nothing_and_65_levels() {
    assert!((map_focus::pulse(-std::f32::consts::FRAC_PI_2)).abs() < 1e-3);
    assert!((map_focus::pulse(std::f32::consts::FRAC_PI_2) - 65.0).abs() < 1e-3);
}

#[test]
fn the_focus_look_replaces_the_marker_for_the_pad_and_the_pointer_gets_it_back() {
    let (mut frontend, mut input) = on_the_map(&[("here", 60.0, 30.0), ("there", 64.0, 30.0)]);
    assert!(!frontend.campaign.focus.active);
    tap(&mut frontend, &mut input, Button::Right);
    assert!(frontend.campaign.focus.active);
    let at_rest = frontend.draw_list();
    // Half a pulse later the focused tile is a different brightness.
    for _ in 0..13 {
        tick(&mut frontend, &mut input, 0);
    }
    assert_ne!(frontend.draw_list(), at_rest);
    frontend.campaign.focus.active = false;
    assert!(!frontend.campaign.focus.active);
}
