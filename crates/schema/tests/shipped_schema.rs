//! Every TOML file of ours that declares a schema validates against it, and every
//! schema we ship is exercised by at least one file. Part of `just check-schemas`.

#![cfg(feature = "validate")]

use std::path::{Path, PathBuf};

use oag_schema::validate::{accepts, schemas_dir};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Shipped documents and the kind each is written as.
fn shipped() -> Vec<(&'static str, &'static str)> {
    vec![
        ("assets/ui/menu.toml", "menu"),
        ("assets/ai/example-pilot.toml", "pilot"),
        ("assets/mods/oag-extras/oag-mod.toml", "mod"),
        ("assets/mods/oag-extras/menu.toml", "mod-menu"),
        // The example mods' submodule (github.com/OpenAntiGrav/mod-examples):
        // read when checked out, skipped by name when not.
        ("assets/mods/examples/example-campaign/oag-mod.toml", "mod"),
        ("assets/mods/examples/hillside/oag-mod.toml", "mod"),
        (
            "assets/mods/examples/hillside/tracks/hillside.track.toml",
            "track",
        ),
        (
            "assets/mods/examples/example-campaign/tracks/practice-octagon.track.toml",
            "track",
        ),
        (
            "assets/mods/examples/example-campaign/campaigns/night-run.toml",
            "campaign",
        ),
        (
            "assets/mods/examples/example-campaign/pilots/rival.toml",
            "pilot",
        ),
        (
            "assets/mods/examples/energy-turbo/fx/discharge.toml",
            "effect",
        ),
        ("crates/trackfile/tests/fixtures/oval.track.toml", "track"),
        ("crates/game/tests/fixtures/music-state.toml", "music-state"),
        ("assets/fallback/Data/XML/SplitHud_Pulse.toml", "split-hud"),
        ("assets/fallback/Data/XML/SplitHud_Pure.toml", "split-hud"),
        ("assets/fallback/Data/XML/SplitHud_HD.toml", "split-hud"),
        ("assets/fallback/Data/XML/SplitHud_2048.toml", "split-hud"),
    ]
}

#[test]
fn every_shipped_document_validates_against_the_schema_it_names() {
    for (path, kind) in shipped() {
        if path.starts_with("assets/mods/examples/") && !root().join(path).is_file() {
            eprintln!("skipped {path}: the example mods submodule is not checked out");
            continue;
        }
        let text = std::fs::read_to_string(root().join(path)).unwrap();
        let table: toml::Table = text.parse().unwrap();
        let header: oag_schema::Header = table[oag_schema::TABLE].clone().try_into().unwrap();
        oag_schema::check(Some(&header), kind, 1).unwrap_or_else(|e| panic!("{path}: {e}"));
        accepts(kind, 1, &text).unwrap_or_else(|e| panic!("{path}: {e:#?}"));
    }
}

#[test]
fn every_schema_is_used_by_a_shipped_document_and_declares_the_language_version() {
    let used: Vec<&str> = shipped().iter().map(|(_, k)| *k).collect();
    for entry in std::fs::read_dir(schemas_dir().join("v1")).unwrap() {
        let path = entry.unwrap().path();
        assert_eq!(path.extension().unwrap(), "tosd", "{}", path.display());
        let kind = path.file_stem().unwrap().to_str().unwrap();
        assert!(used.contains(&kind), "no shipped document uses {kind}.tosd");
        let table: toml::Table = std::fs::read_to_string(&path).unwrap().parse().unwrap();
        assert_eq!(
            table["toml-schema"]["version"].as_str(),
            Some(oag_schema::LANGUAGE_VERSION),
            "{}",
            path.display()
        );
    }
}
