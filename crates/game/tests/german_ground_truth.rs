//! German on every title: the project's own `OAG_` ids resolve, and a disc's own
//! German is only ever *filled*, never replaced.
//!
//! **`#[ignore]`d and never run in CI**; needs the disc images (`just test-data`).
//! Measured 2026-10-06 against each title's English table: Pulse PSP EU leaves
//! `M_SAVE_GHOST` and `HM_SAVE_GHOST` empty in German, 2048 leaves four ids
//! empty, and Pulse PS2, Pure's menus, HD and Omega have no gap that is not a
//! proper name, a font test string or a legal notice
//! (`docs/ui/project-languages.md`).

use oag_ui::language::{Language, StringTable, load};

struct Opened {
    languages: Vec<Language>,
    raw_languages: Vec<Language>,
    archives: oag_assets::Archives,
}

fn open(source: &str) -> Opened {
    let opened =
        oag_source::title::open_source(source, Vec::new(), Vec::new()).expect("the source opens");
    let mut archives = opened.archives;
    let front_end = opened.title.front_end.expect("a front end");
    let plugins = front_end.offered_languages(archives.layout.serial.as_deref());
    let languages = load::load_languages(
        &mut archives,
        plugins,
        front_end.disc_strings,
        &mut Vec::new(),
    );
    let raw_languages = load::load_languages(&mut archives, plugins, None, &mut Vec::new());
    Opened {
        languages,
        raw_languages,
        archives,
    }
}

fn german(opened: &mut Opened, raw: bool) -> StringTable {
    let languages = if raw {
        &opened.raw_languages
    } else {
        &opened.languages
    };
    load::load_strings(
        &mut opened.archives,
        languages,
        Some("German"),
        &mut Vec::new(),
    )
}

fn non_empty<'a>(table: &'a StringTable, id: &str) -> Option<&'a str> {
    table.get(id).filter(|text| !text.trim().is_empty())
}

/// Checks `ids` against one source: every one is empty or absent in the disc's
/// own German, filled once the title's namespace is read (`expect_filled`), and
/// every `OAG_` id this build ships resolves.
fn check(source: &str, ids: &[&str], expect_filled: bool) {
    let mut opened = open(source);
    let raw = german(&mut opened, true);
    let table = german(&mut opened, false);
    for id in ids {
        if expect_filled {
            assert!(
                non_empty(&raw, id).is_none(),
                "{source}: {id} is not a gap, the file would replace the disc"
            );
            assert!(non_empty(&table, id).is_some(), "{source}: {id} not filled");
        } else {
            assert_eq!(
                table.get(id),
                raw.get(id),
                "{source}: {id} must stay the disc's own"
            );
        }
    }
    for id in ["OAG_RACE_TRACK", "OAG_REMIX_PAGE_TITLE", "OAG_MENU_BACK"] {
        assert!(non_empty(&table, id).is_some(), "{source}: {id} missing");
    }
    assert_eq!(table.get("OAG_RACE_TRACK"), Some("STRECKE"));
}

const PULSE_GAPS: &[&str] = &["M_SAVE_GHOST", "HM_SAVE_GHOST"];
const VITA_2048_GAPS: &[&str] = &[
    "3D_STRENGTH",
    "FE_CAMPSEL_MODES",
    "FE_PERCENT_COMPLETE",
    "STATS_ONLINE_RACES_LOST",
];

#[test]
#[ignore = "needs data/images"]
fn pulse_psp_fills_the_two_ghost_ids() {
    let Some(path) = oag_testdata::image("pulse-psp-eu.chd") else {
        return;
    };
    check(&path.display().to_string(), PULSE_GAPS, true);
}

#[test]
#[ignore = "needs data/images"]
fn pulse_ps2_keeps_the_discs_own_ghost_words() {
    let Some(path) = oag_testdata::image("pulse-ps2-eu.chd") else {
        return;
    };
    check(&path.display().to_string(), PULSE_GAPS, false);
}

#[test]
#[ignore = "needs data/extracted/vita"]
fn vita_2048_fills_its_four_empty_ids() {
    let Some(path) = oag_testdata::exact("data/extracted/vita/PCSF00007") else {
        return;
    };
    check(&path.display().to_string(), VITA_2048_GAPS, true);
}

#[test]
#[ignore = "needs data/images"]
fn pure_and_hd_carry_the_oag_ids() {
    for name in ["pure-psp-eu.chd", "hdfury-ps3-eu-dec.iso"] {
        let Some(path) = oag_testdata::image(name) else {
            return;
        };
        check(&path.display().to_string(), &[], false);
    }
}

#[test]
#[ignore = "needs data/extracted/ps4"]
fn omega_keeps_its_own_german_for_the_2048_ids() {
    let Some(path) = oag_testdata::exact("data/extracted/ps4") else {
        return;
    };
    check(&path.display().to_string(), VITA_2048_GAPS, false);
}
