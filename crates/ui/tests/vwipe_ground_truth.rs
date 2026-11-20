//! HD's `vwipe` Images: the authored `delay` of each, against the delays read in RPCS3's RAM
//! (`docs/ghidra/functions/ps3-hdfury-eu/image-widget.md`, "Page entry").
//!
//! `#[ignore]`d: needs the disc image and Fury's update chain under `data/`. `just test-data`.

use oag_assets::UpdateFolder;
use oag_tables::fexml;
use oag_ui::screen::{Screens, Vwipe};

const FURY: &str = "data/extracted/ps3-update-chain/BCES00664";
const PS3_IMAGE: &str = "hdfury-ps3-eu-dec.iso";
const GUI: &str = r"Data\Plugins\Frontend\Gui";

/// Every XML the front end loads its screens from, by stem.
const FILES: [&str; 17] = [
    "skin",
    "racebox_definition",
    "recordgrid_definition",
    "cellmode_definition",
    "additional_definition",
    "multiplayer_definition",
    "controls_definition_ps3",
    "credits_definition",
    "credits_text_eu",
    "online_definition",
    "network_definition",
    "showunlocks_definition",
    "selection_definition",
    "stats_definition",
    "team_selection_definition",
    "track_selection_definition",
    "manual_definition_eu",
];

/// One `vwipe` Image: the file, the screens it sits under (outermost first), its name.
#[derive(Debug, Clone, PartialEq)]
struct Found {
    file: &'static str,
    screens: Vec<String>,
    name: String,
    wipe: Vwipe,
}

fn walk(
    node: &fexml::Node,
    screens: &Screens,
    file: &'static str,
    chain: &mut Vec<String>,
    out: &mut Vec<Found>,
) {
    if node.name.eq_ignore_ascii_case("Image")
        && let Some(wipe) = screens.vwipe_from_node(node)
    {
        out.push(Found {
            file,
            screens: chain.clone(),
            name: node.attr("name").unwrap_or("").to_string(),
            wipe,
        });
    }
    let named = node.name.eq_ignore_ascii_case("Screen") || node.name.eq_ignore_ascii_case("Image");
    if named {
        chain.push(node.attr("name").unwrap_or("").to_string());
    }
    for child in &node.children {
        walk(child, screens, file, chain, out);
    }
    if named {
        chain.pop();
    }
}

/// Every `vwipe` Image of the front end as the title's archives serve it: the update chain's copy
/// of each file where it has one, the disc's otherwise.
fn census() -> Option<Vec<Found>> {
    let chain = oag_testdata::exact(FURY)?;
    let image = oag_testdata::image(PS3_IMAGE)?;
    let update = UpdateFolder {
        serial: "BCES-00664".to_string(),
        root: format!("{}/installed", chain.display()),
    };
    let mut archives =
        oag_hd::open_updated(&image.to_string_lossy(), &[update]).expect("the disc opens");
    Some(census_of(|file| {
        archives.read_name(&format!(r"{GUI}\{file}.xml")).ok()
    }))
}

/// [`census`] over any source of the front end's files by stem.
fn census_of(mut read: impl FnMut(&str) -> Option<Vec<u8>>) -> Vec<Found> {
    let screens = Screens::from_xml_with_fallback_globals("", &[]);
    let mut out = Vec::new();
    for file in FILES {
        let Some(raw) = read(file) else {
            continue;
        };
        let xml = fexml::text(&raw).expect("the file is XML");
        walk(
            &fexml::parse(&xml),
            &screens,
            file,
            &mut Vec::new(),
            &mut out,
        );
    }
    out
}

/// `ceil(delay * 60)`: the frame a 60 Hz countdown crosses 0 on, with the slack an `f32` product
/// of an authored decimal needs.
fn frames(wipe: Vwipe) -> u32 {
    (wipe.delay * 60.0 - 1e-3).ceil() as u32
}

/// What the page's `vwipe` Images hold in the census, `(name, frames)` sorted.
fn page(found: &[Found], screen: &str) -> Vec<(String, u32)> {
    let mut out: Vec<(String, u32)> = found
        .iter()
        .filter(|f| f.screens.last().is_some_and(|s| s == screen))
        .map(|f| (f.name.clone(), frames(f.wipe)))
        .collect();
    out.sort();
    out
}

fn expect(rows: &[(&str, u32)]) -> Vec<(String, u32)> {
    let mut out: Vec<(String, u32)> = rows.iter().map(|(n, f)| (n.to_string(), *f)).collect();
    out.sort();
    out
}

/// The frames at 60 Hz each page's Images waited after the page became current, read in RPCS3's
/// RAM (`image-widget.md`, "What RAM shows"): the countdown `+0xa0` held the authored `delay`
/// and the ramp started the frame it crossed 0. A widget RAM names `Item N` authors no name.
/// Each row is `(screen, [(XML name, frames)])`.
const MEASURED: [(&str, &[(&str, u32)]); 8] = [
    ("Controls", &[("", 15), ("", 42)]),
    ("Credits", &[("", 36), ("Logo", 48)]),
    (
        "Race Records",
        &[
            ("", 36),
            ("TaskLogo", 48),
            ("", 48),
            ("TrackLogo", 60),
            ("", 60),
            ("ClassLogo", 72),
        ],
    ),
    ("Single Player", &[("", 27), ("BigIcon", 48)]),
    ("TrackSelectionTopLevel", &[("Emblem", 36)]),
    ("Team Selection Top Level", &[("Logo", 30)]),
    ("Grid Selection", &[("unlockboxbrackets", 27)]),
    ("unlockbox", &[("flyerlogo", 39)]),
];

/// Fury's own `Team Selection` and `Track Select` files serve the Logo and the Emblem; the older
/// `selection_definition.xml` copies of those screens (an unnamed 0.3 s and 0.25 s image beside
/// them) are no longer served.
#[test]
#[ignore = "needs the disc image and Fury's update chain in data/"]
fn every_measured_page_authors_the_delays_ram_showed() {
    let Some(found) = census() else { return };
    for (screen, rows) in MEASURED {
        assert_eq!(page(&found, screen), expect(rows), "{screen}");
    }
    assert_eq!(
        page(&found, "Cell Selection")
            .into_iter()
            .filter(|(name, _)| name == "flyerlogo")
            .collect::<Vec<_>>(),
        expect(&[("flyerlogo", 48)]),
        "Cell Selection flyerlogo"
    );
}

/// Cell Selection's four emblems: RAM read 3, 8, 12 and 17 frames after the 0.37 s hold.
#[test]
#[ignore = "needs the disc image and Fury's update chain in data/"]
fn the_cell_emblems_author_the_delays_that_follow_the_hold() {
    let Some(found) = census() else { return };
    let emblems: Vec<(String, u32)> = page(&found, "Cell Selection")
        .into_iter()
        .filter(|(name, _)| name.ends_with(" Emblem"))
        .collect();
    assert_eq!(
        emblems,
        expect(&[
            ("Event Emblem", 3),
            ("Speed Class Emblem", 12),
            ("Track Emblem", 8),
            ("Weapons Emblem", 17),
        ])
    );
    assert!(
        found
            .iter()
            .filter(|f| f.name.ends_with(" Emblem")
                && f.screens.last().is_some_and(|s| s == "Cell Selection"))
            .all(|f| f.wipe.seconds == 0.3)
    );
}

/// Where the XML and RAM disagree, named so a change on either side fails. RAM's 71 `vwipe`
/// widgets against the XML's 76 (`image-widget.md`, "Page entry"): the files the update chain
/// does not serve any more are not counted here, and these widgets are authored where RAM did not
/// show one.
#[test]
#[ignore = "needs the disc image and Fury's update chain in data/"]
fn the_xml_authors_widgets_ram_did_not_show() {
    let Some(found) = census() else { return };
    let outside_any_screen: Vec<(&str, u32)> = found
        .iter()
        .filter(|f| {
            f.file == "racebox_definition" && f.screens.is_empty() || f.screens == ["WeaponBG"]
        })
        .map(|f| (f.name.as_str(), frames(f.wipe)))
        .collect();
    assert_eq!(
        outside_any_screen,
        [("", 27), ("WeaponBG", 30), ("WeaponIcon", 33)]
    );
    assert_eq!(
        page(&found, "Cell Selection")
            .into_iter()
            .filter(|(name, _)| name.is_empty())
            .collect::<Vec<_>>(),
        expect(&[("", 30)]),
        "an unnamed Cell Selection image (0.5 s delay, 0.4 s in) that RAM's five did not include"
    );
}

/// Pages RAM never entered: Show Unlocks and SoundTest play by the same law, so their authored
/// delays are inherited and unmeasured.
#[test]
#[ignore = "needs the disc image and Fury's update chain in data/"]
fn the_pages_ram_never_entered_author_their_delays() {
    let Some(found) = census() else { return };
    assert_eq!(
        page(&found, "Show Unlocks Root"),
        expect(&[("closed", 6), ("dots", 6), ("open", 6)])
    );
    assert_eq!(page(&found, "SoundTest"), expect(&[("", 12)]));
}

/// Omega's front end is HD's: its own copies of the same files author the same delays on the
/// pages RAM measured (inherited by lineage, not measured on Omega), except where its screens are
/// re-authored: Race Records' six images sit in a `logos_group` under `Race Records Base`, and
/// Track Select adds a `TrackPreview` at 0.4 s.
#[test]
#[ignore = "needs data/images/omega-ps4-eu.pkg"]
fn omega_authors_the_delays_ram_showed_on_hd_where_its_screens_are_hds() {
    let Some(pkg) = oag_testdata::image("data/images/omega-ps4-eu.pkg") else {
        return;
    };
    let mut archives = oag_omega::open(&pkg.display().to_string()).expect("Omega opens");
    let found = census_of(|file| {
        archives
            .read_name(&format!("/data/plugins/frontend/gui/{file}.xml"))
            .ok()
    });
    for (screen, rows) in MEASURED {
        match screen {
            "Race Records" => {
                assert_eq!(page(&found, "logos_group"), expect(rows), "{screen}");
            }
            "TrackSelectionTopLevel" => assert_eq!(
                page(&found, screen),
                expect(&[("Emblem", 36), ("TrackPreview", 24)]),
                "{screen}"
            ),
            "unlockbox" => assert!(
                page(&found, screen)
                    .iter()
                    .all(|row| *row == ("flyerlogo".to_string(), 39)),
                "Grid Selection, its Fury and its 2048 variants each author the flyerlogo at 39"
            ),
            _ => assert_eq!(page(&found, screen), expect(rows), "{screen}"),
        }
    }
}
