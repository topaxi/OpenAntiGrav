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
//!
//! # The first three are a decision; the last three are a guess
//!
//! Steps 1 to 3 are a player *stating* which source they want, and [`explicit`]
//! is those three alone. Steps 4 to 6 are this module guessing, and
//! [`candidates`] is the whole guess rather than its first hit - which is what
//! [`crate::launcher`] puts on screen when there is more than one, instead of
//! opening one and never mentioning the rest. `resolve` is still the two halves
//! joined, unchanged, for every route that wants one answer and no screen.

use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};

/// The default the repository checkout has, and the documented example.
pub const DEFAULT_IMAGE: &str = "data/images/pulse-psp-usa.chd";

/// Names a disc image is looked for under, in order.
///
/// The normalised names from `data/README.md`. Pulse PSP first: it is the
/// platform the implementation follows, and the PS2 release is corroboration.
/// Pure and HD/Fury are listed too, so a directory holding only one of them is
/// found by name rather than by alphabetical luck. **Both now boot** - each has
/// a front end of its own, HD's off a chain its XML declares rather than one
/// anyone has watched (ADR-0025) - though neither is played past the menus yet;
/// see roadmap M8. Pure's EU image is listed before its USA one so a directory
/// holding both auto-detects as EU, matching the default region elsewhere.
pub const IMAGE_NAMES: [&str; 5] = [
    "pulse-psp-usa.chd",
    "pulse-ps2-eu.chd",
    "pure-psp-eu.chd",
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
    if let Some(source) = explicit(given, from_settings)? {
        return Ok(source);
    }

    let searched = search_path();
    for directory in &searched {
        if let Some(found) = first_image(directory) {
            return Ok(display(&found));
        }
    }

    Err(nothing_found(&searched))
}

/// What an explicit channel names, if any of them does.
///
/// The three ways a player *states* which source they want - the command line,
/// `$OAG_IMAGE`, `settings.toml`'s `[source] image` - in that order, with the
/// directory scan deliberately left out. `Ok(None)` means none of them said
/// anything, which is the one situation in which [`candidates`] and a chooser
/// have any business appearing: a stated source is a decision already taken,
/// and a screen asking it again would be asking a player to repeat themselves.
///
/// **`$OAG_IMAGE` naming something unusable is an error, not a `None`.** It is
/// the strongest statement of intent of the three, and falling through to the
/// search path would boot a different disc than the one that was asked for
/// without saying so.
///
/// # Errors
///
/// `$OAG_IMAGE` set to a path that is neither a disc image nor a directory
/// holding one.
pub fn explicit(given: Option<&str>, from_settings: Option<&str>) -> Result<Option<String>> {
    stated(given, std::env::var_os(IMAGE_ENV), from_settings)
}

/// [`explicit`] with the environment handed in rather than read.
///
/// The split is what makes the `$OAG_IMAGE` branch testable at all. Setting a
/// process-wide variable would race every other test in this binary - the
/// reason the DLC tests say they leave `$OAG_DLC` alone - and this workspace
/// forbids `unsafe`, which `std::env::set_var` now needs. Passing the value in
/// leaves one impure line above and a decision that can be checked below.
fn stated(
    given: Option<&str>,
    from_env: Option<std::ffi::OsString>,
    from_settings: Option<&str>,
) -> Result<Option<String>> {
    if let Some(source) = given {
        return Ok(Some(source.to_string()));
    }

    if let Some(from_env) = from_env {
        let path = PathBuf::from(from_env);
        if path.is_file() {
            return Ok(Some(display(&path)));
        }
        if let Some(found) = first_image(&path) {
            return Ok(Some(display(&found)));
        }
        return Err(anyhow!(
            "{IMAGE_ENV} is set to {}, which is not a disc image and holds none",
            path.display()
        ));
    }

    Ok(from_settings.map(str::to_string))
}

/// Every disc image on the search path, in the order a chooser should list them.
///
/// Each directory contributes its known names first and then everything else
/// alphabetically - the same order [`first_image`] picks from, so the first row
/// of a chooser is the image a boot with nothing named would have opened.
///
/// [`package_directories`] is appended last, once, rather than folded into the
/// per-directory loop above: unlike an image, an extracted package is not
/// something a player drops beside the executable, it is the one fixed
/// location `data/README.md` documents, so there is exactly one place to look
/// rather than one per search-path entry. Appending it after every image
/// candidate keeps `first_image` - and so a no-argument boot - picking the
/// same source it always did on a machine that also has a `.chd` or `.iso`.
///
/// Empty is an ordinary answer: it means this machine has no image anywhere the
/// engine looks, and [`resolve`] is what turns that into the message about it.
#[must_use]
pub fn candidates() -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = Vec::new();
    let mut seen: Vec<PathBuf> = Vec::new();

    for directory in search_path() {
        // Two entries can name one directory - a checkout with the AppImage
        // sitting in it, say - and the same image listed twice is a chooser
        // offering the same row twice. `canonicalize` collapses them, and it
        // fails on a directory that does not exist, which most of the search
        // path usually is; there the path itself is key enough, nothing being
        // read out of it anyway.
        let key = directory
            .canonicalize()
            .unwrap_or_else(|_| directory.clone());
        if seen.contains(&key) {
            continue;
        }
        seen.push(key);
        found.extend(images_in(&directory));
    }

    found.extend(package_directories());
    found
}

/// Where Wipeout 2048's own extracts live, unlike every other title's a
/// **directory** rather than a disc image - see `data/README.md` and
/// `oag_2048`'s own module docs. `just play 2048` reads the same location.
const PACKAGE_ROOT: &str = "data/extracted/vita";

/// Every 2048 package extract under [`PACKAGE_ROOT`].
///
/// One line over [`package_directories_in`], parameterised on the same terms
/// [`first_image`] is over [`images_in`]: so the real search and a test
/// fixture cannot answer this question differently.
fn package_directories() -> Vec<PathBuf> {
    package_directories_in(Path::new(PACKAGE_ROOT))
}

/// Every 2048 package extract directly under `root`, alphabetical for the
/// same reason [`images_in`]'s `containers` is.
///
/// **Cheap and title-blind, the same way [`is_container`] is**: this only asks
/// whether `<candidate>/base/PSP2/data.psarc` exists, which is what tells an
/// extracted package apart from an empty or partial one - it does not open the
/// archives and does not decide this is Wipeout 2048. [`crate::launcher`]'s own
/// `survey` is what opens each candidate and decides whether it is playable at
/// all, the same as it does for every `.chd` and `.iso` this module finds.
fn package_directories_in(root: &Path) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(root)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.join("base/PSP2/data.psarc").is_file())
        .collect();
    found.sort();
    found
}

/// The not-found message, which for a packaged build is the entire interface.
fn nothing_found(searched: &[PathBuf]) -> anyhow::Error {
    anyhow!(
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
    )
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
/// One line over [`images_in`] rather than a scan of its own, so that "which
/// image does a boot with nothing named open" and "which image does a chooser
/// list first" cannot drift into answering differently.
fn first_image(directory: &Path) -> Option<PathBuf> {
    images_in(directory).into_iter().next()
}

/// Every image in one directory, known names first and the rest alphabetically.
///
/// Alphabetical rather than whatever order the filesystem hands out, so two
/// runs in a directory holding two images list them the same way - and open the
/// same one, [`first_image`] being the head of this list.
fn images_in(directory: &Path) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = Vec::new();

    for name in IMAGE_NAMES {
        let candidate = directory.join(name);
        if candidate.is_file() {
            found.push(candidate);
        }
    }

    let mut containers: Vec<PathBuf> = std::fs::read_dir(directory)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_file() && is_container(path))
        .collect();
    containers.sort();

    for path in containers {
        if !found.contains(&path) {
            found.push(path);
        }
    }

    found
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
mod tests;
