//! Wipeout HD/Fury's own `Campaign Selection` screen, read off `DATA06.PSARC`'s
//! copy of `Data\Plugins\Frontend\Gui\CellMode_Definition.xml` directly - see
//! `crates/hd/src/campaign.rs`'s own doc on `SCREEN_ENTRY`/`SELECTION_SCREEN_ARCHIVE`
//! for why this is a different archive than `Grid Selection`/`Cell Selection`
//! read from, and `docs/ui/campaign-screens.md`'s "Wipeout HD/Fury: `Campaign
//! Selection`" section for the full measurement this file proves.
//!
//! **Checked as raw text, not through `oag_ui::screen::Screens`** - that
//! crate already depends on `oag-hd`, so pulling it in here as a
//! dev-dependency would be a cycle; a plain substring/ordering check over
//! the archive's own bytes needs no such thing and is enough for what this
//! file asserts (document order, entry names).
//!
//! **`#[ignore]`d and never run in CI.** They need game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! ```

use std::path::PathBuf;

use oag_assets::Archives;

const PS3_IMAGE: &str = "hdfury-ps3-eu-dec.iso";

fn image() -> Option<PathBuf> {
    oag_testdata::image(PS3_IMAGE)
}

fn opened() -> Option<Archives> {
    let path = image()?;
    Some(oag_hd::open(path.to_str().expect("utf-8 path")).expect("the disc opens as HD"))
}

/// `DATA06`'s own copy of `SCREEN_ENTRY`, as plain UTF-8 text - the same
/// blob `oag_game::campaign::load_hd_campaign_selection` reads.
fn data06_screen_text(archives: &mut Archives) -> String {
    let copies = archives.read_every_name(oag_hd::campaign::SCREEN_ENTRY);
    let (_, blob) = copies
        .into_iter()
        .find(|(label, _)| label.ends_with(oag_hd::campaign::SELECTION_SCREEN_ARCHIVE))
        .expect("DATA06 carries a copy of SCREEN_ENTRY");
    String::from_utf8(blob).expect("plain UTF-8, not dictionary-shortened")
}

/// `DATA02`'s own copy, the one `oag_assets::Archives::read_name`'s
/// precedence resolves to for everything else this title reads off
/// `SCREEN_ENTRY` - confirmed to lack `Campaign Selection` entirely, the
/// fact `oag_game::campaign::load_hd_campaign_selection` exists to work
/// around.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn the_precedence_resolved_copy_has_no_campaign_selection_screen() {
    let Some(mut archives) = opened() else {
        return;
    };
    let blob = archives
        .read_name(oag_hd::campaign::SCREEN_ENTRY)
        .expect("SCREEN_ENTRY reads through precedence");
    let text = String::from_utf8(blob).expect("plain UTF-8");
    assert!(
        !text.contains("Campaign Selection"),
        "DATA02's own copy should not author Campaign Selection at all"
    );
    assert!(
        !text.contains("Grid Selection Fury"),
        "DATA02's own copy should not author Grid Selection Fury either"
    );
}

/// `DATA06`'s own copy authors all four screen names, `Campaign Selection`
/// first (its own top-level `<Screen>`), then `Grid Selection`/`Grid
/// Selection Fury`/`Cell Selection` inside the anonymous wrapper sibling to
/// it.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn data06s_copy_carries_all_four_screens_in_document_order() {
    let Some(mut archives) = opened() else {
        return;
    };
    let text = data06_screen_text(&mut archives);
    let selection_at = text
        .find(r#"name="Campaign Selection""#)
        .expect("Campaign Selection should be in the text");
    let grid_at = text
        .find(r#"name="Grid Selection""#)
        .expect("Grid Selection should be in the text");
    let fury_at = text
        .find(r#"name="Grid Selection Fury""#)
        .expect("Grid Selection Fury should be in the text");
    let cell_at = text
        .find(r#"name="Cell Selection""#)
        .expect("Cell Selection should be in the text");
    assert!(
        selection_at < grid_at,
        "Campaign Selection should come first"
    );
    assert!(
        grid_at < fury_at,
        "Grid Selection should come before its Fury sibling"
    );
    assert!(
        fury_at < cell_at,
        "Grid Selection Fury should come before Cell Selection"
    );
}

/// `campaignList`'s own two `<Entry>`s, in the disc's own document order -
/// `FE_RC_FURY` before `FE_RC_HD`, which is what
/// `oag_ui::campaign::selection::CampaignSelection::new`'s own `index: 0`
/// landing on `Fury` is read off.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn campaign_list_names_fury_before_the_base_hd_campaign() {
    let Some(mut archives) = opened() else {
        return;
    };
    let text = data06_screen_text(&mut archives);
    let fury_at = text
        .find("FE_RC_FURY")
        .expect("campaignList should author an FE_RC_FURY entry");
    let hd_at = text
        .find("FE_RC_HD")
        .expect("campaignList should author an FE_RC_HD entry");
    assert!(
        fury_at < hd_at,
        "FE_RC_FURY should be the first campaignList entry, matching the measured default"
    );
}

/// `Grid Selection`'s own `flyerlist` names `grid0`..`grid7`; `Grid Selection
/// Fury`'s own names `grid8`..`grid15` - the 0/8 and 8/16 split
/// `oag_hd::campaign::HD_GRID_RANGE`/`FURY_GRID_RANGE` and
/// `crate::campaign_stage::CampaignStage::open_grid_selection` (in
/// `oag-game`) both assume.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn each_grid_screens_own_flyerlist_matches_its_campaigns_grid_range() {
    let Some(mut archives) = opened() else {
        return;
    };
    let text = data06_screen_text(&mut archives);
    let grid_selection_at = text
        .find(r#"name="Grid Selection""#)
        .expect("Grid Selection should be in the text");
    let grid_selection_fury_at = text
        .find(r#"name="Grid Selection Fury""#)
        .expect("Grid Selection Fury should be in the text");
    let cell_selection_at = text
        .find(r#"name="Cell Selection""#)
        .expect("Cell Selection should be in the text");
    assert!(grid_selection_at < grid_selection_fury_at);
    assert!(grid_selection_fury_at < cell_selection_at);

    let base_block = &text[grid_selection_at..grid_selection_fury_at];
    let fury_block = &text[grid_selection_fury_at..cell_selection_at];
    for index in oag_hd::campaign::HD_GRID_RANGE {
        let name = format!(r#""grid{index}""#);
        assert!(
            base_block.contains(&name),
            "Grid Selection's own flyerlist should name {name}"
        );
    }
    for index in oag_hd::campaign::FURY_GRID_RANGE {
        let name = format!(r#""grid{index}""#);
        assert!(
            fury_block.contains(&name),
            "Grid Selection Fury's own flyerlist should name {name}"
        );
    }
}

/// `DATA06`'s own `Cell Selection`'s `Target0/1/2 Medal` widgets author the
/// crop `oag_ui::campaign::hd::hd_medal_frame` reads for `DATA02`'s own
/// `Medal_{x}_{y}` (a different widget on a different archive's copy of the
/// screen, but the same shared `Hexmedal_HD` atlas) - the only place on
/// either disc that authors a `u`/`v`/`TxtrWidth`/`TxtrHeight` crop of this
/// texture at all, so it is the ground truth `hd_medal_frame`'s own doc
/// cites. Pins the exact attributes so a re-extraction of the disc, not
/// just this test file, is what could move these numbers.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn target_medal_widgets_author_the_hexmedal_atlas_crop_hd_medal_frame_reads() {
    let Some(mut archives) = opened() else {
        return;
    };
    let text = data06_screen_text(&mut archives);
    let cases = [
        (r#"name="Target0 Medal""#, r#"v="0""#),
        (r#"name="Target1 Medal""#, r#"v="61""#),
        (r#"name="Target2 Medal""#, r#"v="122""#),
    ];
    for (widget, v) in cases {
        let at = text
            .find(widget)
            .unwrap_or_else(|| panic!("{widget} should be on Cell Selection"));
        // The widget's own `<Values>` sits a short, bounded distance after
        // its opening tag - slicing a window rather than searching the
        // whole rest of the file keeps this from matching a later widget's
        // own `v="..."` by accident.
        let window = &text[at..(at + 200).min(text.len())];
        assert!(
            window.contains(r#"width="60" height="60" u="0""#),
            "{widget} should crop a 60x60 frame at u=0, got: {window}"
        );
        assert!(
            window.contains(v),
            "{widget} should author {v}, got: {window}"
        );
        assert!(
            window.contains(r#"TxtrWidth="60" TxtrHeight="60""#),
            "{widget} should sample a 60x60 source rect, got: {window}"
        );
        assert!(
            window.contains(r"Data\FE\Images\Hexmedal_HD.gtf"),
            "{widget} should source Hexmedal_HD, got: {window}"
        );
    }
}
