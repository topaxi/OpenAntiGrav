//! Validates the [`kdcol`](oag_vex::kdcol) decoder against every
//! `track_col.col` Wipeout 2048 ships.
//!
//! **`#[ignore]`d, needs a disc image** (`just test-data`; ADR-0006). Skips when it
//! is absent; `OAG_REQUIRE_GAME_DATA=1` makes absence a failure.
//!
//! The tests skip with a printed message when the packages are absent. Set
//! `OAG_REQUIRE_GAME_DATA=1` to turn absence into a failure, which is what a
//! release check wants: a skipped ground-truth test is green and proves nothing.
//!
//! # What this is for
//!
//! Three invariants, and each one is an arithmetic closure that cannot come out
//! even unless the reading is right - the same argument the `.vex` collision
//! decoder rests on:
//!
//! - **the file accounts for every byte**, ending exactly on its final `"----"`;
//! - **the leaf runs tile the leaf index array**, no gap and no overlap, which
//!   is what says a k-d node's `+0x10`/`+0x14` are a count and a start rather
//!   than two unrelated words;
//! - **the stated bounds reproduce the geometry's own**, which is what says the
//!   pair is a centre and a half-extent rather than a min and a max.
//!
//! It also asserts the census the surface-byte table was recovered against, so
//! a value nothing has placed shows up here rather than as a hole in a track.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use oag_vex::kdcol;

/// The three packages an EU 2048 install carries, base first.
const PACKAGES: [&str; 3] = [
    "vita/PCSF00007/base/PSP2/data.psarc",
    "vita/PCSF00007/dlc1/PSP2/dlc1.psarc",
    "vita/PCSF00007/dlc2/PSP2/dlc2.psarc",
];

fn package(name: &str) -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/extracted")
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

/// Every `track_col.col` in every package, decoded, with its entry name.
fn every_file() -> Vec<(String, kdcol::KdCollision)> {
    let mut out = Vec::new();
    for name in PACKAGES {
        let Some(path) = package(name) else { continue };
        let mut archive = oag_assets::psarc::Archive::open(&path.display().to_string())
            .unwrap_or_else(|e| panic!("opening {}: {e}", path.display()));
        let mut entries: Vec<String> = archive
            .paths()
            .iter()
            .filter(|p| p.to_ascii_lowercase().ends_with("/track_col.col"))
            .cloned()
            .collect();
        entries.sort();
        for entry in entries {
            let blob = archive
                .read_path(&entry)
                .unwrap_or_else(|e| panic!("reading {entry}: {e}"));
            let decoded = kdcol::parse(&blob).unwrap_or_else(|e| panic!("{entry}: {e}"));
            out.push((entry, decoded));
        }
    }
    out
}

/// Every shipped file decodes, and the decode accounts for the whole file.
///
/// The second half is the load-bearing one: [`kdcol::parse`] refuses trailing
/// bytes, so a file that parses at all is one whose every section length was
/// read correctly.
#[test]
#[ignore = "needs the extracted 2048 packages in data/extracted/vita/"]
fn every_shipped_collision_file_decodes_and_accounts_for_itself() {
    let files = every_file();
    if files.is_empty() {
        return;
    }
    assert_eq!(
        files.len(),
        26,
        "the three EU packages ship 26 circuits between them"
    );
    for (name, decoded) in &files {
        assert!(!decoded.nodes.is_empty(), "{name}: no k-d nodes");
        assert!(!decoded.mesh.triangles.is_empty(), "{name}: no triangles");
        assert_eq!(
            decoded.mesh.surfaces.len(),
            decoded.mesh.triangles.len(),
            "{name}: one surface byte per triangle"
        );
    }
}

/// The leaves' runs tile the leaf index array exactly.
///
/// This is what says a node's `+0x10` low half is a triangle count and `+0x14`
/// is where that run starts: sort every leaf's `(start, count)` and they lay
/// end to end from zero to the array's length with nothing left over. A wrong
/// reading of either field breaks it on the first file.
#[test]
#[ignore = "needs the extracted 2048 packages in data/extracted/vita/"]
fn every_leaf_run_tiles_the_leaf_index_array() {
    for (name, decoded) in every_file() {
        let mut runs: Vec<(usize, usize)> = decoded
            .nodes
            .iter()
            .filter(|node| node.is_leaf())
            .map(|node| (node.first_leaf, node.triangle_count))
            .collect();
        runs.sort_unstable();
        let mut at = 0usize;
        for (first, count) in runs {
            assert_eq!(first, at, "{name}: leaf run starts at {first}, not {at}");
            at += count;
        }
        assert_eq!(
            at,
            decoded.leaves.len(),
            "{name}: the runs stop short of the leaf array"
        );
        for &index in &decoded.leaves {
            assert!(
                usize::from(index) < decoded.mesh.triangles.len(),
                "{name}: a leaf names triangle {index} of {}",
                decoded.mesh.triangles.len()
            );
        }
    }
}

/// The soup's stated box reproduces its own vertices, read as centre and
/// half-extent - and does **not** read as a min and a max.
///
/// Both halves are asserted, because the wrong reading is the plausible one and
/// was in this project's own notes until the numbers were checked.
#[test]
#[ignore = "needs the extracted 2048 packages in data/extracted/vita/"]
fn the_stated_bounds_are_a_centre_and_a_half_extent() {
    for (name, decoded) in every_file() {
        let mut low = [f32::MAX; 3];
        let mut high = [f32::MIN; 3];
        for vertex in &decoded.mesh.vertices {
            for a in 0..3 {
                low[a] = low[a].min(vertex[a]);
                high[a] = high[a].max(vertex[a]);
            }
        }
        let stated = decoded.mesh.bounds;
        for a in 0..3 {
            let slack = (high[a] - low[a]).abs() * 1e-4 + 1e-3;
            assert!(
                (stated.min()[a] - low[a]).abs() <= slack
                    && (stated.max()[a] - high[a]).abs() <= slack,
                "{name}: axis {a} spans {}..{} but centre+extent says {}..{}",
                low[a],
                high[a],
                stated.min()[a],
                stated.max()[a]
            );
            // The same numbers read as a min and a max put the "max" corner
            // nowhere near the geometry, on every axis of every file.
            assert!(
                (stated.extent[a] - high[a]).abs() > slack,
                "{name}: axis {a} cannot tell centre+extent from min+max"
            );
        }
        // The tree's own box contains the soup's, which is what a k-d root is.
        for a in 0..3 {
            assert!(decoded.bounds.min()[a] <= stated.min()[a] + 1e-3);
            assert!(decoded.bounds.max()[a] >= stated.max()[a] - 1e-3);
        }
    }
}

/// The surface-byte census, and no byte outside the nine that were measured.
///
/// A new value would be a surface with no behaviour, which is a hole in a
/// track - so it fails here rather than being discovered by falling through
/// one. The counts are the corpus the table in
/// [`kdcol::class_of`](oag_vex::kdcol::class_of) was recovered against.
#[test]
#[ignore = "needs the extracted 2048 packages in data/extracted/vita/"]
fn every_surface_byte_is_one_the_table_places() {
    let files = every_file();
    if files.is_empty() {
        return;
    }
    let mut census: BTreeMap<u8, usize> = BTreeMap::new();
    for (name, decoded) in &files {
        for &surface in &decoded.mesh.surfaces {
            *census.entry(surface).or_default() += 1;
            assert!(
                kdcol::surface_kind(surface).is_some(),
                "{name}: surface byte {surface} is not placed"
            );
        }
    }
    println!("surface byte census: {census:?}");
    assert_eq!(
        census.keys().copied().collect::<Vec<u8>>(),
        vec![2, 3, 4, 5, 6, 7, 10, 11, 12],
        "the nine values the corpus carries"
    );
    // Floor is the bulk of it, and every circuit has some: a circuit with no
    // floor is a circuit a craft falls through.
    for (name, decoded) in &files {
        let floors = decoded
            .mesh
            .surfaces
            .iter()
            .filter(|&&s| kdcol::surface_kind(s) == Some(oag_vex::collision::SurfaceKind::Floor))
            .count();
        assert!(floors > 0, "{name}: no drivable floor at all");
    }
}
