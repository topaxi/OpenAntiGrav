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

/// The track a command line names: a path to a `.wtrk` that exists, or a name
/// matched against the file stems under `bases`, exactly (ignoring case) before
/// by substring. `None` when nothing matches or a substring matches several.
#[must_use]
pub fn resolve_in(bases: &[PathBuf], name: &str) -> Option<PathBuf> {
    let direct = Path::new(name);
    if direct.is_file() {
        return Some(direct.to_path_buf());
    }
    let wanted = name.to_lowercase();
    let all = tracks_in(bases);
    let stem = |p: &PathBuf| {
        p.file_stem()
            .map(|s| s.to_string_lossy().to_lowercase())
            .unwrap_or_default()
    };
    if let Some(exact) = all.iter().find(|p| stem(p) == wanted) {
        return Some(exact.clone());
    }
    let mut partial = all.iter().filter(|p| stem(p).contains(&wanted));
    let first = partial.next()?;
    partial.next().is_none().then(|| first.clone())
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
        assert_eq!(resolve_in(&bases, "A").unwrap(), base.join("tracks/a.wtrk"));
        assert_eq!(resolve_in(&bases, "b"), Some(base.join("tracks/b.WTRK")));
        std::fs::write(base.join("tracks/lion one.wtrk"), b"x").unwrap();
        std::fs::write(base.join("tracks/lion two.wtrk"), b"x").unwrap();
        let bases = [base.clone()];
        assert_eq!(resolve_in(&bases, "lion"), None, "two partial matches");
        assert_eq!(
            resolve_in(&bases, "one"),
            Some(base.join("tracks/lion one.wtrk"))
        );
        assert_eq!(resolve_in(&bases, "zzz"), None);
        std::fs::remove_dir_all(&base).unwrap();
    }
}
