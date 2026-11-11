use std::collections::BTreeMap;
use std::path::PathBuf;

use super::*;

const MANIFEST: &str = r#"
toml-schema = { location = "https://oag.topaxi.com/mods/schemas/v1/mod.tosd", version = "1.0.0" }

[mod]
id = "test.packaged"
name = "Packaged"
version = "1.0.0"
authors = ["A. Modder"]
license = "CC-BY-4.0"
description = "A mod the package carries."
"#;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("oag-mobile-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// A package's assets: the files under [`ROOT`] and the index the build writes for them.
fn package(files: &[(&str, &[u8])]) -> BTreeMap<String, Vec<u8>> {
    let mut assets = BTreeMap::new();
    let mut index = String::new();
    for (path, data) in files {
        index.push_str(&format!("{} {:064} {path}\n", data.len(), 0));
        assets.insert(format!("{ROOT}/{path}"), data.to_vec());
    }
    assets.insert(INDEX.to_owned(), index.into_bytes());
    assets
}

fn reader(assets: &BTreeMap<String, Vec<u8>>) -> impl FnMut(&str) -> io::Result<Vec<u8>> + '_ {
    move |name| {
        assets
            .get(name)
            .cloned()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, name.to_owned()))
    }
}

#[test]
fn a_first_launch_writes_every_file_the_index_names_with_its_subfolders() {
    let dest = scratch("fresh").join("assets/mods/official");
    let assets = package(&[
        ("remix/oag-mod.toml", MANIFEST.as_bytes()),
        ("remix/strings/english.toml", b"[strings]\n"),
        ("slipstream/ext/slipstream.wasm", b"\0asm"),
    ]);
    let done = install(&mut reader(&assets), &dest).unwrap();
    assert_eq!(
        done,
        Installed::Extracted {
            files: 3,
            bytes: (MANIFEST.len() + 10 + 4) as u64
        }
    );
    assert_eq!(
        std::fs::read(dest.join("slipstream/ext/slipstream.wasm")).unwrap(),
        b"\0asm"
    );
    assert_eq!(
        std::fs::read(dest.join("remix/strings/english.toml")).unwrap(),
        b"[strings]\n"
    );
}

/// The path `oag_mod::mods_search_path` lists first after the working directory's `data/mods`
/// is [`oag_mod::OFFICIAL`]; with the working directory the app's files folder it is where
/// `android_main` installs, and what it installs opens as a mod.
#[test]
fn what_is_installed_under_the_official_path_is_found_and_opened_by_the_mod_search() {
    let files = scratch("search");
    let dest = files.join(oag_mod::OFFICIAL);
    let assets = package(&[("packaged/oag-mod.toml", MANIFEST.as_bytes())]);
    install(&mut reader(&assets), &dest).unwrap();

    let bases = [files.join(oag_mod::OFFICIAL)];
    let found = oag_mod::search(&bases);
    assert_eq!(found.len(), 1, "{found:?}");
    let module = oag_mod::Mod::open(&found[0].path).unwrap();
    assert_eq!(module.manifest.id, "test.packaged");
}

#[test]
fn a_second_launch_of_the_same_build_leaves_the_folder_alone() {
    let dest = scratch("again").join("official");
    let assets = package(&[("a/oag-mod.toml", MANIFEST.as_bytes())]);
    install(&mut reader(&assets), &dest).unwrap();
    std::fs::write(dest.join("a/marker"), b"x").unwrap();
    assert_eq!(
        install(&mut reader(&assets), &dest).unwrap(),
        Installed::Current
    );
    assert!(
        dest.join("a/marker").exists(),
        "a current folder is not rewritten"
    );
}

#[test]
fn a_newer_build_replaces_the_folder_whole_and_a_dropped_mod_does_not_linger() {
    let dest = scratch("newer").join("official");
    let old = package(&[
        ("gone/oag-mod.toml", MANIFEST.as_bytes()),
        ("kept/oag-mod.toml", b"old"),
    ]);
    install(&mut reader(&old), &dest).unwrap();
    let new = package(&[("kept/oag-mod.toml", b"newer")]);
    assert!(matches!(
        install(&mut reader(&new), &dest).unwrap(),
        Installed::Extracted { files: 1, .. }
    ));
    assert!(!dest.join("gone").exists());
    assert_eq!(
        std::fs::read(dest.join("kept/oag-mod.toml")).unwrap(),
        b"newer"
    );
    assert!(!dest.with_file_name("official.new").exists());
}

#[test]
fn a_package_without_an_index_is_an_error_and_leaves_the_previous_folder() {
    let dest = scratch("noindex").join("official");
    let assets = package(&[("a/oag-mod.toml", b"x")]);
    install(&mut reader(&assets), &dest).unwrap();
    let error = install(&mut reader(&BTreeMap::new()), &dest).unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::NotFound);
    assert!(dest.join("a/oag-mod.toml").exists());
}

#[test]
fn a_file_that_is_not_the_size_the_index_says_is_refused_and_nothing_is_swapped_in() {
    let dest = scratch("size").join("official");
    let mut assets = package(&[("a/oag-mod.toml", b"four")]);
    assets.insert(format!("{ROOT}/a/oag-mod.toml"), b"truncated".to_vec());
    let error = install(&mut reader(&assets), &dest).unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    assert!(!dest.exists());
}

#[test]
fn an_index_path_that_climbs_out_of_the_folder_or_is_absolute_is_refused() {
    for path in ["../escape", "a/../../escape", "/etc/passwd"] {
        let text = format!("1 {:064} {path}\n", 0);
        assert!(parse(&text).is_err(), "{path}");
    }
    assert!(parse("").is_err());
    assert!(parse("not-a-number abc a/b\n").is_err());
    assert_eq!(
        parse(&format!("7 {:064} a/b c.toml\n", 0)).unwrap(),
        [Entry {
            size: 7,
            path: "a/b c.toml"
        }]
    );
}

/// The build writes the assets this module reads; dropping either half breaks the other.
#[test]
fn the_apk_build_stages_the_official_mods_and_writes_the_index_this_module_reads() {
    let script = include_str!("../../../../scripts/build-apk.sh");
    assert!(
        script.contains("scripts/stage-official-mods.sh"),
        "mods are staged"
    );
    assert!(
        script.contains(&format!("assets/{ROOT}")),
        "under the asset root"
    );
    assert!(
        script.contains(&format!("assets/{INDEX}")),
        "with the index"
    );
    let ios = include_str!("../../../../scripts/build-ios-app.sh");
    assert!(
        ios.contains("scripts/stage-official-mods.sh"),
        "the iOS bundle carries them too"
    );
}
