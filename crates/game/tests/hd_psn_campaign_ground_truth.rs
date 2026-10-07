//! RACE CAMPAIGN on the plain Wipeout HD PSN download (`NPEA00057` v3.00): the
//! base campaign opens directly, with no HD or Fury chooser in front of it, and
//! it is the same campaign the Fury disc's HD branch holds.
//!
//! **`#[ignore]`d and never run in CI.** Needs the install under
//! `data/extracted/ps3/hd-psn-eu/` (steps in `docs/overview/installing.md`) and,
//! for the comparisons, the decrypted Fury disc.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
//!     -E 'binary(hd_psn_campaign_ground_truth)'
//! ```

use oag_hud::sprite::Sheet;
use oag_ui::language::StringTable;
use oag_ui_screens::picker::FaceScales;

const INSTALL: &str = "data/extracted/ps3/hd-psn-eu";
const DISC: &str = "data/images/hdfury-ps3-eu-dec.iso";

fn psn() -> Option<oag_assets::Archives> {
    let path = oag_testdata::exact(INSTALL)?;
    Some(oag_hd::open(&path.display().to_string()).expect("the install opens as HD"))
}

fn disc() -> Option<oag_assets::Archives> {
    let path = oag_testdata::exact(DISC)?;
    Some(oag_hd::open(&path.display().to_string()).expect("the disc opens as HD"))
}

fn load(
    archives: &mut oag_assets::Archives,
    title: &'static oag_title::Title,
) -> oag_game::campaign::Campaign {
    let base = Sheet::build(&[], &mut Vec::new());
    oag_game::campaign::load(
        archives,
        &StringTable::default(),
        FaceScales::default(),
        [1920.0, 1080.0],
        &base,
        &[],
        title,
    )
    .expect("the campaign loads")
}

fn copy<'a>(copies: &'a [(String, Vec<u8>)], label: &str) -> &'a [u8] {
    &copies
        .iter()
        .find(|(name, _)| name.to_ascii_lowercase().ends_with(label))
        .unwrap_or_else(|| panic!("no {label} copy"))
        .1
}

/// RACE CAMPAIGN on PSN reads eight grids and the base `Grid Selection` and
/// `Cell Selection`, and has no `Campaign Selection` nor `Grid Selection Fury`.
#[test]
#[ignore = "needs the PSN install in data/extracted/ps3/hd-psn-eu"]
fn psn_opens_the_base_campaign_with_no_chooser() {
    let Some(mut archives) = psn() else { return };
    let title = oag_hd::title_of(&archives);
    assert_eq!(
        title.race.track,
        oag_hd::psn::DEFAULT_TRACK,
        "the PSN variant"
    );
    assert_eq!(title.campaign.screen_archive, None);
    let campaign = load(&mut archives, title);
    assert_eq!(campaign.grids.len(), 8, "grid_00..grid_07");
    assert!(campaign.selection_layout.is_none(), "no HD/Fury chooser");
    assert!(campaign.grid_layout_fury.is_none());
}

/// The disc's HD branch is unchanged: its chooser and Fury screen still read.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn the_disc_still_has_its_chooser() {
    let Some(mut archives) = disc() else { return };
    let campaign = load(&mut archives, oag_hd::TITLE);
    assert_eq!(campaign.grids.len(), 16);
    assert!(campaign.selection_layout.is_some());
    assert!(campaign.grid_layout_fury.is_some());
}

/// What the PSN package carries is the disc's own, byte for byte, where it
/// carries it: its `data02` and `data04` copies of the grids and of the screen
/// file are the disc's `DATA02` and `DATA04` ones, and its `data04` grids are
/// `DATA06`'s HD branch with the one `Campaign="HD"` attribute left out.
#[test]
#[ignore = "needs the PSN install and a decrypted PS3 disc image"]
fn the_psn_campaign_is_the_discs_hd_branch() {
    let (Some(mut psn), Some(mut disc)) = (psn(), disc()) else {
        return;
    };
    let screen = oag_hd::campaign::SCREEN_ENTRY;
    let psn_screen = psn.read_every_name(screen);
    let disc_screen = disc.read_every_name(screen);
    assert_eq!(
        copy(&psn_screen, "data02.psarc"),
        copy(&disc_screen, "data02.psarc"),
        "the PSN screen file is the disc's DATA02 copy"
    );
    assert_eq!(
        psn_screen.len(),
        1,
        "no second copy, so no Campaign Selection"
    );

    for index in 0..8 {
        let grid = oag_hd::campaign::entry_name(index);
        let psn_copies = psn.read_every_name(&grid);
        let disc_copies = disc.read_every_name(&grid);
        for label in ["data02.psarc", "data04.psarc"] {
            assert_eq!(
                copy(&psn_copies, label),
                copy(&disc_copies, label),
                "{grid} {label}"
            );
        }
        let hd = String::from_utf8(copy(&disc_copies, "data06.psarc").to_vec()).unwrap();
        let per_difficulty = String::from_utf8(copy(&psn_copies, "data04.psarc").to_vec()).unwrap();
        assert_eq!(
            hd.replace(r#" Campaign="HD""#, ""),
            per_difficulty,
            "{grid}: DATA06's HD copy against the PSN data04 one"
        );
    }
}

/// Which copy of a grid and of the medal atlas each source carries: grids are
/// served from `data02`/`DATA02` (flat) on both, the atlas has the same two
/// copies with the same sizes, so the PSN campaign reads what the disc's does.
#[test]
#[ignore = "needs the PSN install and a decrypted PS3 disc image"]
fn the_grid_and_medal_copies_are_the_same_on_both_sources() {
    let (Some(mut psn), Some(mut disc)) = (psn(), disc()) else {
        return;
    };
    let grid = oag_hd::campaign::entry_name(0);
    let atlas = r"Data\FE\Images\Hexmedal_HD.gtf";
    for archives in [&mut psn, &mut disc] {
        let served = archives.read_name(&grid).expect("grid served");
        let copies = archives.read_every_name(&grid);
        assert_eq!(
            served,
            copy(&copies, "data02.psarc"),
            "the flat copy is served"
        );
        let sizes: Vec<usize> = archives
            .read_every_name(atlas)
            .into_iter()
            .map(|(_, blob)| blob.len())
            .collect();
        assert_eq!(sizes, [262_272, 786_560], "flat then per-difficulty");
    }
}
