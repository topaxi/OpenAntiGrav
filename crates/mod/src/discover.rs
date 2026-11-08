//! Finding mods on disk, and loading what is found.

use std::path::{Path, PathBuf};

use crate::{Error, MANIFEST, Mod, texture_pack};

/// A colon-separated (semicolon on Windows) list of directories that replaces
/// the default search path, as `OAG_OVERRIDES` does for overrides.
pub const SEARCH_ENV: &str = "OAG_MODS";

/// The repository's example mods, relative to the working directory: found when
/// the game runs from a checkout (`just play`), the way `data/mods` is, and by
/// nothing a player installs, which has no such folder.
pub const EXAMPLES: &str = "assets/mods/examples";

/// The project's official mods (`OpenAntiGrav/official-mods`, ADR-0083), relative to the
/// working directory: a checkout's submodule. The game runs without it.
pub const OFFICIAL: &str = "assets/mods/official";

/// Where PPSSPP texture packs are looked for beside the mods: a checkout's
/// `data/texture-packs`, which `just fetch-texture-packs` fills.
pub const TEXTURE_PACKS: &str = "data/texture-packs";

/// Where mods are looked for when nothing names a directory: `data/mods` in a
/// checkout, [`OFFICIAL`] and [`EXAMPLES`] in a checkout, `mods` beside the AppImage file and beside the program (inside the image
/// for an AppImage), and `<data dir>/oag/mods`; then the texture packs: [`TEXTURE_PACKS`],
/// `<data dir>/oag/texture-packs` and a PPSSPP memstick's `PSP/TEXTURES`.
#[must_use]
pub fn mods_search_path() -> Vec<PathBuf> {
    if let Some(from_env) = std::env::var_os(SEARCH_ENV) {
        let paths: Vec<PathBuf> = std::env::split_paths(&from_env).collect();
        if !paths.is_empty() {
            return paths;
        }
    }
    let mut paths = vec![PathBuf::from("data/mods")];
    paths.push(PathBuf::from(OFFICIAL));
    paths.push(PathBuf::from(EXAMPLES));
    let appimage = std::env::var_os("APPIMAGE").map(PathBuf::from);
    let program = std::env::current_exe().ok();
    packaged_mods(appimage.as_deref(), program.as_deref(), &mut paths);
    if let Some(data) = dirs::data_dir() {
        paths.push(data.join("oag").join("mods"));
    }
    paths.push(PathBuf::from(TEXTURE_PACKS));
    if let Some(data) = dirs::data_dir() {
        paths.push(data.join("oag").join("texture-packs"));
    }
    if let Some(config) = dirs::config_dir() {
        paths.push(config.join("ppsspp").join("PSP").join("TEXTURES"));
    }
    paths
}

/// Adds the `mods` folders a packaged build ships: beside the AppImage file (where a
/// player puts their own) and beside the program, which for an AppImage is inside the
/// mounted image (`usr/bin`, where the build puts the official mods).
pub(crate) fn packaged_mods(
    appimage: Option<&Path>,
    program: Option<&Path>,
    paths: &mut Vec<PathBuf>,
) {
    for file in [appimage, program].into_iter().flatten() {
        if let Some(directory) = file.parent() {
            let mods = directory.join("mods");
            if !paths.contains(&mods) {
                paths.push(mods);
            }
        }
    }
}

/// One candidate under a search directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    /// The folder, `.zip` or `.oagmod`.
    pub path: PathBuf,
}

/// Every candidate directly inside `bases`: a folder holding an `oag-mod.toml`, a `.zip`, or a
/// `.oagmod`, and a texture pack: a folder holding a `textures.ini` (read as it lies) or a
/// `textures.zip` (the zip is the candidate, as PPSSPP prefers it). Sorted by name within a
/// base, the bases in the order given; a path already found under another name (a memstick
/// folder linked once per game serial) is found once.
#[must_use]
pub fn search(bases: &[PathBuf]) -> Vec<Found> {
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for base in bases {
        let Ok(read) = std::fs::read_dir(base) else {
            continue;
        };
        let mut here: Vec<PathBuf> = read
            .flatten()
            .map(|entry| entry.path())
            .filter_map(|path| candidate(&path))
            .collect();
        here.sort();
        for path in here {
            let canonical = std::fs::canonicalize(&path).unwrap_or_else(|_| path.clone());
            if seen.insert(canonical) {
                out.push(Found { path });
            }
        }
    }
    out
}

/// What a directory entry offers as a mod, if anything.
fn candidate(path: &Path) -> Option<PathBuf> {
    if path.is_dir() {
        if path.join(MANIFEST).is_file() {
            return Some(path.to_path_buf());
        }
        let zip = path.join(texture_pack::ZIP);
        if zip.is_file() {
            return Some(zip);
        }
        return path
            .join(texture_pack::INI)
            .is_file()
            .then(|| path.to_path_buf());
    }
    path.extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("zip") || ext.eq_ignore_ascii_case("oagmod"))
        .then(|| path.to_path_buf())
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
