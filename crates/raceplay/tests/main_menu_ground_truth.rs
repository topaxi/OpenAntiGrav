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
//!
//! The same pass reads each title's pause screens (`Title::pause_menu`) out of the
//! `InGame_Definition.xml` beside it: every authored screen's row list, whole, and the
//! default the pause redirect names.

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
const PULSE_INGAME: &str = r"Data\Plugins\PI001\GUI\InGame_Definition.xml";
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

/// Checked, differs: the PS2 pressing authors two split-screen rows after the first
/// (`FE_RACE_CAM_SS`, `FE_RACE_CAM_SSH`) and neither the multiplayer nor the website row. This
/// build shows the PSP's list on both pressings, with MULTIPLAYER (`FE_MP`, the PSP's multiplayer
/// entry) where the PS2 has its split rows' replacement: the split screen is LOCAL MULTIPLAYER on
/// the page behind that row, so the PS2's two rows are the one thing `Title::main_menu` does not
/// build for them.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn pulses_ps2_main_menu_has_split_rows_that_multiplayer_replaces() {
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
    };
    let from_disc: Vec<&str> = ps2.iter().filter_map(|id| opens(id)).collect();
    assert_eq!(
        from_disc,
        ["campaign", "racebox", "options"],
        "what the PS2 disc's own rows build here, the split rows opening nothing of theirs"
    );
    let built: Vec<&str> = menu.built(false).collect();
    assert_eq!(
        built,
        ["campaign", "racebox", "multiplayer", "options"],
        "and MULTIPLAYER stands where the PSP's FE_MP does"
    );
}

/// The wording of Pulse's MULTIPLAYER row and its help is the PSP disc's own `FE_MP` and
/// `FE_HELP_MP`: `assets/ui/strings/english.toml` carries it, so the PS2 pressing and the
/// others, which author no such row (or the same word), show the same words.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_multiplayer_rows_words_are_the_psp_discs_own() {
    let Some(image) = image("pulse-psp-eu.chd") else {
        return;
    };
    let mut archives = oag_pulse::open(&image.display().to_string()).expect("opening the disc");
    // English is `PI000` on the EU pressing and `PI012` on the USA one.
    let blob = ["PI000", "PI012"]
        .iter()
        .find_map(|plugin| {
            archives
                .read_name(&format!(r"Data\Plugins\{plugin}\entries.xml"))
                .ok()
        })
        .expect("an English plugin on the PSP disc");
    let table = oag_ui::language::StringTable::from_xml(&fexml::text(&blob).expect("text"));
    let ours = oag_ui::strings::project_table(None);
    let row = oag_pulse::TITLE
        .main_menu
        .multiplayer
        .expect("a worded row");
    for (disc, wording) in [("FE_MP", row.label), ("FE_HELP_MP", row.subtitle)] {
        assert_eq!(
            ours.get(wording.id),
            table.get(disc),
            "{} against the disc's {disc}",
            wording.id
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

/// The `goto` of the first `<Default>` under the screen named `screen`.
fn default_goto(root: &Node, screen: &str) -> Option<String> {
    fn find<'a>(node: &'a Node, screen: &str) -> Option<&'a Node> {
        if node.name.eq_ignore_ascii_case("Screen") && node.attr("name") == Some(screen) {
            return Some(node);
        }
        node.children.iter().find_map(|child| find(child, screen))
    }
    fn default(node: &Node) -> Option<&Node> {
        if node.name.eq_ignore_ascii_case("Default") {
            return Some(node);
        }
        node.children.iter().find_map(default)
    }
    default(find(root, screen)?)?
        .attr("goto")
        .map(str::to_string)
}

/// Every pause screen the title authors, row for row, and the screen its pause redirect
/// opens by default. `InGame Pause MP` and its tournament host are the network modes' and a
/// race here never opens them, so they are not in the table.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn pulses_psp_pause_screens_are_the_four_single_player_lists_on_both_pressings() {
    let pause = oag_pulse::TITLE
        .pause_menu
        .expect("Pulse authors pause screens");
    for name in ["pulse-psp-eu.chd", "pulse-psp-usa.chd"] {
        let Some(image) = image(name) else { continue };
        let mut archives = oag_pulse::open(&image.display().to_string()).expect("opening the disc");
        let root = read(&mut archives, PULSE_INGAME);
        for screen in pause.screens {
            let rows: Vec<&str> = screen.rows.iter().map(|row| row.disc_id).collect();
            assert_eq!(
                entries(&root, screen.name, "Option"),
                rows,
                "{name}: {:?} against `Title::pause_menu`",
                screen.name
            );
        }
        assert_eq!(
            default_goto(&root, "InGame Pause Redirect").as_deref(),
            Some(pause.default_screen),
            "{name}: the screen the pause redirect opens by default"
        );
        for (_, screen) in pause.by_mode {
            assert!(
                pause.screen(screen).is_some(),
                "{name}: {screen:?} is a table row"
            );
        }
    }
}

/// Checked, differs: the PS2 pressing authors the same four screens without `EXT_PHOTO` and
/// `IG_PAUSE_GMAN`. The table is the PSP's; the PS2 gets the same rows built, because the game
/// manual is not built anywhere and PHOTO MODE is this project's row after OPTIONS where the
/// screen authors none (`PauseMenu::offers_photo`).
#[test]
#[ignore = "needs a disc image in data/images/"]
fn pulses_ps2_pause_screens_are_the_psps_without_photo_and_the_manual() {
    let pause = oag_pulse::TITLE
        .pause_menu
        .expect("Pulse authors pause screens");
    let Some(image) = image("pulse-ps2-eu.chd") else {
        return;
    };
    let mut archives = oag_pulse::open(&image.display().to_string()).expect("opening the disc");
    let root = read(&mut archives, PULSE_INGAME);
    for screen in pause.screens {
        let rows: Vec<&str> = screen
            .rows
            .iter()
            .map(|row| row.disc_id)
            .filter(|id| !["EXT_PHOTO", "IG_PAUSE_GMAN"].contains(id))
            .collect();
        assert_eq!(
            entries(&root, screen.name, "Option"),
            rows,
            "PS2: {:?} against `Title::pause_menu` without the two rows it lacks",
            screen.name
        );
    }
    assert_eq!(
        default_goto(&root, "InGame Pause Redirect").as_deref(),
        Some(pause.default_screen),
        "PS2: the screen the pause redirect opens by default"
    );
}

/// The `idstring` of every live row of the HD-style pause screen `screen`: its `Block` and
/// `List` widgets in document order. The `aVertMenu` and `aBlock` elements the file spells with
/// an `a` prefix are switched off, and are not rows.
fn widget_rows(root: &Node, screen: &str) -> Vec<String> {
    fn find<'a>(node: &'a Node, screen: &str) -> Option<&'a Node> {
        if node.name.eq_ignore_ascii_case("Screen") && node.attr("name") == Some(screen) {
            return Some(node);
        }
        node.children.iter().find_map(|child| find(child, screen))
    }
    let screen = find(root, screen).unwrap_or_else(|| panic!("no screen {screen:?}"));
    screen
        .children
        .iter()
        .filter(|node| {
            node.name.eq_ignore_ascii_case("Block") || node.name.eq_ignore_ascii_case("List")
        })
        .filter_map(|node| {
            let values = node
                .children
                .iter()
                .find(|child| child.name.eq_ignore_ascii_case("Values"))?;
            values.attr("idstring").or_else(|| values.attr("IDString"))
        })
        .map(str::to_string)
        .collect()
}

const HD_INGAME: &str = r"Data\Plugins\Frontend\Gui\ingame_definition.xml";

/// Every pause screen `pause` names, row for row, and the screen its pause redirect opens by
/// default, in `root`.
fn check_widget_pause_screens(pause: &oag_title::PauseMenu, root: &Node, what: &str) {
    for screen in pause.screens {
        let rows: Vec<&str> = screen.rows.iter().map(|row| row.disc_id).collect();
        assert_eq!(
            widget_rows(root, screen.name),
            rows,
            "{what}: {:?} against `Title::pause_menu`",
            screen.name
        );
    }
    assert_eq!(
        default_goto(root, "InGame Pause Redirect").as_deref(),
        Some(pause.default_screen),
        "{what}: the screen the pause redirect opens by default"
    );
    for (_, screen) in pause.by_mode {
        assert!(
            pause.screen(screen).is_some(),
            "{what}: {screen:?} is a table row"
        );
    }
}

/// HD's pause screens are `Block` and `List` widgets, not an `Option` menu's entries. The file
/// the game reads (`Archives::read_name`) is the table. The other four copies are **checked,
/// differs**: `DATA05` has no `FE_GHOST` row, and the base game's `DATA02` and `DATA03` neither
/// that nor `ONL_VIEW_INV`, and their Speed Lap screen also has RESTART and QUIT. Everything else
/// agrees. The split-screen and network screens are the ones a race here never opens, so they
/// are not in the table.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn hds_pause_screens_are_the_five_single_player_widget_lists() {
    let Some(image) = image("hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let pause = oag_hd::TITLE.pause_menu.expect("HD authors pause screens");
    let mut archives = oag_hd::open(&image.display().to_string()).expect("opening the disc");
    check_widget_pause_screens(
        pause,
        &read(&mut archives, HD_INGAME),
        "the file the game reads",
    );
    check_pause_layout(&mut archives, pause);
    let all = copies(&mut archives, HD_INGAME);
    assert_eq!(
        all.len(),
        5,
        "{:?}",
        all.iter().map(|c| &c.0).collect::<Vec<_>>()
    );
    for (label, root) in &all {
        for screen in pause
            .screens
            .iter()
            .filter(|screen| !screen.name.ends_with("Speed Lap"))
        {
            let rows: Vec<&str> = screen
                .rows
                .iter()
                .map(|row| row.disc_id)
                .filter(|id| !["ONL_VIEW_INV", "FE_GHOST"].contains(id))
                .collect();
            let found: Vec<String> = widget_rows(root, screen.name)
                .into_iter()
                .filter(|id| !["ONL_VIEW_INV", "FE_GHOST"].contains(&id.as_str()))
                .collect();
            assert_eq!(found, rows, "{label}: {:?}", screen.name);
        }
    }
}

/// The globals each archive's copy of the front end's `Skin.xml` declares, by archive label.
fn skin_globals(
    archives: &mut oag_assets::Archives,
) -> Vec<(String, std::collections::HashMap<String, String>)> {
    const SKIN: &str = r"Data\Plugins\Frontend\Gui\Skin.xml";
    let containers = std::iter::once(&mut archives.data)
        .chain(archives.patch.iter_mut())
        .chain(archives.fe.iter_mut())
        .chain(archives.extra.iter_mut());
    let mut out = Vec::new();
    for container in containers {
        if !container.contains(SKIN) {
            continue;
        }
        let label = container.label().to_string();
        let blob = container.read_entry(SKIN).expect("the skin");
        let xml = fexml::text(&blob).expect("text");
        out.push((label, oag_ui::screen::Screens::from_xml(&xml).globals));
    }
    out
}

/// Where HD's pause screens draw their rows, read off the file the game reads: every row of
/// every screen is a `Block` or `List` at `x = 480`, fifty apart from `y = 310`, in the table's
/// order, and the chrome screen behind them carries the two rules (`y = 292` and `790`, 960
/// wide). The focused row's ink is `HD_Blue`, which the HD-style archive resolves to
/// `0xff8ac0ca` and the Fury-style one to red: the original's pause screen on RPCS3 in the HD
/// style measured (138, 192, 202) on its focused row and (100, 100, 100) on the others.
fn check_pause_layout(archives: &mut oag_assets::Archives, pause: &oag_title::PauseMenu) {
    let layout = pause
        .layout
        .expect("HD names where its screens are drawn from");
    let blob = archives.read_name(layout.definition).expect("the file");
    let xml = fexml::text(&blob).expect("text");
    for (label, globals) in skin_globals(archives) {
        let blue = globals.get("HD_Blue").map(String::as_str);
        let grey = globals.get("HD_Grey").map(String::as_str);
        if label.contains("DATA06") {
            assert_eq!(
                blue,
                Some("0xff8ac0ca"),
                "{label}: the HD style's focused ink"
            );
            assert_eq!(
                grey,
                Some("0xff646464"),
                "{label}: the HD style's resting ink"
            );
        } else if label.contains("DATA00") {
            assert_ne!(
                blue,
                Some("0xff8ac0ca"),
                "{label}: the Fury style is not cyan"
            );
        }
    }
    let screens = oag_ui::screen::Screens::from_xml(&xml);
    let chrome = screens.by_name(layout.chrome).expect("the chrome screen");
    let rules: Vec<[f32; 4]> = chrome
        .fills
        .iter()
        .map(|fill| {
            [
                fill.x,
                fill.y,
                fill.width.unwrap_or(0.0),
                fill.height.unwrap_or(0.0),
            ]
        })
        .collect();
    assert_eq!(
        rules,
        [[480.0, 292.0, 960.0, 4.0], [480.0, 790.0, 960.0, 4.0]],
        "{}: the two rules",
        layout.chrome
    );
    for screen in pause.screens {
        let read = screens.by_name(screen.name).expect(screen.name);
        let mut at: Vec<(f32, f32)> = read
            .blocks
            .iter()
            .map(|block| (block.x, block.y))
            .chain(read.lists.iter().map(|list| (list.x, list.y)))
            .collect();
        at.sort_by(|a, b| a.1.total_cmp(&b.1));
        assert!(at.len() >= screen.rows.len(), "{}", screen.name);
        for (index, (x, y)) in at.iter().take(screen.rows.len()).enumerate() {
            assert_eq!(*x, 480.0, "{}: row {index}", screen.name);
            assert_eq!(
                *y,
                310.0 + 50.0 * index as f32,
                "{}: row {index}",
                screen.name
            );
        }
    }
}

/// Omega's `ingame_definition.xml` (`data00`, `data08`, `data09`) is HD's file with the same five
/// pause screens **without `ONL_VIEW_INV`** (checked, differs): the table is the same rows less
/// that one, in all three copies.
#[test]
#[ignore = "needs the decrypted PS4 package pair in data/extracted/ps4/"]
fn omegas_pause_screens_are_hds_without_the_invites_row_in_every_archive() {
    let Some(source) = oag_testdata::exact("data/extracted/ps4") else {
        return;
    };
    let pause = oag_omega::pause_menu::PAUSE_MENU;
    let mut archives = oag_omega::open(&source.display().to_string()).expect("opening Omega");
    let all = copies(&mut archives, HD_INGAME);
    assert!(
        all.len() >= 3,
        "{:?}",
        all.iter().map(|c| &c.0).collect::<Vec<_>>()
    );
    for (label, root) in &all {
        check_widget_pause_screens(pause, root, label);
    }
    let hd = oag_hd::TITLE.pause_menu.expect("HD authors pause screens");
    for screen in hd.screens {
        let hd_rows: Vec<&str> = screen
            .rows
            .iter()
            .map(|row| row.disc_id)
            .filter(|id| *id != "ONL_VIEW_INV")
            .collect();
        let omega_rows: Vec<&str> = pause
            .screen(screen.name)
            .expect("the same screens")
            .rows
            .iter()
            .map(|row| row.disc_id)
            .collect();
        assert_eq!(omega_rows, hd_rows, "{:?}", screen.name);
    }
    assert_eq!(pause.by_mode, hd.by_mode, "the same mode switch");
}
