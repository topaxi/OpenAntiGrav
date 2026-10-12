use super::*;
use crate::tests::race_with_a_grid;

/// The node's world matrix carries the 0.75 craft scale and the original's push
/// along its row 1 is not normalised (`hd-leach-render`, read live: the grid
/// Mirage's node, row 1 length 0.7496, anchor 1.275 below the node). Normalising
/// the row gives 1.7 here and fails this.
#[test]
fn the_anchor_is_pushed_by_the_scaled_row_not_a_unit_one() {
    let race = race_with_a_grid();
    let locator = Mat4::from_translation(Vec3::new(0.0, -1.2787, -2.778));
    let mut anchors = [None; MAX_SHIPS];
    anchors[0] = Some(locator);

    let node = race.ship_model_matrix_of(0) * locator;
    let anchor = race.leach_anchor(&anchors, 0);
    let push = (anchor - node.w_axis.truncate()).length();
    assert!(
        (push - 1.7 * oag_fx::exhaust::CRAFT_ROW_SCALE).abs() < 1e-4,
        "push {push}"
    );
}

/// A hull with no locator falls back to the body, whose rows are unit length.
#[test]
fn a_hull_with_no_locator_pushes_a_full_one_point_seven() {
    let race = race_with_a_grid();
    let anchors = [None; MAX_SHIPS];
    let body = race.ship_model_matrix_of(0).w_axis.truncate();
    let push = (race.leach_anchor(&anchors, 0) - body).length();
    assert!((push - 1.7).abs() < 1e-4, "push {push}");
}

/// The wobble's phases stand still between beams and carry into the next one,
/// as the original's do (read live, `hd-leach-2`): with no beam alive a tick
/// leaves them where the last beam left them. Resetting them per beam fails this.
#[test]
fn the_wobble_phases_carry_over_between_beams() {
    let mut race = race_with_a_grid();
    let mut strip = Strip::new([None; MAX_SHIPS]);
    strip.phases = [1.5; 3];
    race.view.leach_strip = Some(strip);
    assert!(race.leach_beam_live().is_none());
    race.advance_leach_strip();
    let phases = race.view.leach_strip.as_ref().map(|strip| strip.phases);
    assert_eq!(phases, Some([1.5; 3]));
}
