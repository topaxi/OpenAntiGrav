//! Finding mods on disk, and loading what is found.

use std::path::{Path, PathBuf};

use crate::{Error, MANIFEST, Mod};

/// A colon-separated (semicolon on Windows) list of directories that replaces
/// the default search path, as `OAG_OVERRIDES` does for overrides.
pub const SEARCH_ENV: &str = "OAG_MODS";

/// Where mods are looked for when nothing names a directory: `data/mods` in a
/// checkout, `mods` beside the program, and `<data dir>/oag/mods`.
#[must_use]
pub fn mods_search_path() -> Vec<PathBuf> {
    if let Some(from_env) = std::env::var_os(SEARCH_ENV) {
        let paths: Vec<PathBuf> = std::env::split_paths(&from_env).collect();
        if !paths.is_empty() {
            return paths;
        }
    }
    let mut paths = vec![PathBuf::from("data/mods")];
    let beside = std::env::var_os("APPIMAGE")
        .map(PathBuf::from)
        .or_else(|| std::env::current_exe().ok())
        .and_then(|exe| exe.parent().map(Path::to_path_buf));
    if let Some(directory) = beside {
        paths.push(directory.join("mods"));
    }
    if let Some(data) = dirs::data_dir() {
        paths.push(data.join("oag").join("mods"));
    }
    paths
}

/// One candidate under a search directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    /// The folder, `.zip` or `.oagmod`.
    pub path: PathBuf,
}

/// Every candidate directly inside `bases`: a folder holding an
/// `oag-mod.toml`, a `.zip`, or a `.oagmod`. Sorted by name within a base, the
/// bases in the order given.
#[must_use]
pub fn search(bases: &[PathBuf]) -> Vec<Found> {
    let mut out = Vec::new();
    for base in bases {
        let Ok(read) = std::fs::read_dir(base) else {
            continue;
        };
        let mut here: Vec<PathBuf> = read
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| {
                if path.is_dir() {
                    path.join(MANIFEST).is_file()
                } else {
                    path.extension().is_some_and(|ext| {
                        ext.eq_ignore_ascii_case("zip") || ext.eq_ignore_ascii_case("oagmod")
                    })
                }
            })
            .collect();
        here.sort();
        out.extend(here.into_iter().map(|path| Found { path }));
    }
    out
}

/// What a search loaded, and what it had to leave out.
#[derive(Debug, Default)]
pub struct Loaded {
    /// The mods that opened, in the order found.
    pub mods: Vec<Mod>,
    /// One line for each candidate left out, naming it and why.
    pub problems: Vec<String>,
}

/// Opens every candidate under `bases`. A broken one, a second copy of an id
/// already loaded, and one for another game version are left out and named in
/// [`Loaded::problems`]: a bad mod never stops the game from starting.
#[must_use]
pub fn load(bases: &[PathBuf], game: &semver::Version) -> Loaded {
    let mut loaded = Loaded::default();
    for found in search(bases) {
        match Mod::open(&found.path) {
            Ok(module) => {
                if !module.manifest.works_with(game) {
                    loaded.problems.push(format!(
                        "{}: {} asks for oag {}, this is {game}",
                        found.path.display(),
                        module.manifest.id,
                        module
                            .manifest
                            .oag
                            .as_ref()
                            .map_or_else(String::new, ToString::to_string),
                    ));
                } else if let Some(first) = loaded
                    .mods
                    .iter()
                    .find(|have| have.manifest.id == module.manifest.id)
                {
                    loaded.problems.push(format!(
                        "{}: id {:?} is already loaded from {}",
                        found.path.display(),
                        module.manifest.id,
                        first.label()
                    ));
                } else {
                    loaded.mods.push(module);
                }
            }
            Err(error) => loaded.problems.push(error_line(&error)),
        }
    }
    loaded
}

fn error_line(error: &Error) -> String {
    error.to_string()
}
