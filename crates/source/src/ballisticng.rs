//! Where a player's BallisticNG content lives: the foreign-track source.
//!
//! A foreign track is its own source beside the disc images, not an entry of any
//! title's archives and not an asset-override layer (ADR-0062, decision 4). It is
//! looked for the way the override layer is: a directory per base, searched in
//! `data/ballisticng` (a checkout), beside the program, and `<data dir>/oag/ballisticng`, or
//! the path list in `$OAG_BALLISTICNG`. Under a base, tracks are `tracks/*.wtrk` (or
//! the same one or two directories down, the shape of a Steam Workshop download) and ships `ships/*.csf`. Nothing here opens a file: reading belongs to the content
//! layer, and a missing directory is the ordinary state of a copy of the game.

use std::path::{Path, PathBuf};

/// The environment variable that names the base directories outright, split the
/// way the platform splits `$PATH`.
pub const BALLISTICNG_ENV: &str = "OAG_BALLISTICNG";

/// Where bases are looked for when nothing names one.
#[must_use]
pub fn search_path() -> Vec<PathBuf> {
    if let Some(from_env) = std::env::var_os(BALLISTICNG_ENV) {
        let paths: Vec<PathBuf> = std::env::split_paths(&from_env).collect();
        if !paths.is_empty() {
            return paths;
        }
    }
    let mut paths = vec![PathBuf::from("data/ballisticng")];
    if let Some(directory) = crate::source::portable_directory() {
        paths.push(directory.join("ballisticng"));
    }
    if let Some(data) = dirs::data_dir() {
        paths.push(data.join("oag").join("ballisticng"));
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

pub(crate) fn is_file_with(path: &Path, extension: &str) -> bool {
    path.is_file()
        && path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case(extension))
}

/// Whether `path` is a Workshop `.wtrk`. This project's own `*.track.toml`
/// files live in [`crate::tracks`], not here.
fn is_track(path: &Path) -> bool {
    is_file_with(path, "wtrk")
}

/// Every track under `bases`, base by base: `tracks/` first, then the
/// directories one and two levels down, each sorted by path.
///
/// The deeper levels are the shapes a Steam Workshop download takes:
/// `workshop/content/473770/<item>/<name>.wtrk` is a base whose tracks sit one
/// level down, and a base holding that `473770` directory (`data/ballisticng`) has
/// them two down. Only directory listings are read, never a file, so a game
/// install living under a base costs a few `read_dir` calls and finds nothing.
#[must_use]
pub fn tracks_in(bases: &[PathBuf]) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for base in bases {
        let first = subdirectories(base);
        let second: Vec<PathBuf> = first.iter().flat_map(|d| subdirectories(d)).collect();
        let dirs = std::iter::once(base.join("tracks"))
            .chain(first)
            .chain(second);
        for dir in dirs {
            let Ok(entries) = std::fs::read_dir(&dir) else {
                continue;
            };
            let mut found: Vec<PathBuf> = entries
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| is_track(p))
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

pub(crate) fn subdirectories(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut found: Vec<PathBuf> = entries
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_dir())
        .collect();
    found.sort();
    found
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
    resolve_among(&tracks_in(bases), name)
}

/// [`resolve_in`] over a list already made, so a list joining several sources
/// ([`crate::tracks::all_in`]) resolves a name the way one source does.
#[must_use]
pub fn resolve_among(all: &[PathBuf], name: &str) -> Option<PathBuf> {
    let direct = Path::new(name);
    if direct.is_file() {
        return Some(direct.to_path_buf());
    }
    let wanted = name.to_lowercase();
    let stem = |p: &PathBuf| {
        let name = p
            .file_name()
            .map(|s| s.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        name.strip_suffix(".track.toml")
            .or_else(|| name.rsplit_once('.').map(|(stem, _)| stem))
            .unwrap_or(&name)
            .to_string()
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
        let base =
            std::env::temp_dir().join(format!("oag-ballisticng-source-{}", std::process::id()));
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

    #[test]
    fn a_workshop_download_is_found_and_an_own_format_track_is_not() {
        let base =
            std::env::temp_dir().join(format!("oag-ballisticng-layout-{}", std::process::id()));
        // `<base>/<content id>/<item>/<name>.wtrk`, as Steam lays it out.
        std::fs::create_dir_all(base.join("473770/111")).unwrap();
        std::fs::create_dir_all(base.join("tracks")).unwrap();
        std::fs::create_dir_all(base.join("deep/er/still")).unwrap();
        std::fs::write(base.join("473770/111/lion.wtrk"), b"x").unwrap();
        std::fs::write(base.join("473770/111/lion.csf"), b"x").unwrap();
        std::fs::write(base.join("tracks/mine.track.toml"), b"x").unwrap();
        std::fs::write(base.join("deep/er/still/too_deep.wtrk"), b"x").unwrap();
        let found = tracks_in(std::slice::from_ref(&base));
        assert_eq!(
            found,
            [base.join("473770/111/lion.wtrk")],
            "an own-format track is not BallisticNG content"
        );
        assert_eq!(resolve_in(std::slice::from_ref(&base), "MINE"), None);
        // The base is the content directory itself: one level down.
        let content = base.join("473770");
        assert_eq!(tracks_in(&[content]), [base.join("473770/111/lion.wtrk")]);
        std::fs::remove_dir_all(&base).unwrap();
    }
}
