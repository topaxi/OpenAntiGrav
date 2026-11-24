//! Split screen's cameras: an extra view follows its own craft, the player's view
//! is left exactly as it was, and none of it reaches the simulation.

use super::*;
use oag_gameplay::PlayerInputs;

#[test]
fn an_extra_view_follows_its_own_craft_and_the_players_is_untouched() {
    let mut race = race_with_a_grid();
    for _ in 0..30 {
        race.tick(&PlayerInputs::none());
    }
    let own = race.view.view(&race.core);
    let own_fov = race.view.vertical_fov(
        &race.core,
        AUTHORED_ASPECT,
        oag_display::display::Fov::AUTHORED,
    );
    assert_eq!(race.view.viewport_count(), 1);

    assert_eq!(
        race.set_viewport_count(4),
        4,
        "a full grid fields four craft"
    );
    assert_eq!(
        race.view.view(&race.core),
        own,
        "turning the split on does not move the player's view"
    );

    // Settled and not yet ticked, an extra camera is exactly the rig's snap on
    // its own craft - so this fails if the view follows the wrong slot, or the
    // player's camera, or nothing.
    for slot in 1..4 {
        race.view.set_viewpoint(&race.core, slot);
        let target = target_of(&race.core.sim.world.ships[slot]);
        let params = race.view.chase_far;
        let expected = Chase::snapped(target, &params).view(target, &params);
        assert_eq!(
            race.view.view(&race.core),
            expected,
            "view {slot} follows slot {slot}"
        );
        assert_ne!(
            race.view.view(&race.core),
            own,
            "view {slot} is not the player's"
        );
        assert!(race.view.extra_viewpoint() == Some(slot));
        assert_eq!(race.viewed_ship().physics.body.position, target.position);
    }

    // Slot 0 is the player's own view, however it is asked for.
    race.view.set_viewpoint(&race.core, 0);
    assert_eq!(race.view.view(&race.core), own);
    race.view.reset_viewpoint();
    assert_eq!(race.view.view(&race.core), own);
    assert_eq!(
        race.view.vertical_fov(
            &race.core,
            AUTHORED_ASPECT,
            oag_display::display::Fov::AUTHORED
        ),
        own_fov
    );
}

#[test]
fn the_count_is_clamped_to_the_craft_fielded() {
    let mut solo = Race::start(
        setup(Handling::default()),
        crate::tests::fixture_view(),
        crate::extensions::Prepared::default(),
    );
    assert_eq!(
        solo.set_viewport_count(4),
        1,
        "one craft on the grid is one view"
    );
    let mut grid = race_with_a_grid();
    assert_eq!(
        grid.set_viewport_count(99),
        usize::from(grid.core.sim.world.ship_count).min(8)
    );
    assert_eq!(grid.set_viewport_count(0), 1);
}

/// The cameras are view-side. A split moves a picture and cannot move a race.
#[test]
fn viewports_leave_the_simulation_hash_unchanged() {
    let mut plain = race_with_a_grid();
    let mut split = race_with_a_grid();
    split.set_viewport_count(4);
    for tick in 0..120 {
        plain.tick(&PlayerInputs::none());
        split.tick(&PlayerInputs::none());
        // The draw reads every extra camera in turn, as a frame does.
        for slot in 0..4 {
            split.view.set_viewpoint(&split.core, slot);
            let _ = (
                split.view.view(&split.core),
                split.view.camera_position(&split.core),
                split.visibility_sections(),
            );
        }
        split.view.reset_viewpoint();
        assert_eq!(
            plain.core.sim.state_hash(),
            split.core.sim.state_hash(),
            "diverged at tick {tick}"
        );
    }
}

/// The HUD's gate is the primary's flyby and `hud_delay` whatever the view count:
/// a split does not hide it (each view draws its own slot's, `Race::readout_for`),
/// and does not show it where the primary's own rules would not.
#[test]
fn the_hud_gate_is_the_same_split_or_not() {
    let mut race = race_with_a_grid();
    assert!(race.hud_shown());
    race.set_viewport_count(2);
    assert!(race.hud_shown(), "a split no longer hides the HUD");
    assert!(
        race.track_panel_progress().is_none(),
        "no flyby, no track panel, split or not"
    );
    race.set_viewport_count(1);
    assert!(race.hud_shown());
}

/// The cut is one answer for the scene and the HUD: two views beside each other when it is
/// vertical, rows for any other count whatever was asked, and no state the hash holds.
#[test]
fn the_cut_is_one_answer_for_every_view_rectangle_and_is_not_simulation() {
    use oag_render::viewport::Cut;
    const AREA: (f32, f32, f32, f32) = (0.0, 0.0, 1920.0, 1080.0);
    let mut race = race_with_a_grid();
    race.set_viewport_count(2);
    assert_eq!(race.cut(), Cut::Horizontal, "rows until a cut is chosen");
    let rows = race.view_rects(AREA);
    assert_eq!(rows[0].2, 1920.0, "full width");
    let hash = race.core.sim.state_hash();

    race.set_cut(Cut::Vertical);
    assert_eq!(race.cut(), Cut::Vertical);
    let columns = race.view_rects(AREA);
    assert_eq!(columns.len(), 2);
    assert_eq!(columns[0].3, 1080.0, "full height");
    assert!(columns[1].0 > columns[0].0, "side by side");
    assert_eq!(
        race.core.sim.state_hash(),
        hash,
        "the cut is a view, not state"
    );

    race.set_viewport_count(4);
    assert_eq!(
        race.cut(),
        Cut::Horizontal,
        "four views are quadrants whatever the cut"
    );
    assert_eq!(race.view_rects(AREA).len(), 4);
    race.set_viewport_count(1);
    assert_eq!(
        race.view_rects(AREA),
        vec![AREA],
        "one view is the area itself"
    );
}

/// A mode the title holds until the release draws no HUD on the grid or through the
/// countdown, and draws it from the first released tick
/// (`docs/ghidra/functions/ps3-hdfury-eu/hud-start-gate.md`).
#[test]
fn a_mode_held_until_go_draws_no_hud_before_the_release() {
    use oag_race::COUNTDOWN_TICKS;
    let mut race = race_with_a_grid();
    race.view.hud_waits_for_go = true;
    for (tick, shown) in [
        (0, false),
        (COUNTDOWN_TICKS - 1, false),
        (COUNTDOWN_TICKS, true),
        (COUNTDOWN_TICKS + 600, true),
    ] {
        race.core.sim.world.tick = tick;
        assert_eq!(race.hud_shown(), shown, "tick {tick}");
    }
    race.core.sim.world.tick = 0;
    race.view.capture_hud = Some(crate::CaptureHud {
        place: 1,
        lap: 1,
        lap_ticks: 0,
        race_ticks: 0,
        speed_kmh: None,
        shield_percent: None,
        best_lap_ticks: None,
        best_lap_centiseconds: None,
        boost_phase: None,
        autopilot_seconds: None,
        song: None,
        head_to_head: None,
        zone: None,
    });
    assert!(
        race.hud_shown(),
        "an imposed HUD state is a frame with the HUD on screen"
    );
    race.view.capture_hud = None;
    race.view.hud_waits_for_go = false;
    assert!(race.hud_shown(), "a mode not held draws it on the grid");
}

/// HD and Omega hold Zone's HUD until the release and nothing else (a single race draws
/// it through the countdown, read live); the names are the modes' own.
#[test]
fn hd_and_omega_hold_zone_and_only_zone() {
    use oag_race::Mode;
    for title in [oag_hd::TITLE, oag_omega::TITLE, oag_2048::TITLE] {
        let held = title.race.hud_waits_for_go.value;
        assert!(held.contains(&Mode::Zone.name()), "{}", title.name);
        assert!(!held.contains(&Mode::SingleRace.name()), "{}", title.name);
    }
    for title in [oag_pulse::TITLE, oag_pure::TITLE] {
        assert!(
            title.race.hud_waits_for_go.value.is_empty(),
            "{}",
            title.name
        );
    }
}
