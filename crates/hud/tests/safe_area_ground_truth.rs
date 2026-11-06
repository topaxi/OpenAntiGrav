//! The SCREEN SIZE row (`[graphics] hud_safe_area`) against the discs.
//!
//! `#[ignore]`d: needs real images under `data/images/`. `just test-data`.
//!
//! HD's `additional_definition.xml` authors the slider; every title's `HudArt::safe_area` is the
//! range its row offers. This reads the slider off every copy of the file and holds HD's data to
//! it, then reads the other titles' front-end XML for the same option and finds none, so the
//! shared slider they carry stands in for nothing the disc authors.

use oag_tables::fexml::{self, Node};
use oag_title::{Origin, hud::SafeArea};

const WORDS: [&str; 4] = [
    "Safe Area",
    "Screen Size",
    "OPT_SCREEN_SIZE",
    "OPT_SAFE_AREA",
];

fn find<'a>(node: &'a Node, name: &str) -> Option<&'a Node> {
    if node.name.eq_ignore_ascii_case("Slider") && node.attr("name") == Some(name) {
        return Some(node);
    }
    node.children.iter().find_map(|child| find(child, name))
}

fn number(slider: &Node, key: &str) -> u8 {
    slider
        .value(key)
        .unwrap_or_else(|| panic!("the slider has {key}"))
        .trim_end_matches('%')
        .parse()
        .unwrap_or_else(|_| panic!("{key} is a number"))
}

fn slider_label(slider: &Node) -> Option<&str> {
    std::iter::once(slider)
        .chain(slider.children.iter())
        .flat_map(|node| node.attrs.iter())
        .map(|(_, value)| value.as_str())
        .find(|value| value.starts_with("OPT_"))
}

/// Every word of [`WORDS`] the XML `blob` contains.
fn mentions(blob: &[u8]) -> Vec<&'static str> {
    let xml = fexml::text(blob).expect("front-end XML is text");
    WORDS.into_iter().filter(|word| xml.contains(word)).collect()
}

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn hds_slider_is_the_range_the_row_offers_in_every_copy_of_the_file() {
    let Some(image) = oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let mut archives = oag_hd::open(&image.display().to_string()).expect("HD opens");
    let copies = archives.read_every_name("/data/plugins/frontend/gui/additional_definition.xml");
    assert!(
        copies.iter().any(|(label, _)| label.contains("DATA06")),
        "DATA06's copy is the live Game Options screen: {:?}",
        copies.iter().map(|(label, _)| label).collect::<Vec<_>>()
    );
    let art = oag_hd::TITLE.hud_art.safe_area.expect("HD offers the slider");
    assert_eq!(art.origin, Origin::Measured);
    for (label, blob) in &copies {
        let root = fexml::parse(&fexml::text(blob).expect("text"));
        let slider = find(&root, "Safe Area Setting").unwrap_or_else(|| panic!("{label}: no slider"));
        let authored = SafeArea {
            default_percent: number(slider, "default"),
            min_percent: number(slider, "minSlide"),
            max_percent: number(slider, "maxSlide"),
            step_percent: number(slider, "incSlide"),
        };
        assert_eq!(authored, art.value, "{label}");
        assert!(
            slider_label(slider) == Some("OPT_SCREEN_SIZE"),
            "{label}: labelled OPT_SCREEN_SIZE"
        );
        assert_eq!(
            authored.values().collect::<Vec<_>>(),
            (80..=100).step_by(2).collect::<Vec<u8>>(),
            "{label}: the row's values"
        );
    }
}

#[test]
#[ignore = "needs the Pulse discs in data/images/"]
fn pulse_authors_no_screen_size_and_takes_the_shared_slider() {
    let art = oag_pulse::TITLE.hud_art.safe_area.expect("Pulse offers the row");
    assert_eq!((art.value, art.origin), (SafeArea::SHARED, Origin::Chosen));
    for image in [
        "data/images/pulse-psp-eu.chd",
        "data/images/pulse-psp-usa.chd",
        "data/images/pulse-ps2-eu.chd",
    ] {
        let Some(path) = oag_testdata::image(image) else {
            continue;
        };
        let mut archives = oag_assets::Archives::open(&path.to_string_lossy(), oag_pulse::TITLE)
            .expect("the archives open");
        for file in [
            "Additional_Definition",
            "Skin",
            "MainMenu_Definition",
            "Selection_Definition",
            "InGame_Definition",
            "EndRace_Definition",
            "RaceBox_Definition",
        ] {
            let blob = archives
                .read_name(&format!(r"Data\Plugins\PI001\GUI\{file}.xml"))
                .unwrap_or_else(|e| panic!("{image}: {file}: {e}"));
            assert_eq!(mentions(&blob), Vec::<&str>::new(), "{image}: {file}");
        }
    }
}

#[test]
#[ignore = "needs the Pure discs in data/images/"]
fn pure_authors_no_screen_size_and_takes_the_shared_slider() {
    let art = oag_pure::TITLE.hud_art.safe_area.expect("Pure offers the row");
    assert_eq!((art.value, art.origin), (SafeArea::SHARED, Origin::Chosen));
    for image in ["data/images/pure-psp-eu.chd", "data/images/pure-psp-usa.chd"] {
        let Some(path) = oag_testdata::image(image) else {
            continue;
        };
        let mut archives = oag_assets::Archives::open(&path.to_string_lossy(), oag_pure::TITLE)
            .expect("the archives open");
        for file in ["Skin", "Selection_Definition"] {
            let blob = archives
                .read_name(&format!(r"Data\Plugins\PI001\GUI\{file}.xml"))
                .unwrap_or_else(|e| panic!("{image}: {file}: {e}"));
            assert_eq!(mentions(&blob), Vec::<&str>::new(), "{image}: {file}");
        }
    }
}

#[test]
#[ignore = "needs data/images/omega-ps4-eu.pkg"]
fn omega_authors_no_screen_size_and_takes_the_shared_slider() {
    let art = oag_omega::TITLE.hud_art.safe_area.expect("Omega offers the row");
    assert_eq!((art.value, art.origin), (SafeArea::SHARED, Origin::Chosen));
    let Some(pkg) = oag_testdata::image("data/images/omega-ps4-eu.pkg") else {
        return;
    };
    let mut archives = oag_omega::open(&pkg.display().to_string()).expect("Omega opens");
    for file in [
        "additional_definition",
        "mainmenu_definition",
        "racebox_definition",
        "skin",
    ] {
        let copies = archives.read_every_name(&format!("/data/plugins/frontend/gui/{file}.xml"));
        assert!(!copies.is_empty(), "{file} ships");
        for (label, blob) in copies {
            assert_eq!(mentions(&blob), Vec::<&str>::new(), "{label}: {file}");
        }
    }
}

#[test]
#[ignore = "needs the extracted Vita package in data/extracted/vita/"]
fn wipeout_2048_authors_no_screen_size_and_takes_the_shared_slider() {
    let art = oag_2048::TITLE.hud_art.safe_area.expect("2048 offers the row");
    assert_eq!((art.value, art.origin), (SafeArea::SHARED, Origin::Chosen));
    let Some(dir) = oag_testdata::exact("data/extracted/vita/PCSF00007") else {
        return;
    };
    let mut archives = oag_2048::open(&dir.display().to_string()).expect("2048 opens");
    for file in [
        "Definition",
        "Options_Definition",
        "Bootup_Definition_EU",
        "Team_Definition",
        "Profile_Definition",
        "Community_Definition",
        "Skin",
    ] {
        let blob = archives
            .read_name(&format!("Data/plugins/frontend/NEWGUI/{file}.xml"))
            .unwrap_or_else(|e| panic!("{file}: {e}"));
        assert_eq!(mentions(&blob), Vec::<&str>::new(), "{file}");
    }
}
