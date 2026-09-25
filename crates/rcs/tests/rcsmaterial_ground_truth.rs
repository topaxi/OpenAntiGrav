//! What the disc says about a `.rcsmaterial`'s variant table.
//!
//! The unit tests beside [`oag_rcs::rcsmaterial`] run on a synthetic table,
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

use oag_rcs::rcsmaterial::{
    Class, Declared, Features, PASS_WORD_BITS, RcsMaterial, fragment, names,
};
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
    let model = oag_rcs::rcsmodel::Model::parse(&model_bytes).expect("the model parses");

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
        let Ok(model) = oag_rcs::rcsmodel::Model::parse(&bytes) else {
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

/// **Every permutation the disc ships is one this reading can build.**
///
/// The closing check on the whole variant key. Sweeping the 4096 permutation
/// words and adding the five standalone names must account for every distinct
/// `[1]` value in every material - if one were left over, the word's layout or
/// the token order would be wrong somewhere this cannot see.
#[test]
#[ignore]
fn every_shipped_permutation_is_one_this_reading_can_build() {
    let Some(_) = image() else {
        return;
    };
    let mut buildable: BTreeMap<u32, String> = BTreeMap::new();
    for word in 0..(1u32 << PASS_WORD_BITS) {
        let f = Features::from_pass_word(word);
        buildable.insert(f.hash(), f.name());
    }
    let from_word = buildable.len();
    for name in [
        "ZAlphaOnly",
        "AmbientShadow",
        "SunOcclusionLightmap",
        "SunOcclusionVertex",
        "Ambient",
    ] {
        let f = Features::token(name);
        buildable.insert(f.hash(), f.name());
    }

    let mut shipped: BTreeSet<u32> = BTreeSet::new();
    for (_, bytes) in &every_material() {
        if let Ok(parsed) = RcsMaterial::parse(bytes) {
            shipped.extend(parsed.variants.iter().map(|v| v.feature_hash));
        }
    }
    let unmatched: Vec<u32> = shipped
        .iter()
        .copied()
        .filter(|h| !buildable.contains_key(h))
        .collect();
    println!(
        "{} distinct permutation(s) shipped; {from_word} buildable from the word, \
         {} with the standalone names",
        shipped.len(),
        buildable.len()
    );
    for h in &unmatched {
        println!("  unmatched {h:#010x}");
    }
    assert!(
        shipped.len() >= 143,
        "only {} distinct permutations; the corpus narrowed",
        shipped.len()
    );
    assert!(
        unmatched.is_empty(),
        "{} shipped permutation(s) this reading cannot build",
        unmatched.len()
    );
}

/// **Every fragment program on the disc decodes, and stops where it says it
/// does.**
///
/// The decoder's own acceptance test. A wrong instruction stride, a
/// mis-detected inline constant or a wrong end bit all show up here as a block
/// that runs past its declared code length or never reaches an `END`, because
/// the stream would desynchronise within a few instructions.
///
/// The port is checked against `scripts/ps3-microcode.py` rather than only
/// against itself. On `DATA00.PSARC`'s 693 materials the two agree on **10,276
/// of 10,276** blocks reaching a clean `END` and on **330** instructions
/// carrying opcode `0x3e`; diffing three real blocks instruction by
/// instruction - `track_surface` #7 and #9, and
/// `detonator_ship_rich_iridescent` #2 - gives identical opcode and destination
/// sequences, differing only where the reference prints its `op3B`/`op3D`
/// placeholders and this leaves them unnamed (true when this test was
/// written; `0x3b`/`0x3d` are named as `DIVSQ`/`FENCT` since 2026-09-25, see
/// below). The numbers below are larger because this walks all seven
/// archives.
#[test]
#[ignore]
fn every_fragment_program_on_the_disc_decodes_to_a_clean_end() {
    let Some(_) = image() else {
        return;
    };
    let (mut blocks, mut clean, mut instructions) = (0usize, 0usize, 0usize);
    let mut unnamed: BTreeMap<u8, usize> = BTreeMap::new();
    let mut fencb = 0usize;
    for (path, bytes) in &every_material() {
        let mut at = 0usize;
        while let Some(i) = bytes[at..].windows(4).position(|w| w == b"SHO\x08") {
            let block = at + i;
            at = block + 4;
            // Fragment blocks only; the vertex ones are a different encoding.
            if bytes.get(block + 4..block + 8) != Some(&1u32.to_be_bytes()[..]) {
                continue;
            }
            blocks += 1;
            match fragment::Program::parse(bytes, block) {
                Some(p) if p.instructions.last().is_some_and(|i| i.end) => {
                    clean += 1;
                    instructions += p.instructions.len();
                    for i in &p.instructions {
                        fencb += usize::from(i.opcode == 0x3e);
                        if i.name().is_none() {
                            *unnamed.entry(i.opcode).or_default() += 1;
                        }
                    }
                }
                _ => println!("  {path}: block at {block:#x} does not reach an END"),
            }
        }
    }
    println!("{blocks} fragment block(s), {clean} clean, {instructions} instruction(s)");
    println!("  opcodes outside the table: {unnamed:?}");

    assert!(
        blocks >= 37_461,
        "only {blocks} blocks; the corpus narrowed"
    );
    assert_eq!(clean, blocks, "a block did not stop where it said it would");
    assert!(instructions > 1_900_000, "only {instructions} instructions");
    // `0x3b`/`0x3d`/`0x3e` were named `DIVSQ`/`FENCT`/`FENCB` 2026-09-25
    // (RPCS3's own `FPOpcodes.h`, corroborated disc-wide - see `docs/formats/
    // rcsmaterial.md` and `crates/render/examples/hd_op3b_op3d_census.rs`), so
    // every opcode in shipped code is now named. One appearing unnamed would
    // mean the corpus grew or the stride is wrong somewhere and garbage is
    // being read as an opcode.
    assert!(
        unnamed.is_empty(),
        "an opcode outside the table: {unnamed:?}"
    );
    assert_eq!(
        fencb, 1_155,
        "330 in DATA00 alone, which is where an independent Python census counted the \
         same number"
    );
}

/// **The dataflow reproduces two blocks that were read by hand.**
///
/// `track_surface.rcsmaterial` block #9 is the lightmapped lit variant and
/// block #8 the lightmap-less one, both quoted instruction by instruction in
/// `docs/ghidra/functions/ps3-hdfury-eu/renderer.md` under "The lit track
/// material". Both take their interpolated per-vertex light from `f[TC1]` and
/// their albedo coordinate from `f[TC4]`, so the lighting rule must report
/// `TC1` and **not** `TC4` - the coordinate reaches the picture but is not
/// combined into it.
///
/// If the taint leaked through texture lookups this would report TC4 as well,
/// and if it did not propagate at all it would report nothing.
#[test]
#[ignore]
fn the_lighting_dataflow_agrees_with_the_blocks_read_by_hand() {
    let Some(image) = image() else {
        return;
    };
    let spec = format!("{}:PS3_GAME/USRDIR/DATA00.PSARC", image.display());
    let Ok(mut archive) = oag_assets::psarc::Archive::open(&spec) else {
        return;
    };
    let path = "/data/environments/talons_junction/materials/track_surface.rcsmaterial";
    let Ok(bytes) = archive.read_path(path) else {
        return;
    };

    // Fragment blocks in file order, which is how the doc numbers them.
    let mut offsets = Vec::new();
    let mut at = 0usize;
    while let Some(i) = bytes[at..].windows(4).position(|w| w == b"SHO\x08") {
        let block = at + i;
        at = block + 4;
        if bytes.get(block + 4..block + 8) == Some(&1u32.to_be_bytes()[..]) {
            offsets.push(block);
        }
    }
    assert!(offsets.len() > 9, "only {} fragment blocks", offsets.len());

    const TC1: u16 = 1 << 5;
    const TC4: u16 = 1 << 8;
    for (index, lightmapped) in [(8usize, false), (9usize, true)] {
        let p = fragment::Program::parse(&bytes, offsets[index])
            .unwrap_or_else(|| panic!("block #{index} decodes"));
        assert_eq!(
            p.declared.samples_lightmap(),
            lightmapped,
            "block #{index} is the wrong variant"
        );
        let lit = p.output_lit_by();
        println!(
            "  block #{index}: lit by {lit:#06x}, reads {:#06x}",
            p.interpolators()
        );
        assert_ne!(
            lit & TC1,
            0,
            "block #{index} takes its per-vertex light from f[TC1] and the \
             dataflow lost it"
        );
        assert_eq!(
            lit & TC4,
            0,
            "block #{index} uses f[TC4] as a texture coordinate, so taint \
             leaked through a lookup"
        );
        assert_ne!(
            p.interpolators() & TC4,
            0,
            "block #{index} does read f[TC4] - the check above is not vacuous"
        );
    }
}

/// Every name in [`names::KNOWN_SAMPLER_NAMES`] is a preimage of a hash a
/// resolved shader program on this disc actually declares - not merely a
/// string that happens to hash to *something*.
///
/// A typo'd or misattributed entry would still round-trip through
/// `names::sampler_name` (the unit test beside the table already checks
/// that), but it would not show up here, because this asks the disc rather
/// than the table.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn every_known_sampler_name_is_a_hash_the_disc_actually_declares() {
    let Some(_image) = image() else { return };
    let mut declared: BTreeSet<u32> = BTreeSet::new();
    for (_, bytes) in every_material() {
        let Ok(mat) = RcsMaterial::parse(&bytes) else {
            continue;
        };
        let mut seen_offsets = BTreeSet::new();
        for v in &mat.variants {
            for block in [&v.vertex, &v.fragment] {
                if !seen_offsets.insert(block.offset) {
                    continue;
                }
                if let Some(decl) = Declared::parse(&bytes, block.offset) {
                    declared.extend(decl.samplers.iter().map(|(h, _)| *h));
                }
            }
        }
    }
    let missing: Vec<&str> = names::KNOWN_SAMPLER_NAMES
        .iter()
        .copied()
        .filter(|name| !declared.contains(&oag_rcs::rcsmaterial::name_hash(name)))
        .collect();
    assert!(
        missing.is_empty(),
        "these names' hashes are not declared by any sampler on the disc: {missing:?}"
    );
}

/// The same check for [`names::KNOWN_PARAMETER_NAMES`], against every
/// `.rcsmodel` material's own authored parameter table rather than a
/// shader's declaration.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn every_known_parameter_name_is_a_hash_the_disc_actually_authors() {
    let Some(image) = image() else { return };
    let mut authored: BTreeSet<u32> = BTreeSet::new();
    for archive in ARCHIVES {
        let spec = format!("{}:PS3_GAME/USRDIR/{archive}.PSARC", image.display());
        let Ok(mut open) = oag_assets::psarc::Archive::open(&spec) else {
            continue;
        };
        let paths: Vec<String> = open
            .paths()
            .iter()
            .filter(|p| p.ends_with(".rcsmodel"))
            .cloned()
            .collect();
        for path in paths {
            let Ok(blob) = open.read_path(&path) else {
                continue;
            };
            let Ok(model) = oag_rcs::rcsmodel::Model::parse(&blob) else {
                continue;
            };
            for material in &model.materials {
                authored.extend(material.parameters.iter().map(|p| p.hash));
            }
        }
    }
    let missing: Vec<&str> = names::KNOWN_PARAMETER_NAMES
        .iter()
        .copied()
        .filter(|name| !authored.contains(&oag_rcs::rcsmaterial::name_hash(name)))
        .collect();
    assert!(
        missing.is_empty(),
        "these names' hashes are not authored by any material parameter on the disc: {missing:?}"
    );
}
