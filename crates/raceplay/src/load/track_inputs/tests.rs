//! A circuit's `track.vex` answered by the player's override: a byte-identical
//! copy of a source's own file loads, and loads the same circuit as the source.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content; run it with
//! `just test-data`, or only this module:
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-raceplay --run-ignored all \
//!     -E 'test(/^load::track_inputs::tests::/)'
//! ```

use super::*;
use oag_source::title::{Opened, open_source_in};
use std::path::{Path, PathBuf};

fn open(source: &str, overrides: &[PathBuf]) -> Opened {
    open_source_in(source, Vec::new(), Vec::new(), overrides).expect("the source opens")
}

fn circuit(opened: &mut Opened, track: &str) -> (Inputs, Vec<String>) {
    let options = Options {
        track: Some(track.to_string()),
        ..Options::default()
    };
    let mut report = Vec::new();
    let title = opened.title;
    let inputs = resolve(&options, &mut opened.archives, title, title, &mut report)
        .unwrap_or_else(|error| panic!("{track}: {error:#}"));
    (inputs, opened.archives.take_layer_report())
}

/// Copies the source's own copy of its default circuit into an override
/// directory and loads it both ways.
fn an_identical_override_loads_the_circuit(source: &Path, label: &str) {
    let source = source.display().to_string();
    let mut disc = open(&source, &[]);
    let track = disc.title.race.track;
    let blob = disc.archives.read_name(track).expect("the default circuit");

    let overrides = oag_testdata::scratch::dir(label);
    let file = overrides
        .join(oag_source::layers::slug(disc.title))
        .join(track.trim_start_matches(['\\', '/']).replace('\\', "/"));
    std::fs::create_dir_all(file.parent().expect("a parent")).expect("a directory");
    std::fs::write(&file, &blob).expect("the override copy");
    let mut modded = open(&source, &[overrides.to_path_buf()]);

    let (plain, plain_layers) = circuit(&mut disc, track);
    let (overridden, layers) = circuit(&mut modded, track);

    assert!(
        !plain_layers.iter().any(|line| line.contains(track)),
        "{plain_layers:?}"
    );
    assert!(
        layers
            .iter()
            .any(|line| line.contains(track) && line.contains("the player's override")),
        "the override was not what answered {track}: {layers:?}"
    );
    assert!(overridden.track_blob == plain.track_blob, "{track}: bytes");
    assert_eq!(overridden.ai, plain.ai);
    assert_eq!(overridden.start_position, plain.start_position);
    assert_eq!(overridden.nodes, plain.nodes);
    assert_eq!(overridden.speedup_pads, plain.speedup_pads);
    assert_eq!(overridden.weapon_pads, plain.weapon_pads);
    assert!(!plain.nodes.is_empty(), "{track}: no collision to compare");
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn pulse_loads_an_overridden_circuit() {
    let Some(image) = oag_testdata::image("pulse-psp-eu.chd") else {
        return;
    };
    an_identical_override_loads_the_circuit(&image, "override-circuit-pulse");
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn pulse_ps2_loads_an_overridden_circuit() {
    let Some(image) = oag_testdata::image("pulse-ps2-eu.chd") else {
        return;
    };
    an_identical_override_loads_the_circuit(&image, "override-circuit-pulse-ps2");
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn pure_loads_an_overridden_circuit() {
    let Some(image) = oag_testdata::image("pure-psp-eu.chd") else {
        return;
    };
    an_identical_override_loads_the_circuit(&image, "override-circuit-pure");
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn hd_loads_an_overridden_circuit() {
    let Some(image) = oag_testdata::image("hdfury-ps3-eu-dec.iso") else {
        return;
    };
    an_identical_override_loads_the_circuit(&image, "override-circuit-hd");
}

#[test]
#[ignore = "needs data/extracted/vita/PCSF00007"]
fn wipeout_2048_loads_an_overridden_circuit() {
    let Some(path) = oag_testdata::exact("data/extracted/vita/PCSF00007") else {
        return;
    };
    an_identical_override_loads_the_circuit(&path, "override-circuit-2048");
}

#[test]
#[ignore = "needs data/extracted/ps4"]
fn omega_loads_an_overridden_circuit() {
    let Some(path) = oag_testdata::exact("data/extracted/ps4") else {
        return;
    };
    an_identical_override_loads_the_circuit(&path, "override-circuit-omega");
}
