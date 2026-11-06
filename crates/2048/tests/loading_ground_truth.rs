//! The pictures 2048's `LoadingScreen` names, against what the package ships.
//!
//! The names are the executable's own strings, read out of `LoadingScreen_LoadAssets`
//! (`0x81281ab2`) and `LoadingScreen_Construct` (`0x8128074c`)
//! (`docs/ghidra/functions/vita-2048-eu-v104/loading-screen.md`). No title table carries them:
//! `oag_2048::TITLE.loading` is `None` because the screen a race load starts (kind 4) is not
//! built, so this census is what keeps the evidence page honest about what a build of it can
//! read.
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

/// What kind 4, the race load, loads: the controls stills a mode on the HD lineage draws, the
/// still every other mode draws, the bar, the medals and one of three team logos.
const KIND_4: &[&str] = &[
    "data/fe/newimages/loadingscreens/loadingscreen_racer_controls.gxt",
    "data/fe/newimages/loadingscreens/loadingscreen_motion_controls.gxt",
    "data/fe/newimages/loadingscreens/loadingscreen_wipeout_controls.gxt",
    "data/fe/newimages/title_screen.gxt",
    "data/fe/images/loading_screen_demo_bar.gxt",
    "data/fe/newimages/post_race_medals.gxt",
    "data/fe/newimages/team_logos/ag_systems2048.gxt",
    "data/fe/newimages/team_logos/feisar2048.gxt",
    "data/fe/newimages/team_logos/qirex2048.gxt",
];

/// What kind 7, HD's feature screen (started by no call site), loads: six marks and five
/// illustrations, each also with `_fury` before the extension.
const KIND_7_MARKS: &[&str] = &[
    "data/fe/images/corner.gxt",
    "data/fe/images/dot.gxt",
    "data/fe/images/square.gxt",
    "data/fe/images/line.gxt",
    "data/fe/images/title_arrow_hd.gxt",
    "data/fe/images/subtitle_arrow_hd.gxt",
];
const KIND_7_FEATURES: &[&str] = &[
    "data/fe/images/barrel_roll.gxt",
    "data/fe/images/side_shift_tap.gxt",
    "data/fe/images/pilot_assist.gxt",
    "data/fe/images/absorb.gxt",
    "data/fe/images/flip.gxt",
];

fn assert_textures(archives: &mut Archives, names: &[String]) {
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

/// Every picture the race load (kind 4) names is in the package and is a Vita texture.
#[test]
#[ignore = "needs the decrypted Vita package in data/extracted/vita/"]
fn every_picture_the_race_load_names_is_in_the_package() {
    let Some(mut archives) = open() else { return };
    let names: Vec<String> = KIND_4.iter().map(|name| (*name).to_string()).collect();
    assert_textures(&mut archives, &names);
}

/// HD's feature screen's art ships whole: all six marks (both arrows included) and the ten
/// illustrations, in both stylings.
#[test]
#[ignore = "needs the decrypted Vita package in data/extracted/vita/"]
fn the_art_of_the_dormant_feature_screen_ships_whole() {
    let Some(mut archives) = open() else { return };
    let mut names: Vec<String> = KIND_7_MARKS
        .iter()
        .map(|name| (*name).to_string())
        .collect();
    for feature in KIND_7_FEATURES {
        names.push((*feature).to_string());
        names.push(feature.replace(".gxt", "_fury.gxt"));
    }
    assert_eq!(names.len(), 16);
    assert_textures(&mut archives, &names);
}
