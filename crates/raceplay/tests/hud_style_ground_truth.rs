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
        // The default skin's runtime shield readings (HD's measured colours) do not draw over
        // a retro layout, whose shield text the layout authors transparent (`color="0"`).
        assert_eq!(
            loaded.hud.art.runtime.is_some(),
            !retro,
            "{} {}: the runtime shield readout is the default skin's",
            title.title.name,
            skin.name
        );
        if retro {
            let text = layout
                .label("ShieldBarText")
                .expect("the skin's shield text");
            assert!(
                text.color[3] == 0.0,
                "{} {}: color=\"0\" reads as transparent, not white ({:?})",
                title.title.name,
                skin.name,
                text.color
            );
        }
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

    // The `PI_HUD` rows gate each retro skin by loyalty (`HudSkin::loyalty`, offered unlocked
    // here until loyalty is shared across titles) and name its directory.
    let definition = archives
        .read_every_name("/data/plugins/frontend/definition.xml")
        .into_iter()
        .filter_map(|(_, blob)| oag_ui::xml::expand(&blob).ok())
        .find(|xml| xml.contains("<PI_HUD name="))
        .expect("a definition that authors the PI_HUD rows");
    for skin in HD.title.hud.skins.iter().filter(|skin| skin.loyalty != 0) {
        let at = definition
            .find(&format!("<PI_HUD name=\"{}\"", skin.name))
            .unwrap_or_else(|| panic!("a PI_HUD row for {}", skin.name));
        let row = &definition[at..at + definition[at..].find("</PI_HUD>").expect("closed")];
        assert!(
            row.contains(&format!("loyalty=\"{}\"", skin.loyalty)),
            "{}: {row}",
            skin.name
        );
        let root = skin.layouts.arcade.to_ascii_lowercase();
        let location = row.to_ascii_lowercase();
        let dir = root.split('/').nth(3).expect("a skin directory");
        assert!(
            location.contains(dir),
            "{}: {location} names {dir}",
            skin.name
        );
    }
    assert_eq!(names(&omega()), names(&HD), "Omega authors the same list");
}

/// Every root a skin names, single screen and both split cuts, composes whole on the source
/// (nothing missing or skipped) and the skin's font slot is declared by some copy of a language
/// definition. Run on all three sources that author the list, the plain PSN download
/// (`2097_hud` ships 23 files there against the disc's 27) included: a root missing there is no
/// HUD at all for that mode.
fn check_every_root(
    name: &str,
    mut archives: oag_assets::Archives,
    title: &Title,
    absent: &[&str],
) {
    let mut failed = Vec::new();
    for skin in title.title.hud.skins {
        let h = skin.layouts;
        let split = h.split.expect("a split row");
        let vertical = split.vertical.expect("a vertical row");
        let roots = [h.arcade, h.elimination, h.time_trial, h.speed_lap, h.zone]
            .into_iter()
            .chain(split.arcade)
            .chain(split.elimination)
            .chain(vertical.arcade)
            .chain(vertical.elimination);
        for root in roots {
            let whole = oag_hud::compose(root, |path| archives.read_name(path).ok())
                .is_some_and(|c| c.missing.is_empty() && c.layout.skipped.is_empty());
            if !whole && !failed.contains(&root) {
                failed.push(root);
            }
        }
        if let Some(role) = skin.font_role {
            let copies = oag_ui::language::load::definition_copies_with_font(
                &mut archives,
                title.title.front_end.expect("a front end").language_plugins,
                role,
            );
            assert!(
                !copies.is_empty(),
                "{name} {}: no definition declares {role}",
                skin.name
            );
        }
    }
    failed.sort_unstable();
    let mut absent = absent.to_vec();
    absent.sort_unstable();
    assert_eq!(failed, absent, "{name}: the roots that do not compose");
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_root_of_every_skin_composes_on_the_hd_disc() {
    let Some(image) = oag_testdata::image(HD.source) else {
        return;
    };
    let archives = oag_hd::open(&image.display().to_string()).expect("HD opens");
    check_every_root("HD disc", archives, &HD, &[]);
}

#[test]
#[ignore = "needs the extracted HD PSN download in data/extracted/ps3/hd-psn-eu/"]
fn every_root_of_every_skin_composes_on_the_plain_psn_download() {
    let Some(install) = oag_testdata::exact("data/extracted/ps3/hd-psn-eu") else {
        return;
    };
    let archives = oag_hd::open(&install.display().to_string()).expect("the install opens as HD");
    // The plain download ships neither Eliminator split pair of the default skin (the census
    // records it, `docs/ui/hud-assets.md`); a two-view Eliminator there scales the whole-screen
    // layout into each view. Every retro root is present.
    check_every_root(
        "HD PSN",
        archives,
        &HD,
        &[
            "/data/xml/splitscreen_hud/splitscreen_elimination_hud_0.xml",
            "/data/xml/splitscreen_hud/splitscreen_elimination_hud_1.xml",
            "/data/xml/splitscreen_hud/vert_splitscreen_elimination_hud_0.xml",
            "/data/xml/splitscreen_hud/vert_splitscreen_elimination_hud_1.xml",
        ],
    );
}

#[test]
#[ignore = "needs the decrypted PS4 package pair in data/extracted/ps4/"]
fn every_root_of_every_skin_composes_on_omega() {
    let Some(source) = oag_testdata::exact("data/extracted/ps4") else {
        return;
    };
    let archives = oag_omega::open(&source.display().to_string()).expect("Omega opens");
    check_every_root("Omega", archives, &omega(), &[]);
}

/// The speed unit reads `RC_KMH` as a sibling label only where a layout authors one (HD,
/// Omega and their retro skins). `oag_hud::draw` keeps the unit out of the value when it does,
/// so a title whose layouts also carried such a label would lose its unit: none of the
/// others does.
#[test]
#[ignore = "needs the Pulse, Pure and 2048 images"]
fn no_pulse_pure_or_2048_layout_authors_an_rc_kmh_unit_label() {
    let mut checked = 0;
    let sources: [(&oag_title::Title, Option<oag_assets::Archives>); 3] = [
        (
            oag_pulse::TITLE,
            oag_testdata::image("data/images/pulse-psp-usa.chd")
                .map(|image| oag_pulse::open(&image.display().to_string()).expect("Pulse opens")),
        ),
        (
            oag_pure::TITLE,
            oag_testdata::image("data/images/pure-psp-usa.chd")
                .map(|image| oag_pure::open(&image.display().to_string()).expect("Pure opens")),
        ),
        (
            oag_2048::TITLE,
            oag_testdata::exact("data/extracted/vita/PCSF00007/base")
                .map(|path| oag_2048::open(&path.display().to_string()).expect("2048 opens")),
        ),
    ];
    for (title, archives) in sources {
        let Some(mut archives) = archives else {
            continue;
        };
        let h = title.hud;
        for root in [h.arcade, h.elimination, h.time_trial, h.speed_lap, h.zone] {
            let composed = oag_hud::compose(root, |path| archives.read_name(path).ok())
                .unwrap_or_else(|| panic!("{} {root} composes", title.name));
            assert!(
                !composed
                    .layout
                    .labels
                    .iter()
                    .any(|label| label.idstring.as_deref() == Some("RC_KMH")),
                "{} {root} authors an RC_KMH label",
                title.name
            );
            checked += 1;
        }
    }
    assert!(checked == 0 || checked % 5 == 0);
}
