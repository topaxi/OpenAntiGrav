//! What a `LodGroup`'s child count reads as on Wipeout HD, and why the fix for
//! it changes no picture yet.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! # or only this file:
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-render --run-ignored all \
//!     -E 'binary(hd_lod_ground_truth)'
//! ```
//!
//! # Why this exists
//!
//! `mesh::build_with_textures` read a `LodGroup`'s `child_count` with
//! `u32::from_le_bytes` until 2026-08-18. On a PS3 `.vex` that turns every count
//! into itself shifted 24 bits left, the `== 2` test never matched, and
//! **`Lod::Single` would have been silently a no-op on every HD model.** The
//! failure direction was the safe one - a user who chose
//! `graphics.lod = "single"` would have got `Both`'s picture rather than a hole
//! in the track - which is exactly why nothing caught it.
//!
//! **How much the fix buys is smaller than "64 groups" suggests, and this file
//! is where that was found.** The skip only ever fires on a *two*-child group,
//! and HD's 64 split **48 declaring one child and 16 declaring two**. So even
//! read the right way round, three quarters of them would not have skipped
//! anything. The distribution is asserted below rather than described, because
//! the first draft of this work claimed all 64 read `2` and that was an
//! assumption dressed as a measurement.
//!
//! **The fix changes no rendered picture today, and that is asserted here rather
//! than assumed.** HD keeps its render geometry in `.rcsmodel`, not in the
//! `.vex`, so all six of the files carrying a `LodGroup` fail
//! `build_with_textures` with "decoding batches" long before the skip is
//! reached. The correctness fix lands now because the read is wrong now; the
//! test that it *matters* is the one below that will start failing the day
//! `.rcsmodel` geometry reaches this builder, which is the point at which
//! someone should come back and measure the triangle counts.

use std::path::{Path, PathBuf};

use oag_render::mesh::{self, Lod};
use oag_vex::vex;

/// The decrypted PS3 image.
const PS3_IMAGE: &str = "hdfury-ps3-eu-dec.iso";

/// The archives to walk. All seven, because `LodGroup` is not confined to one.
const ARCHIVES: usize = 7;

/// `LodGroup` nodes HD authors, across every `.vex` on the disc.
const EXPECTED_LOD_GROUPS: usize = 64;

/// How those 64 split by declared child count: 48 name one child, 16 name two.
///
/// Only the 16 are reachable by [`Lod::Single`] at all, so this is the size of
/// what the byte-order fix makes possible rather than a curiosity.
const EXPECTED_CHILD_COUNTS: &[(u32, usize)] = &[(1, 48), (2, 16)];

/// Files carrying at least one.
const EXPECTED_FILES: usize = 6;

/// Offset of the child count inside a `LodGroup` payload.
const CHILD_COUNT_AT: usize = 0x50;

fn image() -> Option<PathBuf> {
    oag_testdata::image(PS3_IMAGE)
}

/// Every `.vex` on the disc that authors a `LodGroup`, as `(path, bytes)`.
fn lod_group_files(image: &Path) -> Vec<(String, Vec<u8>)> {
    let mut found = Vec::new();
    let mut groups = 0;
    for n in 0..ARCHIVES {
        let spec = format!("{}:PS3_GAME/USRDIR/DATA0{n}.PSARC", image.display());
        let mut archive = oag_assets::psarc::Archive::open(&spec).expect("the archive opens");
        let paths: Vec<String> = archive
            .paths()
            .iter()
            .filter(|p| p.ends_with(".vex"))
            .cloned()
            .collect();
        for path in paths {
            let blob = archive.read_path(&path).expect("the .vex reads");
            let Ok(nodes) = vex::nodes(&blob) else {
                continue;
            };
            // The class number is version-dependent, so it comes off the file's
            // own class table rather than from the constant - the same care
            // `mesh::build_class` takes, and for the same reason.
            let Some(lod_group) = vex::classes_of(&blob).ok().and_then(|c| c.lod_group) else {
                continue;
            };
            let count = vex::nodes_by_class(&nodes, lod_group).count();
            if count > 0 {
                groups += count;
                found.push((path, blob));
            }
        }
    }
    assert_eq!(groups, EXPECTED_LOD_GROUPS, "LodGroup nodes on the disc");
    assert_eq!(found.len(), EXPECTED_FILES, "files carrying one");
    found
}

#[test]
#[ignore = "needs a decrypted PS3 disc image under data/images"]
fn every_hd_lod_group_declares_a_small_child_count_read_the_files_own_way() {
    let Some(image) = image() else {
        return;
    };
    let mut seen = 0;
    let mut counts: std::collections::BTreeMap<u32, usize> = std::collections::BTreeMap::new();
    for (path, blob) in lod_group_files(&image) {
        let nodes = vex::nodes(&blob).expect("the tree walks");
        let lod_group = vex::classes_of(&blob)
            .expect("a class table")
            .lod_group
            .expect("version 6 numbers LodGroup");
        let order = vex::byte_order(&blob);
        for node in vex::nodes_by_class(&nodes, lod_group) {
            let payload = &blob[node.payload()];
            if payload.len() < CHILD_COUNT_AT + 4 {
                continue;
            }
            seen += 1;
            let read = order.u32(payload, CHILD_COUNT_AT);
            *counts.entry(read).or_default() += 1;
            let wrong = u32::from_le_bytes(
                payload[CHILD_COUNT_AT..CHILD_COUNT_AT + 4]
                    .try_into()
                    .expect("four bytes"),
            );
            // A child count is a small number. This is the assertion the fix
            // makes true: read the file's own way it is a plausible count, and
            // read the host's way it is that count shifted 24 bits left.
            assert!(
                read > 0 && read < 16,
                "{path}: LodGroup child count {read} at {CHILD_COUNT_AT:#x} is not a count"
            );
            assert_eq!(
                wrong,
                read.swap_bytes(),
                "{path}: the little-endian read is not the byte-reverse of the right one, \
                 so this field is not what this test thinks it is"
            );
            assert!(
                wrong >= 0x0100_0000,
                "{path}: the little-endian read is {wrong}, small enough to have matched \
                 the old `== 2` test - the bug this pins would not have shown here"
            );
        }
    }
    for (count, nodes) in &counts {
        println!("child count {count}: {nodes} node(s)");
    }
    assert_eq!(seen, EXPECTED_LOD_GROUPS, "LodGroup payloads long enough");
    // The distribution, not just the range. `Lod::Single` fires only on a
    // two-child group, so this is what says how much of the disc the fix can
    // reach - and pinning it stops "HD's LodGroups declare 2" being restated as
    // a fact about all 64 when it is a fact about 16.
    let measured: Vec<(u32, usize)> = counts.into_iter().collect();
    assert_eq!(
        measured, EXPECTED_CHILD_COUNTS,
        "HD's LodGroup child counts are not the measured split"
    );
}

#[test]
#[ignore = "needs a decrypted PS3 disc image under data/images"]
fn no_hd_lod_group_file_reaches_the_mesh_builder_yet() {
    let Some(image) = image() else {
        return;
    };
    // The honest scope of the byte-order fix. **When this test starts failing,
    // that is the good news**: it means HD geometry now builds, and the thing to
    // do is replace it with a triangle-count comparison of `Lod::Both` against
    // `Lod::Single` on whichever file started working - checking that `Single`
    // is smaller and, crucially, not empty.
    for (path, blob) in lod_group_files(&image) {
        let built = mesh::build_with_textures(&path, &blob, None, Lod::Original);
        assert!(
            built.is_err(),
            "{path} now builds a mesh from its .vex, so `Lod::Single` can finally change an \
             HD picture - measure it and rewrite this test"
        );
    }
}
