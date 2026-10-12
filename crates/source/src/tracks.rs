//! Where a player's own-format tracks live, and the list a custom-track picker
//! offers: these, then BallisticNG's ([`crate::ballisticng`]).
//!
//! A track in this project's own format is a `*.track.toml` manifest with its
//! `.glb` beside it (ADR-0063). It is searched in `data/tracks` (a checkout),
//! `tracks/` beside the program and `<data dir>/oag/tracks`, or the path list in
//! `$OAG_TRACKS`. Unlike a BallisticNG base, a directory here holds the
//! manifests themselves, and one level of subdirectory is read too. Only
//! directory listings are read, never a file.

use std::path::{Path, PathBuf};

use crate::ballisticng::{is_file_with, subdirectories};

/// The environment variable that names the directories outright, split the way
/// the platform splits `$PATH`.
pub const TRACKS_ENV: &str = "OAG_TRACKS";

/// Where own-format tracks are looked for when nothing names a directory.
#[must_use]
pub fn search_path() -> Vec<PathBuf> {
    if let Some(from_env) = std::env::var_os(TRACKS_ENV) {
        let paths: Vec<PathBuf> = std::env::split_paths(&from_env).collect();
        if !paths.is_empty() {
            return paths;
        }
    }
    let mut paths = vec![PathBuf::from("data/tracks")];
    if let Some(directory) = crate::source::portable_directory() {
        paths.push(directory.join("tracks"));
    }
    if let Some(data) = dirs::data_dir() {
        paths.push(data.join("oag").join("tracks"));
    }
    paths
}

fn is_manifest(path: &Path) -> bool {
    is_file_with(path, "toml")
        && path
            .file_name()
            .is_some_and(|n| n.to_string_lossy().to_lowercase().ends_with(".track.toml"))
}

/// Every `*.track.toml` in `dirs`, each directory's own files first and then
/// those one level down, sorted by path.
#[must_use]
pub fn own_in(dirs: &[PathBuf]) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for dir in dirs {
        let levels = std::iter::once(dir.clone()).chain(subdirectories(dir));
        for level in levels {
            let Ok(entries) = std::fs::read_dir(&level) else {
                continue;
            };
            let mut found: Vec<PathBuf> = entries
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| is_manifest(p))
                .collect();
            found.sort();
            for path in found {
                if !out.contains(&path) {
                    out.push(path);
                }
            }
        }
    }
    out
}

/// Own-format tracks in `own`, then BallisticNG's under `bases`.
#[must_use]
pub fn all_in(own: &[PathBuf], bases: &[PathBuf]) -> Vec<PathBuf> {
    let mut out = own_in(own);
    for path in crate::ballisticng::tracks_in(bases) {
        if !out.contains(&path) {
            out.push(path);
        }
    }
    out
}

/// [`all_in`] over both default search paths.
#[must_use]
pub fn all() -> Vec<PathBuf> {
    all_in(&search_path(), &crate::ballisticng::search_path())
}

/// The track a command line names, among [`all`]: see
/// [`crate::ballisticng::resolve_among`].
#[must_use]
pub fn resolve(name: &str) -> Option<PathBuf> {
    crate::ballisticng::resolve_among(&all(), name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn own_tracks_are_found_flat_and_one_level_down_and_listed_before_workshop_ones() {
        let base = std::env::temp_dir().join(format!("oag-own-tracks-{}", std::process::id()));
        let bng = base.join("bng");
        std::fs::create_dir_all(base.join("own/pack")).unwrap();
        std::fs::create_dir_all(base.join("own/pack/deeper")).unwrap();
        std::fs::create_dir_all(bng.join("tracks")).unwrap();
        std::fs::write(base.join("own/mine.track.toml"), b"x").unwrap();
        std::fs::write(base.join("own/mine.glb"), b"x").unwrap();
        std::fs::write(base.join("own/notes.toml"), b"x").unwrap();
        std::fs::write(base.join("own/pack/two.TRACK.TOML"), b"x").unwrap();
        std::fs::write(base.join("own/pack/deeper/lost.track.toml"), b"x").unwrap();
        std::fs::write(bng.join("tracks/lion.wtrk"), b"x").unwrap();
        let own = [base.join("own"), base.join("missing")];
        assert_eq!(
            own_in(&own),
            [
                base.join("own/mine.track.toml"),
                base.join("own/pack/two.TRACK.TOML")
            ]
        );
        let both = all_in(&own, std::slice::from_ref(&bng));
        assert_eq!(both.last(), Some(&bng.join("tracks/lion.wtrk")));
        assert_eq!(both.len(), 3);
        std::fs::remove_dir_all(&base).unwrap();
    }
}
