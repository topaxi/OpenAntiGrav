//! Where Omega keeps its campaign: `Data\Plugins\grids\Definition.xml` and
//! the nineteen `grid_NN.xml` files it lists - confirmed by direct listing
//! against `data09.psarc` (`grid_00.xml` through `grid_18.xml`, ground-truthed
//! in `crates/omega/tests`), **not** HD's sixteen.
//!
//! Same schema, same screen idiom as [`oag_hd::campaign`]: `CellMode_Definition.xml`
//! authors `Grid Selection`/`Cell Selection` as `type="FlyerSelection"`/
//! `"CellSelection"` screens in the disc's own 1920x1080 grid, plain UTF-8
//! rather than Pulse's dictionary-shortened copy - see
//! `docs/formats/omega-frontend.md`'s "2048's campaign" section, which found
//! a fifth screen (`Grid Selection 2048`, over `grid_16`..`grid_18`) whose
//! events this module does not read: they are `Campaign2048_Definition.xml`'s
//! (whole, 29,919 bytes), in 2048's own `FE3DCanvas`/`CanvasLabel`
//! vocabulary rather than this schema, and the three grids themselves author
//! no `RequiredPoints`.

/// The file that lists every grid, confirmed present in `data09.psarc`.
pub const DEFINITION_ENTRY: &str = r"Data\Plugins\grids\Definition.xml";

/// Nineteen - read by direct listing (`grid_00.xml`..`grid_18.xml` in
/// `data09.psarc`), not copied from [`oag_hd::campaign::GRID_COUNT`]'s
/// sixteen. See `crates/omega/tests/omega_title_ground_truth.rs`'s
/// `data09_carries_nineteen_grids`.
pub const GRID_COUNT: u8 = 19;

/// The entry name for one grid, `Data\Plugins\grids\grid_00.xml` through
/// `grid_18.xml`.
#[must_use]
pub fn entry_name(index: u8) -> String {
    format!(r"Data\Plugins\grids\grid_{index:02}.xml")
}

/// The screen `Grid Selection`/`Cell Selection` are both authored in -
/// confirmed present in `data09.psarc` at the same relative path HD's own
/// copy uses (`Data\Plugins\Frontend\Gui\...`, not the PSP titles'
/// `Data\Plugins\PI001\GUI\...`).
pub const SCREEN_ENTRY: &str = r"Data\Plugins\Frontend\Gui\CellMode_Definition.xml";

/// The grid `CellMode_Definition.xml` is authored in. Not independently
/// re-measured against this file's own widget coordinates the way
/// `oag_hd::campaign::AUTHORED_GRID`'s doc comment was - carried at HD's
/// figure since `skin.xml`'s own `FEGlobals` already confirmed the same
/// 1920x1080 grid for the rest of this title's front end
/// (`crate::frontend::MENU_SKIN::space`).
pub const AUTHORED_GRID: [f32; 2] = [1920.0, 1080.0];
