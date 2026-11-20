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

/// One `vwipe` Image: the file, the screens it sits under (outermost first), its name.
#[derive(Debug, Clone, PartialEq)]
struct Found {
    /// The file's stem and the archive copy it was read from.
    file: String,
    copy: String,
    screens: Vec<String>,
    name: String,
    wipe: Vwipe,
}

fn walk(
    node: &fexml::Node,
    screens: &Screens,
    (file, copy): (&str, &str),
    chain: &mut Vec<String>,
    out: &mut Vec<Found>,
) {
    if node.name.eq_ignore_ascii_case("Image")
        && let Some(wipe) = screens.vwipe_from_node(node)
    {
        out.push(Found {
            file: file.to_string(),
            copy: copy.to_string(),
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
        walk(child, screens, (file, copy), chain, out);
    }
    if named {
        chain.pop();
    }
}

/// Every `vwipe` Image of every copy of every front-end XML the archives hold, each tagged with
/// its file stem and the archive copy it sits in. The copies are listed from the archives' own
/// path lists, not a list kept here.
fn census_of(archives: &mut oag_assets::Archives) -> Vec<Found> {
    use oag_assets::Container;
    let mut paths: Vec<String> = Vec::new();
    let mut containers: Vec<&Container> = vec![&archives.data];
    containers.extend(archives.patch.iter());
    containers.extend(archives.fe.iter());
    containers.extend(archives.extra.iter());
    for container in containers {
        if let Container::Psarc(psarc) = container {
            paths.extend(
                psarc
                    .paths()
                    .iter()
                    .map(|path| path.to_ascii_lowercase())
                    .filter(|path| path.ends_with(".xml") && path.contains("frontend/gui/")),
            );
        }
    }
    paths.sort();
    paths.dedup();
    let screens = Screens::from_xml_with_fallback_globals("", &[]);
    let mut out = Vec::new();
    for path in &paths {
        let stem = path
            .rsplit('/')
            .next()
            .unwrap_or(path)
            .trim_end_matches(".xml");
        for (copy, raw) in archives.read_every_name(path) {
            if !raw.windows(5).any(|w| w.eq_ignore_ascii_case(b"vwipe")) {
                continue;
            }
            let Ok(xml) = fexml::text(&raw) else { continue };
            let copy = copy.rsplit(['/', ':']).next().unwrap_or(&copy).to_string();
            walk(
                &fexml::parse(&xml),
                &screens,
                (stem, &copy),
                &mut Vec::new(),
                &mut out,
            );
        }
    }
    out
}

/// Fury's update chain over the disc, as the original ran it.
fn census() -> Option<Vec<Found>> {
    let chain = oag_testdata::exact(FURY)?;
    let image = oag_testdata::image(PS3_IMAGE)?;
    let update = UpdateFolder {
        serial: "BCES-00664".to_string(),
        root: format!("{}/installed", chain.display()),
    };
    let mut archives =
        oag_hd::open_updated(&image.to_string_lossy(), &[update]).expect("the disc opens");
    Some(census_of(&mut archives))
}

#[test]
#[ignore = "scratch sweep: prints every vwipe Image of every copy; OAG_SWEEP=1"]
fn sweep() {
    if std::env::var_os("OAG_SWEEP").is_none() {
        return;
    }
    for f in census().expect("images") {
        eprintln!(
            "SWEEP {} | {} | {:?} | {} | {:?}",
            f.file, f.copy, f.screens, f.name, f.wipe
        );
    }
}

/// `ceil(delay * 60)`: the frame a 60 Hz countdown crosses 0 on, with the slack an `f32` product
/// of an authored decimal needs.
fn frames(wipe: Vwipe) -> u32 {
    (wipe.delay * 60.0 - 1e-3).ceil() as u32
}

/// The number in an archive's name (`DATA06.PSARC`, `data08.psarc`): a later one is served before
/// an earlier one (the update archives over every disc copy, `DATA06` over `DATA02`).
fn archive_number(copy: &str) -> u32 {
    copy.chars()
        .filter(char::is_ascii_digit)
        .collect::<String>()
        .parse()
        .unwrap_or(0)
}

/// What the page's `vwipe` Images hold in the copy of each file that is served (the
/// highest-numbered archive that authors the screen), `(name, frames)` sorted. A screen authored by
/// two files must read the same in both.
fn page(found: &[Found], screen: &str) -> Vec<(String, u32)> {
    let mut files: Vec<&str> = found
        .iter()
        .filter(|f| f.screens.iter().any(|s| s == screen))
        .map(|f| f.file.as_str())
        .collect();
    files.sort_unstable();
    files.dedup();
    let rows = |file: &str| {
        let on_page = |f: &&Found| f.file == file && f.screens.iter().any(|s| s == screen);
        let served = found
            .iter()
            .filter(on_page)
            .map(|f| archive_number(&f.copy))
            .max();
        let mut out: Vec<(String, u32)> = found
            .iter()
            .filter(on_page)
            .filter(|f| Some(archive_number(&f.copy)) == served)
            .map(|f| (f.name.clone(), frames(f.wipe)))
            .collect();
        out.sort();
        out
    };
    let first = files.first().map(|file| rows(file)).unwrap_or_default();
    for file in &files {
        assert_eq!(rows(file), first, "{screen} in {file}");
    }
    first
}

fn expect(rows: &[(&str, u32)]) -> Vec<(String, u32)> {
    let mut out: Vec<(String, u32)> = rows.iter().map(|(n, f)| (n.to_string(), *f)).collect();
    out.sort();
    out
}

/// The frames at 60 Hz each page's Images waited after the page became current, read in RPCS3's
/// RAM (`image-widget.md`, "What RAM shows"): the countdown `+0xa0` held the authored `delay`
/// and the ramp started the frame it crossed 0. A widget RAM names `Item N` authors no name.
/// Each row is `(screen, [(XML name, frames)])`; Cell Selection's emblems wait 3, 8, 12 and 17
/// frames after their 0.37 s hold.
const MEASURED: [(&str, &[(&str, u32)]); 10] = [
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
    (
        "Grid Selection Fury",
        &[("unlockboxbrackets", 27), ("flyerlogo", 39)],
    ),
    (
        "Cell Selection",
        &[
            ("flyerlogo", 48),
            ("Event Emblem", 3),
            ("Track Emblem", 8),
            ("Speed Class Emblem", 12),
            ("Weapons Emblem", 17),
        ],
    ),
    (
        "Show Unlocks Root",
        &[("closed", 6), ("dots", 6), ("open", 6)],
    ),
    ("SoundTest", &[("", 12)]),
];

/// Pages RAM never entered: their delays are read at rest, so they are authored by the served
/// file but inherited, not measured, as page entries.
const NOT_ENTERED: [&str; 2] = ["Show Unlocks Root", "SoundTest"];

/// Each page's authored `(name, delay)` set, in the copy the original serves, equals what RAM
/// read at entry, per widget name and frame count. `Grid Selection Fury` is read from `DATA06`'s
/// `CellMode_Definition.xml`, the only copy that authors it. This fails if the `delay` key is no
/// longer read.
#[test]
#[ignore = "needs the disc image and Fury's update chain in data/"]
fn every_measured_page_authors_the_delays_ram_showed() {
    let Some(found) = census() else { return };
    for (screen, rows) in MEASURED {
        assert_eq!(page(&found, screen), expect(rows), "{screen}");
    }
    let fury: Vec<&str> = found
        .iter()
        .filter(|f| f.screens.iter().any(|s| s == "Grid Selection Fury"))
        .map(|f| f.copy.as_str())
        .collect();
    assert!(
        fury.iter()
            .all(|copy| copy.eq_ignore_ascii_case("DATA06.PSARC")),
        "only DATA06 authors Grid Selection Fury: {fury:?}"
    );
    for screen in NOT_ENTERED {
        assert!(MEASURED.iter().any(|(s, _)| *s == screen));
    }
}

/// Where the XML authors a `vwipe` Image that no page entry in RAM showed, named so a change on
/// either side fails: `racebox_definition.xml` holds three outside any `Screen` (unnamed 0.45 s,
/// `WeaponBG` 0.5 s, its `WeaponIcon` 0.55 s: the 5 of the XML against RAM's 2 under `Single
/// Player`), and the copies the update chain superseded still carry an unnamed 0.5 s image
/// (in 0.4 s) on `Cell Selection` in `DATA02` and an unnamed 0.3 s on `Team Selection` in
/// `selection_definition.xml`, which the served `DATA06` files no longer author.
#[test]
#[ignore = "needs the disc image and Fury's update chain in data/"]
fn the_xml_authors_widgets_ram_did_not_show() {
    let Some(found) = census() else { return };
    let loose: Vec<(&str, u32)> = found
        .iter()
        .filter(|f| {
            f.file == "racebox_definition"
                && archive_number(&f.copy) == 6
                && (f.screens.is_empty() || f.screens == ["WeaponBG"])
        })
        .map(|f| (f.name.as_str(), frames(f.wipe)))
        .collect();
    assert_eq!(loose, [("", 27), ("WeaponBG", 30), ("WeaponIcon", 33)]);
    let stale = |file: &str, screen: &str| {
        found
            .iter()
            .filter(|f| {
                f.file == file
                    && f.name.is_empty()
                    && f.screens.last().is_some_and(|s| s == screen)
                    && archive_number(&f.copy) == 2
            })
            .map(|f| frames(f.wipe))
            .collect::<Vec<_>>()
    };
    assert_eq!(stale("cellmode_definition", "Cell Selection"), [30]);
    assert_eq!(stale("selection_definition", "Team Selection"), [18]);
    assert_eq!(stale("selection_definition", "Track Creation"), [15]);
}

/// Omega's front end is HD's: its own served files author the same delays on the pages RAM
/// measured (inherited by lineage, not measured on Omega), except where its screens are
/// re-authored: Race Records' six images sit in a `logos_group` under `Race Records Base`, Track
/// Select adds a `TrackPreview` at 0.4 s.
#[test]
#[ignore = "needs data/images/omega-ps4-eu.pkg"]
fn omega_authors_the_delays_ram_showed_on_hd_where_its_screens_are_hds() {
    let Some(pkg) = oag_testdata::image("data/images/omega-ps4-eu.pkg") else {
        return;
    };
    let mut archives = oag_omega::open(&pkg.display().to_string()).expect("Omega opens");
    let found = census_of(&mut archives);
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
            _ => assert_eq!(page(&found, screen), expect(rows), "{screen}"),
        }
    }
}
