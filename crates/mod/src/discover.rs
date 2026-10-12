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

/// A candidate the search left out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failed {
    /// The folder, `.zip` or `.oagmod`.
    pub path: PathBuf,
    /// The mod's id, when its manifest was read before the reason arose.
    pub id: Option<String>,
    /// Why it was left out.
    pub reason: String,
}

impl Failed {
    /// [`Self::reason`] without the leading `<path>: `, for a list that names the file already.
    #[must_use]
    pub fn cause(&self) -> &str {
        self.reason
            .strip_prefix(&format!("{}: ", self.path.display()))
            .unwrap_or(&self.reason)
    }

    /// The path's last component, which is what a list names a mod that never opened by.
    #[must_use]
    pub fn file_name(&self) -> String {
        self.path.file_name().map_or_else(
            || self.path.display().to_string(),
            |name| name.to_string_lossy().into_owned(),
        )
    }
}

impl std::fmt::Display for Failed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.reason)
    }
}

/// What a search loaded, and what it had to leave out.
#[derive(Debug, Default)]
pub struct Loaded {
    /// The mods that opened, in the order found.
    pub mods: Vec<Mod>,
    /// Each candidate left out, with the reason; its `Display` is the load report's line.
    pub failed: Vec<Failed>,
}

/// Opens every candidate under `bases`. A broken one, a second copy of an id
/// already loaded, and one for another game version are left out and named in
/// [`Loaded::failed`]: a bad mod never stops the game from starting.
#[must_use]
pub fn load(bases: &[PathBuf], game: &semver::Version) -> Loaded {
    let mut loaded = Loaded::default();
    for found in search(bases) {
        match Mod::open(&found.path) {
            Ok(module) => {
                if !module.manifest.works_with(game) {
                    loaded.failed.push(Failed {
                        reason: format!(
                            "{}: {} asks for oag {}, this is {game}",
                            found.path.display(),
                            module.manifest.id,
                            module
                                .manifest
                                .oag
                                .as_ref()
                                .map_or_else(String::new, ToString::to_string),
                        ),
                        id: Some(module.manifest.id.clone()),
                        path: found.path,
                    });
                } else if let Some(first) = loaded
                    .mods
                    .iter()
                    .find(|have| have.manifest.id == module.manifest.id)
                {
                    loaded.failed.push(Failed {
                        reason: format!(
                            "{}: id {:?} is already loaded from {}",
                            found.path.display(),
                            module.manifest.id,
                            first.label()
                        ),
                        id: Some(module.manifest.id.clone()),
                        path: found.path,
                    });
                } else {
                    loaded.mods.push(module);
                }
            }
            Err(error) => loaded.failed.push(Failed {
                reason: error_line(&error),
                id: None,
                path: found.path,
            }),
        }
    }
    loaded
}

fn error_line(error: &Error) -> String {
    error.to_string()
}
