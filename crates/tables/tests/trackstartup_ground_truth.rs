//! What the disc's own startup manifests contain.
//!
//! **The claim worth pinning is that everything they name is there.** 118
//! `<Billboard>` entries over 16 circuits, 18 distinct advert models, and each
//! model present in an archive - so the reason this project draws no hoardings
//! is that nothing places them, not that anything is missing.
//!
//! `#[ignore]`d because it needs `data/images/hdfury-ps3-eu-dec.iso`; run with
//! `just test-data`.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use oag_tables::trackstartup::TrackStartup;

/// The decrypted HD/Fury image, if it is there.
///
/// A local copy rather than a shared helper: `rcsmodel_common` lives with the
/// `RCSMODEL` tests in `oag-rcs`, and a manifest is not a model. Every other
/// disc-backed test in this crate carries its own locator for the same reason.
fn image() -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/images/hdfury-ps3-eu-dec.iso");

    if path.exists() {
        return Some(path);
    }
    assert!(
        std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
        "OAG_REQUIRE_GAME_DATA is set but {} is missing",
        path.display()
    );
    println!("skipping: {} not present", path.display());
    None
}

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
                if let oag_tables::trackstartup::Fill::Colour(name) = &billboard.fill {
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

/// **Slots 7 and 8 are reserved, and slot 8 is the start/finish line.**
///
/// The structure that turns `num` from an opaque index into something with
/// meaning. Over all 16 circuits:
///
/// - **slot 8 is `321Go_StartFinish.vex` on every one of them**, and that model
///   appears in no other slot;
/// - **slot 7 is `fx350.vex` on every one**, likewise exclusive;
/// - slots 1 to 6 vary - nine circuits share one default six, the other seven
///   customise, and four of those use colour slots instead of models.
///
/// **A player's observation is what prompted the check**, and it agrees:
/// driving HD, slot 8's hoarding is consistently at the start/finish line while
/// the others are scattered through the circuit. The model's own name says the
/// same thing, and the two together are much stronger than either alone -
/// neither is geometry this project has recovered, and where a slot's transform
/// comes from is still unknown.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn slots_seven_and_eight_carry_the_same_model_on_every_circuit() {
    if image().is_none() {
        return;
    }
    let disc = sweep();
    let leaf = |num: u32, m: &TrackStartup| -> Option<String> {
        Some(
            m.billboard(num)?
                .location()?
                .rsplit('/')
                .next()?
                .to_ascii_lowercase(),
        )
    };
    for (num, want) in [(7u32, "fx350.vex"), (8, "321go_startfinish.vex")] {
        for (path, manifest) in &disc.manifests {
            assert_eq!(
                leaf(num, manifest).as_deref(),
                Some(want),
                "{path}: slot {num}",
            );
        }
        // And exclusively: the reserved models never appear anywhere else.
        for (path, manifest) in &disc.manifests {
            for billboard in &manifest.billboards {
                if billboard.num == num {
                    continue;
                }
                let Some(location) = billboard.location() else {
                    continue;
                };
                assert!(
                    !location.to_ascii_lowercase().ends_with(want),
                    "{path}: slot {} also carries {want}",
                    billboard.num,
                );
            }
        }
    }
}
