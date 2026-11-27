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
//! # A mod's entries: `fallback/<title>/` of an enabled mod
//!
//! The composition root, which knows which mods are on, hands each enabled mod's
//! `fallback/<title>/` folder over as a [`ModFallback`] ([`register_mods`]); every
//! open that mounts the player's layers ([`Layering::player`]) puts the ones for
//! its title between the DLC and the embedded fallback, so they too answer only
//! a name no disc or DLC holds
//! (`docs/architecture/adr/0100-a-mod-carries-fallback-entries.md`).
//!
//! # The fallback: this project's own art, embedded
//!
//! Carried in the binary from `assets/fallback/`, so a packaged build needs no
//! directory beside it. It answers only a name no archive holds.

use std::path::PathBuf;
use std::sync::RwLock;

use oag_assets::{Archives, Platform};
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

/// One enabled mod's entries for one title: its `fallback/<slug>/` folder as a
/// [`Kind::Mod`] layer.
#[derive(Debug, Clone)]
pub struct ModFallback {
    /// The title the entries are for, as [`slug`] spells it.
    pub slug: String,
    /// The entries, named by their path under `fallback/<slug>/`.
    pub layer: Layer,
    /// Whether a LAN or online race mounts them: the mod's `[mod] network-races`.
    pub network: bool,
}

impl ModFallback {
    /// Every `fallback/<slug>/` folder of the mod folder `root` that holds a file, one each,
    /// labelled `label`: a mod read from a folder without its manifest consulted (a test that
    /// mounts an official mod; the game reads the manifest first).
    #[must_use]
    pub fn in_folder(root: &std::path::Path, label: &str) -> Vec<Self> {
        let Ok(read) = std::fs::read_dir(root.join("fallback")) else {
            return Vec::new();
        };
        let mut dirs: Vec<PathBuf> = read
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.is_dir())
            .collect();
        dirs.sort();
        dirs.into_iter()
            .filter_map(|dir| {
                let slug = dir.file_name()?.to_string_lossy().into_owned();
                let layer = Layer::from_dir(Kind::Mod, &dir)?.labelled(label);
                Some(Self {
                    slug,
                    layer,
                    network: true,
                })
            })
            .collect()
    }
}

/// One enabled mod, for the list a recorded race carries (`oag_replay::ActiveMod`): every mod
/// that is on, whether or not it holds any entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnabledMod {
    /// The mod's id.
    pub id: String,
    /// The mod's release.
    pub version: String,
    /// The titles it is for, as [`slug`] spells them; empty is every title.
    pub titles: Vec<String>,
    /// Whether it is for a PSP source alone (a PPSSPP texture pack).
    pub psp_only: bool,
    /// Whether a LAN or online race mounts it: the mod's `[mod] network-races`.
    pub network: bool,
}

impl EnabledMod {
    /// Whether the mod is on for a race on any of `titles` from a source of `platform`.
    #[must_use]
    pub fn is_for(&self, titles: &[&Title], platform: Platform) -> bool {
        (!self.psp_only || platform == Platform::Psp)
            && (self.titles.is_empty()
                || titles.iter().any(|title| {
                    let slug = slug(title);
                    self.titles.iter().any(|t| t.eq_ignore_ascii_case(&slug))
                }))
    }
}

/// What an open mounts around a title's archives besides the embedded fallback.
#[derive(Debug, Clone, Default)]
pub struct Layering {
    /// The directories the player's override is looked for in.
    pub overrides: Vec<PathBuf>,
    /// The enabled mods' entries, every title's; an open keeps its own title's.
    pub mods: Vec<ModFallback>,
    /// Every enabled mod, entries or none, in load order.
    pub enabled: Vec<EnabledMod>,
}

impl Layering {
    /// These layers for a LAN or online race: without the mods whose entries serve only
    /// single-player races ([`ModFallback::network`]).
    #[must_use]
    pub fn for_network(mut self) -> Self {
        self.mods.retain(|m| m.network);
        self.enabled.retain(|m| m.network);
        self
    }
}

impl Layering {
    /// The player's layers: the override search path, the mods [`register_mods`]
    /// last registered and the list [`register_enabled`] last was handed.
    #[must_use]
    pub fn player() -> Self {
        Self {
            overrides: overrides_search_path(),
            mods: registered_mods(),
            enabled: ENABLED
                .read()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .clone(),
        }
    }

    /// None of the player's layers, the embedded fallback alone: a benchmark that runs
    /// without the player's files.
    #[must_use]
    pub fn none() -> Self {
        Self::default()
    }

    /// The mod layers for `title`, in registration order.
    #[must_use]
    pub fn mods_for(&self, title: &Title) -> Vec<Layer> {
        let slug = slug(title);
        self.mods
            .iter()
            .filter(|m| m.slug == slug)
            .map(|m| m.layer.clone())
            .collect()
    }
}

static MODS: RwLock<Vec<ModFallback>> = RwLock::new(Vec::new());
static ENABLED: RwLock<Vec<EnabledMod>> = RwLock::new(Vec::new());

/// Replaces the list of enabled mods every later [`Layering::player`] carries. Set by the
/// composition root beside [`register_mods`], so a race records what its archives mounted.
pub fn register_enabled(mods: Vec<EnabledMod>) {
    *ENABLED
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = mods;
}

/// Replaces the enabled mods' entries every later [`Layering::player`] mounts. Set by the
/// composition root at start and whenever a mod is switched on or off; nothing else writes it.
pub fn register_mods(mods: Vec<ModFallback>) {
    *MODS
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = mods;
}

/// Registers the mod folder `root`'s entries ([`ModFallback::in_folder`]) as the only enabled
/// mod: a test that races a mode whose data a mod carries.
pub fn register_mod_folder(root: &std::path::Path, label: &str) {
    register_mods(ModFallback::in_folder(root, label));
}

/// What [`register_mods`] last registered.
#[must_use]
pub fn registered_mods() -> Vec<ModFallback> {
    MODS.read()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone()
}

/// Whether a mod [`register_mods`] last registered for `title` holds `name`.
#[must_use]
pub fn registered_mods_hold(title: &Title, name: &str) -> bool {
    let slug = slug(title);
    MODS.read()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .iter()
        .any(|m| m.slug == slug && m.layer.contains(name))
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

/// A file embedded from `assets/fallback/`, by its path under that directory.
macro_rules! embedded {
    ($($path:literal),* $(,)?) => {
        &[$((
            $path,
            include_bytes!(concat!("../../../assets/fallback/", $path)) as &[u8],
        )),*]
    };
}

/// Every embedded entry of the fallback layer except the rumble patterns, one
/// per line and sorted by path: a layer's entry is named by its path relative
/// to the directory, so a new file is one new line here. The own rocket effect
/// behind `graphics.own_rockets` is three effect files in the format a mod's
/// effect is written in and the puff sprites they name
/// (`docs/rendering/own-rockets.md`).
const EMBEDDED: &[(&str, &[u8])] = embedded![
    "Data/HUD/Textures/AssistIndicator.pix",
    "Data/Psys/OAGRocketFlight.toml",
    "Data/Psys/OAGRocketImpactCraft.toml",
    "Data/Psys/OAGRocketImpactTrack.toml",
    "Data/Psys/Tex/OAGRocketPuff1.oagmask",
    "Data/Psys/Tex/OAGRocketPuff2.oagmask",
    "Data/Psys/Tex/OAGRocketPuff3.oagmask",
    "Data/Psys/Tex/OAGRocketPuff4.oagmask",
    "Data/Psys/Tex/OAGSplash.oagmask",
    "Data/XML/BenchmarkPresets_2048.toml",
    "Data/XML/BenchmarkPresets_HD.toml",
    "Data/XML/BenchmarkPresets_Omega.toml",
    "Data/XML/BenchmarkPresets_Pulse.toml",
    "Data/XML/BenchmarkPresets_Pure.toml",
    "Data/XML/HUD_assist_indicator.xml",
    "Data/XML/LoadingScreen_2048.toml",
    "Data/XML/LoadingScreen_HD.toml",
    "Data/XML/LoadingScreen_Omega.toml",
    "Data/XML/NowPlaying_2048.xml",
    "Data/XML/SplitHud_2048.toml",
    "Data/XML/SplitHud_HD.toml",
    "Data/XML/SplitHud_Pulse.toml",
    "Data/XML/SplitHud_Pure.toml",
];

/// This project's own substitutes, carried in the binary.
///
/// Every entry is **art or layout this project drew**, named for the entry a
/// later title authors for the same job; none is extracted from a disc
/// (`just audit-leakage` checks the directory it is embedded from).
#[must_use]
pub fn fallback_layer() -> Layer {
    let mut layer = Layer::new(Kind::Fallback, "this project's own assets");
    for (name, bytes) in EMBEDDED {
        layer.insert_bytes(name, bytes);
    }
    for (name, bytes) in RUMBLE_PATTERNS {
        layer.insert_bytes(name, bytes);
    }
    layer
}

/// `archives` with the layers `layering` names and the embedded fallback mounted.
#[must_use]
pub fn mount_in(archives: Archives, title: &Title, layering: &Layering) -> Archives {
    let archives = archives
        .with_mods(layering.mods_for(title))
        .with_fallback(fallback_layer());
    match override_layer_in(&layering.overrides, title) {
        Some(layer) => archives.with_override(layer),
        None => archives,
    }
}

/// [`mount_in`] over [`Layering::player`].
#[must_use]
pub fn mount(archives: Archives, title: &Title) -> Archives {
    mount_in(archives, title, &Layering::player())
}

#[cfg(test)]
mod tests;
