//! Every `.vex` class ID any shipped file carries has a name.
//!
//! The check the class table exists to pass, and the one that would have caught
//! the two errors a 2026-08-18 pass fixed: `sea` and `seaweed` swapped, and
//! three ids absent, one of which Wipeout HD's own circuits use.
//!
//! `#[ignore]`d because it reads real disc images; run with `just test-data`.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use oag_formats::vex;

/// The archives on the HD disc that hold `.vex` files.
const PS3_ARCHIVES: &[&str] = &[
    "DATA00", "DATA01", "DATA02", "DATA03", "DATA04", "DATA05", "DATA06",
];

/// The `.vex` version whose class ids [`vex::class_name`] enumerates.
const CURRENT_VERSION: u32 = 6;

fn image(name: &str) -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/images")
        .join(name);
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

/// **Nothing on Wipeout HD's disc carries a class this table cannot name.**
///
/// A sweep rather than a spot check, because the failure it guards against is
/// silent: an unnamed class reads as `?` in a census and as "a node we do not
/// handle" everywhere else, which is indistinguishable from a class that is
/// genuinely not interesting. `Track Wall Collision` sat that way in Talon's
/// Junction until the table was read whole out of `EBOOT.elf` - one node,
/// 99 KiB of payload, and the file's own node name `collision_trackwall` was
/// saying what it was the entire time.
#[test]
#[ignore]
fn every_class_on_the_hd_disc_has_a_name() {
    let Some(image) = image("hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let mut seen: BTreeMap<u32, (usize, String)> = BTreeMap::new();
    let mut files = 0usize;
    for archive in PS3_ARCHIVES {
        let spec = format!("{}:PS3_GAME/USRDIR/{archive}.PSARC", image.display());
        let mut open = oag_assets::psarc::Archive::open(&spec).expect("the archive opens");
        let paths: Vec<String> = open
            .paths()
            .iter()
            .filter(|p| p.ends_with(".vex"))
            .cloned()
            .collect();
        for path in paths {
            let Ok(bytes) = open.read_path(&path) else {
                continue;
            };
            let Ok(nodes) = vex::nodes(&bytes) else {
                continue;
            };
            files += 1;
            for node in &nodes {
                let entry = seen
                    .entry(node.class_id)
                    .or_insert_with(|| (0, path.clone()));
                entry.0 += 1;
            }
        }
    }
    println!("{files} .vex files, {} distinct classes", seen.len());
    let mut unnamed = Vec::new();
    for (id, (count, first)) in &seen {
        match vex::class_name(*id) {
            Some(name) => println!("  {id:#06x}  {count:7}  {name}"),
            None => {
                println!("  {id:#06x}  {count:7}  UNNAMED, first in {first}");
                unnamed.push(*id);
            }
        }
    }
    assert!(files > 500, "the sweep found almost nothing: {files} files");
    assert!(
        unnamed.is_empty(),
        "classes with no name: {:x?} - the table is at EBOOT.elf 0x00921110, \
         see docs/ghidra/functions/ps3-hdfury-eu/vex-classes.md",
        unnamed
    );
}

/// **The PSP and PS2 titles agree with the table read out of HD's executable.**
///
/// What says the table is one table rather than a per-title one, and therefore
/// what makes reading it out of *any* of the executables a reading of all of
/// them. It is also the check a swapped pair survives: `sea` and `seaweed` are
/// adjacent ids with adjacent names, which is exactly the error a single-title
/// read cannot catch.
///
/// Swept **by index rather than by name**, because a PSP WAD directory stores a
/// hash of each entry's name and not the name - the same reason
/// `vex_ps2_ground_truth.rs` sweeps that way.
///
/// # Version 6 only, and the version-4 files are the reason
///
/// **There is more than one id space, and this table is version 6's.** Pulse's
/// PSP disc carries version-4 `.vex` files whose nodes use `0x6d`, `0xee`,
/// `0xf1`, `0x101`, `0x11e`, `0x372`, `0x373`, `0x375`, `0x378`, `0x382`,
/// `0x38f` and `0x397` - none of which this table names, and none of which it
/// should: `0x378` is version 4's `skycube` where version 6's is `0x3c6`. So
/// this asserts over version-6 files and *reports* the others, rather than
/// pretending one enumeration covers both.
///
/// **This test is what forced the generic Maya classes into the table.** They
/// were left out on the reasoning that no shipped node uses one; Pulse's
/// version-6 files carry 379 nodes of class `0x0000` and one of `0x0108`.
#[test]
#[ignore]
fn the_class_ids_agree_across_the_lineage() {
    let mut checked = 0usize;
    for (label, name) in [
        ("pulse-psp-usa", "pulse-psp-usa.chd"),
        ("pulse-psp-eu", "pulse-psp-eu.chd"),
        ("pulse-ps2-eu", "pulse-ps2-eu.chd"),
    ] {
        let Some(path) = image(name) else { continue };
        let candidates = oag_pulse::TITLE.archives;
        let archives: Vec<&str> = candidates
            .data
            .iter()
            .chain(candidates.fe)
            .chain(candidates.extra)
            .map(|(entry, _)| *entry)
            .collect();
        for archive in archives {
            let spec = format!("{}:{archive}", path.display());
            let Ok(mut open) = oag_assets::Archive::open(&spec) else {
                continue;
            };
            let count = open.directory().entries.len();
            let mut ids: BTreeSet<u32> = BTreeSet::new();
            let mut legacy: BTreeSet<u32> = BTreeSet::new();
            for index in 0..count {
                let Ok(bytes) = open.read(index) else {
                    continue;
                };
                if !vex::has_magic(&bytes) {
                    continue;
                }
                let Ok(nodes) = vex::nodes(&bytes) else {
                    continue;
                };
                let current = vex::version(&bytes).is_ok_and(|v| v >= CURRENT_VERSION);
                let into = if current { &mut ids } else { &mut legacy };
                into.extend(nodes.iter().map(|n| n.class_id));
            }
            if ids.is_empty() && legacy.is_empty() {
                continue;
            }
            checked += 1;
            let unnamed: Vec<u32> = ids
                .iter()
                .copied()
                .filter(|id| vex::class_name(*id).is_none())
                .collect();
            let outside: Vec<u32> = legacy
                .iter()
                .copied()
                .filter(|id| !ids.contains(id) && vex::class_name(*id).is_none())
                .collect();
            println!(
                "{label} {archive}: {} version-6 classes, {} unnamed; {} version-4 classes, \
                 {} outside this table's id space {outside:x?}",
                ids.len(),
                unnamed.len(),
                legacy.len(),
                outside.len()
            );
            assert!(
                unnamed.is_empty(),
                "{label} {archive} carries version-6 classes with no name: {unnamed:x?}"
            );
        }
    }
    println!("{checked} archive(s) checked");
}
