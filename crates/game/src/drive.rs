//! Driving our own menus without a window: the script, the state the game reports and the
//! rule that moves focus to a row.
//!
//! `oag-game --drive` runs the real windowed session on a device with no window
//! (`crate::main::drive`, in the binary) and reads this module's three halves:
//!
//! - [`script`]: the steps a run is told to take (`until page=race`, `select track`, `press cross`);
//! - [`state`]: what the game says about itself after every frame, one JSON line per change;
//! - [`select`]: how focus gets to a named row by pressing buttons, judged only from that state.
//!
//! **Plain data on purpose.** Nothing here names a stage, a menu or a GPU: the binary fills a
//! [`state::State`] from whatever is on screen and this module decides what to press. That is
//! what lets a new screen (a campaign grid, a prompt) become drivable by reporting its rows,
//! with no list of pages kept here. See `docs/tools/oag-game.md`, "Driving the menus".

pub mod script;
pub mod select;
pub mod state;

pub use script::{Cond, Script, Step};
pub use select::{Next, Select, SetChoice};
pub use state::{Focus, Opt, Resolved, Row, Screen, State};

#[cfg(test)]
mod tests;
