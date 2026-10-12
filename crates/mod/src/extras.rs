//! `oag-extras`: the rows and strings this project adds around each title's
//! own menu, carried as a mod and embedded in the binary like
//! `assets/fallback/`.
//!
//! It is the worked example of the format: the same files a player's mod has,
//! read through the same reader, differing only in being compiled in.

use crate::{Container, Error, Mod};

/// The extras' id.
pub const ID: &str = "oag.extras";

static FILES: &[(&str, &[u8])] = &[
    (
        "oag-mod.toml",
        include_bytes!("../../../assets/mods/oag-extras/oag-mod.toml"),
    ),
    (
        "menu.toml",
        include_bytes!("../../../assets/mods/oag-extras/menu.toml"),
    ),
];

/// The built-in mod.
///
/// # Errors
///
/// Only if the embedded manifest is wrong, which a test pins.
pub fn builtin() -> Result<Mod, Error> {
    Mod::from_container(Container::memory("built-in oag-extras", FILES))
}
