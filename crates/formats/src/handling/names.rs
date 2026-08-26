//! How a team's handling-stats entry is spelled, and the one directory that
//! moved.
//!
//! Split out of [`super`] because the file is over this project's 1,000-line
//! ceiling and these three items are the ones Wipeout 2048 changed - see
//! `oag_title::RaceDefaults::ship_dir`, which is the axis they exist for.

/// The archive entry name for a team's handling stats, under the ship
/// directory this title keeps its roster in.
///
/// Assembled the way the loader assembles it, with backslashes, which is what
/// [`wad::hash_name`](crate::wad::hash_name) needs to find the entry. `dir`
/// comes from `oag_title::RaceDefaults::ship_dir`, because which directory a
/// title's ships live under is a title fact and this crate must not know which
/// title it is reading.
#[must_use]
pub fn entry_name_in(dir: &str, team: &str) -> String {
    format!(r"{dir}\{team}\handlingstats.xml")
}

/// [`entry_name_in`] under [`SHIP_DIR`], the directory three of the four
/// titles keep their roster in.
#[must_use]
pub fn entry_name(team: &str) -> String {
    entry_name_in(SHIP_DIR, team)
}

/// The ship directory Pulse, Pure and Wipeout HD all spell identically.
///
/// **The axis is `oag_title::RaceDefaults::ship_dir`, not this.** This crate
/// cannot see `oag-title` - a format reader must not know which title it is
/// reading - so the string sits on both sides of that boundary. Wipeout 2048
/// holds a different one and reaches [`entry_name_in`] directly.
pub const SHIP_DIR: &str = r"Data\Ships";
