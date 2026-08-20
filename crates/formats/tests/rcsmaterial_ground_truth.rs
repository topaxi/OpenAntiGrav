//! What the disc says about a `.rcsmaterial`'s variant table.
//!
//! The unit tests beside [`oag_formats::rcsmaterial`] run on a synthetic table,
//! which proves the framing arithmetic and nothing about the disc. These are the
//! other half: every material Wipeout HD ships, parsed, with the counts
//! asserted rather than described.
//!
//! **The counts here are larger than the ones
//! [`rcsmaterial.md`](../../../docs/formats/rcsmaterial.md) quotes**, and the
//! difference is the corpus, not the reading: the survey that recovered the key
//! swept the 693 materials of `DATA00.PSARC`, while this walks all seven
//! archives and finds 1,632. Every structural claim holds at the larger scale -
//! no unknown class, no repeated key, no disagreeing content hash - which is
//! the more useful result, so the numbers below are the disc's own.
//!
//! `#[ignore]`d because it needs `data/images/hdfury-ps3-eu-dec.iso`; run with
//! `just test-data`.

mod rcsmodel_common;

use std::collections::{BTreeMap, BTreeSet};

use oag_formats::rcsmaterial::{Class, Features, RcsMaterial};
use rcsmodel_common::image;

/// Every archive on the disc that holds `.rcsmaterial` files.
const ARCHIVES: &[&str] = &[
    "DATA00", "DATA01", "DATA02", "DATA03", "DATA04", "DATA05", "DATA06",
];

/// Every `.rcsmaterial` on the disc, as `(path, bytes)`.
fn every_material() -> Vec<(String, Vec<u8>)> {
    let image = image().expect("checked by the caller");
    let mut out = Vec::new();
    for archive in ARCHIVES {
        let spec = format!("{}:PS3_GAME/USRDIR/{archive}.PSARC", image.display());
        let Ok(mut open) = oag_assets::psarc::Archive::open(&spec) else {
            continue;
        };
        let paths: Vec<String> = open
            .paths()
            .iter()
            .filter(|p| p.ends_with(".rcsmaterial"))
            .cloned()
            .collect();
        for path in paths {
            if let Ok(bytes) = open.read_path(&path) {
                out.push((path, bytes));
            }
        }
    }
    out
}

/// The whole variant table reads, and the class census is the one the reading
/// was built on.
///
/// The four counts are the load-bearing numbers: if a fifth class appeared, or
/// the distribution moved, the `[0]` word would not be what this reads it as.
#[test]
#[ignore]
fn every_material_on_the_disc_parses_and_its_classes_are_the_four_known_ones() {
    let Some(_) = image() else {
        return;
    };
    let materials = every_material();
    assert!(
        materials.len() >= 1_632,
        "only {} .rcsmaterial found; the corpus narrowed",
        materials.len()
    );

    let (mut variants, mut unknown) = (0usize, 0usize);
    let mut per_class: BTreeMap<&'static str, usize> = BTreeMap::new();
    for (path, bytes) in &materials {
        let parsed =
            RcsMaterial::parse(bytes).unwrap_or_else(|e| panic!("{path} does not parse: {e}"));
        variants += parsed.variants.len();
        for v in &parsed.variants {
            match v.class {
                Some(c) => *per_class.entry(c.name()).or_default() += 1,
                None => {
                    unknown += 1;
                    println!("  {path}: unknown class hash {:#010x}", v.class_hash);
                }
            }
        }
    }
    println!("{} material(s), {variants} variant(s)", materials.len());
    println!("  by class: {per_class:?}");

    assert_eq!(
        unknown, 0,
        "a class hash outside the four the reading names"
    );
    assert!(
        variants >= 76_358,
        "only {variants} variants; the table framing or the corpus narrowed"
    );
    assert_eq!(per_class.get("Static").copied(), Some(43_370));
    assert_eq!(per_class.get("StaticQuake").copied(), Some(19_257));
    assert_eq!(per_class.get("RigidBody").copied(), Some(13_724));
    assert_eq!(
        per_class.get("StaticUncompressed").copied(),
        Some(7),
        "the few occurrences the weakest of the four classes rests on"
    );
}

/// `(class, features)` is a key, not a filter.
///
/// If two records in one file shared a pair, selecting by it would be ambiguous
/// and the whole mechanism would be a filter that happens to resolve. Zero
/// collisions across all 76,358 is what makes it a key.
#[test]
#[ignore]
fn the_class_and_feature_pair_is_unique_within_every_file() {
    let Some(_) = image() else {
        return;
    };
    let mut collisions = 0usize;
    let mut checked = 0usize;
    for (path, bytes) in &every_material() {
        let Ok(parsed) = RcsMaterial::parse(bytes) else {
            continue;
        };
        let mut seen: BTreeSet<(u32, u32)> = BTreeSet::new();
        for v in &parsed.variants {
            checked += 1;
            if !seen.insert((v.class_hash, v.feature_hash)) {
                collisions += 1;
                println!(
                    "  {path}: repeated key {:#010x}/{:#010x}",
                    v.class_hash, v.feature_hash
                );
            }
        }
    }
    println!("{checked} variant(s) checked");
    assert_eq!(collisions, 0, "the pair does not identify a single variant");
}

/// A program content hash is constant across every variant that shares the
/// program, which is why variants share blocks rather than duplicating them.
#[test]
#[ignore]
fn a_shared_program_offset_carries_a_shared_content_hash() {
    let Some(_) = image() else {
        return;
    };
    let (mut checked, mut disagreed) = (0usize, 0usize);
    for (path, bytes) in &every_material() {
        let Ok(parsed) = RcsMaterial::parse(bytes) else {
            continue;
        };
        let mut seen: BTreeMap<(bool, usize), u32> = BTreeMap::new();
        for v in &parsed.variants {
            for (is_vertex, block) in [(true, v.vertex), (false, v.fragment)] {
                checked += 1;
                match seen.insert((is_vertex, block.offset), block.program_hash) {
                    Some(prior) if prior != block.program_hash => {
                        disagreed += 1;
                        println!(
                            "  {path}: block +{:#x} carries {prior:#010x} and {:#010x}",
                            block.offset, block.program_hash
                        );
                    }
                    _ => {}
                }
            }
        }
    }
    println!("{checked} block reference(s) checked");
    assert_eq!(
        disagreed, 0,
        "two variants name the same program offset with different content hashes, \
         so [8]/[9] are not what this reads them as"
    );
}

/// **The half of the key a chunk decides selects a variant that exists.**
///
/// This is what a renderer would actually do: take a chunk's declaration, build
/// the chunk-determined tokens, add the ones an ordinary lit pass sets, and look
/// the record up. It is asserted as a *majority* rather than as every chunk,
/// because the pass-determined half is confidence 75 and this test would
/// otherwise be asserting that guess - see
/// `docs/formats/rcsmaterial.md`, "Which half the chunk decides".
#[test]
#[ignore]
fn a_chunk_determined_key_selects_a_variant_that_exists() {
    let Some(image) = image() else {
        return;
    };
    let spec = format!("{}:PS3_GAME/USRDIR/DATA00.PSARC", image.display());
    let Ok(mut archive) = oag_assets::psarc::Archive::open(&spec) else {
        return;
    };
    let model_path = "/data/environments/talons_junction/track.rcsmodel";
    let Ok(model_bytes) = archive.read_path(model_path) else {
        return;
    };
    let model = oag_formats::rcsmodel::Model::parse(&model_bytes).expect("the model parses");

    // What an ordinary lit race pass adds beyond the chunk's own half. Every
    // token here is pass-determined and unread, so a miss below is expected
    // rather than a failure - the assertion is on the rate.
    let pass = Features::token("HalfBright")
        .with(Features::token("Sun"))
        .with(Features::token("Spot0"));

    let (mut tried, mut hit) = (0usize, 0usize);
    let mut misses: BTreeMap<String, usize> = BTreeMap::new();
    for mesh in &model.meshes {
        let Some(material) = model.material_of(mesh) else {
            continue;
        };
        let Ok(bytes) = archive.read_path(&format!("/{}", material.name)) else {
            continue;
        };
        let Ok(parsed) = RcsMaterial::parse(&bytes) else {
            continue;
        };
        let key = Features::for_chunk(mesh.decl.as_ref()).with(pass);
        tried += 1;
        if parsed.variant(Class::Static, key).is_some() {
            hit += 1;
        } else {
            *misses.entry(key.name()).or_default() += 1;
        }
    }
    println!("{hit} of {tried} chunk(s) resolved to a shipped variant");
    for (name, n) in &misses {
        println!("  missed {n:4}x {name}");
    }
    assert!(tried > 500, "only {tried} chunks had a readable material");
    assert!(
        hit * 2 > tried,
        "only {hit} of {tried} chunks resolved, so the chunk-determined half of \
         the key is not what this reads it as"
    );
}

/// **Every chunk sharing a material slot needs the same variant.**
///
/// This is an architectural invariant, not a curiosity. `oag-render` binds one
/// texture bind group per *material slot*, not per draw, so a per-material
/// variant selector can ride in that group only if a slot never has to be two
/// things at once. The chunk-to-material map is many-to-one, so it could.
///
/// Measured over all 123 `.rcsmodel` of `DATA00.PSARC` and their 3,566 slots in
/// use: it never does. If this ever fails, the selector has to move from the
/// bind group to the draw call.
#[test]
#[ignore]
fn a_material_slot_never_needs_two_different_variants() {
    let Some(image) = image() else {
        return;
    };
    let spec = format!("{}:PS3_GAME/USRDIR/DATA00.PSARC", image.display());
    let Ok(mut archive) = oag_assets::psarc::Archive::open(&spec) else {
        return;
    };
    let paths: Vec<String> = archive
        .paths()
        .iter()
        .filter(|p| p.ends_with(".rcsmodel"))
        .cloned()
        .collect();

    let (mut models, mut slots, mut split) = (0usize, 0usize, 0usize);
    for path in paths {
        let Ok(bytes) = archive.read_path(&path) else {
            continue;
        };
        let Ok(model) = oag_formats::rcsmodel::Model::parse(&bytes) else {
            continue;
        };
        models += 1;
        let mut per_slot: BTreeMap<u32, BTreeSet<String>> = BTreeMap::new();
        for mesh in &model.meshes {
            per_slot
                .entry(mesh.material)
                .or_default()
                .insert(Features::for_chunk(mesh.decl.as_ref()).name());
        }
        for (slot, keys) in &per_slot {
            slots += 1;
            if keys.len() > 1 {
                split += 1;
                println!("  {path} slot {slot}: {keys:?}");
            }
        }
    }
    println!("{models} model(s), {slots} material slot(s) in use");
    assert!(models >= 123, "only {models} models; the corpus narrowed");
    assert!(slots >= 3_566, "only {slots} slots; the corpus narrowed");
    assert_eq!(
        split, 0,
        "a material slot serves chunks needing different variants, so a \
         per-slot selector is not enough and it has to move to the draw call"
    );
}
