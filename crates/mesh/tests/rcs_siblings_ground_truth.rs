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
use oag_mesh::mesh::rcs::siblings::{Siblings, Spelling};

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

/// What one title's `.vex` entries resolve to.
#[derive(Debug, PartialEq, Eq)]
struct Counts {
    vex: usize,
    with_model: usize,
    skeletons: usize,
    clips: usize,
    pvs: usize,
    /// Zone-race answers that are a `trackZone` model.
    zone_models: usize,
    /// Directories holding both a plain and a `.final` model.
    both_spellings: usize,
}

/// Resolves every `.vex` in a Zone and a non-Zone race, as the loader does:
/// the first candidate whose model the title holds, then the companions beside
/// that model.
fn census(title: &Title) -> Counts {
    let vexes = title.vex_names();
    let mut counts = Counts {
        vex: vexes.len(),
        with_model: 0,
        skeletons: 0,
        clips: 0,
        pvs: 0,
        zone_models: 0,
        both_spellings: 0,
    };
    for vex in &vexes {
        let held =
            |spelling| Siblings::spelled(vex, spelling).is_some_and(|set| title.exists(&set.model));
        counts.both_spellings += usize::from(held(Spelling::Plain) && held(Spelling::Cooked));
        for zone in [false, true] {
            let found = Siblings::probe(vex, zone)
                .into_iter()
                .find(|set| title.exists(&set.model));
            let Some(set) = found else { continue };
            assert_eq!(
                Siblings::beside(&set.model).as_ref(),
                Some(&set),
                "{vex}: the companions follow the model found"
            );
            if zone {
                counts.zone_models += usize::from(set.spelling == Spelling::Zone);
            } else {
                counts.with_model += 1;
                counts.skeletons += usize::from(title.exists(&set.skeleton));
                counts.clips += usize::from(title.exists(&set.clip));
                counts.pvs += usize::from(title.exists(&set.pvs));
            }
        }
    }
    counts
}

/// What each title's `.vex` entries resolve to, pinned: how many have a model,
/// and how many of those name a skeleton, clip and PVS the title holds
/// (`docs/formats/rcsmodel-siblings.md`, "Census"). Dropping the `.final` or
/// `trackZone` candidate, or the stem rule, moves these counts.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn every_hd_vex_resolves_to_its_pinned_companions() {
    let Some(title) = hd() else { return };
    assert_eq!(
        census(&title),
        Counts {
            vex: 721,
            with_model: 623,
            skeletons: 0,
            clips: 0,
            pvs: 28,
            zone_models: 0,
            both_spellings: 0
        }
    );
}

#[test]
#[ignore = "needs data/extracted/vita"]
fn every_2048_vex_resolves_to_its_pinned_companions() {
    let Some(title) = vita() else { return };
    assert_eq!(
        census(&title),
        Counts {
            vex: 1182,
            with_model: 974,
            skeletons: 58,
            clips: 58,
            pvs: 38,
            zone_models: 40,
            both_spellings: 0
        }
    );
}

#[test]
#[ignore = "needs data/extracted/ps4"]
fn every_omega_vex_resolves_to_its_pinned_companions() {
    let Some(title) = omega() else { return };
    assert_eq!(
        census(&title),
        Counts {
            vex: 1385,
            with_model: 1183,
            skeletons: 58,
            clips: 58,
            pvs: 34,
            zone_models: 50,
            both_spellings: 0
        }
    );
}
