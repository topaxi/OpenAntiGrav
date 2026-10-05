//! The pointer's targets on `Grid Selection`/`Cell Selection`, built from a
//! loaded campaign's own layout and decoded sprite sheet.
//!
//! In the library rather than the session so the live mouse path and a
//! disc-backed test build the same targets: a hex is hit where its hexagon is
//! drawn, which only the decoded pixels say (see
//! [`oag_ui_screens::campaign::pointer`]'s "Not the sprite's own size").

use oag_ui_screens::campaign::pointer::Target;
use oag_ui_screens::campaign::{CellSelection, GridSelection, Layout, pointer};

use oag_hud::sprite::Sheet;

/// `Cell Selection`'s targets: one hexagon per occupied cell, where it is drawn.
#[must_use]
pub fn cell_targets(model: &CellSelection, layout: &Layout, sheet: &Sheet) -> Vec<Target> {
    pointer::cell_targets(model, layout, &|src| sheet.get(src), &|src| {
        sheet.opaque_extent(src)
    })
}

/// `Grid Selection`'s targets: the paging arrows and the current page's tiles.
#[must_use]
pub fn grid_targets(model: &GridSelection, layout: &Layout, sheet: &Sheet) -> Vec<Target> {
    pointer::grid_targets(model, layout, &|src| sheet.get(src), &|src| {
        sheet.opaque_extent(src)
    })
}
