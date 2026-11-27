//! Where a test writes: under this build's `target/tmp`, never the OS temp
//! directory, which on Linux is often a `tmpfs` with a per-user quota.
//!
//! [`dir`] is a directory of one test's own, removed when its guard drops,
//! which includes a panic unwinding. [`cache`] is a stable path that outlives
//! the run, for work later runs reuse (a transcoded movie or track). A test
//! killed from outside drops nothing; what it leaves stays under `target/` and
//! goes with `cargo clean`.
//!
//! `OAG_KEEP_SCRATCH=1` keeps every [`dir`] so its screenshots and logs can be
//! read afterwards.

use std::ffi::OsStr;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// A scratch directory that is removed when this value drops.
///
/// Bind it to a name for as long as the files are needed: a temporary
/// (`scratch::dir("x").join("a.png")`) is removed at the end of its statement.
#[derive(Debug)]
#[must_use = "the directory is removed as soon as this guard drops"]
pub struct TempDir {
    path: PathBuf,
}

impl TempDir {
    /// The directory's path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Deref for TempDir {
    type Target = Path;

    fn deref(&self) -> &Path {
        &self.path
    }
}

impl AsRef<Path> for TempDir {
    fn as_ref(&self) -> &Path {
        &self.path
    }
}

impl AsRef<OsStr> for TempDir {
    fn as_ref(&self) -> &OsStr {
        self.path.as_os_str()
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        if std::env::var_os("OAG_KEEP_SCRATCH").is_none() {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }
}

/// A new, empty directory for this call alone, named after `label`.
///
/// # Panics
///
/// If it cannot be created.
pub fn dir(label: &str) -> TempDir {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let path = root().join("scratch").join(format!(
        "{label}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    // A killed run may have left this name behind, and process ids are reused.
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("a scratch directory under target/tmp");
    TempDir { path }
}

/// The directory `name` that every run of this checkout shares and none removes.
///
/// Only for work worth reusing, keyed so a second run overwrites rather than
/// adds. Anything else belongs in [`dir`].
///
/// # Panics
///
/// If it cannot be created.
#[must_use]
pub fn cache(name: &str) -> PathBuf {
    let path = root().join("cache").join(name);
    std::fs::create_dir_all(&path).expect("a cache directory under target/tmp");
    path
}

/// `<target>/tmp` of the build that made the running test binary, the same
/// directory Cargo hands integration tests as `CARGO_TARGET_TMPDIR`.
///
/// Found at run time because unit tests are given no such variable: Cargo marks
/// a target directory's root with `CACHEDIR.TAG`.
#[must_use]
pub fn root() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|exe| {
            exe.ancestors()
                .find(|dir| dir.join("CACHEDIR.TAG").is_file())
                .map(Path::to_path_buf)
        })
        .unwrap_or_else(|| crate::repo_root().join("target"))
        .join("tmp")
}

#[cfg(test)]
mod tests {
    use super::{cache, dir, root};

    #[test]
    fn the_root_is_inside_the_target_directory_and_not_the_os_temp_directory() {
        let root = root();
        let target = root.parent().expect("target/tmp has a parent");
        assert!(target.join("CACHEDIR.TAG").is_file(), "{}", root.display());
    }

    #[test]
    fn a_directory_and_its_files_are_removed_when_the_guard_drops() {
        let scratch = dir("drop");
        let path = scratch.path().to_path_buf();
        std::fs::write(scratch.join("file"), b"x").expect("write");
        assert!(path.starts_with(root()));
        drop(scratch);
        assert!(!path.exists(), "{} survived", path.display());
    }

    #[test]
    fn a_panicking_test_still_removes_its_directory() {
        let mut left = None;
        let unwound = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let scratch = dir("panic");
            left = Some(scratch.path().to_path_buf());
            panic!("a failing test");
        }));
        assert!(unwound.is_err());
        assert!(!left.expect("the directory was made").exists());
    }

    #[test]
    fn two_calls_with_one_label_do_not_share_a_directory() {
        let (first, second) = (dir("same"), dir("same"));
        assert_ne!(first.path(), second.path());
    }

    #[test]
    fn a_cache_has_one_stable_path() {
        let first = cache("testdata-selftest");
        assert!(first.is_dir());
        assert_eq!(first, cache("testdata-selftest"));
    }
}
