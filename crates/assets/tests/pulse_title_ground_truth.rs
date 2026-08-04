//! Confirms a real Wipeout Pure disc is rejected by name, not silently opened
//! as Pulse.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! ```
//!
//! # What this is for
//!
//! Wipeout Pure ships its PSP archives (`Data.wad`, `FE.wad`) under the exact
//! same names Pulse does, so [`pulse::Layout::resolve`] cannot tell the two
//! apart by archive name alone - before this check existed, a directory or
//! disc holding only a Pure image would open exactly as if it were Pulse,
//! silently. This is the regression guard: opening Pure's own disc through
//! the Pulse-specific archive layer must fail with [`Error::WrongTitle`],
//! naming the disc's serial and the title it belongs to, before any archive
//! matching runs.

use std::path::{Path, PathBuf};

use oag_assets::{Error, pulse};

fn image(name: &str) -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/images")
        .join(name);

    if path.exists() {
        return Some(path);
    }
    assert!(
        std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
        "OAG_REQUIRE_GAME_DATA is set but {} is missing",
        path.display()
    );
    println!("skipping: {} not present", path.display());
    None
}

#[test]
#[ignore = "needs data/images/pure-psp-usa.chd"]
fn opening_pure_through_pulse_archives_is_rejected_by_name() {
    let Some(image) = image("pure-psp-usa.chd") else {
        return;
    };

    match pulse::Archives::open(&image.display().to_string()) {
        Err(Error::WrongTitle { serial, title, .. }) => {
            assert_eq!(serial, "UCUS-98612");
            assert_eq!(title, "Wipeout Pure");
        }
        other => panic!("expected Error::WrongTitle, got {other:?}"),
    }
}
