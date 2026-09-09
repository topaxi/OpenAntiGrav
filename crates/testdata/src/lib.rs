//! Where a test finds the disc images this project does not ship.
//!
//! **Test-only, and deliberately tiny.** It holds no game knowledge - no image
//! names, no archive names, no title. It answers one question: *given a name,
//! where is that file in this checkout, and is it there at all?* Everything
//! about what the file contains belongs to `oag-disc` and the title crates.
//!
//! # Why this exists
//!
//! The same twelve-line `fn image()` was copy-pasted into **130 definitions
//! across 148 test files**. Every copy did the same three things - resolve a
//! repo-relative path, return `Some` if it exists, and otherwise skip *unless*
//! `OAG_REQUIRE_GAME_DATA` is set, which turns the skip into a failure - and
//! the copies had drifted: some printed to stdout, some to stderr, some not at
//! all; some read `OAG_REQUIRE_GAME_DATA` with `var_os`, some compared it to
//! `"1"`; one had lost the `assert!` entirely and skipped silently even under
//! `OAG_REQUIRE_GAME_DATA=1`, which is the one failure mode the variable exists
//! to prevent.
//!
//! It is also the only place [`image`]'s extracted-image preference could live.
//!
//! # The skip contract
//!
//! A missing file returns `None` and the test returns early having asserted
//! nothing. That is what makes `just test` runnable with an empty `data/`.
//! Setting **`OAG_REQUIRE_GAME_DATA=1`** turns every such skip into a panic, so
//! a run that is *supposed* to have the data cannot quietly pass by testing
//! nothing - which is exactly what happens in a fresh worktree, where `data/`
//! is gitignored and does not travel. Run `just link-data` there.

use std::path::{Path, PathBuf};

/// This checkout's root, from the crate that is compiled into the test.
///
/// `CARGO_MANIFEST_DIR` is this crate's own directory whichever test links it,
/// so the `../..` is fixed rather than depending on the caller's depth.
#[must_use]
pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// A disc image by name, **preferring an already-extracted copy of it**.
///
/// `name` is either a bare file name under `data/images/`
/// (`"pulse-psp-usa.chd"`) or a path relative to the checkout root
/// (`"data/images/pulse-psp-usa.chd"`). Both spellings were in use across the
/// copies this replaced, so both keep working.
///
/// # The extracted-image preference
///
/// A CHD is compressed, so every read of one decompresses a hunk.
/// `Archives::open` measures **31 ms** against `pulse-psp-usa.chd` and
/// **0.05 ms** against the raw `.iso` `just extract-iso` writes beside it -
/// about six hundred times - and `DiscImage::open` sniffs by magic, so the two
/// are interchangeable to every caller. When `data/cache/<stem>.iso` exists,
/// this hands it back instead.
///
/// **Only when it is newer than the image it came from.** A stale extract would
/// otherwise feed wrong bytes to a hundred and fifty test files in silence,
/// which is a far worse failure than being slow. Re-run `just extract-iso`
/// after replacing an image; the extract being *absent* is always safe and
/// simply costs the decompression.
///
/// Use [`exact`] where the container format is itself what is under test -
/// `oag-disc`'s own ground truth compares a CHD against its extract and must
/// not be handed the extract twice.
#[must_use]
pub fn image(name: &str) -> Option<PathBuf> {
    let path = exact(name)?;
    Some(extracted(&path).unwrap_or(path))
}

/// A file under the checkout, with no extracted-image substitution.
///
/// The same resolution and the same skip contract as [`image`] - see this
/// module's own docs for what `OAG_REQUIRE_GAME_DATA` does - for a caller that
/// needs the file it named and not an equivalent one.
#[must_use]
pub fn exact(name: &str) -> Option<PathBuf> {
    let root = repo_root();
    // A bare name is by far the common spelling; a relative path is the older
    // one and still in use. Neither can be mistaken for the other, since a bare
    // name has no separator in it.
    let path = if Path::new(name).components().count() > 1 {
        root.join(name)
    } else {
        root.join("data/images").join(name)
    };

    if path.exists() {
        return Some(path);
    }
    assert!(
        std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
        "OAG_REQUIRE_GAME_DATA is set but {} is missing. In a worktree this is \
         usually a missing `just link-data`, not a missing image - `data/` is \
         gitignored and does not travel.",
        path.display()
    );
    println!("skipping: {} not present", path.display());
    None
}

/// `data/cache/<stem>.iso`, if it is there and is newer than `image`.
fn extracted(image: &Path) -> Option<PathBuf> {
    // An image that is already a raw `.iso` has nothing to substitute.
    if image
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("iso"))
    {
        return None;
    }
    let stem = image.file_stem()?;
    let iso = repo_root()
        .join("data/cache")
        .join(stem)
        .with_extension("iso");

    let fresh = |p: &Path| p.metadata().ok()?.modified().ok();
    let (extract, source) = (fresh(&iso)?, fresh(image)?);
    (extract >= source).then_some(iso)
}
