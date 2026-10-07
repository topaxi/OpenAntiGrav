//! A PSN Wipeout HD install is a source a player can name, and opens as HD.
//!
//! **`#[ignore]`d and never run in CI**: needs `data/extracted/ps3/hd-psn-eu/`
//! (`docs/overview/installing.md`, "Wipeout HD from the PSN download"). Skips
//! without it and fails under `OAG_REQUIRE_GAME_DATA=1` when it is missing.

use oag_source::title::{identify, open_source};

const INSTALL: &str = "data/extracted/ps3/hd-psn-eu";

/// The install folder opens as Wipeout HD with the PSN defaults, and its
/// parent (`data/extracted/ps3`) resolves to it the way `$OAG_IMAGE` does.
#[test]
#[ignore = "needs the PSN install in data/extracted/ps3/hd-psn-eu"]
fn the_install_identifies_as_hd_with_its_own_defaults() {
    let Some(path) = oag_testdata::exact(INSTALL) else {
        return;
    };
    let source = path.to_str().expect("utf-8 path");

    let opened = open_source(source, Vec::new(), Vec::new()).expect("the install opens");
    assert_eq!(opened.title.name, oag_hd::TITLE.name);
    assert_eq!(opened.title.race.track, oag_hd::psn::DEFAULT_TRACK);
    assert_eq!(identify(source).map(|t| t.name), Some("Wipeout HD"));

    let serial = opened.archives.layout.serial.as_deref();
    assert_eq!(serial, Some("NPEA-00057"), "the root PARAM.SFO is read");
}
