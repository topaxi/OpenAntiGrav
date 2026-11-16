//! `Race::ribbons_first`: where the frame records the ribbons against the circuit's
//! blended list, from `trail_order` and the title's ribbon.

use super::*;
use oag_display::display::TrailOrder;

/// The PSP preset draws its ribbons first by default (the original's queue order),
/// and `over-floors` puts them back after the blended list.
#[test]
fn the_trail_order_setting_flips_the_psp_ribbon_order() {
    let mut race = race_with_a_grid();
    race.view.hd_trail_active = false;
    assert!(race.ribbons_first(), "the original order is the default");
    race.set_trail_order(TrailOrder::OverFloors);
    assert!(!race.ribbons_first());
    race.set_trail_order(TrailOrder::Original);
    assert!(race.ribbons_first());
}

/// HD's tube keeps its own order whichever value the profile holds.
#[test]
fn hds_tube_ignores_the_trail_order() {
    let mut race = race_with_a_grid();
    race.view.hd_trail_active = true;
    for order in TrailOrder::ALL {
        race.set_trail_order(order);
        assert!(!race.ribbons_first(), "{order}");
    }
}

/// Foliage writes no depth of its own by default, and `depth` asks for it; HD's tube keeps its
/// own order whichever value the profile holds.
#[test]
fn the_scenery_sort_setting_asks_for_foliage_depth_on_the_psp_circuit_only() {
    use oag_display::display::ScenerySort;
    let mut race = race_with_a_grid();
    race.view.hd_trail_active = false;
    assert!(!race.writes_foliage_depth(), "the original writes none");
    race.set_scenery_sort(ScenerySort::Depth);
    assert!(race.writes_foliage_depth());
    race.view.hd_trail_active = true;
    assert!(!race.writes_foliage_depth());
}

/// The setting places the ribbons; the circuit's blended list stays after the pads and hulls
/// under both orders, and only HD's tube keeps the circuit whole.
#[test]
fn the_blended_list_is_after_the_craft_whichever_order_the_ribbons_take() {
    let mut race = race_with_a_grid();
    race.view.hd_trail_active = false;
    for order in TrailOrder::ALL {
        race.set_trail_order(order);
        assert!(race.blended_after_craft(), "{order}");
    }
    race.view.hd_trail_active = true;
    assert!(!race.blended_after_craft());
}
