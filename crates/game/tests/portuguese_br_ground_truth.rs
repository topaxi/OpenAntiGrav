//! Brazilian Portuguese, the language this build offers on every title.
//!
//! **`#[ignore]`d and never run in CI**; needs the disc images (`just test-data`).
//!
//! What is pinned: the project language is offered after the disc's own on each
//! source, resolves disc ids through the title's own overlay and then the disc's
//! English (never empty), carries this project's own ids, uses Omega's own
//! `portuguesebr` plugin as the base there, and has every pt-BR letter in the
//! faces the front end draws with - except `ç` in Pulse's and Pure's HUD faces,
//! where the base letter stands in. Measured 2026-10-06 off the discs.

use oag_ui::language::{Language, StringTable, load};

struct Opened {
    languages: Vec<Language>,
    archives: oag_assets::Archives,
    disc_strings: Option<&'static str>,
}

fn open(path: &std::path::Path) -> Opened {
    let opened =
        oag_source::title::open_source(&path.display().to_string(), Vec::new(), Vec::new())
            .expect("the source opens");
    let mut archives = opened.archives;
    let front_end = opened.title.front_end.expect("a front end");
    let plugins = front_end.offered_languages(archives.layout.serial.as_deref());
    let languages = load::load_languages(
        &mut archives,
        plugins,
        front_end.disc_strings,
        &mut Vec::new(),
    );
    Opened {
        languages,
        archives,
        disc_strings: front_end.disc_strings,
    }
}

fn strings(opened: &mut Opened, language: &str) -> StringTable {
    load::load_strings(
        &mut opened.archives,
        &opened.languages,
        Some(language),
        &mut Vec::new(),
    )
}

#[test]
#[ignore = "needs data/images"]
fn pulse_translates_its_disc_ids_and_falls_back_to_the_discs_english() {
    let Some(path) = oag_testdata::image("pulse-psp-eu.chd") else {
        return;
    };
    let mut opened = open(&path);
    assert_eq!(opened.disc_strings, Some("pulse"));
    let english = strings(&mut opened, "English");
    let table = strings(&mut opened, "PortugueseBR");

    assert_ne!(
        table.get("FE_MM"),
        english.get("FE_MM"),
        "a translated disc id"
    );
    assert!(table.get("FE_MM").is_some_and(|text| text.contains("MENU")));
    assert_eq!(table.get("IG_HUD_POSITION"), Some("Posição"));
    // An id this project did not translate (a proper name) reads the disc's own
    // English rather than nothing.
    assert_eq!(table.get("Venom"), english.get("Venom"));
    assert!(table.get("Venom").is_some());
    // Every disc id still resolves, and ours ride on top.
    assert!(
        table.len() >= english.len(),
        "{} < {}",
        table.len(),
        english.len()
    );
    assert_eq!(table.get("OAG_MENU_QUIT"), Some("SAIR"));
}

/// The ids a shipped disc-keyed file translates, read off the file's own lines
/// (`ID = { text = ... }`, the id bare or quoted).
fn translated_ids(namespace: &str) -> Vec<String> {
    let path = oag_testdata::repo_root()
        .join("assets/ui/strings/disc")
        .join(namespace)
        .join("portuguesebr.toml");
    let text = std::fs::read_to_string(&path).expect("the shipped disc file");
    text.lines()
        .filter(|line| !line.starts_with('#') && line.contains(" = { text = "))
        .map(|line| {
            let key = line.split(" = { text = ").next().expect("a key");
            key.trim_matches('"').to_string()
        })
        .collect()
}

/// A project-language disc file must only name ids its title's base table
/// carries (a typo, or another title's id, would translate nothing), and the
/// letters it uses must exist in the faces the front end draws with.
fn assert_file_resolves_and_is_drawable(
    opened: &mut Opened,
    namespace: &str,
    base: &StringTable,
    table: &StringTable,
) {
    let keys = translated_ids(namespace);
    // A `h_xxxxxxxx` key stands for the base id that hashes to it; one that
    // matches none is a typo or another title's id.
    let mut ids = Vec::new();
    let mut unmatched = Vec::new();
    for key in keys {
        if key.len() == 10 && key.starts_with("h_") && base.get(&key).is_none() {
            let before = ids.len();
            ids.extend(
                base.ids()
                    .filter(|id| oag_ui::strings::hashed_key(id) == key)
                    .map(str::to_string),
            );
            if ids.len() == before {
                unmatched.push(key);
            }
        } else {
            ids.push(key);
        }
    }
    assert!(
        unmatched.is_empty(),
        "{namespace}: hashed keys that match no id: {unmatched:?}"
    );
    assert!(ids.len() > 100, "{namespace}: {} ids", ids.len());
    let unknown: Vec<&String> = ids.iter().filter(|id| base.get(id).is_none()).collect();
    assert!(
        unknown.is_empty(),
        "{namespace}: not in the base: {unknown:?}"
    );
    let differing = ids
        .iter()
        .filter(|id| table.get(id) != base.get(id))
        .count();
    assert!(
        differing * 100 >= ids.len() * 95,
        "{namespace}: only {differing} of {} read differently from the base",
        ids.len()
    );
    let chars: std::collections::BTreeSet<char> = ids
        .iter()
        .filter_map(|id| table.get(id))
        .flat_map(str::chars)
        .filter(|c| !c.is_ascii() || c.is_ascii_graphic())
        .collect();
    let base_language = opened
        .languages
        .iter()
        .find(|l| l.name == "English" || l.name == "American")
        .expect("a base language")
        .clone();
    for (role, file) in &base_language.fonts {
        if role == "Buttons" {
            continue;
        }
        let font = opened.archives.read_font(file).expect("the face reads");
        let atlas = oag_ui::font::Atlas::from_font(&font);
        // Every letter draws as itself or, where the face lacks it, as its
        // base letter (`ç` as `c`, `º` as `o`): never as nothing.
        let undrawn: String = chars
            .iter()
            .filter(|c| c.is_ascii_alphabetic() || WANT.contains(**c) || **c == 'º')
            .filter(|c| atlas.cell(**c).is_none())
            .collect();
        assert_eq!(
            undrawn, "",
            "{namespace} {role} {file}: letters that draw nothing"
        );
    }
}

#[test]
#[ignore = "needs data/images"]
fn the_2048_disc_text_is_translated_and_falls_back_to_the_discs_base() {
    let Some(path) = oag_testdata::exact("data/extracted/vita") else {
        return;
    };
    let mut opened = open(&path);
    assert_eq!(opened.disc_strings, Some("2048"));
    let base = strings(&mut opened, "American");
    let table = strings(&mut opened, "PortugueseBR");

    assert_eq!(table.get("FE_BACK"), Some("VOLTAR"));
    assert_eq!(table.get("IG_HUD_LAP"), Some("VOLTA"));
    assert_ne!(table.get("FE_OPTIONS"), base.get("FE_OPTIONS"));
    // A team name is left to read as the disc writes it.
    assert_eq!(table.get("Qirex"), base.get("Qirex"));
    assert!(table.get("Qirex").is_some());
    assert_eq!(table.get("OAG_MENU_QUIT"), Some("SAIR"));
    assert!(table.len() >= base.len());
    assert_file_resolves_and_is_drawable(&mut opened, "2048", &base, &table);
}

/// The EU release stands on plain English, not `American`: the same file must
/// resolve there id for id.
#[test]
#[ignore = "needs data/extracted/vita"]
fn the_2048_eu_release_reads_the_same_file_over_its_english() {
    let Some(path) = oag_testdata::exact("data/extracted/vita/PCSF00007") else {
        return;
    };
    let mut opened = open(&path);
    assert_eq!(opened.disc_strings, Some("2048"));
    let base = strings(&mut opened, "English");
    let table = strings(&mut opened, "PortugueseBR");
    assert_eq!(table.get("FE_BACK"), Some("VOLTAR"));
    assert_eq!(table.get("Qirex"), base.get("Qirex"));
    assert_file_resolves_and_is_drawable(&mut opened, "2048", &base, &table);
}

#[test]
#[ignore = "needs data/images"]
fn the_hd_disc_text_is_translated_and_falls_back_to_the_discs_english() {
    let Some(path) = oag_testdata::image("hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let mut opened = open(&path);
    assert_eq!(opened.disc_strings, Some("hd"));
    let base = strings(&mut opened, "English");
    let table = strings(&mut opened, "PortugueseBR");

    assert_eq!(table.get("FE_BACK"), Some("VOLTAR"));
    assert_eq!(table.get("IG_HUD_LAP"), Some("VOLTA"));
    assert_ne!(table.get("FE_MM"), base.get("FE_MM"));
    // A team name and a circuit key are left to read as the disc writes them.
    assert_eq!(table.get("Qirex"), base.get("Qirex"));
    assert!(table.get("Qirex").is_some());
    assert_eq!(table.get("01_TRACK"), base.get("01_TRACK"));
    assert_eq!(table.get("OAG_MENU_QUIT"), Some("SAIR"));
    assert!(table.len() >= base.len());
    assert_file_resolves_and_is_drawable(&mut opened, "hd", &base, &table);
}

#[test]
#[ignore = "needs data/images"]
fn the_pure_disc_text_is_translated_and_falls_back_to_the_discs_english() {
    let Some(path) = oag_testdata::image("pure-psp-eu.chd") else {
        return;
    };
    let mut opened = open(&path);
    assert_eq!(opened.disc_strings, Some("pure"));
    let base = strings(&mut opened, "English");
    assert!(
        !base.is_empty(),
        "Pure keeps English inline in its definition"
    );
    let table = strings(&mut opened, "PortugueseBR");

    // Pure keys most strings by their English text, so the key is the id.
    assert_eq!(table.get("Continue"), Some("Continuar"));
    assert_eq!(table.get("HUD_Lap"), Some("Volta"));
    assert_eq!(table.get("1st"), Some("1º"));
    // The 719 entries keyed by a hash of the disc id (no English committed)
    // land on the ids themselves, and no hashed key survives as an id.
    let hashed = translated_ids("pure")
        .iter()
        .filter(|key| key.starts_with("h_"))
        .count();
    assert!(hashed > 700, "{hashed}");
    assert!(table.ids().all(|id| !id.starts_with("h_")));
    // A team name is left to read as the disc writes it, and still resolves.
    assert_eq!(table.get("Qirex"), base.get("Qirex"));
    assert!(table.get("Qirex").is_some());
    assert_eq!(table.get("OAG_MENU_QUIT"), Some("SAIR"));
    // Same ids as English (the disc's, plus our `OAG_` ones): nothing is lost
    // and nothing is empty.
    assert_eq!(table.len(), base.len());
    assert_file_resolves_and_is_drawable(&mut opened, "pure", &base, &table);
}

#[test]
#[ignore = "needs data/images"]
fn omega_uses_its_own_portuguesebr_plugin_as_the_base() {
    let Some(path) = oag_testdata::exact("data/extracted/ps4") else {
        return;
    };
    let mut opened = open(&path);
    let matching: Vec<&str> = opened
        .languages
        .iter()
        .filter(|l| l.name.eq_ignore_ascii_case("PortugueseBR"))
        .map(|l| l.plugin.as_str())
        .collect();
    assert_eq!(
        matching,
        [r"Languages\portuguesebr"],
        "the disc's, not a copy"
    );
    let table = strings(&mut opened, "PortugueseBR");
    assert_eq!(table.get("FE_BACK"), Some("VOLTAR"), "Omega's own text");
    assert_eq!(table.get("OAG_MENU_QUIT"), Some("SAIR"), "our id on top");
}

/// **The race's HUD reads the project language too.** `load_hud` and the track
/// panel call `load_languages` themselves; if either stopped passing the title's
/// `disc_strings`, the HUD would drop to the disc's English while the menus stayed
/// Portuguese, and nothing else here would notice.
#[test]
#[ignore = "needs data/images"]
fn a_pulse_race_hud_captions_come_out_in_portuguese() {
    let Some(path) = oag_testdata::image("pulse-psp-eu.chd") else {
        return;
    };
    let load = |language: &str| {
        oag_raceplay::load(&oag_raceplay::Options {
            source: path.display().to_string(),
            language: Some(language.to_string()),
            ..oag_raceplay::Options::default()
        })
        .expect("loading the race")
    };
    assert_eq!(
        load("PortugueseBR").hud.strings.get("IG_HUD_LAP"),
        Some("Volta")
    );
    assert_eq!(load("English").hud.strings.get("IG_HUD_LAP"), Some("Lap"));
}

const WANT: &str = "ãõçáéíóúâêôàÃÕÇÁÉÍÓÚÂÊÔÀ";

#[test]
#[ignore = "needs data/images"]
fn every_title_draws_the_brazilian_letters_except_c_cedilla_in_two_hud_faces() {
    let sources: [(&str, Option<std::path::PathBuf>); 6] = [
        ("pulse-psp-eu", oag_testdata::image("pulse-psp-eu.chd")),
        ("pulse-ps2-eu", oag_testdata::image("pulse-ps2-eu.chd")),
        ("pure-psp-eu", oag_testdata::image("pure-psp-eu.chd")),
        ("hd", oag_testdata::image("hdfury-ps3-eu-dec.iso")),
        ("omega", oag_testdata::exact("data/extracted/ps4")),
        ("2048", oag_testdata::exact("data/extracted/vita")),
    ];
    for (tag, path) in sources {
        let Some(path) = path else { continue };
        let mut opened = open(&path);
        let english = opened
            .languages
            .iter()
            .find(|l| l.name == "English" || l.name == "American")
            .expect("English, or American on a release with no plain English")
            .clone();
        for (role, file) in &english.fonts {
            // The button-glyph face carries no letters on any title.
            if role == "Buttons" {
                continue;
            }
            let font = opened.archives.read_font(file).expect("the face reads");
            let missing: String = WANT
                .chars()
                .filter(|c| {
                    !font
                        .glyphs
                        .iter()
                        .any(|g| u32::from(g.codepoint) == *c as u32)
                })
                .collect();
            // Pure's own `small.fnt` is a different, larger file than Pulse's.
            let hud_face_without_cedilla = (tag.starts_with("pulse") || tag.starts_with("pure"))
                && (role == "HUD" || (role == "HUDSmall" && tag.starts_with("pulse")));
            let expected = if hud_face_without_cedilla { "çÇ" } else { "" };
            assert_eq!(missing, expected, "{tag} {role} {file}");
            if hud_face_without_cedilla {
                let atlas = oag_ui::font::Atlas::from_font(&font);
                assert!(
                    atlas.cell('ç').is_some() && atlas.cell('Ç').is_some(),
                    "{tag} {file}: the base letter C stands in for a missing cedilla"
                );
            }
        }
    }
}
