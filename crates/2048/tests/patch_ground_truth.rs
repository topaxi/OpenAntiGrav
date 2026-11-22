//! The v1.04 patch is mounted over the base package: which of the three
//! archives answers for a path all of them carry, and that a source pointed at
//! the base package alone still has no patch.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-2048 --run-ignored all \
//!     -E 'test(/^patch_ground_truth::/)'
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

/// Detonator's three layout files resolve through the title axis on the mounted archives.
#[test]
#[ignore = "needs the decrypted Vita package in data/extracted/vita/"]
fn the_detonator_layout_entries_resolve() {
    let Some(mut archives) = open("data/extracted/vita/PCSF00007") else {
        return;
    };
    let layouts = oag_2048::TITLE
        .race
        .detonator
        .and_then(|d| d.layouts)
        .expect("2048 names the layout files");
    let patterns = archives
        .read_name(layouts.pattern_file)
        .expect("PatternFile");
    assert!(String::from_utf8_lossy(&patterns).contains("<Pattern"));
    let values = archives
        .read_name(layouts.gameplay_values)
        .expect("GamePlayValues");
    assert!(String::from_utf8_lossy(&values).contains("MineCountStage14"));
    let scoring = archives
        .read_name(layouts.scoring)
        .expect("DetonatorScoring");
    let scoring = oag_tables::detonator_layout::parse_scoring(&String::from_utf8_lossy(&scoring));
    assert_eq!(
        (
            scoring.new_mine_score,
            scoring.old_mine_score,
            scoring.bomb_score,
            scoring.bomb_increment
        ),
        (100, 50, 2000, 500)
    );
}

/// Detonator flies its own hull, livery folder, handling file and object models out of the mounted archives
/// (`oag_2048::race::DETONATOR`): the entries its title data names resolve, the handling file
/// is the classless one HD's is, and the hull sits where Zone's does, under `hdships`.
#[test]
#[ignore = "needs the decrypted Vita package in data/extracted/vita/"]
fn the_detonator_hull_handling_and_livery_entries_resolve() {
    let Some(mut archives) = open("data/extracted/vita/PCSF00007") else {
        return;
    };
    let race = oag_2048::TITLE.race;
    let detonator = race.detonator.expect("2048 races Detonator");
    let craft = detonator.craft.expect("2048 names the Detonator craft");
    let ships = race.ships_for("Qirex");
    let hull = ships.hull_entry_name("Qirex", Some(craft.hull), None);
    assert!(hull.ends_with(r"hdships\Detonator\Ship.vex"), "{hull}");
    assert!(archives.read_name(&hull).is_ok(), "{hull}");
    let handling = detonator.handling.expect("2048 names the handling file");
    let text = archives
        .read_name(handling.entry.value)
        .expect("Detonator's handlingstats.xml");
    let text = String::from_utf8_lossy(&text);
    assert!(text.contains(r#"width="4.0""#), "Detonator's Misc.width");
    assert!(!text.contains("<Class"), "a classless file, as HD's");
    let objects = detonator.objects.expect("2048 names the object models");
    for entry in objects
        .mine
        .iter()
        .chain(&objects.bomb)
        .chain([&objects.pad])
    {
        assert!(archives.read_name(entry).is_ok(), "{entry}");
    }
    // Census: each ships under `Data\Weapons2048` too, and under no `Data\art\published` spelling.
    assert!(
        archives
            .read_name(r"Data\Weapons2048\mine_rita_football.vex")
            .is_ok()
    );
    assert!(
        archives
            .read_name(r"Data\art\published\Weapons\mine_rita_football.vex")
            .is_err()
    );
    let key = craft.hull.livery_key().expect("the livery key");
    assert_eq!(key, "zoneship_zone");
    let texture = hull.replace(r"Ship.vex", &format!(r"{key}\Detonator_AO.gxt"));
    assert!(
        archives.read_name(&texture).is_ok(),
        "the hull's own default livery folder holds {texture}"
    );
}
