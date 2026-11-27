//! The card chooser end to end: `oag-game --launcher --drive` over real disc
//! images, with no window.
//!
//! What only the running binary shows: the art is read off the discs and
//! reaches the screen, a picture switched with the cog is there on the next
//! launch, and both the flag and the setting bring back the list.
//!
//! The sheet is judged by how many colours a card holds, never against disc
//! pixels, and no picture is kept.
//!
//! `#[ignore]`d: needs the PSP images under `data/images/`. `just test-data`.

use std::collections::BTreeSet;
use std::path::Path;

use serde_json::Value;

const IMAGES: [&str; 3] = ["pulse-psp-eu.chd", "pulse-psp-usa.chd", "pure-psp-eu.chd"];

/// A run directory whose `data/images` holds just [`IMAGES`], so the search
/// path is the same on every machine, with a profile of its own.
#[cfg(unix)]
fn library(tag: &str) -> Option<oag_testdata::scratch::TempDir> {
    let root = oag_testdata::scratch::dir(&format!("launcher-cards-{tag}"));
    let images = root.join("data/images");
    std::fs::create_dir_all(&images).expect("a scratch directory");
    std::fs::create_dir_all(root.join("mods")).expect("an empty mods directory");
    for name in IMAGES {
        let real = oag_testdata::exact(name)?
            .canonicalize()
            .expect("the image resolves");
        std::os::unix::fs::symlink(real, images.join(name)).expect("a link to the image");
    }
    Some(root)
}

/// Runs `script` against the chooser and returns the state it reported last.
fn drive(root: &Path, flags: &[&str], script: &str) -> Value {
    let log = root.join("state.jsonl");
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_oag-game"))
        .args(["--launcher", "--no-audio", "--drive", script])
        .args(flags)
        .arg("--state-log")
        .arg(&log)
        .current_dir(root)
        .env("XDG_CONFIG_HOME", root.join("config"))
        .env("XDG_DATA_HOME", root.join("xdg-data"))
        .env("XDG_STATE_HOME", root.join("state"))
        .env("OAG_MODS", root.join("mods"))
        .env_remove("OAG_IMAGE")
        .env_remove("APPIMAGE")
        .env_remove("WAYLAND_DISPLAY")
        .env_remove("DISPLAY")
        .output()
        .expect("running oag-game");
    assert!(
        output.status.success(),
        "{:?}\n{}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    let states = std::fs::read_to_string(&log).expect("a state log");
    serde_json::from_str(states.lines().last().expect("a state line")).expect("JSON")
}

fn settings(root: &Path) -> String {
    std::fs::read_to_string(root.join("config/oag/settings.toml")).expect("a settings file")
}

/// Distinct colours inside the first card of two, the cog's corner left out.
fn colours_in_the_first_card(shot: &Path) -> usize {
    let file = std::io::BufReader::new(std::fs::File::open(shot).expect("a screenshot"));
    let mut reader = png::Decoder::new(file).read_info().expect("a PNG");
    let mut pixels = vec![0; reader.output_buffer_size().expect("a size")];
    let frame = reader.next_frame(&mut pixels).expect("a frame");
    let channels = frame.color_type.samples();
    let (width, scale) = (frame.width as usize, frame.width as f32 / 480.0);
    let [x, y, w, h] = oag_game::launcher::cards::layout(2, 0)[0];
    let mut found = BTreeSet::new();
    for row in ((y + h * 0.3) * scale) as usize..((y + h * 0.9) * scale) as usize {
        for column in ((x + 2.0) * scale) as usize..((x + w - 2.0) * scale) as usize {
            let at = (row * width + column) * channels;
            found.insert([pixels[at], pixels[at + 1], pixels[at + 2]]);
        }
    }
    found.len()
}

#[cfg(unix)]
#[test]
#[ignore = "needs the Pulse and Pure PSP images in data/images/"]
fn the_cards_draw_the_discs_art_and_remember_the_picture_switched_to() {
    let Some(root) = library("remember") else {
        return;
    };
    std::fs::create_dir_all(root.join("config/oag")).expect("a profile");
    std::fs::write(
        root.join("config/oag/settings.toml"),
        "[display]\nwindow_size = \"1440x816\"\n",
    )
    .expect("a settings file");

    let shot = root.join("cards.png");
    let last = drive(
        &root,
        &[],
        &format!(
            "until focus=pulse-psp-eu.chd:pic1 within 3000; ticks 3; screenshot {}; \
             expect page=launcher-cards; press square; expect focus=pulse-psp-usa.chd:pic1",
            shot.display()
        ),
    );
    assert_eq!(
        last["screen"]["rows"].as_array().map(Vec::len),
        Some(2),
        "one card per title: {last}"
    );
    // A card with no art is one flat fill, or that and a name.
    let colours = colours_in_the_first_card(&shot);
    assert!(colours > 20, "the card holds {colours} colours");
    assert!(
        settings(&root).contains("\"UCUS-98712:pic1\""),
        "{}",
        settings(&root)
    );

    drive(
        &root,
        &[],
        "until focus=pulse-psp-usa.chd:pic1 within 3000; press cross; \
         until stage=loading within 600",
    );
}

#[cfg(unix)]
#[test]
#[ignore = "needs the Pulse and Pure PSP images in data/images/"]
fn the_list_is_opted_into_by_flag_or_setting() {
    let Some(root) = library("list") else {
        return;
    };
    let last = drive(&root, &["--launcher-list"], "expect page=launcher-list");
    assert_eq!(
        last["screen"]["rows"].as_array().map(Vec::len),
        Some(IMAGES.len()),
        "the list names every image: {last}"
    );
    drive(&root, &[], "expect page=launcher-cards");

    std::fs::write(
        root.join("config/oag/settings.toml"),
        "[launcher]\nstyle = \"list\"\n",
    )
    .expect("a settings file");
    drive(&root, &[], "expect page=launcher-list");
}
