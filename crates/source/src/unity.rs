//! Where a player's Unity content lives: the foreign-track source.
//!
//! A foreign track is its own source beside the disc images, not an entry of any
//! title's archives and not an asset-override layer (ADR-0062, decision 4). It is
//! looked for the way the override layer is: a directory per base, searched in
//! `data/unity` (a checkout), beside the program, and `<data dir>/oag/unity`, or
//! the path list in `$OAG_UNITY`. Under a base, tracks are `tracks/*.wtrk` and
//! ships `ships/*.csf`. Nothing here opens a file: reading belongs to the content
//! layer, and a missing directory is the ordinary state of a copy of the game.

use std::path::{Path, PathBuf};

/// The environment variable that names the base directories outright, split the
/// way the platform splits `$PATH`.
pub const UNITY_ENV: &str = "OAG_UNITY";

/// Where bases are looked for when nothing names one.
#[must_use]
pub fn search_path() -> Vec<PathBuf> {
    if let Some(from_env) = std::env::var_os(UNITY_ENV) {
        let paths: Vec<PathBuf> = std::env::split_paths(&from_env).collect();
        if !paths.is_empty() {
            return paths;
        }
    }
    let mut paths = vec![PathBuf::from("data/unity")];
    if let Some(directory) = crate::source::portable_directory() {
        paths.push(directory.join("unity"));
    }
    if let Some(data) = dirs::data_dir() {
        paths.push(data.join("oag").join("unity"));
    }
    paths
}

fn listed(bases: &[PathBuf], sub: &str, extension: &str) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for base in bases {
        let Ok(entries) = std::fs::read_dir(base.join(sub)) else {
            continue;
        };
        let mut found: Vec<PathBuf> = entries
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| is_file_with(p, extension))
            .collect();
        found.sort();
        out.extend(found);
    }
    out
}

fn is_file_with(path: &Path, extension: &str) -> bool {
    path.is_file()
        && path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case(extension))
}

/// Every `.wtrk` track under `bases`, base by base, each base sorted by path.
#[must_use]
pub fn tracks_in(bases: &[PathBuf]) -> Vec<PathBuf> {
    listed(bases, "tracks", "wtrk")
}

/// Every `.csf` ship under `bases`, in the same order.
#[must_use]
pub fn ships_in(bases: &[PathBuf]) -> Vec<PathBuf> {
    listed(bases, "ships", "csf")
}

/// [`tracks_in`] over [`search_path`].
#[must_use]
pub fn tracks() -> Vec<PathBuf> {
    tracks_in(&search_path())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracks_and_ships_are_listed_by_extension_per_base() {
        let base = std::env::temp_dir().join(format!("oag-unity-source-{}", std::process::id()));
        std::fs::create_dir_all(base.join("tracks")).unwrap();
        std::fs::create_dir_all(base.join("ships")).unwrap();
        std::fs::write(base.join("tracks/b.WTRK"), b"x").unwrap();
        std::fs::write(base.join("tracks/a.wtrk"), b"x").unwrap();
        std::fs::write(base.join("tracks/notes.txt"), b"x").unwrap();
        std::fs::write(base.join("ships/s.csf"), b"x").unwrap();
        let bases = [base.clone(), base.join("missing")];
        let names = |v: Vec<PathBuf>| -> Vec<String> {
            v.iter()
                .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
                .collect()
        };
        assert_eq!(names(tracks_in(&bases)), ["a.wtrk", "b.WTRK"]);
        assert_eq!(names(ships_in(&bases)), ["s.csf"]);
        std::fs::remove_dir_all(&base).unwrap();
    }
}
