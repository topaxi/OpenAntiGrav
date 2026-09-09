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
//! same names Pulse does, so [`oag_assets::Layout::resolve`] cannot tell the two
//! apart by archive name alone - before this check existed, a directory or
//! disc holding only a Pure image would open exactly as if it were Pulse,
//! silently. This is the regression guard: opening Pure's own disc through
//! the Pulse-specific archive layer must fail with [`Error::WrongTitle`],
//! naming the disc's serial and the title it belongs to, before any archive
//! matching runs.

use std::path::PathBuf;

use oag_assets::Error;
use oag_pulse as pulse;

fn image(name: &str) -> Option<PathBuf> {
    oag_testdata::image(name)
}

#[test]
#[ignore = "needs data/images/pure-psp-usa.chd"]
fn opening_pure_through_pulse_archives_is_rejected_by_name() {
    let Some(image) = image("pure-psp-usa.chd") else {
        return;
    };

    match pulse::open(&image.display().to_string()) {
        Err(Error::WrongTitle { serial, title, .. }) => {
            assert_eq!(serial, "UCUS-98612");
            assert_eq!(title, "Wipeout Pure");
        }
        other => panic!("expected Error::WrongTitle, got {other:?}"),
    }
}

/// The EU disc's own regression guard - added after `just play pure-eu`
/// (`data/images/pure-psp-eu.chd`) was found booting straight into the menu
/// system instead of failing, because `FOREIGN_SERIALS` only carried the USA
/// serial. See [`opening_pure_through_pulse_archives_is_rejected_by_name`]
/// above for the USA disc.
#[test]
#[ignore = "needs data/images/pure-psp-eu.chd"]
fn opening_pure_eu_through_pulse_archives_is_rejected_by_name() {
    let Some(image) = image("pure-psp-eu.chd") else {
        return;
    };

    match pulse::open(&image.display().to_string()) {
        Err(Error::WrongTitle { serial, title, .. }) => {
            assert_eq!(serial, "UCES-00001");
            assert_eq!(title, "Wipeout Pure");
        }
        other => panic!("expected Error::WrongTitle, got {other:?}"),
    }
}
