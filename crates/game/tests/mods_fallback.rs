//! A mod's fallback entries (ADR-0100): `fallback/<title>/` of a mod that is on, read in place
//! from a folder or a `.oagmod`, one layer per title its `titles` admit, none when it is off.

use std::io::Write;
use std::path::Path;

use oag_game::mods::Mods;
use oag_testdata::scratch::TempDir;
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

fn manifest(id: &str) -> String {
    manifest_with(id, "")
}

fn manifest_with(id: &str, extra: &str) -> String {
    format!(
        r#"
toml-schema = {{ location = "https://oag.topaxi.com/mods/schemas/v1/mod.tosd", version = "1.0.0" }}

[mod]
id = "{id}"
name = "{id}"
version = "1.0.0"
authors = ["A. Modder"]
license = "CC0-1.0"
description = "Fallback entries."
titles = ["wipeout-pulse", "wipeout-hd"]
{extra}"#
    )
}

const FILES: [(&str, &[u8]); 3] = [
    ("fallback/wipeout-pulse/Data/XML/Ours.xml", b"pulse"),
    ("fallback/wipeout-hd/Data/XML/Ours.xml", b"hd"),
    (
        "fallback/wipeout-pure/Data/XML/Ours.xml",
        b"pure, not in titles",
    ),
];

fn base(name: &str) -> TempDir {
    oag_testdata::scratch::dir(&format!("mods-fallback-{name}"))
}

fn folder(root: &Path, id: &str) {
    std::fs::create_dir_all(root).unwrap();
    std::fs::write(root.join("oag-mod.toml"), manifest(id)).unwrap();
    for (name, bytes) in FILES {
        let path = root.join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, bytes).unwrap();
    }
}

fn packed(path: &Path, id: &str) {
    let mut writer = ZipWriter::new(std::fs::File::create(path).unwrap());
    let options = SimpleFileOptions::default();
    writer.start_file("oag-mod.toml", options).unwrap();
    writer.write_all(manifest(id).as_bytes()).unwrap();
    for (name, bytes) in FILES {
        writer.start_file(name, options).unwrap();
        writer.write_all(bytes).unwrap();
    }
    writer.finish().unwrap();
}

fn load(base: &Path, off: &[&str]) -> Mods {
    let switches = oag_game::settings::Mods {
        disabled: off.iter().map(ToString::to_string).collect(),
        ..Default::default()
    };
    Mods::load_from(&[base.to_path_buf()], &switches).unwrap()
}

#[test]
fn a_folder_and_an_oagmod_each_give_one_layer_per_title_they_are_for() {
    let dir = base("both");
    folder(&dir.join("loose"), "test.loose");
    packed(&dir.join("packed.oagmod"), "test.packed");
    let mods = load(&dir, &[]);
    let fallbacks = mods.fallbacks();
    let mut seen: Vec<(String, String)> = fallbacks
        .iter()
        .map(|f| {
            (
                f.layer.label().split(' ').next().unwrap().to_string(),
                f.slug.clone(),
            )
        })
        .collect();
    seen.sort();
    assert_eq!(
        seen,
        [
            ("test.loose", "wipeout-hd"),
            ("test.loose", "wipeout-pulse"),
            ("test.packed", "wipeout-hd"),
            ("test.packed", "wipeout-pulse"),
        ]
        .map(|(a, b)| (a.to_string(), b.to_string())),
        "Pure is not in the mods' titles"
    );
    for fallback in &fallbacks {
        assert_eq!(fallback.layer.kind(), oag_assets::layers::Kind::Mod);
        let want: &[u8] = if fallback.slug == "wipeout-pulse" {
            b"pulse"
        } else {
            b"hd"
        };
        assert_eq!(
            fallback.layer.read("Data\\XML\\ours.xml").unwrap(),
            want,
            "{}",
            fallback.layer.label()
        );
    }
    assert!(
        load(&dir, &["test.loose", "test.packed"])
            .fallbacks()
            .is_empty()
    );
}

/// Fallback entries can hold a race's own tables, so a mod with them counts in the LAN
/// fingerprint (ADR-0083) while it is on.
#[test]
fn a_mod_with_fallback_entries_changes_the_lan_fingerprint() {
    let dir = base("fingerprint");
    folder(&dir.join("loose"), "test.loose");
    let none = oag_net::discovery::NO_MODS;
    assert_ne!(
        load(&dir, &[]).content_fingerprint(&Default::default()),
        none
    );
    assert_eq!(
        load(&dir, &["test.loose"]).content_fingerprint(&Default::default()),
        none
    );
}

/// `[mod] network-races = false`: the entries serve only single-player races, so a network race
/// does not mount them and the mod leaves the fingerprint alone.
#[test]
fn a_mod_off_network_races_leaves_the_fingerprint_and_is_not_mounted_there() {
    let dir = base("single-player");
    folder(&dir.join("loose"), "test.loose");
    std::fs::write(
        dir.join("loose/oag-mod.toml"),
        manifest_with("test.loose", "network-races = false\n"),
    )
    .unwrap();
    let mods = load(&dir, &[]);
    assert_eq!(
        mods.content_fingerprint(&Default::default()),
        oag_net::discovery::NO_MODS
    );
    let fallbacks = mods.fallbacks();
    assert!(!fallbacks.is_empty() && fallbacks.iter().all(|f| !f.network));
    let layering = oag_source::layers::Layering {
        overrides: Vec::new(),
        mods: fallbacks,
    };
    assert_eq!(layering.mods_for(oag_pulse::TITLE).len(), 1);
    assert!(layering.for_network().mods_for(oag_pulse::TITLE).is_empty());
}

/// The official `oag.detonator-pulse` is single player: one player with it on and one with it
/// off fingerprint the same, so they can race each other.
#[test]
fn players_with_and_without_pulse_detonator_share_a_fingerprint() {
    let official = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/mods/official");
    if !official.join("detonator-pulse/oag-mod.toml").is_file() {
        eprintln!("skipped: the official-mods submodule is not checked out");
        return;
    }
    let on = load(&official, &[]);
    assert!(!on.is_disabled("oag.detonator-pulse"));
    assert!(!on.fallbacks().is_empty(), "the mod carries Pulse entries");
    let off = load(&official, &["oag.detonator-pulse"]);
    assert_eq!(
        on.content_fingerprint(&Default::default()),
        off.content_fingerprint(&Default::default())
    );
}
