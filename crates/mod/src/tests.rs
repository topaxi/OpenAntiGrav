use std::io::Write;
use std::path::{Path, PathBuf};

use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

use super::*;

const MANIFEST_TEXT: &str = r#"
toml-schema = { location = "https://oag.topaxi.com/mods/schemas/v1/mod.tosd", version = "1.0.0" }

[mod]
id = "test.example"
name = "Example"
version = "1.2.3"
authors = ["A. Modder"]
license = "CC-BY-4.0"
description = "A mod for the tests."
"#;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("oag-mod-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn folder(dir: &Path, manifest: &str) {
    std::fs::write(dir.join(MANIFEST), manifest).unwrap();
    std::fs::create_dir_all(dir.join("strings")).unwrap();
    std::fs::write(dir.join("menu.toml"), "# rows\n".repeat(200)).unwrap();
    std::fs::write(dir.join("strings/english.toml"), "[strings]\n").unwrap();
    std::fs::write(dir.join("icon.png"), [0x89, b'P', b'N', b'G']).unwrap();
}

fn zip_with(path: &Path, method: CompressionMethod, wrapper: &str) {
    let mut writer = ZipWriter::new(std::fs::File::create(path).unwrap());
    let options = SimpleFileOptions::default().compression_method(method);
    for (name, bytes) in [
        (MANIFEST, MANIFEST_TEXT.as_bytes().to_vec()),
        ("menu.toml", "# rows\n".repeat(200).into_bytes()),
    ] {
        writer
            .start_file(format!("{wrapper}{name}"), options)
            .unwrap();
        writer.write_all(&bytes).unwrap();
    }
    writer.finish().unwrap();
}

#[test]
fn a_folder_a_deflate_zip_a_stored_zip_and_a_zstd_oagmod_all_read_the_same_mod() {
    let dir = scratch("four");
    let folder_mod = dir.join("folder");
    std::fs::create_dir_all(&folder_mod).unwrap();
    folder(&folder_mod, MANIFEST_TEXT);
    zip_with(&dir.join("deflate.zip"), CompressionMethod::Deflated, "");
    zip_with(&dir.join("stored.zip"), CompressionMethod::Stored, "");
    pack::pack(&folder_mod, &dir.join("zstd.oagmod")).unwrap();

    let want = "# rows\n".repeat(200);
    for (path, origin) in [
        (folder_mod.clone(), Origin::Folder),
        (dir.join("deflate.zip"), Origin::Zip),
        (dir.join("stored.zip"), Origin::Zip),
        (dir.join("zstd.oagmod"), Origin::OagMod),
    ] {
        let module = Mod::open(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        assert_eq!(module.manifest.id, "test.example", "{}", path.display());
        assert_eq!(module.origin(), origin);
        assert_eq!(module.menu().unwrap().as_deref(), Some(want.as_str()));
        assert_eq!(module.read("nothing.toml").unwrap(), None);
    }
}

#[test]
fn pack_writes_zstd_for_text_and_stores_what_is_already_compressed() {
    let dir = scratch("pack");
    let source = dir.join("m");
    std::fs::create_dir_all(&source).unwrap();
    folder(&source, MANIFEST_TEXT);
    let out = dir.join("m.oagmod");
    let packed = pack::pack(&source, &out).unwrap();
    assert_eq!(packed.stored, 1, "icon.png");
    assert_eq!(packed.zstd, 3, "manifest, menu, strings");
    assert!(packed.bytes_out < packed.bytes_in + 600);

    let mut zip = zip::ZipArchive::new(std::fs::File::open(&out).unwrap()).unwrap();
    let method = |zip: &mut zip::ZipArchive<std::fs::File>, name: &str| {
        zip.by_name(name).unwrap().compression()
    };
    assert_eq!(method(&mut zip, "menu.toml"), CompressionMethod::Zstd);
    assert_eq!(method(&mut zip, "icon.png"), CompressionMethod::Stored);
    assert_eq!(zip.by_index(0).unwrap().name().unwrap(), MANIFEST);
}

#[test]
fn a_zip_of_the_folder_itself_reads_with_its_wrapping_directory_stripped() {
    let dir = scratch("wrapped");
    zip_with(
        &dir.join("wrapped.zip"),
        CompressionMethod::Deflated,
        "mymod/",
    );
    let module = Mod::open(&dir.join("wrapped.zip")).unwrap();
    assert_eq!(module.manifest.id, "test.example");
    assert!(module.names().contains(&"menu.toml".to_string()));
}

#[test]
fn a_path_out_of_the_mod_is_refused() {
    let dir = scratch("escape");
    folder(&dir, MANIFEST_TEXT);
    let module = Mod::open(&dir).unwrap();
    for name in ["../x", "/etc/passwd", "a/../../b", "c:/x"] {
        assert!(
            matches!(module.read(name), Err(Error::BadName { .. })),
            "{name}"
        );
    }
}

#[test]
fn no_license_loads_and_is_marked_and_the_other_checks_refuse_by_key() {
    let none = MANIFEST_TEXT.replace("license = \"CC-BY-4.0\"\n", "");
    let parsed = Manifest::parse(&none, &|_| false).unwrap();
    assert_eq!(parsed.license, License::None);
    assert_eq!(parsed.license.report(), "no license stated");

    let both = MANIFEST_TEXT.replace(
        "license = \"CC-BY-4.0\"",
        "license = \"MIT\"\nlicense-file = \"LICENSE\"",
    );
    for (text, key) in [
        (both.as_str(), "license"),
        (&MANIFEST_TEXT.replace("1.2.3", "one"), "version"),
        (&MANIFEST_TEXT.replace("test.example", "Test Example"), "id"),
        (&MANIFEST_TEXT.replace("CC-BY-4.0", "MIT OR"), "license"),
        (&MANIFEST_TEXT.replace("[\"A. Modder\"]", "[]"), "authors"),
    ] {
        let error = Manifest::parse(text, &|_| true).unwrap_err().to_string();
        assert!(error.contains(key), "{key}: {error}");
    }
    let missing = MANIFEST_TEXT.replace("license = \"CC-BY-4.0\"", "license-file = \"LICENSE\"");
    assert!(Manifest::parse(&missing, &|_| false).is_err());
    assert!(Manifest::parse(&missing, &|n| n == "LICENSE").is_ok());
}

#[test]
fn a_manifest_without_the_schema_header_or_with_an_unknown_key_is_refused() {
    let headerless = MANIFEST_TEXT.lines().skip(2).collect::<Vec<_>>().join("\n");
    assert!(Manifest::parse(&headerless, &|_| true).is_err());
    let extra = MANIFEST_TEXT.replace("[mod]", "[mod]\nsurprise = 1");
    assert!(Manifest::parse(&extra, &|_| true).is_err());
}

#[test]
fn a_search_finds_each_kind_skips_a_broken_one_and_names_a_duplicate() {
    let dir = scratch("search");
    let folder_mod = dir.join("one");
    std::fs::create_dir_all(&folder_mod).unwrap();
    folder(&folder_mod, MANIFEST_TEXT);
    zip_with(&dir.join("two.zip"), CompressionMethod::Deflated, "");
    std::fs::write(dir.join("broken.oagmod"), b"not a zip").unwrap();
    std::fs::create_dir_all(dir.join("plain-folder")).unwrap();

    let loaded = load(std::slice::from_ref(&dir), &semver::Version::new(0, 1, 0));
    assert_eq!(loaded.mods.len(), 1, "{:?}", loaded.failed);
    assert_eq!(loaded.failed.len(), 2, "{:?}", loaded.failed);
    assert!(
        loaded
            .failed
            .iter()
            .any(|p| p.reason.contains("broken.oagmod") && p.id.is_none())
    );
    assert!(
        loaded
            .failed
            .iter()
            .any(|p| p.reason.contains("already loaded") && p.id.as_deref().is_some())
    );
}

#[test]
fn the_built_in_extras_open_and_carry_the_projects_own_license() {
    let extras = extras::builtin().unwrap();
    assert_eq!(extras.manifest.id, extras::ID);
    assert_eq!(extras.origin(), Origin::Builtin);
    assert_eq!(
        extras.manifest.license,
        License::Spdx(env!("CARGO_PKG_LICENSE").to_string()),
        "the workspace's own licence, so the two cannot drift"
    );
    assert!(extras.menu().unwrap().is_some());
}

#[test]
fn an_extension_table_names_its_rules_module_and_opens_nothing() {
    let text = format!(
        "{MANIFEST_TEXT}\n[extension]\nsimulation = true\napi = 1\nstate_bytes = 64\n\n[extension.rules]\nmodule = \"ext/curse.wasm\"\n"
    );
    let manifest = Manifest::parse(&text, &|path| path == "ext/curse.wasm").unwrap();
    assert_eq!(
        manifest.extension,
        Some(Extension {
            simulation: true,
            api: 1,
            memory_pages: 16,
            state_bytes: 64,
            rules: Some(Rules {
                module: "ext/curse.wasm".to_string()
            }),
            track: None,
        })
    );
    assert_eq!(
        Manifest::parse(MANIFEST_TEXT, &|_| false)
            .unwrap()
            .extension,
        None
    );
}

#[test]
fn a_rules_module_without_simulation_or_missing_is_refused_by_name() {
    let unsaid =
        format!("{MANIFEST_TEXT}\n[extension]\napi = 1\n[extension.rules]\nmodule = \"a.wasm\"\n");
    let error = Manifest::parse(&unsaid, &|_| true).unwrap_err().to_string();
    assert!(error.contains("simulation = true"), "{error}");
    let missing = format!(
        "{MANIFEST_TEXT}\n[extension]\nsimulation = true\napi = 1\n[extension.rules]\nmodule = \"a.wasm\"\n"
    );
    let error = Manifest::parse(&missing, &|_| false)
        .unwrap_err()
        .to_string();
    assert!(error.contains("not in the mod"), "{error}");
}

#[test]
fn a_track_module_names_its_module_and_its_track_file() {
    let text = format!(
        "{MANIFEST_TEXT}\n[extension]\nsimulation = true\napi = 1\nstate_bytes = 3\n\n[extension.track]\nmodule = \"ext/rain.wasm\"\ntrack = \"tracks/a.track.toml\"\n"
    );
    let all = |path: &str| matches!(path, "ext/rain.wasm" | "tracks/a.track.toml");
    let extension = Manifest::parse(&text, &all).unwrap().extension.unwrap();
    assert_eq!(extension.rules, None);
    assert_eq!(
        extension.track,
        Some(Track {
            module: "ext/rain.wasm".to_string(),
            track: "tracks/a.track.toml".to_string(),
        })
    );
    let no_track_file = |path: &str| path == "ext/rain.wasm";
    let error = Manifest::parse(&text, &no_track_file)
        .unwrap_err()
        .to_string();
    assert!(error.contains("extension.track.track"), "{error}");
    let unsaid = text.replace("simulation = true\n", "");
    let error = Manifest::parse(&unsaid, &all).unwrap_err().to_string();
    assert!(error.contains("simulation = true"), "{error}");
}

#[test]
fn a_manifest_carries_icons_that_are_pngs_in_the_mod() {
    let text = format!("{MANIFEST_TEXT}\n[icons]\nbadge = \"ui/badge.png\"\n");
    let parsed = Manifest::parse(&text, &|path| path == "ui/badge.png").unwrap();
    assert_eq!(parsed.icons["badge"], "ui/badge.png");
    let missing = Manifest::parse(&text, &|_| false).unwrap_err().to_string();
    assert!(
        missing.contains("badge") && missing.contains("not a .png in the mod"),
        "{missing}"
    );
    let wrong = text.replace("ui/badge.png", "ui/badge.jpg");
    assert!(Manifest::parse(&wrong, &|_| true).is_err(), "only a PNG");
    let spaced = text.replace("badge =", "\"bad name\" =");
    assert!(Manifest::parse(&spaced, &|_| true).is_err());
    assert!(
        Manifest::parse(MANIFEST_TEXT, &|_| true)
            .unwrap()
            .icons
            .is_empty()
    );
}
