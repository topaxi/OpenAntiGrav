//! Where the disc image is looked for when the command line does not name one.
//!
//! **No game content ships with this engine, ever** - see
//! `docs/overview/legal.md`. A packaged build is the engine and nothing else,
//! so the one thing packaging has to answer is how a player's own image is
//! found. This module is that answer, and it lives in the composition root
//! because it is policy: it reads environment variables and the user's home
//! directory, neither of which `oag-assets` is allowed to know about.
//!
//! The search, in order, and the first hit wins:
//!
//! 1. The **command-line argument**, used verbatim. A file, a directory
//!    extracted with `oag-unpack`, or an `image:path` archive spec - whatever
//!    `oag_assets::Archives::open` accepts.
//! 2. **`$OAG_IMAGE`**, the same three things.
//! 3. **`settings.toml`'s `[source] image`**, the same three things again,
//!    persisted so a player who always plays off one disc does not have to
//!    repeat it on every run. See `crate::settings::Source`.
//! 4. **`data/images/`** relative to the current directory, which is what a
//!    repository checkout has and what keeps `just play` working.
//! 5. **Beside the AppImage**: `$APPIMAGE`'s own directory, and an `images/`
//!    directory in it. This is portable mode - copy the AppImage and an image
//!    into the same folder on a Steam Deck and run it.
//! 6. **`<data dir>/oag/images/`**, i.e. `~/.local/share/oag/images` on Linux.
//!    The stable place to keep an image that is not next to the AppImage.
//!
//! `$APPDIR` is deliberately **not** searched. That is the mounted AppImage
//! itself, and looking inside it would invite someone to bundle an image into
//! the package, which is the one thing that must never happen.

use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};

/// The default the repository checkout has, and the documented example.
pub const DEFAULT_IMAGE: &str = "data/images/pulse-psp-usa.chd";

/// Names a disc image is looked for under, in order.
///
/// The normalised names from `data/README.md`. Pulse PSP first: it is the
/// platform the implementation follows, and the PS2 release is corroboration.
/// Pure and HD/Fury are listed too, purely so a directory holding only one of
/// them is found by name rather than by alphabetical luck - `oag-game` does
/// not yet play either; opening one now fails with a clear error naming the
/// title instead of silently loading it as Pulse. See ADR-0009 and roadmap
/// M8.
pub const IMAGE_NAMES: [&str; 4] = [
    "pulse-psp-usa.chd",
    "pulse-ps2-eu.chd",
    "pure-psp-usa.chd",
    "hdfury-ps3-eu.iso",
];

/// Container extensions a directory scan will accept, lowercase.
const EXTENSIONS: [&str; 2] = ["chd", "iso"];

/// The environment variable that names an image outright.
pub const IMAGE_ENV: &str = "OAG_IMAGE";

/// The environment variable that names where downloadable content lives.
///
/// A path list, split the way the platform splits `$PATH`, so several folders
/// can be named at once.
pub const DLC_ENV: &str = "OAG_DLC";

/// Where [`resolve_dlc`] looks for packs when nothing names one.
///
/// The same three places as [`search_path`], under `dlc/` rather than
/// `images/`. Deliberately a sibling: content a player downloaded is content
/// they own, and it belongs beside the disc image rather than inside the
/// installation. `$APPDIR` is excluded here for exactly the reason it is
/// excluded there - see the module docs and ADR-0006.
#[must_use]
pub fn dlc_search_path() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let mut push = |path: PathBuf| {
        if !paths.contains(&path) {
            paths.push(path);
        }
    };

    push(PathBuf::from("data/dlc"));

    if let Some(appimage) = std::env::var_os("APPIMAGE")
        && let Some(directory) = Path::new(&appimage).parent()
    {
        push(directory.join("dlc"));
    }

    if let Some(data) = dirs::data_dir() {
        push(data.join("oag").join("dlc"));
    }

    paths
}

/// Resolves where to look for downloadable content.
///
/// The same precedence as [`resolve`] - command line, environment,
/// `settings.toml`, then the search path - with one difference that matters:
/// this **cannot fail**. No DLC is the ordinary state of a copy of the game,
/// so a root that does not exist is not an error and an empty result is not a
/// problem to report. Anything genuinely wrong shows up later, as a pack that
/// would not unpack; see [`crate::dlc::packs`].
#[must_use]
pub fn resolve_dlc(given: &[String], from_settings: &[String]) -> Vec<PathBuf> {
    if !given.is_empty() {
        return given.iter().map(PathBuf::from).collect();
    }

    if let Some(from_env) = std::env::var_os(DLC_ENV) {
        let paths: Vec<PathBuf> = std::env::split_paths(&from_env).collect();
        if !paths.is_empty() {
            return paths;
        }
    }

    if !from_settings.is_empty() {
        return from_settings.iter().map(PathBuf::from).collect();
    }

    dlc_search_path()
}

/// Resolves the source the game should open.
///
/// `given` is the command line's, if it named one. `from_settings` is
/// `settings.toml`'s `[source] image`, if set - tried after `$OAG_IMAGE` and
/// before the directories below, the same three forms as `given`. The error
/// names every path searched: for a "copy the AppImage and an image into one
/// folder" workflow, that message is the entire user interface.
pub fn resolve(given: Option<&str>, from_settings: Option<&str>) -> Result<String> {
    if let Some(source) = given {
        return Ok(source.to_string());
    }

    if let Some(from_env) = std::env::var_os(IMAGE_ENV) {
        let path = PathBuf::from(from_env);
        if path.is_file() {
            return Ok(display(&path));
        }
        if let Some(found) = first_image(&path) {
            return Ok(display(&found));
        }
        return Err(anyhow!(
            "{IMAGE_ENV} is set to {}, which is not a disc image and holds none",
            path.display()
        ));
    }

    if let Some(source) = from_settings {
        return Ok(source.to_string());
    }

    let searched = search_path();
    for directory in &searched {
        if let Some(found) = first_image(directory) {
            return Ok(display(&found));
        }
    }

    Err(anyhow!(
        "no disc image found. OpenAntiGrav ships no game content: supply your own \
         image, from your own copy of the game.\n\nSearched:\n{}\n\nName one \
         directly (`oag-game path/to/pulse-psp-usa.chd`), set {IMAGE_ENV}, or put \
         a {} beside the executable.",
        searched
            .iter()
            .map(|path| format!("  {}", path.display()))
            .collect::<Vec<_>>()
            .join("\n"),
        IMAGE_NAMES[0],
    ))
}

/// Every directory [`resolve`] looks in, in order.
///
/// Built rather than iterated lazily so the not-found message can list exactly
/// what was tried, including the paths that do not exist.
#[must_use]
pub fn search_path() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let mut push = |path: PathBuf| {
        if !paths.contains(&path) {
            paths.push(path);
        }
    };

    push(PathBuf::from("data/images"));

    // `$APPIMAGE` is the path of the running AppImage file, set by its runtime.
    // Its *parent* is the portable directory; `$APPDIR`, the mounted image, is
    // not searched on purpose.
    if let Some(appimage) = std::env::var_os("APPIMAGE")
        && let Some(directory) = Path::new(&appimage).parent()
    {
        // The directory itself first - "copy both files into one folder" is
        // the simplest thing a player can do - then an `images/`
        // subdirectory, for someone who would rather keep it tidy.
        push(directory.to_path_buf());
        push(directory.join("images"));
    }

    if let Some(data) = dirs::data_dir() {
        push(data.join("oag").join("images"));
    }

    paths
}

/// The first image in a directory: a known name if there is one, otherwise the
/// alphabetically first container.
///
/// Alphabetical rather than whatever order the filesystem hands out, so two
/// runs in a directory holding two images open the same one.
fn first_image(directory: &Path) -> Option<PathBuf> {
    for name in IMAGE_NAMES {
        let candidate = directory.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }

    let mut containers: Vec<PathBuf> = std::fs::read_dir(directory)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_file() && is_container(path))
        .collect();
    containers.sort();
    containers.into_iter().next()
}

/// Whether a path looks like a disc image this engine can open.
///
/// Public because [`crate::audio::MusicDiscs`] walks the same directories
/// looking for the *other* release rather than the first one, and a second
/// spelling of "what counts as an image" is a second thing to keep in step.
#[must_use]
pub fn is_container(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            let lowercase = extension.to_ascii_lowercase();
            EXTENSIONS.contains(&lowercase.as_str())
        })
}

/// A path as the rest of the game spells a source: a string.
///
/// Lossy on the vanishingly rare non-UTF-8 path, which is the same trade the
/// `--source` argument already makes by being a `String`.
fn display(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_named_source_is_used_verbatim() {
        // Including an archive spec, which is not a path at all.
        let spec = "data/images/pulse-psp-usa.chd:PSP_GAME/USRDIR/FE.wad";
        assert_eq!(resolve(Some(spec), None).unwrap(), spec);
        assert_eq!(
            resolve(Some("nonexistent.chd"), None).unwrap(),
            "nonexistent.chd"
        );
    }

    /// The DLC roots follow the same precedence, with `$OAG_DLC` left out of
    /// the test on purpose: setting a process-wide variable would race the
    /// other tests in this binary, and what is worth pinning is the ordering
    /// this function decides rather than that `var_os` works.
    #[test]
    fn dlc_roots_follow_the_command_line_then_the_settings_file() {
        let cli = ["/from/cli".to_string()];
        let settings = ["/from/settings".to_string()];

        assert_eq!(
            resolve_dlc(&cli, &settings),
            [PathBuf::from("/from/cli")],
            "the command line wins"
        );
        assert_eq!(
            resolve_dlc(&[], &settings),
            [PathBuf::from("/from/settings")],
            "and the settings file is next"
        );
    }

    /// No DLC is the ordinary state of a copy of the game, so the empty case
    /// falls through to the search path rather than failing or returning
    /// nothing.
    #[test]
    fn dlc_roots_fall_back_to_the_search_path_and_never_fail() {
        // Only when the environment is not already pointing somewhere, which
        // is a real thing a developer's shell may do.
        if std::env::var_os(DLC_ENV).is_some() {
            return;
        }
        assert_eq!(resolve_dlc(&[], &[]), dlc_search_path());
        assert!(
            dlc_search_path().contains(&PathBuf::from("data/dlc")),
            "a checkout's own directory is always searched"
        );
    }

    /// The images and the packs are looked for in sibling directories, never
    /// the same one: a disc image is not a pack and scanning one folder for
    /// both would make every image a candidate archive.
    #[test]
    fn the_dlc_search_path_is_a_sibling_of_the_image_search_path() {
        for path in dlc_search_path() {
            assert!(
                !search_path().contains(&path),
                "{} is searched for both images and packs",
                path.display()
            );
        }
    }

    #[test]
    fn a_settings_file_source_is_tried_before_the_search_path() {
        assert_eq!(
            resolve(None, Some("from-settings.chd")).unwrap(),
            "from-settings.chd"
        );
    }

    #[test]
    fn the_command_line_wins_over_the_settings_file() {
        assert_eq!(
            resolve(Some("from-cli.chd"), Some("from-settings.chd")).unwrap(),
            "from-cli.chd"
        );
    }

    #[test]
    fn the_checkouts_own_directory_is_searched_first() {
        assert_eq!(search_path().first().unwrap(), Path::new("data/images"));
    }

    /// The mounted AppImage must never be searched: an image found inside the
    /// package would mean game content had been packaged, which is the one
    /// thing `docs/overview/legal.md` forbids outright.
    #[test]
    fn the_mounted_appimage_is_not_searched() {
        for path in search_path() {
            let text = path.to_string_lossy().to_lowercase();
            assert!(
                !text.contains("/tmp/.mount_") && !text.contains("appdir"),
                "the package itself is on the search path: {}",
                path.display()
            );
        }
    }

    #[test]
    fn a_known_name_wins_over_an_alphabetically_earlier_image() {
        let directory = temp_dir("known-name");
        std::fs::write(directory.join("aaa-other.iso"), b"").unwrap();
        std::fs::write(directory.join(IMAGE_NAMES[0]), b"").unwrap();

        assert_eq!(
            first_image(&directory).unwrap().file_name().unwrap(),
            std::ffi::OsStr::new(IMAGE_NAMES[0])
        );
        std::fs::remove_dir_all(&directory).unwrap();
    }

    /// `IMAGE_NAMES[0]` is interpolated into the not-found hint text, and the
    /// module doc's "PSP first" rationale depends on this order.
    #[test]
    fn image_names_starts_with_pulse_psp_matching_the_hint_text() {
        assert_eq!(IMAGE_NAMES[0], "pulse-psp-usa.chd");
        assert_eq!(IMAGE_NAMES[1], "pulse-ps2-eu.chd");
    }

    /// The maintainer's actual `data/images/` holds all four documented
    /// names at once - this is the check that extending `IMAGE_NAMES` to
    /// recognise Pure and HD/Fury by name does not change which file that
    /// directory resolves to.
    #[test]
    fn a_known_pulse_name_still_wins_when_every_documented_name_is_present() {
        let directory = temp_dir("all-four-names");
        for name in IMAGE_NAMES {
            std::fs::write(directory.join(name), b"").unwrap();
        }

        assert_eq!(
            first_image(&directory).unwrap().file_name().unwrap(),
            std::ffi::OsStr::new(IMAGE_NAMES[0])
        );
        std::fs::remove_dir_all(&directory).unwrap();
    }

    #[test]
    fn any_image_is_found_under_a_name_nobody_normalised() {
        let directory = temp_dir("any-name");
        std::fs::write(directory.join("notes.txt"), b"").unwrap();
        std::fs::write(directory.join("zzz.chd"), b"").unwrap();
        std::fs::write(directory.join("my-copy.ISO"), b"").unwrap();

        // Alphabetical, so the same image is opened on every run.
        assert_eq!(
            first_image(&directory).unwrap().file_name().unwrap(),
            std::ffi::OsStr::new("my-copy.ISO")
        );
        std::fs::remove_dir_all(&directory).unwrap();
    }

    #[test]
    fn a_directory_with_nothing_in_it_is_not_a_source() {
        let directory = temp_dir("empty");
        assert_eq!(first_image(&directory), None);
        std::fs::remove_dir_all(&directory).unwrap();
    }

    #[test]
    fn a_missing_directory_is_not_an_error() {
        assert_eq!(first_image(Path::new("/nonexistent/oag/images")), None);
    }

    /// A directory of this test's own, so the tests do not depend on - or
    /// disturb - whatever the developer has in `data/images`.
    fn temp_dir(name: &str) -> PathBuf {
        let directory = std::env::temp_dir().join(format!("oag-source-{name}"));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).unwrap();
        directory
    }
}
