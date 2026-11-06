//! What 2048's loading screen names, against what the package ships.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-2048 --run-ignored all \
//!     -E 'test(/^loading_ground_truth::/)'
//! ```

use oag_assets::Archives;

fn open() -> Option<Archives> {
    let source = oag_testdata::exact("data/extracted/vita/PCSF00007")?;
    oag_2048::open(&source.display().to_string()).ok()
}

/// Every entry the `Loading` table names - the five illustrations and the six marks, both
/// arrows included - is in the package and is a Vita texture.
#[test]
#[ignore = "needs the decrypted Vita package in data/extracted/vita/"]
fn every_entry_of_the_loading_table_is_in_the_package() {
    let Some(mut archives) = open() else { return };
    let loading = oag_2048::TITLE
        .loading
        .expect("2048 names a loading screen");
    assert_eq!(loading.features.len(), 1, "one styling is reachable");
    let mut names: Vec<&str> = loading.chrome.iter().map(|(_, entry)| *entry).collect();
    names.extend(
        loading.features[0]
            .features
            .iter()
            .map(|feature| feature.image),
    );
    assert_eq!(names.len(), 11, "six marks and five illustrations");
    assert!(
        archives
            .locate("data/fe/images/not_a_loading_mark.gxt")
            .is_none(),
        "the lookup is not permissive"
    );
    for name in names {
        let blob = archives
            .read_name(name)
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(&blob[..4], b"GXT\0", "{name} is a GXT texture");
    }
}

/// The styling the executable never picks ships too: each illustration's name with `_fury`
/// before the extension is in the package, which is why the table's one style is a reading of
/// the executable's flag and not a gap in the disc.
#[test]
#[ignore = "needs the decrypted Vita package in data/extracted/vita/"]
fn the_unreached_fury_pictures_ship_beside_the_base_ones() {
    let Some(archives) = open() else { return };
    let loading = oag_2048::TITLE
        .loading
        .expect("2048 names a loading screen");
    for feature in loading.features[0].features {
        let fury = feature.image.replace(".gxt", "_fury.gxt");
        assert!(archives.locate(&fury).is_some(), "{fury}");
    }
}
