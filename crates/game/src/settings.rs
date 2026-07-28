//! Persisted graphics settings.
//!
//! Read from `<config dir>/oag/settings.toml` on startup and rewritten with
//! every recognised key filled in, so the file on disk always documents what
//! it can hold rather than starting empty or silently dropping a key an older
//! version never wrote. A CLI flag still wins over whatever is on disk for
//! that one run; see `--anisotropy` in `main.rs`.

use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use oag_render::mesh_render::Anisotropy;

/// Mirrors [`Anisotropy`] for serde, which cannot derive on a type this crate
/// does not own. Named the same as `Display`/`FromStr` already spell it in
/// `oag-render`, so the two stay one vocabulary.
#[derive(Serialize, Deserialize)]
#[serde(remote = "Anisotropy")]
enum AnisotropyDef {
    #[serde(rename = "off")]
    Off,
    #[serde(rename = "2x")]
    X2,
    #[serde(rename = "4x")]
    X4,
    #[serde(rename = "8x")]
    X8,
    #[serde(rename = "16x")]
    X16,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub graphics: Graphics,
    #[serde(default)]
    pub source: Source,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Source {
    /// A disc image, or a directory extracted with `oag-unpack`, opened when
    /// neither the command line nor `$OAG_IMAGE` name one.
    ///
    /// Persisted so a player who always plays off one disc - the PS2 release,
    /// say - does not have to spell it out on every run. The command line
    /// still wins over this for that one run; see `source::resolve`.
    #[serde(default)]
    pub image: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Graphics {
    /// Anisotropic filtering level for track and ship textures: `off`, `2x`,
    /// `4x`, `8x` or `16x`.
    ///
    /// Defaults to the highest level. Anisotropy is the one texture-filtering
    /// setting worth exposing at all - see `mesh_render::mip_chain` - and a
    /// user turns it down for performance, never up, so defaulting low would
    /// leave most of that work unused out of the box.
    #[serde(with = "AnisotropyDef", default)]
    pub anisotropy: Anisotropy,
}

/// Where the settings file lives: `<config dir>/oag/settings.toml`.
///
/// `None` on a platform `dirs` cannot place a config directory on, in which
/// case [`load`] falls back to defaults with nothing to persist them to.
pub fn path() -> Option<PathBuf> {
    dirs::config_dir().map(|dir| dir.join("oag").join("settings.toml"))
}

const HEADER: &str = "\
# OpenAntiGrav settings.
#
# Rewritten on every run with every recognised key present, so this file is
# always a complete reference for what can go in it. A value you set is kept;
# a key that's missing (an older version, a fresh install, a typo'd table
# name) is added back with its default. Comments other than this header are
# not preserved across a rewrite.

";

/// Loads settings from [`path`], creating the file - or filling in whatever
/// keys it is missing - with defaults.
///
/// A malformed value is an error rather than a silently discarded default: a
/// typo should be visible, not swallowed. Absence is not malformed, so a
/// missing file or config directory is created rather than reported.
pub fn load() -> Result<Settings> {
    let Some(path) = path() else {
        return Ok(Settings::default());
    };

    let on_disk = match std::fs::read_to_string(&path) {
        Ok(text) => Some(text),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(e).with_context(|| format!("reading {}", path.display())),
    };

    let settings: Settings = match &on_disk {
        Some(text) => {
            toml::from_str(text).with_context(|| format!("parsing {}", path.display()))?
        }
        None => Settings::default(),
    };

    let canonical = format!(
        "{HEADER}{}",
        toml::to_string_pretty(&settings).context("serialising settings")?
    );
    if on_disk.as_deref() != Some(canonical.as_str()) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }
        std::fs::write(&path, &canonical).with_context(|| format!("writing {}", path.display()))?;
    }

    Ok(settings)
}
