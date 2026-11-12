use std::io::Write;
use std::path::{Path, PathBuf};

use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

use crate::texture_pack::{Written, assign_titles, titles, write_manifest};
use crate::{Mod, search};

const INI_TEXT: &str = "[options]\nhash = xxh64\n[games]\nUCES00465 = textures.ini\nUCUS98712 = textures.ini\n[hashes]\n000000007d9d618f1a5be898 = intro/a.png\n";

/// A registry of two serials, as the composition root's would be: the serials of
/// `INI_TEXT` are one title's, and `UCES00001` another's.
fn resolve(serial: &str) -> Option<String> {
    match serial.replace('-', "").as_str() {
        "UCES00465" | "UCUS98712" => Some("wipeout-pulse".to_string()),
        "UCES00001" => Some("wipeout-pure".to_string()),
        _ => None,
    }
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("oag-pack-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn zip_of(path: &Path, entries: &[(&str, &[u8])]) {
    let mut writer = ZipWriter::new(std::fs::File::create(path).unwrap());
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    for (name, bytes) in entries {
        writer.start_file(*name, options).unwrap();
        writer.write_all(bytes).unwrap();
    }
    writer.finish().unwrap();
}

#[test]
fn a_folder_with_a_textures_ini_is_a_mod_that_starts_off() {
    let base = scratch("folder");
    let pack = base.join("WipEout Pulse (Upscale) 2026");
    std::fs::create_dir_all(&pack).unwrap();
    std::fs::write(pack.join("textures.ini"), INI_TEXT).unwrap();

    let found = search(std::slice::from_ref(&base));
    assert_eq!(found.len(), 1);
    let module = Mod::open(&found[0].path).unwrap();
    let m = &module.manifest;
    assert_eq!(m.id, "ppsspp.wipeout-pulse-upscale-2026");
    assert_eq!(m.name, "WipEout Pulse (Upscale) 2026");
    assert!(
        !m.default_enabled,
        "a recognised pack is off until switched on"
    );
    assert_eq!(m.license, crate::License::None);
    let pack = m.texture_pack.as_ref().unwrap();
    assert_eq!(pack.ini, "textures.ini");
    assert_eq!(pack.serials, ["UCES00465", "UCUS98712"]);
}

#[test]
fn a_zip_with_a_textures_ini_at_its_root_is_recognised_in_place() {
    let base = scratch("zip");
    let zip = base.join("Pulse Pack.zip");
    zip_of(
        &zip,
        &[
            ("textures.ini", INI_TEXT.as_bytes()),
            ("intro/a.png", b"png"),
        ],
    );

    let module = Mod::open(&zip).unwrap();
    assert_eq!(module.manifest.id, "ppsspp.pulse-pack");
    assert!(!module.manifest.default_enabled);
    assert_eq!(module.read("intro/a.png").unwrap().unwrap(), b"png");
    assert_eq!(module.origin(), crate::Origin::Zip);
}

#[test]
fn a_zip_wrapped_in_one_folder_is_read_through_it() {
    let base = scratch("wrapped");
    let zip = base.join("wrapped.zip");
    zip_of(
        &zip,
        &[
            ("Pack/textures.ini", INI_TEXT.as_bytes()),
            ("Pack/a.png", b"x"),
        ],
    );

    let module = Mod::open(&zip).unwrap();
    assert_eq!(
        module.manifest.texture_pack.as_ref().unwrap().serials.len(),
        2
    );
    assert_eq!(module.read("a.png").unwrap().unwrap(), b"x");
}

#[test]
fn a_folder_holding_textures_zip_is_found_as_the_zip_and_named_after_the_folder() {
    let base = scratch("nested");
    let pack = base.join("Pure-Pulse 2026-07-25");
    std::fs::create_dir_all(&pack).unwrap();
    zip_of(
        &pack.join("textures.zip"),
        &[("textures.ini", INI_TEXT.as_bytes())],
    );
    std::fs::write(pack.join("Readme.txt"), "not part of the pack").unwrap();

    let found = search(std::slice::from_ref(&base));
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].path, pack.join("textures.zip"));
    let module = Mod::open(&found[0].path).unwrap();
    assert_eq!(module.manifest.id, "ppsspp.pure-pulse-2026-07-25");
}

#[test]
fn a_written_manifest_wins_and_a_pack_still_starts_off_unless_it_says_otherwise() {
    let base = scratch("written");
    let manifest = |extra: &str| {
        format!(
            "toml-schema = {{ location = \"https://oag.topaxi.com/mods/schemas/v1/mod.tosd\", version = \"1.0.0\" }}\n\
             [mod]\nid = \"someone.pack\"\nname = \"Someone's pack\"\nversion = \"1.0.0\"\nauthors = [\"S\"]\n\
             description = \"d\"\n{extra}\n[texture-pack]\nformat = \"ppsspp\"\n"
        )
    };
    for (name, extra, on) in [("off", "", false), ("on", "default-enabled = true", true)] {
        let dir = base.join(name);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("oag-mod.toml"), manifest(extra)).unwrap();
        std::fs::write(dir.join("textures.ini"), INI_TEXT).unwrap();
        let module = Mod::open(&dir).unwrap();
        assert_eq!(module.manifest.id, "someone.pack");
        assert_eq!(module.manifest.default_enabled, on, "{name}");
        assert_eq!(module.manifest.texture_pack.unwrap().serials.len(), 2);
    }
}

#[test]
fn a_pack_format_this_build_does_not_read_is_refused_by_name() {
    let dir = scratch("format");
    std::fs::write(
        dir.join("oag-mod.toml"),
        "toml-schema = { location = \"https://oag.topaxi.com/mods/schemas/v1/mod.tosd\", version = \"1.0.0\" }\n\
         [mod]\nid = \"a.b\"\nname = \"n\"\nversion = \"1.0.0\"\nauthors = [\"S\"]\ndescription = \"d\"\n\
         [texture-pack]\nformat = \"dolphin\"\n",
    )
    .unwrap();
    let error = Mod::open(&dir).unwrap_err().to_string();
    assert!(error.contains("texture-pack.format"), "{error}");
    assert!(error.contains("dolphin"), "{error}");
}

#[test]
fn an_ordinary_mod_is_on_by_default() {
    let dir = scratch("ordinary");
    std::fs::write(
        dir.join("oag-mod.toml"),
        "toml-schema = { location = \"https://oag.topaxi.com/mods/schemas/v1/mod.tosd\", version = \"1.0.0\" }\n\
         [mod]\nid = \"a.b\"\nname = \"n\"\nversion = \"1.0.0\"\nauthors = [\"S\"]\ndescription = \"d\"\n",
    )
    .unwrap();
    assert!(Mod::open(&dir).unwrap().manifest.default_enabled);
}

#[cfg(unix)]
#[test]
fn a_pack_linked_once_per_serial_is_one_candidate() {
    let base = scratch("linked");
    let real = base.join("real");
    std::fs::create_dir_all(&real).unwrap();
    std::fs::write(real.join("textures.ini"), INI_TEXT).unwrap();
    let memstick = base.join("PSP").join("TEXTURES");
    std::fs::create_dir_all(&memstick).unwrap();
    for serial in ["UCES00465", "UCUS98712"] {
        std::os::unix::fs::symlink(&real, memstick.join(serial)).unwrap();
    }
    let found = search(&[base.clone(), memstick]);
    assert_eq!(found.len(), 1, "{found:?}");
}

#[test]
fn a_folder_with_neither_a_manifest_nor_an_ini_is_not_a_candidate() {
    let base = scratch("empty");
    std::fs::create_dir_all(base.join("something")).unwrap();
    std::fs::write(base.join("something/readme.txt"), "x").unwrap();
    assert!(search(&[base]).is_empty());
}

fn folder_pack(name: &str, pack_name: &str) -> PathBuf {
    let pack = scratch(name).join(pack_name);
    std::fs::create_dir_all(&pack).unwrap();
    std::fs::write(pack.join("textures.ini"), INI_TEXT).unwrap();
    pack
}

const REPOSITORY: &str = "https://github.com/someone/WipEout-Pulse-Upscale-Version.git";

#[test]
fn a_written_manifest_is_the_mod_detection_made_with_its_own_fields_winning() {
    let pack = folder_pack("generated", "WipEout Pulse (Upscale) 2026");
    let recognised = Mod::open(&pack).unwrap().manifest;

    let written = write_manifest(&pack, Some(REPOSITORY), &resolve).unwrap();
    assert_eq!(written, Written::Wrote(pack.join("oag-mod.toml")));
    let module = Mod::open(&pack).unwrap();
    let m = &module.manifest;

    assert_eq!(m.id, recognised.id);
    assert_eq!(m.name, recognised.name);
    assert_eq!(m.texture_pack, recognised.texture_pack);
    assert_eq!(
        m.texture_pack.as_ref().unwrap().serials,
        ["UCES00465", "UCUS98712"]
    );
    assert!(!m.default_enabled);
    assert_eq!(m.default_enabled, recognised.default_enabled);
    assert_eq!(m.license, recognised.license);
    assert_eq!(
        m.repository.as_deref(),
        Some(REPOSITORY),
        "the manifest's own field wins"
    );
    assert_eq!(m.authors, ["someone"]);
    assert!(recognised.authors.is_empty() && recognised.repository.is_none());
}

#[test]
fn the_written_text_validates_against_the_mod_schema_and_names_no_serials() {
    let pack = folder_pack("schema", "A \"quoted\" pack");
    write_manifest(&pack, None, &resolve).unwrap();
    let text = std::fs::read_to_string(pack.join("oag-mod.toml")).unwrap();
    oag_schema::validate::accepts("mod", 1, &text).unwrap_or_else(|e| panic!("{e:?}\n{text}"));
    assert!(!text.contains("serials"), "{text}");
    assert!(text.contains("default-enabled = false"), "{text}");
    assert!(text.contains("authors = [\"unknown\"]"), "{text}");
    assert_eq!(Mod::open(&pack).unwrap().manifest.name, "A \"quoted\" pack");
}

#[test]
fn a_manifest_that_is_already_there_is_left_alone_byte_for_byte() {
    let pack = folder_pack("kept", "pack");
    write_manifest(&pack, Some(REPOSITORY), &resolve).unwrap();
    let edited = std::fs::read_to_string(pack.join("oag-mod.toml"))
        .unwrap()
        .replace("authors = [\"someone\"]", "authors = [\"Me\"]");
    std::fs::write(pack.join("oag-mod.toml"), &edited).unwrap();

    let again = write_manifest(&pack, Some("https://example.com/other/repo"), &resolve).unwrap();
    assert_eq!(again, Written::Kept(pack.join("oag-mod.toml")));
    assert_eq!(
        std::fs::read_to_string(pack.join("oag-mod.toml")).unwrap(),
        edited
    );
    assert_eq!(Mod::open(&pack).unwrap().manifest.authors, ["Me"]);
}

#[test]
fn a_folder_without_a_textures_ini_gets_no_manifest() {
    let base = scratch("not-a-pack");
    let error = write_manifest(&base, None, &resolve)
        .unwrap_err()
        .to_string();
    assert!(error.contains("textures.ini"), "{error}");
    assert!(!base.join("oag-mod.toml").exists());
}

#[test]
fn a_pack_is_for_the_titles_its_serials_resolve_to_each_once_in_order() {
    let serials = |list: &[&str]| list.iter().map(|s| (*s).to_string()).collect::<Vec<_>>();
    assert_eq!(
        titles(
            &serials(&["UCES00465", "UCUS98712", "UCES-00001"]),
            &resolve
        ),
        ["wipeout-pulse", "wipeout-pure"]
    );
    assert_eq!(
        titles(&serials(&["UCJS10007", "UCES00001"]), &resolve),
        ["wipeout-pure"],
        "an unknown serial adds nothing"
    );
    assert!(
        titles(&serials(&["UCJS10007"]), &resolve).is_empty(),
        "no serial resolves: for every title, as a manifest with no titles is"
    );
}

#[test]
fn the_written_manifest_names_the_titles_and_a_recognised_pack_gets_them_assigned() {
    let pack = folder_pack("titles", "pulse-pack");
    write_manifest(&pack, None, &resolve).unwrap();
    let text = std::fs::read_to_string(pack.join("oag-mod.toml")).unwrap();
    oag_schema::validate::accepts("mod", 1, &text).unwrap_or_else(|e| panic!("{e:?}\n{text}"));
    assert!(text.contains("titles = [\"wipeout-pulse\"]"), "{text}");
    assert_eq!(Mod::open(&pack).unwrap().manifest.titles, ["wipeout-pulse"]);

    let bare = folder_pack("titles-recognised", "dropped-in");
    let mut manifest = Mod::open(&bare).unwrap().manifest;
    assert!(
        manifest.titles.is_empty(),
        "recognition alone cannot see titles"
    );
    assign_titles(&mut manifest, &resolve);
    assert_eq!(manifest.titles, ["wipeout-pulse"]);
    manifest.titles = vec!["wipeout-hd".to_string()];
    assign_titles(&mut manifest, &resolve);
    assert_eq!(manifest.titles, ["wipeout-hd"], "an author's titles win");
}

#[test]
fn a_pack_no_serial_of_which_resolves_writes_no_titles() {
    let pack = scratch("unresolved").join("odd");
    std::fs::create_dir_all(&pack).unwrap();
    std::fs::write(
        pack.join("textures.ini"),
        "[games]\nUCJS10007 = textures.ini\n",
    )
    .unwrap();
    write_manifest(&pack, None, &resolve).unwrap();
    let text = std::fs::read_to_string(pack.join("oag-mod.toml")).unwrap();
    assert!(!text.contains("titles"), "{text}");
}
