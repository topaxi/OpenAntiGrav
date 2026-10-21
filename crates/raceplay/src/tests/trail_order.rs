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
