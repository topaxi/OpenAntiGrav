//! Running a binary a test has just built, without ever exec'ing a file another process may
//! be writing.
//!
//! `target/<profile>/<bin>` is re-laid by every cargo build that touches the binary (another
//! test's inner build, the run's own build with a different feature set), and a long-lived
//! child such as `oag-server lobby` execs its own `current_exe()` again for each race. A
//! write to that file while it is exec'd fails with `ETXTBSY` ("Text file busy"). So a test
//! execs a private copy: nothing else knows its path, and its descriptor is closed before
//! the first spawn.

use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::sync::Mutex;

/// Held while a child is spawned and while a binary is copied, so a child forked by one test
/// thread never inherits another thread's write handle on a file it is about to exec. One
/// test per process (nextest) never contends for it.
static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());

/// `command.spawn()` under the process-wide lock.
///
/// # Errors
/// Whatever [`Command::spawn`] returns.
pub fn spawn(command: &mut Command) -> std::io::Result<Child> {
    let _guard = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    command.spawn()
}

/// Runs `build` and then copies `built` into `dest_dir`, returning the copy's path. Every test
/// process of one target directory serialises on a lock file in `lock_dir`
/// (`CARGO_TARGET_TMPDIR`), so no other build lands between the build and the copy.
///
/// # Panics
/// If the lock or the copy fails, or `built` changes while it is copied.
pub fn private_copy(
    lock_dir: &Path,
    built: &Path,
    dest_dir: &Path,
    build: impl FnOnce(),
) -> PathBuf {
    let name = built.file_name().expect("a binary has a name");
    let lock = File::create(lock_dir.join(format!("{}-copy.lock", name.to_string_lossy())))
        .expect("the copy lock");
    lock.lock().expect("the copy lock is taken");
    build();
    let copy = dest_dir.join(name);
    let _guard = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    let stamp = |path: &Path| {
        let meta = std::fs::metadata(path).expect("the built binary");
        (meta.len(), meta.modified().ok())
    };
    let before = stamp(built);
    std::fs::copy(built, &copy).expect("the binary is copied");
    assert_eq!(
        stamp(built),
        before,
        "{} changed while it was copied",
        built.display()
    );
    assert_eq!(stamp(&copy).0, before.0, "the copy is whole");
    copy
}
