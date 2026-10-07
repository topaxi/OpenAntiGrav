//! The PS4 Omega packages, read in place, against the folders `LibOrbisPkg`
//! extracted from the same files (`data/extracted/ps4/omega-eu{,-patch}`).
//!
//! The file lists and sizes are compared exactly. Contents: every file under
//! 64 MiB byte for byte; the multi-gigabyte archives by head, tail and 48
//! spread 256 KiB windows (a full compare of 45 GB would sit past the per-test
//! budget). `OAG_PS4_PKG_FULL=1` compares every byte of every file; that run
//! is recorded in `docs/formats/ps4-package.md`.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;

use oag_disc::package::PackageSource;
use oag_disc::ps4_pkg::Ps4Pkg;

const BASE: &str = "omega-ps4-eu.pkg";
const PATCH: &str = "omega-ps4-eu-patch.pkg";
const WHOLE_LIMIT: u64 = 64 << 20;
const WINDOW: u64 = 256 << 10;

const BASE_FILES: [(&str, u64); 14] = [
    ("data00.psarc", 13_546_361_856),
    ("data01.psarc", 10_741_437_238),
    ("data02.psarc", 9_739_820_661),
    ("data03.psarc", 2_576_997_583),
    ("data04.psarc", 6_397_417_167),
    ("eboot.bin", 10_493_719),
    ("sce_discmap.plt", 3_288_792),
    ("session_image.jpg", 70_634),
    ("sce_module/libSceFios2.prx", 451_057),
    ("sce_module/libSceNpToolkit2.prx", 593_934),
    ("sce_module/libc.prx", 1_073_730),
    ("sce_sys/keystone", 96),
    ("sce_sys/about/right.sprx", 16_025),
    ("sce_sys/keymap_rp/000.png", 57_681),
];

const PATCH_FILES: [(&str, u64); 11] = [
    ("data05.psarc", 684_918_648),
    ("data07.psarc", 20_274_064),
    ("data08.psarc", 5_664_332_014),
    ("data09.psarc", 7_173_845),
    ("eboot.bin", 11_004_033),
    ("sce_discmap.plt", 3_288_792),
    ("sce_discmap_patch.plt", 3_288_792),
    ("sce_module/libSceFios2.prx", 446_777),
    ("sce_module/libSceNpToolkit2.prx", 693_254),
    ("sce_module/libc.prx", 1_069_730),
    ("sce_sys/about/right.sprx", 16_025),
];

fn folder(patch: bool) -> PathBuf {
    let name = if patch { "omega-eu-patch" } else { "omega-eu" };
    oag_testdata::repo_root()
        .join("data/extracted/ps4")
        .join(name)
        .join("uroot")
}

fn compare(patch: bool, path: &str) {
    let image = if patch { PATCH } else { BASE };
    let Some(pkg_path) = oag_testdata::exact(image) else {
        return;
    };
    let mut pkg = Ps4Pkg::open(&pkg_path).expect("package opens");
    let index = pkg
        .files()
        .iter()
        .position(|f| f.path == path)
        .unwrap_or_else(|| panic!("{path} is not in {image}"));
    let size = pkg.files()[index].size;
    let mut reference = File::open(folder(patch).join(path)).expect("extracted file");
    assert_eq!(reference.metadata().unwrap().len(), size, "{path}: size");

    let full = std::env::var_os("OAG_PS4_PKG_FULL").is_some() || size <= WHOLE_LIMIT;
    let mut windows: Vec<(u64, u64)> = Vec::new();
    if full {
        let mut at = 0;
        while at < size {
            windows.push((at, (8 << 20).min(size - at)));
            at += 8 << 20;
        }
    } else {
        windows.push((0, WINDOW));
        windows.push((size - WINDOW.min(size), WINDOW.min(size)));
        for i in 1..=48u64 {
            let at = (size / 49 * i) & !0xFFFF;
            windows.push((at.saturating_sub(100), WINDOW));
        }
    }
    for (at, len) in windows {
        let len = len.min(size - at);
        let got = pkg.read(index, at, len).expect("read");
        let mut want = vec![0u8; len as usize];
        reference.seek(SeekFrom::Start(at)).unwrap();
        reference.read_exact(&mut want).unwrap();
        assert!(got == want, "{path}: bytes differ in {at}..{}", at + len);
    }
}

#[test]
#[ignore = "needs data/images/omega-ps4-eu*.pkg and data/extracted/ps4"]
fn file_lists_and_sizes_match_the_extracted_folders() {
    for (image, expected, patch) in [
        (BASE, &BASE_FILES[..], false),
        (PATCH, &PATCH_FILES[..], true),
    ] {
        let Some(path) = oag_testdata::exact(image) else {
            return;
        };
        let pkg = Ps4Pkg::open(&path).expect("package opens");
        let mut got: Vec<(String, u64)> = pkg
            .files()
            .iter()
            .map(|f| (f.path.clone(), f.size))
            .collect();
        let mut want: Vec<(String, u64)> = expected
            .iter()
            .map(|(p, s)| ((*p).to_string(), *s))
            .collect();
        got.sort();
        want.sort();
        assert_eq!(got, want, "{image}");
        let mut on_disk = 0;
        walk(&folder(patch), &mut on_disk);
        assert_eq!(
            on_disk,
            want.len(),
            "{image}: files in the extracted folder"
        );
        assert_eq!(pkg.content_id, "EP9000-CUSA05670_00-WIPEOUTOMEGA00EU");
    }
}

fn walk(dir: &std::path::Path, count: &mut usize) {
    for e in std::fs::read_dir(dir).unwrap().flatten() {
        let p = e.path();
        if p.is_dir() {
            walk(&p, count);
        } else {
            *count += 1;
        }
    }
}

macro_rules! contents {
    ($($name:ident: $patch:expr, $path:expr;)*) => {$(
        #[test]
        #[ignore = "needs data/images/omega-ps4-eu*.pkg and data/extracted/ps4"]
        fn $name() {
            compare($patch, $path);
        }
    )*};
}

contents! {
    base_data00: false, "data00.psarc";
    base_data01: false, "data01.psarc";
    base_data02: false, "data02.psarc";
    base_data03: false, "data03.psarc";
    base_data04: false, "data04.psarc";
    base_eboot: false, "eboot.bin";
    base_discmap: false, "sce_discmap.plt";
    base_session_image: false, "session_image.jpg";
    base_fios2: false, "sce_module/libSceFios2.prx";
    base_toolkit: false, "sce_module/libSceNpToolkit2.prx";
    base_libc: false, "sce_module/libc.prx";
    base_keystone: false, "sce_sys/keystone";
    base_right_sprx: false, "sce_sys/about/right.sprx";
    base_keymap: false, "sce_sys/keymap_rp/000.png";
    patch_data05: true, "data05.psarc";
    patch_data07: true, "data07.psarc";
    patch_data08: true, "data08.psarc";
    patch_data09: true, "data09.psarc";
    patch_eboot: true, "eboot.bin";
    patch_discmap: true, "sce_discmap.plt";
    patch_discmap_patch: true, "sce_discmap_patch.plt";
    patch_fios2: true, "sce_module/libSceFios2.prx";
    patch_toolkit: true, "sce_module/libSceNpToolkit2.prx";
    patch_libc: true, "sce_module/libc.prx";
    patch_right_sprx: true, "sce_sys/about/right.sprx";
}

/// `DiscImage` opens either package of the pair as the same one tree, the base
/// first, identified as a PS4 title, with the paths the extracted folders have.
#[test]
#[ignore = "needs data/images/omega-ps4-eu*.pkg"]
fn disc_image_opens_the_pair_as_one_ps4_title() {
    for name in [BASE, PATCH] {
        let Some(path) = oag_testdata::exact(name) else {
            return;
        };
        let mut image = oag_disc::DiscImage::open(&path).expect("opens");
        let info = image.identify().expect("identifies");
        assert_eq!(info.platform, oag_disc::Platform::Ps4, "{name}");
        assert!(info.serial.is_some_and(|s| s.contains("5670")), "{name}");
        let entries = image.entries().expect("entries").to_vec();
        assert_eq!(entries.len(), 25, "{name}: files in both packages");
        assert!(
            entries
                .iter()
                .any(|e| e.path == "omega-ps4-eu-patch/uroot/data09.psarc"),
            "{name}"
        );
        assert!(entries[0].path.starts_with("omega-ps4-eu/uroot/"), "{name}");
    }
}

/// A fake-package category is read from the plaintext entry, and a Vita `.pkg`
/// is not mistaken for one.
#[test]
#[ignore = "needs data/images/omega-ps4-eu*.pkg"]
fn category_and_siblings_come_from_the_plaintext_param_sfo() {
    let (Some(base), Some(patch)) = (oag_testdata::exact(BASE), oag_testdata::exact(PATCH)) else {
        return;
    };
    assert_eq!(oag_disc::ps4_pkg::category(&base).as_deref(), Some("gd"));
    assert_eq!(oag_disc::ps4_pkg::category(&patch).as_deref(), Some("gp"));
    assert_eq!(
        oag_disc::ps4_pkg::set::siblings(&patch),
        vec![base.clone(), patch.clone()]
    );
    if let Some(vita) = oag_testdata::exact("2048-vita-eu.pkg") {
        assert_eq!(oag_disc::ps4_pkg::category(&vita), None);
    }
}
