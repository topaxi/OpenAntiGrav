//! The `HUD Style` setting, on the discs that author it: a race of Wipeout HD / Fury and
//! of Omega loads the skin the setting names - its root layouts, both split cuts, and its
//! own typeface - and nothing else.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this project does not
//! ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-raceplay --run-ignored all \
//!     -E 'test(/hud_style_ground_truth::/)'
//! ```
//!
//! Each claim is one that a dropped wiring breaks: an `Options::hud_style` nothing reads
//! draws the default skin in every case, and a face looked up in the first copy of the
//! language definition draws 5x7 (HD's `2097HUD` slot is in `DATA03`'s copy alone).

use oag_raceplay as race;

const RETRO_BAR: &str = "SpeedBarWo3";
/// The 5x7 built-in face's line height, which a face that failed to load falls back to.
const FALLBACK_LINE: f32 = 8.0;

struct Title {
    title: &'static oag_title::Title,
    source: &'static str,
}

const HD: Title = Title {
    title: oag_hd::TITLE,
    source: "data/images/hdfury-ps3-eu-dec.iso",
};

fn load(title: &Title, style: Option<&str>, views: u8) -> Option<race::Loaded> {
    let source = if title.title.name == oag_omega::TITLE.name {
        oag_testdata::exact(title.source)?
    } else {
        oag_testdata::image(title.source)?
    };
    Some(
        race::load(&race::Options {
            source: source.display().to_string(),
            class: "VENOM".to_string(),
            mode: oag_race::Mode::SingleRace,
            hud_style: style.map(str::to_string),
            views,
            ..race::Options::default()
        })
        .expect("loading the race"),
    )
}

fn omega() -> Title {
    Title {
        title: oag_omega::TITLE,
        source: "data/extracted/ps4",
    }
}

fn names(title: &Title) -> Vec<&'static str> {
    title.title.hud.skins.iter().map(|skin| skin.name).collect()
}

/// A skin's race: its own bars are in the layout, its face is its own and loaded, and the
/// default skin has neither.
fn check_single_screen(title: &Title) {
    let mut faces = Vec::new();
    for skin in title.title.hud.skins {
        let Some(loaded) = load(title, Some(skin.name), 0) else {
            return;
        };
        let layout = loaded.hud.layout.as_ref().expect("a HUD layout");
        let retro = skin.font_role.is_some();
        assert_eq!(
            layout.sprite(RETRO_BAR).is_some(),
            retro,
            "{} {}: the retro speed bar is authored by the retro skins alone",
            title.title.name,
            skin.name
        );
        assert!(
            loaded
                .report
                .iter()
                .any(|line| line == &format!("HUD Style: {}", skin.name)),
            "{} {}: the report names the skin, {:?}",
            title.title.name,
            skin.name,
            loaded.report
        );
        if retro {
            let face = loaded.hud.skin_font.line_height;
            assert!(
                (face - FALLBACK_LINE).abs() > f32::EPSILON
                    && (face - loaded.hud.font.line_height).abs() > f32::EPSILON,
                "{} {}: the skin's own face loaded, not 5x7 and not the HUD face ({face})",
                title.title.name,
                skin.name
            );
            faces.push(face);
        }
    }
    assert_eq!(
        faces.len(),
        2,
        "two retro skins, each with a face of its own"
    );
}

/// The two-view race reads the skin's own split roots in both cuts.
fn check_split(title: &Title) {
    for skin in title.title.hud.skins {
        let Some(loaded) = load(title, Some(skin.name), 2) else {
            return;
        };
        let wanted = skin.layouts.split.expect("every skin authors a split");
        let rows = loaded.hud.split.as_ref().expect("the top-and-bottom pair");
        let columns = loaded
            .hud
            .split_columns
            .as_ref()
            .expect("the left-and-right pair");
        assert_eq!(rows.layouts.len(), 2, "{} {}", title.title.name, skin.name);
        assert_eq!(
            columns.layouts.len(),
            2,
            "{} {}",
            title.title.name,
            skin.name
        );
        for root in wanted
            .arcade
            .iter()
            .chain(&wanted.vertical.expect("vert").arcade)
        {
            assert!(
                loaded
                    .report
                    .iter()
                    .any(|line| line.starts_with(&format!("HUD split {root}:"))),
                "{} {}: {root} composed, {:?}",
                title.title.name,
                skin.name,
                loaded.report
            );
        }
    }
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn hd_races_each_skin_the_setting_names() {
    check_single_screen(&HD);
}

#[test]
#[ignore = "needs the decrypted PS4 package pair in data/extracted/ps4/"]
fn omega_races_each_skin_the_setting_names() {
    check_single_screen(&omega());
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn hd_split_screen_reads_the_skins_own_roots_in_both_cuts() {
    check_split(&HD);
}

#[test]
#[ignore = "needs the decrypted PS4 package pair in data/extracted/ps4/"]
fn omega_split_screen_reads_the_skins_own_roots_in_both_cuts() {
    check_split(&omega());
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_name_the_title_does_not_list_draws_the_default_skin_and_says_so() {
    let Some(loaded) = load(&HD, Some("nonsense"), 0) else {
        return;
    };
    let layout = loaded.hud.layout.as_ref().expect("a HUD layout");
    assert!(layout.sprite(RETRO_BAR).is_none());
    assert!(
        loaded
            .report
            .iter()
            .any(|line| line.contains("\"nonsense\" is not in this title's list")),
        "{:?}",
        loaded.report
    );
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_list_the_title_names_is_the_discs_own_hud_style_list() {
    // The disc's `additional_definition.xml` authors `HUD Style` with these entries, default
    // first; the title packages carry them as `HudLayouts::skins`.
    let Some(image) = oag_testdata::image(HD.source) else {
        return;
    };
    let mut archives = oag_hd::open(&image.display().to_string()).expect("HD opens");
    // The list is in `DATA03`'s copy alone (the first copy `read_name` returns lacks it), so
    // every copy is read and the one that authors it is the answer.
    let xml = archives
        .read_every_name("/data/plugins/frontend/gui/additional_definition.xml")
        .into_iter()
        .filter_map(|(_, blob)| oag_ui::xml::expand(&blob).ok())
        .find(|xml| xml.contains("name=\"HUD Style\""))
        .expect("a copy that authors the HUD Style list");
    let at = xml.find("name=\"HUD Style\"").expect("a HUD Style list");
    let list = &xml[at..xml[at..].find("</List>").map_or(xml.len(), |end| at + end)];
    let entries: Vec<&str> = list
        .split("<Entry string=\"")
        .skip(1)
        .filter_map(|entry| entry.split('"').next())
        .collect();
    assert_eq!(entries, names(&HD));
    assert_eq!(names(&omega()), names(&HD), "Omega authors the same list");
}
