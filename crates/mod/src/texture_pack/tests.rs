use std::io::Write;
use std::path::{Path, PathBuf};

use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

use crate::{Mod, search};

const INI_TEXT: &str = "[options]\nhash = xxh64\n[games]\nUCES00465 = textures.ini\nUCUS98712 = textures.ini\n[hashes]\n000000007d9d618f1a5be898 = intro/a.png\n";

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
