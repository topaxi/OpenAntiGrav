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

#[test]
#[ignore = "needs data/images"]
fn pure_reads_the_discs_english_with_our_ids_in_portuguese() {
    let Some(path) = oag_testdata::image("pure-psp-eu.chd") else {
        return;
    };
    let mut opened = open(&path);
    assert_eq!(
        opened.disc_strings, None,
        "no Pure namespace is written yet"
    );
    let english = strings(&mut opened, "English");
    let table = strings(&mut opened, "PortugueseBR");
    assert!(
        !english.is_empty(),
        "Pure keeps English inline in its definition"
    );
    // Same ids as English (the disc's, plus our `OAG_` ones): nothing is lost
    // and nothing is empty, only our own ids differ.
    assert_eq!(table.len(), english.len());
    assert_ne!(table.get("OAG_MENU_QUIT"), english.get("OAG_MENU_QUIT"));
    assert_eq!(table.get("OAG_MENU_QUIT"), Some("SAIR"));
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
            .find(|l| l.name == "English")
            .expect("English")
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
