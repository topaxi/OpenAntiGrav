//! Which languages each release offers, read off its own executable.
//!
//! **`#[ignore]`d and never run in CI**; needs the disc images (`just test-data`).
//!
//! The list comes from [`oag_title::FrontEnd::offered_languages`], which holds
//! what each release's executable manifest names (`oag_pulse::LANGUAGE_MANIFESTS`,
//! `oag_pure::LANGUAGE_MANIFESTS`), and every plugin it names must resolve to a
//! language on that disc. The retail listings cited below are corroboration, not
//! measurement: Pulse PSP EU and US and Pulse PS2 EU list English, French,
//! German, Spanish and Italian; Pure EU lists those five and Pure US three.
//!
//! HD, 2048 and Omega have **no manifest and no picker**: their executables pick
//! one plugin from the console's system language, and what a release offers is
//! what that choice can reach (`oag_hd::frontend::LANGUAGE_MANIFESTS`,
//! `oag_2048::frontend::LANGUAGE_MANIFESTS`, `oag_omega::frontend::LANGUAGE_MANIFESTS`).
//! Retail listings, corroboration only: 2048 EU lists thirteen languages (no
//! American, Japanese, Korean or Traditional Chinese), which is exactly what its
//! executable reaches; Omega's EU store page lists twelve against the fourteen
//! its patched table gives; HD EU's listing is unknown. The picker order for
//! those three is chosen, not measured, except Omega's, which is its table's.

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

const HD_EU: &[(&str, &str)] = &[
    (r"Languages\English", "English"),
    (r"Languages\French", "French"),
    (r"Languages\Spanish", "Spanish"),
    (r"Languages\German", "German"),
    (r"Languages\Italian", "Italian"),
    (r"Languages\Dutch", "Dutch"),
    (r"Languages\Portuguese", "Portuguese"),
    (r"Languages\Russian", "Russian"),
    (r"Languages\Finnish", "Finnish"),
    (r"Languages\Swedish", "Swedish"),
    (r"Languages\Danish", "Danish"),
    (r"Languages\Norwegian", "Norwegian"),
];
const V2048_EU: &[(&str, &str)] = &[
    (r"languages\english", "English"),
    (r"languages\french", "French"),
    (r"languages\spanish", "Spanish"),
    (r"languages\german", "German"),
    (r"languages\italian", "Italian"),
    (r"languages\dutch", "Dutch"),
    (r"languages\portuguese", "Portuguese"),
    (r"languages\russian", "Russian"),
    (r"languages\finnish", "Finnish"),
    (r"languages\swedish", "Swedish"),
    (r"languages\danish", "Danish"),
    (r"languages\norwegian", "Norwegian"),
    (r"languages\polish", "Polish"),
];
const V2048_US: &[(&str, &str)] = &[
    (r"languages\american", "American"),
    (r"languages\french", "French"),
    (r"languages\spanish", "Spanish"),
];
const OMEGA_EU: &[(&str, &str)] = &[
    (r"Languages\english", "English"),
    (r"Languages\french", "French"),
    (r"Languages\spanish", "Spanish"),
    (r"Languages\german", "German"),
    (r"Languages\italian", "Italian"),
    (r"Languages\dutch", "Dutch"),
    (r"Languages\portuguese", "Portuguese"),
    (r"Languages\russian", "Russian"),
    (r"Languages\finnish", "Finnish"),
    (r"Languages\swedish", "Swedish"),
    (r"Languages\danish", "Danish"),
    (r"Languages\norwegian", "Norwegian"),
    (r"Languages\polish", "Polish"),
    (r"Languages\turkish", "Turkish"),
    (r"Languages\portuguesebr", "portuguesebr"),
];

fn check(image: &str, serial: &str, expected: &[(&str, &str)]) {
    let Some(path) = oag_testdata::image(image) else {
        return;
    };
    check_at(image, &path, Some(serial), serial, expected);
}

/// `reported` is what the source itself says its serial is: `None` for the one
/// tree that keeps no `param.sfo`, whose release the title then assumes.
fn check_at(
    image: &str,
    path: &std::path::Path,
    reported: Option<&str>,
    serial: &str,
    expected: &[(&str, &str)],
) {
    let opened =
        oag_source::title::open_source(&path.display().to_string(), Vec::new(), Vec::new())
            .expect("the source opens");
    let mut archives = opened.archives;
    assert_eq!(archives.layout.serial.as_deref(), reported, "{image}");
    let front_end = opened.title.front_end.expect("a front end");
    let plugins = front_end.offered_languages(archives.layout.serial.as_deref());
    assert_eq!(
        plugins,
        front_end.offered_languages(Some(serial)),
        "{image}: the release this source reports is the one that was pinned"
    );
    let mut report = Vec::new();
    let languages =
        oag_ui::language::load::load_languages(&mut archives, plugins, None, &mut report);
    let got: Vec<(&str, &str)> = languages
        .iter()
        .map(|l| (l.plugin.as_str(), l.name.as_str()))
        .collect();
    // The disc's own languages in the disc's order, then the project language
    // after them, standing on the disc's English plugin (2026-10-06).
    // A disc that ships it itself (Omega) is the base instead and gets no copy.
    let mut expected = expected.to_vec();
    if !expected
        .iter()
        .any(|(_, name)| name.eq_ignore_ascii_case("PortugueseBR"))
    {
        expected.push((expected[0].0, "PortugueseBR"));
    }
    assert_eq!(got, expected, "{image}: {report:?}");
    assert_eq!(
        oag_ui::language::load::chosen_language(&languages, None).map(|l| l.name.as_str()),
        Some(expected[0].1),
        "{image}: the first language (English, or American where the release has no English) is what a boot with nothing saved reads"
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

#[test]
#[ignore = "needs data/images"]
fn hd_eu_reaches_twelve_and_not_the_four_it_ships_unreachable() {
    let Some(path) = oag_testdata::image("hdfury-ps3-eu-dec.iso") else {
        return;
    };
    check_at(
        "hdfury-ps3-eu-dec.iso",
        &path,
        Some("BCES-00664"),
        "BCES-00664",
        HD_EU,
    );
}

#[test]
#[ignore = "needs data/extracted/vita"]
fn two_thousand_forty_eight_eu_reaches_thirteen_not_american_or_the_asian_three() {
    let Some(path) = oag_testdata::exact("data/extracted/vita/PCSF00007") else {
        return;
    };
    check_at(
        "PCSF00007",
        &path,
        Some("PCSF-00007"),
        "PCSF-00007",
        V2048_EU,
    );
}

#[test]
#[ignore = "needs data/extracted/vita"]
fn two_thousand_forty_eight_usa_reaches_american_french_and_spanish() {
    let Some(path) = oag_testdata::exact("data/extracted/vita/PCSA00015") else {
        return;
    };
    check_at(
        "PCSA00015",
        &path,
        Some("PCSA-00015"),
        "PCSA-00015",
        V2048_US,
    );
}

#[test]
#[ignore = "needs data/extracted/ps4"]
fn omega_eu_reaches_fourteen_lists_its_own_brazilian_portuguese_and_assumes_its_release() {
    let Some(path) = oag_testdata::exact("data/extracted/ps4") else {
        return;
    };
    check_at("ps4", &path, None, "CUSA-05670", OMEGA_EU);
}
