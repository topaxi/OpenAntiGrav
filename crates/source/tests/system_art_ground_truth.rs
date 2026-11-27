//! The system art each title's own files carry, read the way the launcher
//! reads it (`docs/ui/launcher.md`, "Where the art comes from").
//!
//! Sizes only: no pixel of a disc is asserted or kept here.
//!
//! `#[ignore]`d: needs the images under `data/`. `just test-data`.

use std::path::{Path, PathBuf};

use oag_source::art;

/// `(name, width, height, logo size)` of what `source` offers.
type Found = Vec<(&'static str, u32, u32, Option<(u32, u32)>)>;

fn offered(source: &Path, ps4: Vec<PathBuf>) -> Found {
    art::read_with(&source.to_string_lossy(), move || ps4)
        .into_iter()
        .map(|art| {
            assert_eq!(
                art.picture.rgba.len(),
                art.picture.width as usize * art.picture.height as usize * 4
            );
            (
                art.name,
                art.picture.width,
                art.picture.height,
                art.logo.map(|logo| (logo.width, logo.height)),
            )
        })
        .collect()
}

#[test]
#[ignore = "needs a disc image under data/images/"]
fn every_pressing_offers_the_art_its_platform_keeps() {
    let psp: Found = vec![("pic1", 480, 272, None)];
    let ps3: Found = vec![("pic1", 1920, 1080, Some((320, 176)))];
    let expected: [(&str, Found); 6] = [
        ("pulse-psp-eu.chd", psp.clone()),
        ("pulse-psp-usa.chd", psp.clone()),
        ("pure-psp-eu.chd", psp.clone()),
        ("pure-psp-usa.chd", psp),
        ("hdfury-ps3-eu-dec.iso", ps3.clone()),
        // A PS2 disc has no system art.
        ("pulse-ps2-eu.chd", Vec::new()),
    ];
    for (name, wanted) in expected {
        let Some(path) = oag_testdata::exact(name) else {
            continue;
        };
        assert_eq!(offered(&path, Vec::new()), wanted, "{name}");
    }
}

#[test]
#[ignore = "needs an installed PSN folder under data/extracted/ps3/"]
fn an_installed_psn_folder_offers_its_picture_and_logo() {
    let Some(root) = oag_testdata::exact("data/extracted/ps3/hd-psn-eu") else {
        return;
    };
    assert_eq!(
        offered(&root, Vec::new()),
        vec![("pic1", 1920, 1080, Some((320, 176)))]
    );
}

/// 2048's `pic0.png` is a palette PNG, which the decoder has to expand.
#[test]
#[ignore = "needs an extracted Vita package under data/extracted/vita/"]
fn a_vita_folder_offers_its_pic0() {
    for name in [
        "data/extracted/vita/PCSF00007",
        "data/extracted/vita/PCSA00015",
    ] {
        let Some(root) = oag_testdata::exact(name) else {
            continue;
        };
        assert_eq!(
            offered(&root, Vec::new()),
            vec![("pic0", 960, 544, None)],
            "{name}"
        );
    }
}

/// The packed form of the same package.
#[test]
#[ignore = "needs data/test-vpk/ (scripts/make-test-vpk.sh)"]
fn a_vpk_offers_its_pic0() {
    let Some(vpk) = oag_testdata::exact("data/test-vpk/2048-PCSF00007.vpk") else {
        return;
    };
    assert_eq!(offered(&vpk, Vec::new()), vec![("pic0", 960, 544, None)]);
}

#[test]
#[ignore = "needs the Omega packages under data/images/"]
fn the_omega_package_and_its_unpacked_folder_offer_both_pictures() {
    let both: Found = vec![("pic0", 1920, 1080, None), ("pic1", 1920, 1080, None)];
    let Some(package) = oag_testdata::exact("omega-ps4-eu.pkg") else {
        return;
    };
    assert_eq!(offered(&package, Vec::new()), both, "the package");
    let Some(folder) = oag_testdata::exact("data/extracted/ps4") else {
        return;
    };
    assert_eq!(
        offered(&folder, vec![package]),
        both,
        "the folder, art out of the package"
    );
    // With no package left, the folder has no art and the card shows the name.
    assert!(offered(&folder, Vec::new()).is_empty());
}
