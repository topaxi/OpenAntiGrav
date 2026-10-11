//! The two loose layers around the disc: an override in front, a fallback behind.

use super::tests::{mounted, pack};
use super::*;
use crate::layers::Layer;
use crate::testing;

fn layer_dir(kind: LayerKind, dir: &std::path::Path, files: &[(&str, &[u8])]) -> Layer {
    for (name, bytes) in files {
        let path = dir.join(name);
        std::fs::create_dir_all(path.parent().expect("a parent")).expect("a layer directory");
        std::fs::write(path, bytes).expect("a layer file");
    }
    Layer::from_dir(kind, dir).expect("a layer with files in it")
}

/// The override is the player's explicit choice: it replaces the disc's entry,
/// and the load report names it.
#[test]
fn an_override_replaces_a_disc_entry_by_name() {
    let dir = testing::temp_dir("layer-override");
    let layer = layer_dir(
        LayerKind::Override,
        &dir.join("mod"),
        &[("Data/HUD/Thing.bin", b"modded")],
    );
    let mut archives =
        mounted(&dir, &[("Data\\HUD\\Thing.bin", b"disc")], Vec::new()).with_override(layer);

    // The WAD spelling, with backslashes and different case, reaches the file.
    assert_eq!(
        archives.read_name("Data\\HUD\\Thing.bin").unwrap(),
        b"modded"
    );
    assert_eq!(
        archives.locations("Data\\HUD\\Thing.bin").len(),
        2,
        "the override and the disc both hold it, the override first"
    );
    let report = archives.take_layer_report();
    assert_eq!(report.len(), 1, "{report:?}");
    assert!(report[0].contains("override"), "{report:?}");
    assert!(archives.take_layer_report().is_empty(), "drained");
}

/// The fallback answers only what nothing else does, so it can never shadow a
/// disc or a DLC entry whatever it carries.
#[test]
fn the_fallback_never_wins_over_a_disc_or_a_dlc_entry() {
    let dir = testing::temp_dir("layer-fallback-loses");
    let mut layer = Layer::new(LayerKind::Fallback, "this project");
    layer.insert_bytes("Disc.bin", b"substitute");
    layer.insert_bytes("Pack.bin", b"substitute");
    layer.insert_bytes("Neither.bin", b"substitute");
    let mut archives = mounted(
        &dir,
        &[("Disc.bin", b"disc")],
        vec![pack(&dir, "one", &[("Pack.bin", b"pack")])],
    )
    .with_fallback(layer);

    assert_eq!(archives.read_name("Disc.bin").unwrap(), b"disc");
    assert_eq!(archives.read_name("Pack.bin").unwrap(), b"pack");
    assert_eq!(archives.read_name("Neither.bin").unwrap(), b"substitute");
    let report = archives.take_layer_report();
    assert_eq!(
        report.len(),
        1,
        "only the answered gap is reported: {report:?}"
    );
    assert!(report[0].contains("Neither.bin") && report[0].contains("chosen, not measured"));
}

/// With neither layer mounted every lookup is the one it always was, and no
/// report line appears.
#[test]
fn with_no_layer_mounted_nothing_is_reported() {
    let dir = testing::temp_dir("layer-none");
    let mut archives = mounted(&dir, &[("Disc.bin", b"disc")], Vec::new());
    assert!(archives.read_name("Absent.bin").is_err());
    assert_eq!(archives.read_name("Disc.bin").unwrap(), b"disc");
    assert!(archives.take_layer_report().is_empty());
    assert_eq!(archives.locations("Disc.bin").len(), 1);
}

/// A PSARC-style path and a WAD-style name are one entry to a layer.
#[test]
fn a_layer_matches_either_spelling_without_regard_to_case() {
    let mut layer = Layer::new(LayerKind::Fallback, "this project");
    layer.insert_bytes("Data\\XML\\Foo.xml", b"x");
    assert!(layer.contains("/data/xml/foo.xml"));
    assert!(layer.contains("DATA/XML/FOO.XML"));
    assert!(!layer.contains("data/xml/bar.xml"));
}

/// An empty layer is not mounted, so "the player supplied nothing" costs nothing.
#[test]
fn an_empty_layer_is_not_mounted() {
    let dir = testing::temp_dir("layer-empty");
    let archives = mounted(&dir, &[("a.bin", b"a")], Vec::new())
        .with_override(Layer::new(LayerKind::Override, "empty"))
        .with_fallback(Layer::new(LayerKind::Fallback, "empty"));
    assert!(archives.overrides.is_none() && archives.fallback.is_none());
}
