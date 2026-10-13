//! Each title's `Title::main_menu` rows, against the main menu its own disc authors.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this project
//! does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! ```
//!
//! # What this is for
//!
//! The vanilla top level of this build's menu is each title's own list
//! ([`oag_title::MainMenu::rows`]), by the `idstring` the disc spells, in the disc's
//! order. This reads the file back on every pressing and every archive that carries
//! it and compares the whole list, so a row dropped, added or reordered fails by
//! name rather than by a menu that quietly drifted. The rows this build has nothing
//! behind (`builds: None`) stay in the list: the list is the disc's.

use std::path::{Path, PathBuf};

use oag_tables::fexml::{self, Node};
use oag_title::{MainMenu, OriginalRow};

/// The `idstring` of every `<Entry>` (or `<TouchButton>`) of the first menu widget
/// named `widget` under the screen named `screen`.
fn entries(root: &Node, screen: &str, widget: &str) -> Vec<String> {
    fn find<'a>(node: &'a Node, screen: &str) -> Option<&'a Node> {
        if node.name.eq_ignore_ascii_case("Screen") && node.attr("name") == Some(screen) {
            return Some(node);
        }
        node.children.iter().find_map(|child| find(child, screen))
    }
    fn widget_of<'a>(node: &'a Node, widget: &str) -> Option<&'a Node> {
        let menu =
            node.name.eq_ignore_ascii_case("Menu") || node.name.eq_ignore_ascii_case("HorizMenu");
        if menu && node.attr("name") == Some(widget) {
            return Some(node);
        }
        node.children
            .iter()
            .find_map(|child| widget_of(child, widget))
    }
    let screen = find(root, screen).unwrap_or_else(|| panic!("no screen {screen:?}"));
    let menu = widget_of(screen, widget).unwrap_or_else(|| panic!("no widget {widget:?}"));
    menu.children
        .iter()
        .filter(|node| node.name.eq_ignore_ascii_case("Entry"))
        .filter_map(|node| node.attr("idstring").or_else(|| node.attr("IDString")))
        .map(str::to_string)
        .collect()
}

fn ids(rows: &[OriginalRow]) -> Vec<&'static str> {
    rows.iter().map(|row| row.disc_id).collect()
}

fn image(name: &str) -> Option<PathBuf> {
    oag_testdata::image(&format!("data/images/{name}"))
}

fn read(archives: &mut oag_assets::Archives, name: &str) -> Node {
    let blob = archives
        .read_name(name)
        .unwrap_or_else(|e| panic!("{name}: {e}"));
    fexml::parse(&fexml::text(&blob).unwrap_or_else(|e| panic!("{name}: {e}")))
}

/// Every archive's own copy of `name`, as `(archive label, parsed root)`.
fn copies(archives: &mut oag_assets::Archives, name: &str) -> Vec<(String, Node)> {
    let mut out = Vec::new();
    let containers = std::iter::once(&mut archives.data)
        .chain(archives.patch.iter_mut())
        .chain(archives.fe.iter_mut())
        .chain(archives.extra.iter_mut());
    for container in containers {
        if !container.contains(name) {
            continue;
        }
        let label = container.label().to_string();
        let blob = container
            .read_entry(name)
            .unwrap_or_else(|e| panic!("{label}: {e}"));
        let xml = fexml::text(&blob).unwrap_or_else(|e| panic!("{label}: {e}"));
        out.push((label, fexml::parse(&xml)));
    }
    out
}

const PULSE_MAIN: &str = r"Data\Plugins\PI001\GUI\MainMenu_Definition.xml";
const HD_MAIN: &str = r"Data\Plugins\Frontend\Gui\MainMenu_Definition.xml";

fn check(menu: &MainMenu, found: &[String], what: &str) {
    assert_eq!(
        found,
        ids(menu.rows_for(false)),
        "{what}: the disc's main menu against `Title::main_menu`"
    );
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn pulses_psp_main_menu_is_the_seven_rows_on_both_pressings() {
    for name in ["pulse-psp-eu.chd", "pulse-psp-usa.chd"] {
        let Some(image) = image(name) else { continue };
        let mut archives = oag_pulse::open(&image.display().to_string()).expect("opening the disc");
        let root = read(&mut archives, PULSE_MAIN);
        check(
            oag_pulse::TITLE.main_menu,
            &entries(&root, "Main Menu", "Mode"),
            name,
        );
    }
}

/// Checked, differs: the PS2 pressing authors two more rows after the first
/// (`FE_RACE_CAM_SS`, `FE_RACE_CAM_SSH`) and neither the multiplayer nor the website
/// row. Those two are `Title::main_menu`'s [`oag_title::Borrowed`] rows, which the PSP
/// borrows, so the list this build shows is the PS2's order exactly: the PS2 disc's list,
/// each id read as the row it builds, equals `MainMenu::built`.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn pulses_ps2_main_menu_puts_the_borrowed_rows_where_the_disc_does() {
    let Some(image) = image("pulse-ps2-eu.chd") else {
        return;
    };
    let mut archives = oag_pulse::open(&image.display().to_string()).expect("opening the disc");
    let root = read(&mut archives, PULSE_MAIN);
    let ps2 = entries(&root, "Main Menu", "Mode");
    assert_eq!(
        ps2,
        [
            "FE_RACE_CAM",
            "FE_RACE_CAM_SS",
            "FE_RACE_CAM_SSH",
            "FE_RACEBOX",
            "FE_PROFILE",
            "FE_OPT_PLUS",
            "FE_EXTRAS"
        ]
    );
    let menu = oag_pulse::TITLE.main_menu;
    let opens = |id: &str| {
        menu.rows
            .iter()
            .find(|row| row.disc_id == id)
            .and_then(|row| row.builds)
            .or_else(|| {
                menu.borrowed
                    .iter()
                    .find(|row| row.disc_id == id)
                    .map(|row| row.builds)
            })
    };
    let from_disc: Vec<&str> = ps2.iter().filter_map(|id| opens(id)).collect();
    let built: Vec<&str> = menu.built(false).collect();
    assert_eq!(
        built, from_disc,
        "what this build shows, in the PS2 disc's order"
    );
    assert_eq!(
        built,
        [
            "campaign",
            "split_vertical",
            "split_horizontal",
            "racebox",
            "options"
        ]
    );
}

/// The wording of the two rows and their help is the PS2's own, `Data\Plugins\PI012`'s
/// English table: `assets/ui/strings/english.toml` carries it, so the PSP and Pure (which
/// ship no such string) show the same words.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_split_rows_words_are_the_ps2_discs_own() {
    let Some(image) = image("pulse-ps2-eu.chd") else {
        return;
    };
    let mut archives = oag_pulse::open(&image.display().to_string()).expect("opening the disc");
    let table = oag_ui::language::StringTable::from_xml(
        &fexml::text(
            &archives
                .read_name(r"Data\Plugins\PI012\entries.xml")
                .expect("PI012 is the PS2 disc's English plugin"),
        )
        .expect("text"),
    );
    let ours = oag_ui::strings::project_table(None);
    for (disc, project) in [
        ("FE_RACE_CAM_SS", "OAG_MENU_SPLIT_VERTICAL"),
        ("FE_RACE_CAM_SSH", "OAG_MENU_SPLIT_HORIZONTAL"),
        ("FE_HELP_SS", "OAG_MENU_SPLIT_HELP"),
    ] {
        assert_eq!(
            ours.get(project),
            table.get(disc),
            "{project} against the disc's {disc}"
        );
        assert!(table.get(disc).is_some(), "{disc} is on the disc");
    }
}

/// The EU disc authors five rows; the USA disc the same four without `Teasers`
/// (checked, differs: the teaser row is a European extra, and nothing is built behind
/// it either way).
#[test]
#[ignore = "needs a disc image in data/images/"]
fn pures_main_menu_is_five_rows_on_the_eu_disc_and_the_same_without_teasers_on_the_usa_one() {
    let menu = oag_pure::TITLE.main_menu;
    for (name, teasers) in [("pure-psp-eu.chd", true), ("pure-psp-usa.chd", false)] {
        let Some(image) = image(name) else { continue };
        let mut archives = oag_pure::open(&image.display().to_string()).expect("opening the disc");
        let root = read(&mut archives, oag_pure::names::FRONTEND_ROOT);
        let found = entries(&root, "Main Menu", "Mode");
        let want: Vec<&str> = ids(menu.rows)
            .into_iter()
            .filter(|id| teasers || *id != "Teasers")
            .collect();
        assert_eq!(found, want, "{name}");
    }
}

/// The five archives that carry `MainMenu_Definition.xml` agree, so the menu does not
/// change with which archive a mounted pack or update adds.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn hds_main_menu_is_the_same_five_rows_in_every_archive() {
    let Some(image) = image("hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let mut archives = oag_hd::open(&image.display().to_string()).expect("opening the disc");
    let all = copies(&mut archives, HD_MAIN);
    assert_eq!(
        all.len(),
        5,
        "{:?}",
        all.iter().map(|c| &c.0).collect::<Vec<_>>()
    );
    for (label, root) in &all {
        check(
            oag_hd::TITLE.main_menu,
            &entries(root, "Main Menu", "Mode"),
            label,
        );
    }
}

/// Omega's own copies (its base package and the day-one patch's `data09.psarc`) author
/// HD's five rows.
#[test]
#[ignore = "needs the decrypted PS4 package pair in data/extracted/ps4/"]
fn omegas_main_menu_is_hds_five_rows_in_every_archive_that_has_one() {
    let Some(source) = oag_testdata::exact("data/extracted/ps4") else {
        return;
    };
    let mut archives = oag_omega::open(&source.display().to_string()).expect("opening Omega");
    let all = copies(&mut archives, HD_MAIN);
    assert!(!all.is_empty());
    for (label, root) in &all {
        check(
            oag_omega::TITLE.main_menu,
            &entries(root, "Main Menu", "Mode"),
            label,
        );
    }
}

/// 2048's `GameModeChoice`: four tiles in the base package, six once the v1.04 patch
/// is mounted. Which list a run uses follows whether the patch is mounted
/// (`MainMenu::rows_for`). Whether the DLC packages hide the two HD and Fury tiles is not
/// read; nothing here depends on it.
#[test]
#[ignore = "needs the extracted Vita package in data/extracted/vita/"]
fn wipeout_2048s_game_mode_grid_is_four_tiles_in_the_base_and_six_with_the_patch() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/extracted/vita/PCSF00007");
    if !path.join("base/PSP2/data.psarc").exists() {
        return;
    }
    let mut archives = oag_2048::open(&path.display().to_string()).expect("opening 2048");
    let name = r"Data\Plugins\Frontend\NEWGUI\Definition.xml";
    let screen = "GameModeChoice";
    let tiles = |root: &Node| -> Vec<String> {
        fn find<'a>(node: &'a Node, screen: &str) -> Option<&'a Node> {
            if node.attr("name") == Some(screen) {
                return Some(node);
            }
            node.children.iter().find_map(|child| find(child, screen))
        }
        // The id lives on the button's own `<Values>`; `confirmButton` carries none and
        // is not a mode tile.
        find(root, screen)
            .expect("GameModeChoice")
            .children
            .iter()
            .filter(|node| node.name.eq_ignore_ascii_case("TouchButton"))
            .filter_map(|node| node.value("idstring"))
            .map(str::to_string)
            .collect()
    };
    let base_label = archives.data.label().to_string();
    let all = copies(&mut archives, name);
    let base = all
        .iter()
        .find(|(label, _)| *label == base_label)
        .expect("the base package carries the file");
    let menu = oag_2048::TITLE.main_menu;
    assert_eq!(tiles(&base.1), ids(menu.rows_for(false)), "base package");
    // `data2.psarc` (v1.04) authors the six tiles and is searched ahead of `data1`;
    // `data1.psarc` (the earlier patch) authors the four plus an HD tile that reuses
    // `FE_SP_CAMPAIGN`, shadowed whenever both mount.
    let by = |suffix: &str| {
        all.iter()
            .find(|(label, _)| label.ends_with(suffix))
            .unwrap_or_else(|| panic!("no copy in {suffix}"))
    };
    assert_eq!(
        tiles(&by("data2.psarc").1),
        ids(menu.rows_for(true)),
        "data2"
    );
    assert_eq!(
        tiles(&by("data1.psarc").1),
        [
            "FE_SP_CAMPAIGN",
            "FE_MP_CAMPAIGN",
            "FE_ADHOC",
            "FE_CROSSPLAY",
            "FE_SP_CAMPAIGN"
        ],
        "data1"
    );
}

/// Pulse's four DLC packs carry team data and front-end art, and no main menu: the
/// menu does not change once they are mounted.
#[test]
#[ignore = "needs a disc image in data/images/ and DLC packs in data/dlc/"]
fn no_pulse_dlc_pack_carries_a_main_menu() {
    let Some(image) = image("pulse-psp-usa.chd") else {
        return;
    };
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/dlc");
    if !root.is_dir() {
        return;
    }
    let cache = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/cache/dlc");
    let (packs, _) = oag_source::dlc::packs(&[root], &cache, &[]);
    assert!(!packs.is_empty(), "the Pulse packs are found");
    let count = packs.len();
    let mut archives = oag_pulse::open_with_packs(&image.display().to_string(), packs)
        .expect("mounting the packs");
    // Four packs, each four `.edat` archives.
    assert!(
        archives.packs.len() >= count,
        "{} containers",
        archives.packs.len()
    );
    for pack in &archives.packs {
        assert!(!pack.contains(PULSE_MAIN), "{}", pack.label());
    }
    let root = read(&mut archives, PULSE_MAIN);
    check(
        oag_pulse::TITLE.main_menu,
        &entries(&root, "Main Menu", "Mode"),
        "with the packs mounted",
    );
}

/// 2048's two DLC archives carry a tracks plugin and no front-end definition at all, so
/// no file on the disc side redefines `GameModeChoice`. That does not say what the
/// executable does with `g_bDlc1Mounted` and `g_bDlc2Mounted` (read at dozens of sites):
/// whether it hides the HD and Fury tiles without them is not read.
#[test]
#[ignore = "needs the extracted Vita package in data/extracted/vita/"]
fn wipeout_2048s_dlc_archives_carry_no_front_end_definition() {
    let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/extracted/vita/PCSF00007");
    for dlc in ["dlc1/PSP2/dlc1.psarc", "dlc2/PSP2/dlc2.psarc"] {
        let path = base.join(dlc);
        if !path.exists() {
            return;
        }
        let archive = oag_assets::psarc::Archive::open(&path.display().to_string()).expect(dlc);
        let found: Vec<String> = archive
            .paths()
            .iter()
            .map(|p| p.to_lowercase())
            .filter(|p| p.contains("newgui") || p.contains("mainmenu") || p.contains("skin.xml"))
            .collect();
        assert!(found.is_empty(), "{dlc}: {found:?}");
    }
}
