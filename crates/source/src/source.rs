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
//! 5. **Beside the program**: `$APPIMAGE`'s own directory, else the directory
//!    the running executable sits in, and an `images/` directory in it. This is
//!    portable mode - copy the AppImage and an image into the same folder on a
//!    Steam Deck and run it, or unzip the Windows build and put an `images`
//!    folder next to `oag-game.exe`.
//! 6. **`<data dir>/oag/images/`**, i.e. `~/.local/share/oag/images` on Linux.
//!    The stable place to keep an image that is not next to the AppImage.
//!
//! `$APPDIR` is deliberately **not** searched. That is the mounted AppImage
//! itself, and looking inside it would invite someone to bundle an image into
//! the package, which is the one thing that must never happen.
//!
//! Wipeout 2048's own extract is a directory rather than a disc image, so it
//! is not one of the four `IMAGE_NAMES` steps 4-6 above scan for; it has its
//! own parallel search, [`package_search_path`], over the same three
//! locations with `extracted/vita` standing in for `images`. [`candidates`]
//! lists them, and [`resolve`] falls back to the first one when no image is
//! found, so a no-argument boot, `--dry-run` and `--screenshot` open a lone
//! extract the chooser would have shown. `$OAG_IMAGE` accepts one too.
//!
//! Wipeout: Omega Collection's own extract is a directory too, but shaped
//! differently from 2048's: one PS4 copy is a base package and a mandatory
//! day-one patch, two fixed-name sibling directories (`omega-eu` and
//! `omega-eu-patch`) under one parent, rather than one self-contained
//! directory a player may name however they like. So [`ps4_search_path`] does
//! not repeat [`package_directories_in`]'s "each child is a copy" scan - the
//! parent itself is the one candidate, checked by the same shape `just play
//! omega` already checks: does `omega-eu-patch/uroot/data09.psarc` exist
//! under it. [`oag_omega::open`]'s own doc names that same parent as its
//! template source.
//!
//! A Wipeout HD copy installed from the PSN download is a directory as well:
//! `PARAM.SFO` beside `USRDIR/`, under `extracted/ps3` ([`ps3_search_path`]).
//! It is found by that shape alone, European `TITLE_ID`s first, so an unpacked
//! disc folder or a raw zip extract beside it is not offered. See
//! `docs/formats/hd-psn.md`.
//!
//! # The first three are a decision; the last three are a guess
//!
//! Steps 1 to 3 are a player *stating* which source they want, and [`explicit`]
//! is those three alone. Steps 4 to 6 are this module guessing, and
//! [`candidates`] is the whole guess rather than its first hit - which is what
//! `crate::launcher` puts on screen when there is more than one, instead of
//! opening one and never mentioning the rest. `resolve` is still the two halves
//! joined, unchanged, for every route that wants one answer and no screen.

use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};

/// The default the repository checkout has, and the documented example.
pub const DEFAULT_IMAGE: &str = "data/images/pulse-psp-eu.chd";

/// The folder a portable install lives in: the AppImage's directory, else the
/// directory the running executable sits in (a Windows zip unpacked anywhere,
/// a tarball, a macOS build run from where it was put).
///
/// Inside an AppImage `current_exe` is under the mounted `$APPDIR`, which is
/// deliberately never searched, so `$APPIMAGE` decides first and the executable
/// is only asked when it is unset.
fn portable_directory() -> Option<PathBuf> {
    portable_directory_of(std::env::var_os("APPIMAGE"), std::env::current_exe().ok())
}

/// [`portable_directory`] with the environment handed in, so both branches are
/// checkable without setting a process-wide variable.
fn portable_directory_of(
    appimage: Option<std::ffi::OsString>,
    exe: Option<PathBuf>,
) -> Option<PathBuf> {
    if let Some(appimage) = appimage {
        return Path::new(&appimage).parent().map(Path::to_path_buf);
    }
    exe.and_then(|exe| exe.parent().map(Path::to_path_buf))
}

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
///
/// **Europe before the USA, for every title that has both** (maintainer
/// decision, 2026-10-07; standing project policy): EU is Pulse PSP's more
/// complete disc (its DLC never shipped on USA, see `data/README.md`) and the
/// Ghidra target of record
/// ([ADR-0048](../../../docs/architecture/adr/0048-eu-is-the-psp-pulse-re-target-of-record.md)).
/// A directory holding both opens the EU one; the USA one is reachable by
/// naming it on the command line, in `$OAG_IMAGE` or in settings. Pulse PSP
/// still leads the list as the platform the implementation follows, and
/// [`IMAGE_NAMES`]`[0]` is the name the not-found hint suggests.
pub const IMAGE_NAMES: [&str; 7] = [
    "pulse-psp-eu.chd",
    "pulse-psp-usa.chd",
    "pulse-ps2-eu.chd",
    "pure-psp-eu.chd",
    "pure-psp-usa.chd",
    "hdfury-ps3-eu-dec.iso",
    "hdfury-ps3-eu.iso",
];

/// Container extensions a directory scan will accept, lowercase.
const EXTENSIONS: [&str; 3] = ["chd", "iso", "vpk"];

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

    if let Some(directory) = portable_directory() {
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

    // The unpacked 2048 and Omega folders: the same scan `candidates` ends
    // with, so a no-argument boot and the chooser find the same things.
    if let Some(found) = first_package(
        &package_search_path(),
        &ps4_search_path(),
        &ps3_search_path(),
    ) {
        return Ok(display(&found));
    }

    Err(nothing_found(&searched))
}

/// The first unpacked 2048 folder under `vita_roots`, else the first Omega
/// root among `ps4_roots`, else the first PSN HD install under `ps3_roots`.
fn first_package(
    vita_roots: &[PathBuf],
    ps4_roots: &[PathBuf],
    ps3_roots: &[PathBuf],
) -> Option<PathBuf> {
    vita_roots
        .iter()
        .flat_map(|root| package_directories_in(root))
        .chain(ps4_package_directories_in(ps4_roots))
        .chain(
            ps3_roots
                .iter()
                .flat_map(|root| ps3_package_directories_in(root)),
        )
        .next()
}

/// An unpacked 2048 or Omega folder `path` is, or holds: a package named
/// directly, or the parent a scan would have found it under.
fn package_at(path: &Path) -> Option<PathBuf> {
    if path.join("base/PSP2/data.psarc").is_file() {
        return Some(path.to_path_buf());
    }
    if is_ps3_install(path) {
        return Some(path.to_path_buf());
    }
    ps4_package_directories_in(&[path.to_path_buf()])
        .into_iter()
        .chain(package_directories_in(path))
        .chain(ps3_package_directories_in(path))
        .next()
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
        if let Some(found) = first_image(&path).or_else(|| package_at(&path)) {
            return Ok(Some(display(&found)));
        }
        return Err(anyhow!(
            "{IMAGE_ENV} is set to {}, which is not a disc image (.chd, .iso), an \
             unpacked 2048 or Omega folder, an installed Wipeout HD PSN folder, or a \
             folder holding one",
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
/// [`package_directories`] and [`ps4_package_directories`] are appended last,
/// once per root on [`package_search_path`] and [`ps4_search_path`]
/// respectively, rather than folded into the per-directory loop above: an
/// extracted package is found by its own shape ([`package_directories_in`],
/// [`ps4_package_directories`]), not by name the way `images_in` matches
/// [`IMAGE_NAMES`], so each needs its own scan rather than a branch inside
/// `images_in`. Appending them after every image candidate keeps
/// `first_image` (and so a no-argument boot) picking the same source it
/// always did on a machine that also has a `.chd` or `.iso`.
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
    found.extend(ps4_package_directories());
    found.extend(ps3_package_directories());
    found
}

/// Where Wipeout 2048's own extracts live, unlike every other title's a
/// **directory** rather than a disc image - see `data/README.md` and
/// `oag_2048`'s own module docs. `just play 2048` reads the same location.
const PACKAGE_ROOT: &str = "data/extracted/vita";

/// Every root [`package_directories`] scans, in order.
///
/// **The same roots [`search_path`] looks for a disc image in, one for one**,
/// with `extracted/vita` standing in for `images`: the checkout's own
/// directory; beside a portable AppImage, both the directory itself (a
/// package dropped straight in, the same "copy both files into one folder"
/// case `search_path` covers for an image) and an `extracted/vita`
/// subdirectory in it; and the stable data directory. This used to be
/// [`PACKAGE_ROOT`] alone, which is exactly the bug a portable deploy hit - a
/// Steam Deck running the AppImage from `~/Desktop` has no `data/` beside its
/// current directory at all, so the package a player extracted was invisible
/// to the launcher no matter where they put it. `$APPDIR` is excluded for the
/// same reason it is excluded from `search_path`: looking inside the mounted
/// image would invite bundling game content into the package.
#[must_use]
pub fn package_search_path() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let mut push = |path: PathBuf| {
        if !paths.contains(&path) {
            paths.push(path);
        }
    };

    push(PathBuf::from(PACKAGE_ROOT));

    if let Some(directory) = portable_directory() {
        push(directory.to_path_buf());
        push(directory.join("extracted").join("vita"));
    }

    if let Some(data) = dirs::data_dir() {
        push(data.join("oag").join("extracted").join("vita"));
    }

    paths
}

/// Every 2048 package extract on [`package_search_path`].
///
/// Dedups by canonical path the same way [`candidates`] does for
/// `search_path`, for the same reason: a checkout with the AppImage sitting
/// in it would otherwise offer one extract twice.
fn package_directories() -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = Vec::new();
    let mut seen: Vec<PathBuf> = Vec::new();

    for root in package_search_path() {
        let key = root.canonicalize().unwrap_or_else(|_| root.clone());
        if seen.contains(&key) {
            continue;
        }
        seen.push(key);
        found.extend(package_directories_in(&root));
    }

    found
}

/// Every 2048 package extract directly under `root`: European serials first,
/// then alphabetical for the same reason [`images_in`]'s `containers` is.
///
/// **Cheap and title-blind, the same way [`is_container`] is**: this only asks
/// whether `<candidate>/base/PSP2/data.psarc` exists, which is what tells an
/// extracted package apart from an empty or partial one - it does not open the
/// archives and does not decide this is Wipeout 2048. `crate::launcher`'s own
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
    found.sort_by_key(|path| (!is_european_serial(path), path.clone()));
    found
}

/// Whether a Vita package folder is named for a European serial (`PCSB`,
/// `PCSF`), which sorts ahead of the American `PCSA`/`PCSE` ones: the same
/// Europe-first policy [`IMAGE_NAMES`] follows. A folder named anything else
/// is not European as far as this can tell, and keeps its alphabetical place.
fn is_european_serial(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with("PCSB") || name.starts_with("PCSF"))
}

/// Where Wipeout: Omega Collection's own PS4 extract lives, unlike 2048's
/// **the parent of two fixed-name siblings** rather than a directory a player
/// names for themselves - see the module doc's "shaped differently" note.
/// `just play omega` reads the same location.
const PS4_PACKAGE_ROOT: &str = "data/extracted/ps4";

/// Every root [`ps4_package_directories`] scans, in order.
///
/// The same three roots [`search_path`] and [`package_search_path`] use, with
/// `extracted/ps4` standing in for `images`/`extracted/vita` - the portable
/// "beside the AppImage" case this mirrors is the same bug 2048's own
/// `package_search_path` doc names: a Steam Deck running the AppImage from a
/// folder with no `data/` in it must still find a PS4 extract dropped beside
/// it.
#[must_use]
pub fn ps4_search_path() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let mut push = |path: PathBuf| {
        if !paths.contains(&path) {
            paths.push(path);
        }
    };

    push(PathBuf::from(PS4_PACKAGE_ROOT));

    if let Some(directory) = portable_directory() {
        push(directory.join("extracted").join("ps4"));
    }

    if let Some(data) = dirs::data_dir() {
        push(data.join("oag").join("extracted").join("ps4"));
    }

    paths
}

/// Every playable Wipeout: Omega Collection root on [`ps4_search_path`].
fn ps4_package_directories() -> Vec<PathBuf> {
    ps4_package_directories_in(&ps4_search_path())
}

/// [`ps4_package_directories`], with the roots handed in rather than found -
/// the split that makes it testable without touching the filesystem
/// [`ps4_search_path`] actually reads.
///
/// **Not [`package_directories_in`]'s "each child is a copy" shape** - a root
/// itself is the one candidate, present only when its `omega-eu-patch`
/// sibling carries [`oag_omega::archives::DATA09`], the one archive
/// `oag_omega::open` hard-requires (see that constant's own doc). A base-only
/// extract, with no patch, is not offered - the same "partial extract must
/// not be offered as if it opens" rule [`package_directories_in`] documents,
/// here because the base package alone has no `skin.xml` to boot rather than
/// because it is incomplete.
fn ps4_package_directories_in(roots: &[PathBuf]) -> Vec<PathBuf> {
    roots
        .iter()
        .filter(|root| {
            root.join("omega-eu-patch")
                .join(oag_omega::archives::DATA09)
                .is_file()
        })
        .cloned()
        .collect()
}

/// Where a Wipeout HD PSN install is looked for: the folder RPCS3 (or a
/// PS3) installed the package into, copied under `data/extracted/ps3/`. See
/// `docs/overview/installing.md`, "Wipeout HD from the PSN download".
const PS3_PACKAGE_ROOT: &str = "data/extracted/ps3";

/// Every root [`ps3_package_directories`] scans: the same three places as
/// [`ps4_search_path`], with `extracted/ps3` standing in for `extracted/ps4`.
#[must_use]
pub fn ps3_search_path() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let mut push = |path: PathBuf| {
        if !paths.contains(&path) {
            paths.push(path);
        }
    };

    push(PathBuf::from(PS3_PACKAGE_ROOT));

    if let Some(directory) = portable_directory() {
        push(directory.join("extracted").join("ps3"));
    }

    if let Some(data) = dirs::data_dir() {
        push(data.join("oag").join("extracted").join("ps3"));
    }

    paths
}

/// Every PSN install on [`ps3_search_path`].
fn ps3_package_directories() -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = Vec::new();
    let mut seen: Vec<PathBuf> = Vec::new();

    for root in ps3_search_path() {
        let key = root.canonicalize().unwrap_or_else(|_| root.clone());
        if seen.contains(&key) {
            continue;
        }
        seen.push(key);
        found.extend(ps3_package_directories_in(&root));
    }

    found
}

/// Whether `path` is a PS3 HDD install: a `PARAM.SFO` at its root beside a
/// `USRDIR`. **That shape only.** A decrypted disc extract keeps these under
/// `PS3_GAME/` and is not offered, nor is a raw zip extract or the unlock
/// key's folder; the disc image is the source for a disc.
fn is_ps3_install(path: &Path) -> bool {
    path.join("PARAM.SFO").is_file() && path.join("USRDIR").is_dir()
}

/// Every PSN HD install directly under `root`, European serials (`NPEA`,
/// `BCES`) first and then alphabetical: the Europe-first policy again, read
/// off the package's own `TITLE_ID` and not off what the player named the
/// folder.
fn ps3_package_directories_in(root: &Path) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(root)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| is_ps3_install(path))
        .collect();
    found.sort_by_key(|path| (!has_european_title_id(path), path.clone()));
    found
}

/// Whether an install's `PARAM.SFO` names a European `TITLE_ID`.
fn has_european_title_id(path: &Path) -> bool {
    std::fs::read(path.join("PARAM.SFO"))
        .ok()
        .and_then(|bytes| oag_disc::sfo::title_id(&bytes))
        .is_some_and(|id| {
            id.starts_with("NPEA") || id.starts_with("NPEB") || id.starts_with("BCES")
        })
}

/// The not-found message, which for a packaged build is the entire interface.
fn nothing_found(searched: &[PathBuf]) -> anyhow::Error {
    let list = |paths: &[PathBuf], what: &str| {
        paths
            .iter()
            .map(|path| format!("  {}  ({what})", path.display()))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let beside = if std::env::var_os("APPIMAGE").is_some() {
        format!(" or put a {} beside the AppImage", IMAGE_NAMES[0])
    } else {
        format!(
            ", or put a {} in data/images/ under the directory you run from",
            IMAGE_NAMES[0]
        )
    };
    anyhow!(
        "no disc image found. OpenAntiGrav ships no game content: supply your own \
         image, from your own copy of the game.\n\nSearched (relative paths are \
         relative to {}):\n{}\n{}\n{}\n{}\n\nA .chd or .iso disc image (Pulse, Pure, \
         HD) is found by extension. A 2048 .vpk is read in place (a NoNpDrm dump, \
         or an unpacked folder). A PS4 Omega .pkg is read in place, with its patch .pkg \
         beside it. A Vita .pkg is NOT read: unpack and decrypt it into a folder first \
         (`docs/overview/installing.md`, \"Wipeout 2048 (Vita)\"). An encrypted HD .iso is read \
         in place with the disc key from a .dkey beside it, or one entered in the chooser.\n\nName a source directly \
         (`oag-game path/to/{}`), set {IMAGE_ENV}{beside}.",
        std::env::current_dir().map_or_else(|_| ".".into(), |dir| dir.display().to_string()),
        list(searched, ".chd / .iso / .vpk / Omega .pkg + patch"),
        list(
            &package_search_path(),
            "unpacked 2048 folder, base/PSP2/data.psarc"
        ),
        list(&ps4_search_path(), "unpacked Omega folder, omega-eu-patch/"),
        list(
            &ps3_search_path(),
            "installed Wipeout HD PSN folder, PARAM.SFO and USRDIR/"
        ),
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
    if let Some(directory) = portable_directory() {
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
        .filter(|path| path.is_file() && is_container(path) && is_listed(path))
        .collect();
    // A PS4 base package is a row only when no unpacked Omega folder already is
    // one: the folder is something the player prepared and stays the default
    // where both exist, so a machine that has both lists, and boots, what it
    // always did. Naming the `.pkg` opens it either way.
    if ps4_package_directories().is_empty() {
        containers.extend(
            std::fs::read_dir(directory)
                .into_iter()
                .flatten()
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .filter(|path| path.is_file() && is_listed_ps4_package(path)),
        );
    }
    containers.sort();

    for path in containers {
        if !found.contains(&path) {
            found.push(path);
        }
    }

    found
}

/// Whether a container is a row of its own. Every disc image is; a `.vpk` is
/// only if it is an application, because a patch or DLC `.vpk` is mounted with
/// its base and offered as nothing on its own.
fn is_listed(path: &Path) -> bool {
    let is_vpk = path
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("vpk"));
    !is_vpk || oag_disc::vpk::category(path).is_none_or(|category| category == "gd")
}

/// Whether `path` is a PS4 base package with a patch beside it: the pair
/// Omega opens (`oag_disc::ps4_pkg::Ps4Set`). A base package alone has no
/// front end to boot, so it is not offered; a patch package is mounted with
/// its base and never a row of its own; a Vita `.pkg` is not read at all.
///
/// Not part of [`is_container`], which `oag_sound`'s music-disc scan also
/// uses and which must keep answering for discs and `.vpk` files alone.
fn is_listed_ps4_package(path: &Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("pkg"))
        && oag_disc::ps4_pkg::category(path).is_some_and(|category| category == "gd")
        && oag_disc::ps4_pkg::set::siblings(path).len() > 1
}

/// Whether a path looks like a disc image this engine can open.
///
/// Public because [`oag_sound::MusicDiscs`] walks the same directories
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
