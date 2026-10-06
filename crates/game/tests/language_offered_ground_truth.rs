//! Which languages each PSP/PS2 release offers, read off its own disc.
//!
//! **`#[ignore]`d and never run in CI**; needs the disc images (`just test-data`).
//!
//! The list comes from [`oag_title::FrontEnd::offered_languages`], which holds
//! what each release's executable manifest names (`oag_pulse::LANGUAGE_MANIFESTS`,
//! `oag_pure::LANGUAGE_MANIFESTS`), and every plugin it names must resolve to a
//! language on that disc. The retail listings cited below are corroboration, not
//! measurement: Pulse PSP EU and US and Pulse PS2 EU list English, French,
//! German, Spanish and Italian; Pure EU lists those five and Pure US three.

const PULSE_EU: &[(&str, &str)] = &[
    ("PI000", "English"),
    ("PI008", "French"),
    ("PI009", "German"),
    ("PI010", "Spanish"),
    ("PI011", "Italian"),
];
const PULSE_US: &[(&str, &str)] = &[
    ("PI012", "English"),
    ("PI010", "Spanish"),
    ("PI008", "French"),
    ("PI009", "German"),
    ("PI011", "Italian"),
];
const PURE_EU: &[(&str, &str)] = &[
    ("PI000", "English"),
    ("PI010", "Spanish"),
    ("PI008", "French"),
    ("PI009", "German"),
    ("PI011", "Italian"),
];
const PURE_US: &[(&str, &str)] = &[
    ("PI000", "English"),
    ("PI010", "Spanish"),
    ("PI008", "French"),
];

fn check(image: &str, serial: &str, expected: &[(&str, &str)]) {
    let Some(path) = oag_testdata::image(image) else {
        return;
    };
    let opened =
        oag_source::title::open_source(&path.display().to_string(), Vec::new(), Vec::new())
            .expect("the source opens");
    let mut archives = opened.archives;
    assert_eq!(archives.layout.serial.as_deref(), Some(serial), "{image}");
    let front_end = opened.title.front_end.expect("a front end");
    let plugins = front_end.offered_languages(Some(serial));
    let mut report = Vec::new();
    let languages =
        oag_ui::language::load::load_languages(&mut archives, plugins, None, &mut report);
    let got: Vec<(&str, &str)> = languages
        .iter()
        .map(|l| (l.plugin.as_str(), l.name.as_str()))
        .collect();
    // The disc's own languages in the disc's order, then the project language
    // after them, standing on the disc's English plugin (2026-10-06).
    let mut expected = expected.to_vec();
    expected.push((expected[0].0, "PortugueseBR"));
    assert_eq!(got, expected, "{image}: {report:?}");
    assert_eq!(
        oag_ui::language::load::chosen_language(&languages, None).map(|l| l.name.as_str()),
        Some("English"),
        "{image}: English is what a boot with nothing saved reads"
    );
}

#[test]
#[ignore = "needs data/images"]
fn pulse_psp_eu_offers_english_from_pi000() {
    check("pulse-psp-eu.chd", "UCES-00465", PULSE_EU);
}

#[test]
#[ignore = "needs data/images"]
fn pulse_psp_usa_offers_its_five() {
    check("pulse-psp-usa.chd", "UCUS-98712", PULSE_US);
}

#[test]
#[ignore = "needs data/images"]
fn pulse_ps2_eu_offers_its_five() {
    check("pulse-ps2-eu.chd", "SCES-54748", PULSE_US);
}

#[test]
#[ignore = "needs data/images"]
fn pure_psp_eu_offers_five_in_manifest_order() {
    check("pure-psp-eu.chd", "UCES-00001", PURE_EU);
}

#[test]
#[ignore = "needs data/images"]
fn pure_psp_usa_offers_three_not_the_five_on_the_disc() {
    check("pure-psp-usa.chd", "UCUS-98612", PURE_US);
}
