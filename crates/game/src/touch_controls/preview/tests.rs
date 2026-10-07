use super::*;
use oag_display::space::Space;

const PSP: Space = Space::PSP;

fn setups() -> [Setup; 4] {
    [
        Setup::new(Scheme::Standard, true),
        Setup::new(Scheme::Standard, false),
        Setup::new(Scheme::Easy, true),
        Setup::new(Scheme::Easy, false),
    ]
}

fn words(list: &[Draw]) -> Vec<String> {
    let mut words: Vec<String> = list
        .iter()
        .filter_map(|d| match d {
            Draw::Text { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect();
    words.sort();
    words
}

#[test]
fn every_pose_draws_with_the_setup_it_is_offered_for() {
    for setup in setups() {
        for demo in poses(setup) {
            assert_eq!(demo.setup(), setup, "{demo:?}");
        }
    }
}

#[test]
fn the_demo_starts_idle_and_walks_the_poses() {
    let setup = Setup::new(Scheme::Standard, true);
    assert_eq!(pose_at(setup, 0.0), Demo::Idle);
    assert_eq!(pose_at(setup, HOLD * 1.5), Demo::GoLeft);
    assert_eq!(pose_at(setup, HOLD * poses(setup).len() as f32), Demo::Idle);
}

#[test]
fn the_panel_sits_inside_the_screen_and_is_phone_shaped() {
    let [x, y, w, h] = panel(PSP);
    assert!(x >= 0.0 && y >= 0.0 && x + w <= 480.0 && y + h <= 272.0);
    assert!((w / h - PHONE.0 / PHONE.1).abs() < 1e-3);
}

#[test]
fn the_preview_follows_the_scheme_and_the_zones() {
    let art = Art::flat();
    let list = |setup| draw(&art, PSP, setup, 1.0, 0.0);
    let standard = list(Setup::new(Scheme::Standard, true));
    let easy = list(Setup::new(Scheme::Easy, true));
    let buttons = list(Setup::new(Scheme::Standard, false));
    assert_ne!(standard, easy);
    assert_ne!(standard, buttons);
    assert_eq!(words(&standard), ["GO", "L", "R"]);
}

#[test]
fn the_preview_follows_the_opacity() {
    let art = Art::flat();
    let setup = Setup::new(Scheme::Standard, true);
    assert_ne!(
        draw(&art, PSP, setup, 1.0, 0.0),
        draw(&art, PSP, setup, 0.4, 0.0)
    );
}

#[test]
fn a_held_pose_differs_from_the_idle_one() {
    let art = Art::flat();
    let setup = Setup::new(Scheme::Standard, true);
    assert_ne!(
        draw(&art, PSP, setup, 1.0, 0.0),
        draw(&art, PSP, setup, 1.0, HOLD * 1.1)
    );
}

#[test]
fn only_the_touch_page_has_a_preview() {
    let art = Art::flat();
    let controls = crate::settings::Controls::default();
    assert!(for_page("controls", &art, PSP, &controls, 0.0).is_empty());
    assert!(!for_page(PAGE, &art, PSP, &controls, 0.0).is_empty());
}
