//! The v1.04 patch is mounted over the base package: which of the three
//! archives answers for a path all of them carry, and that a source pointed at
//! the base package alone still has no patch.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-2048 --run-ignored all \
//!     -E 'binary(patch_ground_truth)'
//! ```

use oag_assets::Archives;

/// Carried by `data.psarc`, `data1.psarc` and `data2.psarc`, three different
/// contents of 30,227, 31,974 and 34,908 bytes.
const THREE_WAY: &str = "data/plugins/frontend/NEWGUI/Definition.xml";

/// In the base package and in neither patch archive.
const BASE_ONLY: &str = "data/art/published/environments/altima/track.vex";

fn open(directory: &str) -> Option<Archives> {
    let source = oag_testdata::exact(directory)?;
    oag_2048::open(&source.display().to_string()).ok()
}

fn tail(label: &str) -> &str {
    label
        .rsplit(['/', '\\'])
        .next()
        .expect("a label has a tail")
}

#[test]
#[ignore = "needs the decrypted Vita package in data/extracted/vita/"]
fn the_patch_archives_are_mounted_ahead_of_the_base_one_data2_first() {
    let Some(archives) = open("data/extracted/vita/PCSF00007") else {
        return;
    };
    let patch: Vec<&str> = archives
        .patch
        .iter()
        .map(|archive| tail(archive.label()))
        .collect();
    assert_eq!(patch, ["data2.psarc", "data1.psarc"]);

    assert_eq!(
        archives
            .locations(THREE_WAY)
            .iter()
            .map(|l| tail(l))
            .collect::<Vec<_>>(),
        ["data2.psarc", "data1.psarc", "data.psarc"],
        "the same order `locate` searches, patch before base"
    );
    assert_eq!(
        tail(archives.locate(THREE_WAY).expect("held")),
        "data2.psarc"
    );
}

#[test]
#[ignore = "needs the decrypted Vita package in data/extracted/vita/"]
fn a_path_the_patch_does_not_carry_is_still_the_bases() {
    let Some(archives) = open("data/extracted/vita/PCSF00007") else {
        return;
    };
    assert_eq!(
        tail(archives.locate(BASE_ONLY).expect("held")),
        "data.psarc"
    );
}

/// The campaign ground truth opens `.../base` on purpose: nothing under it is
/// a patch, so what it reads is the v1.00 package's own copy.
#[test]
#[ignore = "needs the decrypted Vita package in data/extracted/vita/"]
fn a_source_naming_only_the_base_package_mounts_no_patch() {
    let Some(archives) = open("data/extracted/vita/PCSF00007/base") else {
        return;
    };
    assert!(archives.patch.is_empty());
    assert_eq!(
        tail(archives.locate(THREE_WAY).expect("held")),
        "data.psarc"
    );
}
