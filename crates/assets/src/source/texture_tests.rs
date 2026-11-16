//! [`Archives::read_texture`]: the name as written first, then the platform's
//! own spelling of it.

use super::tests::mounted;
use super::*;
use crate::testing;

fn on(platform: Platform, dir: &str, entries: &[(&str, &[u8])]) -> Archives {
    let dir = testing::temp_dir(dir);
    let mut archives = mounted(&dir, entries, Vec::new());
    archives.layout.platform = platform;
    archives
}

#[test]
fn a_name_held_as_written_is_read_as_written() {
    let mut archives = on(
        Platform::Vita,
        "texture-as-written",
        &[("Data\\a.gtf", b"gtf"), ("Data\\a.gxt", b"gxt")],
    );
    let (held, blob) = archives.read_texture("Data\\a.gtf").unwrap();
    assert_eq!(
        (held.as_str(), blob.as_slice()),
        ("Data\\a.gtf", &b"gtf"[..])
    );
}

#[test]
fn the_vita_reads_a_gtf_name_as_its_gxt() {
    let mut archives = on(Platform::Vita, "texture-vita", &[("Data\\a.gxt", b"gxt")]);
    let (held, blob) = archives.read_texture("Data\\a.gtf").unwrap();
    assert_eq!(
        (held.as_str(), blob.as_slice()),
        ("Data\\a.gxt", &b"gxt"[..])
    );
}

#[test]
fn the_ps4_reads_a_mip_name_as_its_gnf_in_its_own_directory() {
    let mut archives = on(
        Platform::Ps4,
        "texture-ps4",
        &[("env\\texturesps4\\billboard8.gnf", b"gnf")],
    );
    let (held, blob) = archives
        .read_texture("env\\textures\\billboard8.mip")
        .unwrap();
    assert_eq!(
        (held.as_str(), blob.as_slice()),
        ("env\\texturesps4\\billboard8.gnf", &b"gnf"[..])
    );
}

#[test]
fn a_platform_does_not_borrow_another_platforms_spelling() {
    let mut archives = on(
        Platform::Ps3,
        "texture-no-borrow",
        &[("Data\\a.gxt", b"gxt")],
    );
    assert!(archives.resolve_texture("Data\\a.gtf").is_none());
    assert!(archives.read_texture("Data\\a.gtf").is_err());
}

#[test]
fn a_miss_names_the_entry_that_was_asked_for() {
    let mut archives = on(Platform::Vita, "texture-miss", &[]);
    let error = archives
        .read_texture("Data\\a.gtf")
        .unwrap_err()
        .to_string();
    assert!(error.contains("a.gtf"), "{error}");
}

#[test]
fn a_fallback_layer_answers_for_the_respelling() {
    let dir = testing::temp_dir("texture-layer");
    let layer = crate::layers::Layer::from_dir(LayerKind::Fallback, &{
        let path = dir.join("fb");
        std::fs::create_dir_all(path.join("Data")).unwrap();
        std::fs::write(path.join("Data/a.gnf"), b"ours").unwrap();
        path
    })
    .unwrap();
    let mut archives = mounted(&dir, &[], Vec::new()).with_fallback(layer);
    archives.layout.platform = Platform::Ps4;
    let (held, blob) = archives.read_texture("Data\\a.gtf").unwrap();
    assert_eq!(
        (held.as_str(), blob.as_slice()),
        ("Data\\a.gnf", &b"ours"[..])
    );
}
