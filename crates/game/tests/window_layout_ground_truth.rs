//! The window layout screen through the real binary: `--menu-page window-layout:PEOPLE:WINDOWS:MONITORS`
//! on a Pulse and an HD disc, which is the path the live screen's drawing, labels and skin
//! take (`oag_ui_screens::window_layout`, `capture::window_layout_page`).
//!
//! The model's rules (windows, seats, cut, monitor, the preview's boxes) are the unit tests of
//! `oag-ui-screens`; what only a disc can say is that the page opens on each title's own skin
//! and that its picture follows the layout: dropping the page from the capture falls back to a
//! different screen, and dropping the preview or the rows leaves two layouts looking the same.

use std::path::{Path, PathBuf};

/// A still of `page`, as the bytes of the PNG the binary wrote.
fn still(image: &Path, page: &str, name: &str) -> Vec<u8> {
    let dir = std::env::temp_dir().join("oag-window-layout-ground-truth");
    std::fs::create_dir_all(&dir).expect("a temp directory");
    let out = dir.join(format!("{name}.png"));
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_oag-game"))
        .arg(image)
        .args([
            "--no-audio",
            "--size",
            "640x360",
            "--menu-page",
            page,
            "--screenshot",
        ])
        .arg(&out)
        .env("XDG_CONFIG_HOME", dir.join("config"))
        .env("XDG_STATE_HOME", dir.join("state"))
        .env("XDG_DATA_HOME", dir.join("data"))
        .status()
        .expect("running oag-game");
    assert!(
        status.success(),
        "oag-game --menu-page {page} on {}",
        image.display()
    );
    std::fs::read(&out).expect("the screenshot")
}

fn check(image: PathBuf, title: &str) {
    let join = still(&image, "join:4", &format!("{title}-join"));
    let one = still(&image, "window-layout:4:1:1", &format!("{title}-one"));
    let two = still(&image, "window-layout:4:2:1", &format!("{title}-two"));
    let monitors = still(&image, "window-layout:4:2:2", &format!("{title}-monitors"));
    let three = still(&image, "window-layout:4:3:2:2", &format!("{title}-three"));
    assert_ne!(
        one, join,
        "{title}: the page is the join screen's, not its own"
    );
    assert_ne!(one, two, "{title}: one window and two look the same");
    assert_ne!(
        two, monitors,
        "{title}: the monitor row and labels are not drawn"
    );
    assert_ne!(
        monitors, three,
        "{title}: three windows and two look the same"
    );
}

#[test]
#[ignore = "needs data/images/pulse-psp-eu.chd and a GPU adapter"]
fn the_window_layout_page_draws_on_pulse() {
    if let Some(image) = oag_testdata::image("data/images/pulse-psp-eu.chd") {
        check(image, "pulse");
    }
}

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu.iso and a GPU adapter"]
fn the_window_layout_page_draws_on_hd() {
    if let Some(image) = oag_testdata::image("data/images/hdfury-ps3-eu.iso") {
        check(image, "hd");
    }
}
