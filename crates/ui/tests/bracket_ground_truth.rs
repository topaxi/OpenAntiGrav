//! The `Bracket` widgets of HD's Cell Selection, Team Selection and Track Select pages, and Omega's
//! copies of the same three, as the files author them: how many, what delay, and which widget each
//! is written inside (`docs/ghidra/functions/ps3-hdfury-eu/image-widget.md`, "The corner
//! brackets"). The wiring that opens them after their delay is tested in `oag-ui-screens`.
//!
//! `#[ignore]`d: needs the disc image and Fury's update chain, and Omega's package, under `data/`.
//! `just test-data`.

use oag_assets::UpdateFolder;
use oag_tables::fexml;
use oag_ui::screen::Screens;

const FURY: &str = "data/extracted/ps3-update-chain/BCES00664";
const PS3_IMAGE: &str = "hdfury-ps3-eu-dec.iso";

const CELL: &str = r"Data\Plugins\Frontend\Gui\CellMode_Definition.xml";
const TEAM: &str = r"Data\Plugins\Frontend\Gui\Team_Selection_Definition.xml";
const TRACK: &str = r"Data\Plugins\Frontend\Gui\Track_Selection_Definition.xml";

/// The number in an archive's name: a later one is served before an earlier one.
fn archive_number(copy: &str) -> u32 {
    copy.chars()
        .filter(char::is_ascii_digit)
        .collect::<String>()
        .parse()
        .unwrap_or(0)
}

/// One bracket: the widget it is written inside and its authored `delay`, in milliseconds.
type Row = (Option<String>, u32);

/// The brackets of `screen` in the served copy of `file`: the highest-numbered archive that
/// authors the screen. Sorted.
fn brackets(archives: &mut oag_assets::Archives, file: &str, screen: &str) -> Vec<Row> {
    let mut copies = archives.read_every_name(file);
    copies.sort_by_key(|(copy, _)| archive_number(copy.rsplit(['/', ':']).next().unwrap_or(copy)));
    for (_, raw) in copies.iter().rev() {
        let Ok(xml) = fexml::text(raw) else { continue };
        let screens = Screens::from_xml_with_fallback_globals(&xml, &[]);
        let Some(found) = screens.by_name(screen) else {
            continue;
        };
        assert!(
            found.brackets.iter().all(|bracket| bracket.corner),
            "{screen}: every bracket is the corner form"
        );
        let mut rows: Vec<Row> = found
            .brackets
            .iter()
            .map(|bracket| {
                (
                    bracket.group.clone(),
                    (bracket.delay * 1000.0).round() as u32,
                )
            })
            .collect();
        rows.sort();
        return rows;
    }
    panic!("no copy of {file} authors {screen}");
}

fn rows(list: &[(Option<&str>, u32)]) -> Vec<Row> {
    let mut out: Vec<Row> = list
        .iter()
        .map(|(group, ms)| (group.map(str::to_string), *ms))
        .collect();
    out.sort();
    out
}

/// What the pages author, shared by HD and Omega where Omega's file is the same: Team Selection's
/// two panel frames and the ship frame, Track Select's circuit panel, hex selector and model frame.
fn check_pages(archives: &mut oag_assets::Archives) {
    assert_eq!(
        brackets(archives, TEAM, "Team Selection Top Level"),
        rows(&[(None, 200), (None, 200), (None, 900)])
    );
    assert_eq!(
        brackets(archives, TRACK, "TrackSelectionTopLevel"),
        rows(&[(None, 200), (None, 300), (None, 1000)])
    );
}

#[test]
#[ignore = "needs the disc image and Fury's update chain in data/"]
fn hd_authors_fifteen_cell_brackets_and_three_each_on_team_and_track() {
    let Some(chain) = oag_testdata::exact(FURY) else {
        return;
    };
    let Some(image) = oag_testdata::image(PS3_IMAGE) else {
        return;
    };
    let update = UpdateFolder {
        serial: "BCES-00664".to_string(),
        root: format!("{}/installed", chain.display()),
    };
    let mut archives =
        oag_hd::open_updated(&image.to_string_lossy(), &[update]).expect("the disc opens");
    check_pages(&mut archives);
    assert_eq!(brackets(&mut archives, CELL, "Cell Selection"), cell_rows());
}

/// Each emblem row has a bracket on the icon and one on the text, `RightColumnText` one frame, and
/// the three target rows two each. The emblems and the column are the widgets the cell refresh
/// re-arms; the target rows are armed at the page's entry alone.
fn cell_rows() -> Vec<Row> {
    rows(&[
        (Some("Event Emblem"), 100),
        (Some("Event Emblem"), 100),
        (Some("Track Emblem"), 200),
        (Some("Track Emblem"), 200),
        (Some("Speed Class Emblem"), 250),
        (Some("Speed Class Emblem"), 250),
        (Some("Weapons Emblem"), 325),
        (Some("Weapons Emblem"), 325),
        (Some("Target Title"), 625),
        (Some("Target Title"), 625),
        (Some("Target Title"), 650),
        (Some("Target Title"), 650),
        (Some("Target Title"), 675),
        (Some("Target Title"), 675),
        (Some("RightColumnText"), 300),
    ])
}

/// Omega ships the same Team Selection and Track Select brackets, and its Cell Selection authors
/// the same rows inside the same widgets.
#[test]
#[ignore = "needs data/images/omega-ps4-eu.pkg"]
fn omega_authors_the_same_brackets_inside_the_same_widgets() {
    let Some(pkg) = oag_testdata::image("data/images/omega-ps4-eu.pkg") else {
        return;
    };
    let mut archives = oag_omega::open(&pkg.display().to_string()).expect("Omega opens");
    assert_eq!(brackets(&mut archives, CELL, "Cell Selection"), cell_rows());
    check_pages(&mut archives);
}
