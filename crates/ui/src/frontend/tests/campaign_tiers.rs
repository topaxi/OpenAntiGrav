//! The downloadable campaigns' maps, on the same fixture as
//! `wipeout2048.rs` with the two tiles the v1.04 patch adds to
//! `GameModeChoice`. The disc-backed twin is
//! `crates/game/tests/vita_2048_dlc_menus_ground_truth.rs`.

use super::wipeout2048::{XML, boot_on, press, reach_the_grid, reach_the_shell, three_events};
use super::*;

use oag_2048::frontend::states as w2048;

/// The fixture's grid with `FE_RC_HD` and `FE_RC_FURY` between the
/// single-player tile and the multiplayer one, at the positions the patched
/// file authors.
fn patched_xml() -> String {
    XML.replace(
        r#"<TouchButton name="multiplayer">"#,
        r#"<TouchButton name="offlinehd"><Values idstring="FE_RC_HD" x="410" y="110" width="135" height="135" toggle="true" StringWidthLimit="175" Src="Data\FE\NewImages\gamemodechoice\gm_SPHDCampaign.gtf"></Values></TouchButton>
        <TouchButton name="offlinefury"><Values idstring="FE_RC_FURY" x="592" y="110" width="135" height="135" toggle="true" StringWidthLimit="175" Src="Data\FE\NewImages\gamemodechoice\gm_SPFuryCampaign.gtf"></Values></TouchButton>
        <TouchButton name="multiplayer">"#,
    )
}

fn cell(name: &str, x: f32, y: f32) -> MapEvent {
    MapEvent {
        name: name.to_string(),
        x,
        y,
        detail: "circuit / mode".to_string(),
        requires: None,
        kind: EventIcon::Race,
        forced_craft: None,
        refused_craft: Vec::new(),
        card: EventCard::default(),
    }
}

fn hd() -> Tier {
    Tier {
        tile: "FE_RC_HD".to_string(),
        events: vec![cell("grid0_3_1", 1.0, 1.0), cell("grid0_1_2", 2.5, 2.0)],
    }
}

/// A frontend whose package for HD is mounted, on the mode grid.
fn on_the_grid(tiers: Vec<Tier>) -> (Frontend, Input) {
    let mut frontend = boot_on(&patched_xml(), 0);
    frontend.set_campaign(three_events());
    frontend.set_tiers(tiers);
    let mut input = Input::new();
    reach_the_grid(&mut frontend, &mut input);
    (frontend, input)
}

/// Chooses the tile `steps` to the right of the single-player one and
/// confirms it.
fn choose(frontend: &mut Frontend, input: &mut Input, steps: usize) {
    for _ in 0..steps {
        press(frontend, input, Button::Right);
    }
    press(frontend, input, Button::Cross);
    press(frontend, input, Button::Cross);
}

#[test]
fn a_mounted_package_opens_its_own_map_and_a_cell_launches_by_name() {
    let (mut frontend, mut input) = on_the_grid(vec![hd()]);
    choose(&mut frontend, &mut input, 1);
    assert!(frontend.machine().is(w2048::NEW_FE_SHELL));
    assert_eq!(frontend.campaign_tile(), Some("FE_RC_HD"));
    let names: Vec<&str> = frontend
        .campaign_events()
        .iter()
        .map(|event| event.name.as_str())
        .collect();
    assert_eq!(names, ["grid0_3_1", "grid0_1_2"]);
    press(&mut frontend, &mut input, Button::Cross);
    press(&mut frontend, &mut input, Button::Cross);
    assert!(frontend.is_finished());
    assert_eq!(
        frontend.launch(),
        Some(&Launch::Cell("grid0_3_1".to_string()))
    );
}

#[test]
fn a_tile_whose_package_is_not_mounted_still_says_it_leads_elsewhere() {
    let (mut frontend, mut input) = on_the_grid(Vec::new());
    choose(&mut frontend, &mut input, 1);
    assert!(
        frontend.machine().is(w2048::GAME_MODE_CHOICE),
        "no map to open, so the grid stays"
    );
    assert_eq!(frontend.campaign_tile(), None);
    let notes = frontend.take_notes();
    assert!(
        notes
            .iter()
            .any(|note| note.contains("FE_RC_HD is another title's campaign")),
        "{notes:#?}"
    );
}

#[test]
fn a_package_that_is_mounted_opens_only_its_own_tile() {
    let (mut frontend, mut input) = on_the_grid(vec![hd()]);
    assert!(frontend.has_campaign_tier("FE_RC_HD"));
    assert!(!frontend.has_campaign_tier("FE_RC_FURY"));
    choose(&mut frontend, &mut input, 2);
    assert!(frontend.machine().is(w2048::GAME_MODE_CHOICE));
    assert_eq!(frontend.chosen_mode(), Some("FE_RC_FURY"));
    assert_eq!(frontend.campaign_tile(), None);
}

#[test]
fn leaving_for_the_mode_grid_and_choosing_single_player_brings_its_map_back() {
    let (mut frontend, mut input) = on_the_grid(vec![hd()]);
    choose(&mut frontend, &mut input, 1);
    assert_eq!(frontend.campaign_events().len(), 2);
    press(&mut frontend, &mut input, Button::Circle);
    assert!(frontend.machine().is(w2048::GAME_MODE_CHOICE));
    // A screen opens with the cursor on its first tile: the single-player one.
    choose(&mut frontend, &mut input, 0);
    assert!(frontend.machine().is(w2048::NEW_FE_SHELL));
    assert_eq!(frontend.campaign_tile(), None);
    assert_eq!(frontend.campaign_events().len(), 3);
    assert_eq!(
        frontend.selected_event().map(|event| event.name.as_str()),
        Some("2048 - Event 1")
    );
}

/// The tile's own colour on the grid: the fill of its rect.
fn tile_fill(frontend: &Frontend, rect: [f32; 4]) -> [f32; 4] {
    frontend
        .draw_list()
        .iter()
        .find_map(|draw| match draw {
            Draw::Fill { rect: at, color } if *at == rect => Some(*color),
            _ => None,
        })
        .expect("the tile's box")
}

#[test]
fn the_tile_draws_as_enabled_with_its_package_and_as_the_network_modes_do_without() {
    let hd_rect = [410.0, 110.0, 135.0, 135.0];
    let sp_rect = [167.0, 190.0, 140.0, 140.0];
    let (with, _) = on_the_grid(vec![hd()]);
    let (without, _) = on_the_grid(Vec::new());
    assert_eq!(
        tile_fill(&with, hd_rect),
        tile_fill(&with, sp_rect),
        "with the package it is as blue as the single-player tile"
    );
    assert_ne!(
        tile_fill(&without, hd_rect),
        tile_fill(&without, sp_rect),
        "without it is the grey of a mode this build cannot enter"
    );
}

#[test]
fn a_save_and_a_lock_reach_a_map_that_is_not_on_screen() {
    let (mut frontend, mut input) = on_the_grid(vec![hd()]);
    frontend.refresh_campaign_progress(|name| (name == "grid0_3_1").then_some(EarnedTier::Elite));
    frontend.refresh_campaign_locks(|name| name == "grid0_1_2");
    choose(&mut frontend, &mut input, 1);
    assert_eq!(
        frontend.campaign_event_state("grid0_3_1"),
        Some(ProgressState::Elite)
    );
    assert_eq!(
        frontend.campaign_event_state("grid0_1_2"),
        Some(ProgressState::Locked)
    );
    // A locked cell refuses to launch, as a gated event does.
    press(&mut frontend, &mut input, Button::Right);
    press(&mut frontend, &mut input, Button::Cross);
    assert!(!frontend.is_finished());
    let notes = frontend.take_notes();
    assert!(
        notes.iter().any(|note| note.contains("is locked")),
        "{notes:#?}"
    );
}

#[test]
fn a_map_opens_on_its_first_open_node_when_the_grids_list_a_closed_one_first() {
    let (mut frontend, mut input) = on_the_grid(vec![hd()]);
    frontend.refresh_campaign_locks(|name| name == "grid0_3_1");
    choose(&mut frontend, &mut input, 1);
    assert_eq!(
        frontend.selected_event().map(|event| event.name.as_str()),
        Some("grid0_1_2")
    );
}

#[test]
fn the_map_a_single_player_save_colours_keeps_its_own_events() {
    let (mut frontend, mut input) = on_the_grid(vec![hd()]);
    frontend
        .refresh_campaign_progress(|name| (name == "2048 - Event 1").then_some(EarnedTier::Pass));
    reach_the_shell_from_grid(&mut frontend, &mut input);
    assert_eq!(
        frontend.campaign_event_state("2048 - Event 1"),
        Some(ProgressState::Passed)
    );
    assert_eq!(frontend.campaign_event_state("grid0_3_1"), None);
}

fn reach_the_shell_from_grid(frontend: &mut Frontend, input: &mut Input) {
    choose(frontend, input, 0);
    assert!(frontend.machine().is(w2048::NEW_FE_SHELL));
}

#[test]
fn the_shell_helper_reaches_the_single_player_map_unchanged() {
    let mut frontend = boot_on(&patched_xml(), 0);
    frontend.set_campaign(three_events());
    let mut input = Input::new();
    reach_the_shell(&mut frontend, &mut input);
    assert_eq!(frontend.campaign_tile(), None);
}
