//! What the disc's own startup manifests contain.
//!
//! **The claim worth pinning is that everything they name is there.** 118
//! `<Billboard>` entries over 16 circuits, 18 distinct advert models, and each
//! model present in an archive - so the reason this project draws no hoardings
//! is that nothing places them, not that anything is missing.
//!
//! `#[ignore]`d because it needs `data/images/hdfury-ps3-eu-dec.iso`; run with
//! `just test-data`.

mod rcsmodel_common;

use std::collections::{BTreeMap, BTreeSet};

use oag_formats::trackstartup::TrackStartup;
use rcsmodel_common::image;

const ARCHIVES: &[&str] = &[
    "DATA00", "DATA01", "DATA02", "DATA03", "DATA04", "DATA05", "DATA06",
];

struct Disc {
    /// Every entry path on the disc, lowercased.
    entries: BTreeSet<String>,
    /// Circuit path -> its manifest.
    manifests: BTreeMap<String, TrackStartup>,
}

fn sweep() -> Disc {
    let image = image().expect("checked by the caller");
    let mut entries = BTreeSet::new();
    let mut manifests = BTreeMap::new();
    for archive in ARCHIVES {
        let spec = format!("{}:PS3_GAME/USRDIR/{archive}.PSARC", image.display());
        let mut open = oag_assets::psarc::Archive::open(&spec).expect("the archive opens");
        for path in open.paths() {
            entries.insert(path.to_lowercase());
        }
        let found: Vec<String> = open
            .paths()
            .iter()
            .filter(|p| p.to_lowercase().ends_with("trackstartup.xml"))
            .cloned()
            .collect();
        for path in found {
            let Ok(blob) = open.read_path(&path) else {
                continue;
            };
            manifests.insert(path, TrackStartup::parse(&String::from_utf8_lossy(&blob)));
        }
    }
    Disc { entries, manifests }
}

/// **Every model a manifest names is on the disc.**
///
/// The measurement that makes the hoardings a placement problem rather than a
/// missing-asset one.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn every_billboard_a_circuit_names_is_present() {
    if image().is_none() {
        return;
    }
    let disc = sweep();
    assert_eq!(disc.manifests.len(), 16, "one manifest per circuit");

    let (mut slots, mut models, mut colours) = (0usize, 0usize, 0usize);
    let mut distinct = BTreeSet::new();
    let mut colour_names = BTreeSet::new();
    let mut missing = Vec::new();
    for (path, manifest) in &disc.manifests {
        for billboard in &manifest.billboards {
            slots += 1;
            let Some(location) = billboard.location() else {
                colours += 1;
                if let oag_formats::trackstartup::Fill::Colour(name) = &billboard.fill {
                    colour_names.insert(name.clone());
                }
                continue;
            };
            models += 1;
            let want = location.to_lowercase();
            distinct.insert(want.clone());
            if !disc.entries.contains(&want) {
                missing.push(format!("{path} num {}: {want}", billboard.num));
            }
            // A model with no geometry beside it would be a placement target
            // with nothing to place.
            let sibling = want.replace(".vex", ".rcsmodel");
            if !disc.entries.contains(&sibling) {
                missing.push(format!("{path} num {}: {sibling}", billboard.num));
            }
        }
    }
    println!("{slots} slot(s): {models} model(s), {colours} colour(s) {colour_names:?}");
    assert_eq!((slots, models, colours), (118, 104, 14));
    assert_eq!(distinct.len(), 18);
    assert!(missing.is_empty(), "not on the disc: {missing:#?}");
}

/// **`num` is 1 to 8 and unique per file, and `type` takes two values.**
///
/// The file's own comment claims the first. The second is why `Billboard::kind`
/// is a string: a first pass over the disc saw only `landscape` - because it
/// only looked at entries carrying a `location` - and `portrait` turned up the
/// moment the colour slots were read too.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn the_slot_numbers_are_unique_and_every_entry_is_landscape() {
    if image().is_none() {
        return;
    }
    let mut kinds: BTreeMap<String, usize> = BTreeMap::new();
    for (path, manifest) in &sweep().manifests {
        let mut seen = BTreeSet::new();
        for billboard in &manifest.billboards {
            assert!(
                (1..=8).contains(&billboard.num),
                "{path}: num {} is outside 1..=8",
                billboard.num,
            );
            assert!(
                seen.insert(billboard.num),
                "{path}: num {} appears twice",
                billboard.num,
            );
            *kinds.entry(billboard.kind.clone()).or_default() += 1;
        }
        assert!(
            manifest.sound_bank.is_some(),
            "{path}: every circuit names a sound bank",
        );
    }
    println!("type: {kinds:?}");
    assert_eq!(
        kinds.keys().cloned().collect::<BTreeSet<_>>(),
        BTreeSet::from(["landscape".to_string(), "portrait".to_string()]),
    );
}
