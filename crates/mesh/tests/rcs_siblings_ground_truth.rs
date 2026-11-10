//! The files beside a `.vex`, resolved by `mesh::rcs::siblings`, against every
//! `.vex` HD, 2048 and the Omega Collection ship.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content; run with `just test-data`.
//!
//! Reads table-of-contents names only, no blob: existence is each archive's own
//! `contains` (the question `oag_assets::Archives` asks to pick the archive that
//! serves a read), over the title's archives together.

use std::path::PathBuf;

use oag_assets::psarc::Archive;
use oag_mesh::mesh::rcs::siblings::Siblings;

/// One title's archives, opened by table of contents.
struct Title {
    archives: Vec<Archive>,
}

impl Title {
    fn open(specs: &[String]) -> Option<Self> {
        let archives: Vec<Archive> = specs.iter().filter_map(|s| Archive::open(s).ok()).collect();
        if archives.len() != specs.len() {
            assert!(
                std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
                "OAG_REQUIRE_GAME_DATA is set but only {} of {} archives open: {specs:?}",
                archives.len(),
                specs.len()
            );
            println!(
                "skipping: {} of {} archives open",
                archives.len(),
                specs.len()
            );
            return None;
        }
        Some(Self { archives })
    }

    fn exists(&self, name: &str) -> bool {
        self.archives.iter().any(|a| a.contains(name))
    }

    /// Every `.vex` entry, sorted and de-duplicated.
    fn vex_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self
            .archives
            .iter()
            .flat_map(|a| a.paths().iter().cloned())
            .filter(|p| p.to_ascii_lowercase().ends_with(".vex"))
            .collect();
        names.sort();
        names.dedup();
        names
    }
}

fn extracted(relative: &[&str]) -> Option<Vec<String>> {
    relative
        .iter()
        .map(|r| oag_testdata::exact(&format!("data/extracted/{r}")))
        .map(|p| p.map(|p: PathBuf| p.display().to_string()))
        .collect()
}

fn hd() -> Option<Title> {
    let image = oag_testdata::image("hdfury-ps3-eu-dec.iso")?;
    let specs: Vec<String> = (0..=6)
        .map(|n| format!("{}:PS3_GAME/USRDIR/DATA0{n}.PSARC", image.display()))
        .collect();
    Title::open(&specs)
}

fn vita() -> Option<Title> {
    Title::open(&extracted(&[
        "vita/PCSF00007/base/PSP2/data.psarc",
        "vita/PCSF00007/patch-v104/PSP2/data1.psarc",
        "vita/PCSF00007/patch-v104/PSP2/data2.psarc",
        "vita/PCSF00007/dlc1/PSP2/dlc1.psarc",
        "vita/PCSF00007/dlc2/PSP2/dlc2.psarc",
    ])?)
}

fn omega() -> Option<Title> {
    Title::open(&extracted(&[
        "ps4/omega-eu-patch/uroot/data08.psarc",
        "ps4/omega-eu-patch/uroot/data09.psarc",
        "ps4/omega-eu-patch/uroot/data07.psarc",
        "ps4/omega-eu-patch/uroot/data05.psarc",
        "ps4/omega-eu/uroot/data00.psarc",
        "ps4/omega-eu/uroot/data01.psarc",
        "ps4/omega-eu/uroot/data02.psarc",
        "ps4/omega-eu/uroot/data03.psarc",
        "ps4/omega-eu/uroot/data04.psarc",
    ])?)
}

/// What the scattered rules answered before the resolver: `sibling_name`, then
/// `sibling_name_cooked`, with `zone_model_name` first in a Zone race;
/// `animation_names_beside` and the PVS rewrite on the model found, or on the
/// `.vex` when none is.
fn before(vex: &str, zone: bool, exists: &dyn Fn(&str) -> bool) -> [String; 4] {
    use oag_mesh::mesh::rcs;
    let zone_model = zone
        .then(|| rcs::psp2::zone_model_name(vex))
        .flatten()
        .filter(|name| exists(name));
    let found = zone_model.or_else(|| {
        [rcs::sibling_name(vex), rcs::sibling_name_cooked(vex)]
            .into_iter()
            .flatten()
            .find(|name| exists(name))
    });
    let pvs = |model: &str| format!("{}.pvs", &model[..model.len() - ".rcsmodel".len()]);
    match found {
        Some(model) => {
            let (skeleton, clip) = rcs::psp2::animation_names_beside(&model).unwrap();
            let pvs = pvs(&model);
            [model, skeleton, clip, pvs]
        }
        None => {
            let (skeleton, clip) = rcs::psp2::animation_names(vex).unwrap_or_default();
            let model = rcs::sibling_name(vex).unwrap_or_default();
            [String::new(), skeleton, clip, pvs(&model)]
        }
    }
}

/// What the resolver answers for the same question.
fn after(vex: &str, zone: bool, exists: &dyn Fn(&str) -> bool) -> [String; 4] {
    let found = Siblings::probe(vex, zone)
        .into_iter()
        .find(|set| exists(&set.model));
    match found {
        Some(s) => [s.model, s.skeleton, s.clip, s.pvs],
        None => {
            let s = Siblings::plain(vex).expect("a .vex name");
            [String::new(), s.skeleton, s.clip, s.pvs]
        }
    }
}

fn census(name: &str, title: &Title) {
    let vexes = title.vex_names();
    assert!(!vexes.is_empty(), "{name}: no .vex entries");
    let exists = |n: &str| title.exists(n);
    let (mut found, mut zone_found, mut skeletons, mut clips, mut pvs) = (0, 0, 0, 0, 0);
    for vex in &vexes {
        for zone in [false, true] {
            let old = before(vex, zone, &exists);
            let new = after(vex, zone, &exists);
            assert_eq!(old, new, "{name}: {vex} (zone {zone})");
            if !zone && !new[0].is_empty() {
                found += 1;
                skeletons += usize::from(exists(&new[1]));
                clips += usize::from(exists(&new[2]));
                pvs += usize::from(exists(&new[3]));
            }
            zone_found += usize::from(zone && new[0].contains("trackZone"));
        }
    }
    println!(
        "{name}: {} .vex, {found} with a model, {skeletons} skeleton, {clips} clip, \
         {pvs} pvs, {zone_found} Zone models",
        vexes.len()
    );
}

#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn hd_resolves_as_the_scattered_rules_did() {
    if let Some(title) = hd() {
        census("HD", &title);
    }
}

#[test]
#[ignore = "needs data/extracted/vita"]
fn wipeout_2048_resolves_as_the_scattered_rules_did() {
    if let Some(title) = vita() {
        census("2048", &title);
    }
}

#[test]
#[ignore = "needs data/extracted/ps4"]
fn omega_resolves_as_the_scattered_rules_did() {
    if let Some(title) = omega() {
        census("Omega", &title);
    }
}
