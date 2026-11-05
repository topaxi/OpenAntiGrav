//! Mounting the two asset layers around a title's own archives: the player's
//! override in front, this project's fallback behind.
//!
//! The mechanism is [`oag_assets::layers`]; this is where they come from. See
//! `docs/architecture/adr/0061-two-asset-layers-around-the-disc.md`.
//!
//! # The override: a player directory per title
//!
//! `overrides/<title>/` beside `images/` and `dlc/`, searched in the same three
//! places (`data/` in a checkout, beside the program, `<data dir>/oag/`), or the
//! path list in `$OAG_OVERRIDES`. `<title>` is [`slug`] of the title's name, such
//! as `wipeout-pulse`. Every file under it is an entry, named by its path
//! relative to that directory: `Data/HUD/Foo.mip` replaces the disc's
//! `Data\HUD\Foo.mip`. It is empty by default, never committed, and every entry
//! it answers is named in the load report.
//!
//! # The fallback: this project's own art, embedded
//!
//! Carried in the binary from `assets/fallback/`, so a packaged build needs no
//! directory beside it. It answers only a name no archive holds.

use std::path::PathBuf;

use oag_assets::Archives;
use oag_assets::layers::{Kind, Layer};
use oag_title::Title;

/// The environment variable that names the override directories outright: a
/// path list, split the way the platform splits `$PATH`, each holding one
/// sub-directory per title.
pub const OVERRIDES_ENV: &str = "OAG_OVERRIDES";

/// `Wipeout Pulse` as a directory name: lowercase, anything but a letter or a
/// digit a hyphen, runs collapsed.
#[must_use]
pub fn slug(title: &Title) -> String {
    let mut out = String::new();
    for c in title.name.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
    }
    out.trim_end_matches('-').to_string()
}

/// Where override directories are looked for when nothing names one.
#[must_use]
pub fn overrides_search_path() -> Vec<PathBuf> {
    if let Some(from_env) = std::env::var_os(OVERRIDES_ENV) {
        let paths: Vec<PathBuf> = std::env::split_paths(&from_env).collect();
        if !paths.is_empty() {
            return paths;
        }
    }
    let mut paths = vec![PathBuf::from("data/overrides")];
    if let Some(directory) = crate::source::portable_directory() {
        paths.push(directory.join("overrides"));
    }
    if let Some(data) = dirs::data_dir() {
        paths.push(data.join("oag").join("overrides"));
    }
    paths
}

/// The player's override for `title` out of the first of `bases` that holds
/// files for it, or `None`: the ordinary state of a copy of the game.
#[must_use]
pub fn override_layer_in(bases: &[PathBuf], title: &Title) -> Option<Layer> {
    let slug = slug(title);
    bases
        .iter()
        .find_map(|base| Layer::from_dir(Kind::Override, &base.join(&slug)))
}

/// Our own rumble patterns (`assets/fallback/Data/XML/Rumble/`), by name.
macro_rules! rumble_patterns {
    ($($name:literal),* $(,)?) => {
        &[$((
            concat!("Data\\XML\\Rumble\\", $name, ".toml"),
            include_bytes!(concat!("../../../assets/fallback/Data/XML/Rumble/", $name, ".toml")) as &[u8],
        )),*]
    };
}

/// Every pattern the fallback layer carries; the disc's own of the same name wins.
pub const RUMBLE_PATTERNS: &[(&str, &[u8])] = rumble_patterns![
    "absorb_any_pickup_rumble",
    "autopilot_active_rumble",
    "cannon_fired_rumble",
    "collision_heavy_looping",
    "collision_left_light_looping",
    "collision_left_med_looping",
    "collision_left_very_light_looping",
    "collision_right_light_looping",
    "collision_right_med_looping",
    "collision_right_very_light_looping",
    "enter_mag_rumble",
    "exit_mag_rumble",
    "landing_heavy",
    "landing_light",
    "landing_med",
    "landing_very_light",
    "leech_fired_rumble",
    "leeching_energy_rumble",
    "lockon_rumble",
    "mine_fired_rumble",
    "missile_fired_rumble",
    "missile_rebound_wall_rumble",
    "pad_speed",
    "plasma_fired_rumble",
    "plasma_hits_wall",
    "quake_fired_rumble",
    "repulser_fired_rumble",
    "rocket_hits_wall",
    "rockets_fired_rumble",
    "ship_exploding_player_rumble",
    "shuriken_fired_rumble",
    "travel_mag_rumble",
    "wrong_way",
];

/// This project's own substitutes, carried in the binary.
///
/// Every entry is **art or layout this project drew**, named for the entry a
/// later title authors for the same job; none is extracted from a disc
/// (`just audit-leakage` checks the directory it is embedded from).
#[must_use]
pub fn fallback_layer() -> Layer {
    let mut layer = Layer::new(Kind::Fallback, "this project's own assets");
    layer.insert_bytes(
        "Data\\XML\\HUD_assist_indicator.xml",
        include_bytes!("../../../assets/fallback/Data/XML/HUD_assist_indicator.xml"),
    );
    layer.insert_bytes(
        "Data\\HUD\\Textures\\AssistIndicator.pix",
        include_bytes!("../../../assets/fallback/Data/HUD/Textures/AssistIndicator.pix"),
    );
    layer.insert_bytes(
        "Data\\XML\\NowPlaying_2048.xml",
        include_bytes!("../../../assets/fallback/Data/XML/NowPlaying_2048.xml"),
    );
    layer.insert_bytes(
        "Data\\XML\\NowPlaying_Pure.xml",
        include_bytes!("../../../assets/fallback/Data/XML/NowPlaying_Pure.xml"),
    );
    layer.insert_bytes(
        "Data\\XML\\SplitHud_Pulse.toml",
        include_bytes!("../../../assets/fallback/Data/XML/SplitHud_Pulse.toml"),
    );
    layer.insert_bytes(
        "Data\\XML\\SplitHud_Pure.toml",
        include_bytes!("../../../assets/fallback/Data/XML/SplitHud_Pure.toml"),
    );
    layer.insert_bytes(
        "Data\\XML\\SplitHud_HD.toml",
        include_bytes!("../../../assets/fallback/Data/XML/SplitHud_HD.toml"),
    );
    layer.insert_bytes(
        "Data\\XML\\SplitHud_2048.toml",
        include_bytes!("../../../assets/fallback/Data/XML/SplitHud_2048.toml"),
    );
    layer.insert_bytes(
        "Data\\XML\\LoadingScreen_HD.toml",
        include_bytes!("../../../assets/fallback/Data/XML/LoadingScreen_HD.toml"),
    );
    layer.insert_bytes(
        "Data\\XML\\LoadingScreen_Omega.toml",
        include_bytes!("../../../assets/fallback/Data/XML/LoadingScreen_Omega.toml"),
    );
    layer.insert_bytes(
        "Data\\Psys\\Tex\\OAGSplash.oagmask",
        include_bytes!("../../../assets/fallback/Data/Psys/Tex/OAGSplash.oagmask"),
    );
    for (name, bytes) in RUMBLE_PATTERNS {
        layer.insert_bytes(name, bytes);
    }
    layer
}

/// `archives` with both layers mounted, the override from `bases`.
#[must_use]
pub fn mount_in(archives: Archives, title: &Title, bases: &[PathBuf]) -> Archives {
    let archives = archives.with_fallback(fallback_layer());
    match override_layer_in(bases, title) {
        Some(layer) => archives.with_override(layer),
        None => archives,
    }
}

/// [`mount_in`] over [`overrides_search_path`].
#[must_use]
pub fn mount(archives: Archives, title: &Title) -> Archives {
    mount_in(archives, title, &overrides_search_path())
}

#[cfg(test)]
mod tests;
