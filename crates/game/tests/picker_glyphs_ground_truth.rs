//! A language the picker offers is a name its face can draw.
//!
//! HD's Russian plugin writes its own name as `P??????` - seven bytes
//! `50 3f 3f 3f 3f 3f 3f` in the file, so no decoder can recover it - and its
//! own string table authors the real one as `OPT_RUSSIAN` = `Русский`. The
//! picker draws in the boot language's `Default` face (`helv.fnt` on HD and
//! Omega), which carries no Cyrillic, so Russian is not offered there; it used
//! to draw as a row of question marks, the one glyph every face has.
//!
//! A coverage check alone would stay green on `P??????`, so the test also
//! refuses a `?`. Dropping `load::pickable` from `boot::load_shell` or
//! `load::repair_native_name` from `load_languages` fails here.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
//!     -E 'binary(picker_glyphs_ground_truth)'
//! ```

use oag_game::boot;
use oag_ui::language::load::undrawable;

fn load(source: &str) -> Option<boot::Boot> {
    let source = oag_testdata::exact(source)?;
    let options = boot::Options {
        language: None,
        source: source.display().to_string(),
        dlc: Vec::new(),
        leg: oag_ui::frontend::Leg::LogoFmv,
        movie: None,
        cache: std::env::temp_dir().join("oag-picker-glyphs-ground-truth"),
        audio_cache: oag_source::cache::default_audio_cache_dir(),
        extent: oag_game::movie::Extent::Frames(oag_game::INTRO_FRAMES_NEEDED),
        no_video: true,
        refresh_video: false,
        prefer_av1_cache: false,
    };
    Some(boot::load(&options).expect("loading the boot sequence"))
}

/// Every offered row's name is lossless and drawn in the face that draws it.
fn check(source: &str) -> Option<Vec<String>> {
    let loaded = load(source)?;
    assert!(
        !loaded.languages.is_empty(),
        "{source}: no language offered"
    );
    for language in &loaded.languages {
        assert_eq!(
            undrawable(&language.native_name, &loaded.font),
            None,
            "{source}: {} is offered as {:?}, which its face cannot spell",
            language.name,
            language.native_name
        );
    }
    Some(
        loaded
            .languages
            .iter()
            .map(|language| language.name.clone())
            .collect(),
    )
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn hd_offers_no_row_it_cannot_draw_and_leaves_russian_out() {
    let Some(names) = check("data/images/hdfury-ps3-eu-dec.iso") else {
        return;
    };
    assert!(!names.iter().any(|name| name == "Russian"), "{names:?}");
    assert!(names.iter().any(|name| name == "Portuguese"));
    assert_eq!(names.len(), 11 + 1, "eleven disc languages and Brazilian");
}

#[test]
#[ignore = "needs the unpacked Omega package in data/extracted/"]
fn omega_offers_no_row_it_cannot_draw() {
    let Some(names) = check("data/extracted/ps4") else {
        return;
    };
    assert!(!names.iter().any(|name| name == "Russian"), "{names:?}");
}

#[test]
#[ignore = "needs the unpacked 2048 package in data/extracted/"]
fn the_vita_offers_no_row_it_cannot_draw() {
    let _ = check("data/extracted/vita/PCSF00007");
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_psp_titles_offer_no_row_they_cannot_draw() {
    let _ = check("data/images/pulse-psp-eu.chd");
    let _ = check("data/images/pure-psp-eu.chd");
}

/// A face that carries Cyrillic reads the disc's own name, not `P??????`.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn russian_reads_its_own_name_from_its_own_table() {
    let Some(image) = oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let mut archives = oag_assets::Archives::open(&image.to_string_lossy(), oag_hd::TITLE)
        .expect("the archives open");
    let mut report = Vec::new();
    let languages = oag_ui::language::load::load_languages(
        &mut archives,
        &[r"Languages\Russian", r"Languages\Japanese"],
        None,
        &mut report,
    );
    let native = |name: &str| {
        languages
            .iter()
            .find(|l| l.name == name)
            .map(|l| l.native_name.as_str())
    };
    assert_eq!(native("Russian"), Some("Русский"), "{report:?}");
    assert!(
        report.iter().any(|line| line.contains("OPT_RUSSIAN")),
        "the repair is reported: {report:?}"
    );
    // Not lossy, only mislabelled: left exactly as the disc wrote it.
    assert_eq!(native("Japanese"), Some("Svenska"));
}
