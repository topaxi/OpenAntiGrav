//! Where a ghost is kept: one replay file per records row, beside
//! `records.toml`.
//!
//! `<config dir>/oag/ghosts/<title>/<mode>/<class>/<track>.oagr`, each part
//! the [`records::Key`] spells, made safe for a file name. The file is an
//! [`oag_replay::Replay`] whose [`Replay::ghost`] is the lap drawn; its header
//! repeats the key verbatim, and a file whose header names another key is
//! ignored rather than raced. See ADR-0055 for the format, and
//! `docs/architecture/persistence.md` for the directory this shares.
//!
//! # What a bad file does
//!
//! `records.toml`'s rule, for the same reason: **never fails a race and never
//! panics.** A file that will not read, will not parse, fails its checksum or
//! names another key is logged and treated as no ghost at all. Unlike
//! `records.toml` it is not moved aside - the next best lap overwrites it,
//! which is the only repair a ghost needs.
//!
//! # Only a quicker lap replaces one
//!
//! [`save`] reads the stored file first and writes only when the new lap is
//! strictly quicker, so a slower run on a machine that has a ghost from an
//! earlier session never replaces it. The write goes to a temporary name and
//! is renamed over the old file, so a crash mid-write leaves the old ghost
//! rather than half of a new one.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use log::warn;
use oag_replay::{Header, Replay};

use crate::records::Key;

/// The extension a ghost file carries.
pub const EXTENSION: &str = "oagr";

/// The directory every ghost lives under: `<config dir>/oag/ghosts`.
#[must_use]
pub fn dir() -> Option<PathBuf> {
    dirs::config_dir().map(|dir| dir.join("oag").join("ghosts"))
}

/// Where `key`'s ghost lives under `root`.
#[must_use]
pub fn path_in(root: &Path, key: &Key) -> PathBuf {
    root.join(segment(&key.title))
        .join(segment(&key.mode))
        .join(segment(&key.class))
        .join(format!("{}.{EXTENSION}", segment(&key.track)))
}

/// A key part as one file-name segment: ASCII letters, digits, `.`, `-` and
/// `_` kept, anything else - a backslash in a disc path, a space in a title -
/// replaced with `_`.
fn segment(part: &str) -> String {
    let safe: String = part
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_') {
                c
            } else {
                '_'
            }
        })
        .collect();
    if safe.is_empty() || safe.chars().all(|c| c == '.') {
        "_".to_string()
    } else {
        safe
    }
}

/// A replay header for a race filed under `key`, flown as `team` from `seed`.
///
/// The key's four parts are copied verbatim, so [`load_from`] can compare
/// them; `build` is this binary's commit, which is informational only - see
/// [`Header::build`].
#[must_use]
pub fn header(key: &Key, team: &str, seed: u64) -> Header {
    let mut header = Header::new(&key.title, &key.track, &key.mode, &key.class, team, seed);
    header.build = env!("OAG_GIT_HASH").to_string();
    header
}

/// What else a replay needs to rebuild its race, beyond the key and the seed:
/// every input to the simulation that is not an [`oag_gameplay::InputSnapshot`].
///
/// **The rule ADR-0055 states, applied**: anything that changes the race from
/// outside `Race::tick` must be recorded here or it will not reproduce. The
/// control scheme maps the snapshot to controls; the AI difficulty tunes the
/// opponents and the operator's autopilot; `--give` writes the pickup slot
/// between ticks. None of them is read back by this build - the file says what
/// the run was, so a later tool can rebuild it or refuse.
#[must_use]
pub fn race_options(
    scheme: oag_gameplay::ControlScheme,
    difficulty: oag_ai::Difficulty,
    autopilot: bool,
    pilot_assist: bool,
    give: Option<oag_tables::weapons::Weapon>,
) -> std::collections::BTreeMap<String, String> {
    let mut options = std::collections::BTreeMap::new();
    options.insert("scheme".to_string(), format!("{scheme:?}").to_lowercase());
    options.insert("difficulty".to_string(), difficulty.name().to_string());
    if autopilot {
        options.insert("autopilot".to_string(), "true".to_string());
    }
    // Pilot Assist changes the player's physics, so a replay that does not know it
    // was on cannot reproduce the run. Absent means off, as every earlier file is.
    if pilot_assist {
        options.insert("pilot_assist".to_string(), "true".to_string());
    }
    if let Some(weapon) = give {
        options.insert("give".to_string(), weapon.as_type().to_string());
    }
    options
}

/// Whether `header` is the ghost for `key`.
fn matches(header: &Header, key: &Key) -> bool {
    header.title == key.title
        && header.track == key.track
        && header.mode == key.mode
        && header.class == key.class
}

/// The ghost stored at `path` for `key`, or `None` - logged - when there is
/// none worth racing.
#[must_use]
pub fn load_from(path: &Path, key: &Key) -> Option<Replay> {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return None,
        Err(e) => {
            warn!("could not read ghost {}: {e:#}", path.display());
            return None;
        }
    };
    let replay = match Replay::from_bytes(&bytes) {
        Ok(replay) => replay,
        Err(e) => {
            warn!("ignoring ghost {}: {e}", path.display());
            return None;
        }
    };
    if !matches(&replay.header, key) {
        warn!(
            "ignoring ghost {}: it is for {}/{}/{}/{}, not this race",
            path.display(),
            replay.header.title,
            replay.header.track,
            replay.header.mode,
            replay.header.class
        );
        return None;
    }
    if replay.ghost.is_none() {
        warn!("ignoring ghost {}: it carries no lap", path.display());
        return None;
    }
    Some(replay)
}

/// [`load_from`] under [`dir`].
#[must_use]
pub fn load(key: &Key) -> Option<Replay> {
    load_from(&path_in(&dir()?, key), key)
}

/// Writes `replay` as `key`'s ghost at `path` when its lap is quicker than
/// the one stored there. Returns whether it wrote.
///
/// # Errors
///
/// A directory that cannot be created or a file that cannot be written. The
/// caller logs it and carries on, as it does for `records.toml`.
pub fn save_to(path: &Path, key: &Key, replay: &Replay) -> Result<bool> {
    let Some(lap) = replay.ghost.as_ref() else {
        return Ok(false);
    };
    if let Some(stored) = load_from(path, key)
        && !lap.beats(stored.ghost.as_ref())
    {
        return Ok(false);
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    let partial = path.with_extension(format!("{EXTENSION}.partial"));
    std::fs::write(&partial, replay.to_bytes())
        .with_context(|| format!("writing {}", partial.display()))?;
    std::fs::rename(&partial, path)
        .with_context(|| format!("renaming {} into place", partial.display()))?;
    Ok(true)
}

/// [`save_to`] under [`dir`]; `Ok(false)` on a platform with no config
/// directory.
///
/// # Errors
///
/// As [`save_to`].
pub fn save(key: &Key, replay: &Replay) -> Result<bool> {
    let Some(root) = dir() else {
        return Ok(false);
    };
    save_to(&path_in(&root, key), key, replay)
}

#[cfg(test)]
mod tests;
