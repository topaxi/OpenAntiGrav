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
    let own = race.view();
    let own_fov = race.vertical_fov(AUTHORED_ASPECT, oag_display::display::Fov::AUTHORED);
    assert_eq!(race.viewport_count(), 1);

    assert_eq!(
        race.set_viewport_count(4),
        4,
        "a full grid fields four craft"
    );
    assert_eq!(
        race.view(),
        own,
        "turning the split on does not move the player's view"
    );

    // Settled and not yet ticked, an extra camera is exactly the rig's snap on
    // its own craft - so this fails if the view follows the wrong slot, or the
    // player's camera, or nothing.
    for slot in 1..4 {
        race.set_viewpoint(slot);
        let target = target_of(&race.sim.world.ships[slot]);
        let params = race.view.chase_far;
        let expected = Chase::snapped(target, &params).view(target, &params);
        assert_eq!(race.view(), expected, "view {slot} follows slot {slot}");
        assert_ne!(race.view(), own, "view {slot} is not the player's");
        assert!(race.extra_viewpoint() == Some(slot));
        assert_eq!(race.viewed_ship().physics.body.position, target.position);
    }

    // Slot 0 is the player's own view, however it is asked for.
    race.set_viewpoint(0);
    assert_eq!(race.view(), own);
    race.reset_viewpoint();
    assert_eq!(race.view(), own);
    assert_eq!(
        race.vertical_fov(AUTHORED_ASPECT, oag_display::display::Fov::AUTHORED),
        own_fov
    );
}

#[test]
fn the_count_is_clamped_to_the_craft_fielded() {
    let mut solo = Race::start(setup(Handling::default()));
    assert_eq!(
        solo.set_viewport_count(4),
        1,
        "one craft on the grid is one view"
    );
    let mut grid = race_with_a_grid();
    assert_eq!(
        grid.set_viewport_count(99),
        usize::from(grid.sim.world.ship_count).min(8)
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
            split.set_viewpoint(slot);
            let _ = (
                split.view(),
                split.camera_position(),
                split.visibility_sections(),
            );
        }
        split.reset_viewpoint();
        assert_eq!(
            plain.sim.state_hash(),
            split.sim.state_hash(),
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
    let hash = race.sim.state_hash();

    race.set_cut(Cut::Vertical);
    assert_eq!(race.cut(), Cut::Vertical);
    let columns = race.view_rects(AREA);
    assert_eq!(columns.len(), 2);
    assert_eq!(columns[0].3, 1080.0, "full height");
    assert!(columns[1].0 > columns[0].0, "side by side");
    assert_eq!(race.sim.state_hash(), hash, "the cut is a view, not state");

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
