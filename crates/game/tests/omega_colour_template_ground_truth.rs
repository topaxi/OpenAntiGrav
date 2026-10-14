//! Omega's `skin.xml` carries one base colour block and one overriding block
//! per front-end style, and the boot reads the one the executable does.
//!
//! The base block, `HD_Colours`, says in its own comment that it is *"IGNORED
//! AND OVERWRITTEN by the WOHD_Colours and Fury_Colours templates"*. The
//! template blocks' `<Variable>`s carry `name=` and no `global=`, so a reader
//! that only follows `global=` never sees them: `HD_Grey` stayed `0xFF646464`
//! and the Ship Select statistics bar, a block whose own fill is `HD_Grey` over
//! an `AlwaysSolidColor` of `0xff646464`, drew one grey from end to end.
//!
//! `FUN_0154dd80` (the boot screen's constructor, on both `eboot.bin`s)
//! chooses with `strcasecmp("HD", "HD")`, so `WOHD_Colours` is taken every
//! time; see `docs/formats/omega-frontend.md`.

use std::path::PathBuf;

use oag_game::boot;
use oag_ui::screen::{Screens, parse_argb};

fn omega() -> Option<PathBuf> {
    oag_testdata::exact("data/extracted/ps4")
}

fn skin(source: &std::path::Path) -> String {
    let mut archives = oag_omega::open(&source.display().to_string()).expect("Omega opens");
    let bytes = archives
        .read_name(oag_omega::frontend::names::FRONTEND_ROOT)
        .expect("skin.xml reads");
    String::from_utf8(bytes).expect("skin.xml is text")
}

fn globals(xml: &str, template: Option<&str>) -> Vec<(String, String)> {
    let mut all: Vec<_> = Screens::from_xml_with_fallbacks(xml, &[], &[], template)
        .globals
        .into_iter()
        .collect();
    all.sort();
    all
}

/// The ids whose value the template changes, against the file read without it.
#[test]
#[ignore = "needs the decrypted PS4 package pair in data/extracted/ps4/"]
fn the_template_moves_exactly_the_colour_ids_the_base_block_declares_differently() {
    let Some(source) = omega() else { return };
    let xml = skin(&source);
    let base = globals(&xml, None);
    let wohd = globals(&xml, Some("WOHD_Colours"));
    let changed: Vec<(&str, &str, &str)> = base
        .iter()
        .zip(&wohd)
        .filter(|(before, after)| before != after)
        .map(|((name, before), (_, after))| (name.as_str(), before.as_str(), after.as_str()))
        .collect();
    assert_eq!(
        changed,
        [("HD_Grey", "0xFF646464", "0xFF010001")],
        "HD_Colours and WOHD_Colours agree on HD_Blue, HD_LightGrey, HD_White, HD_BG and \
         HD_transBG, and HD_SubGrey is a registration (global=) both read last-wins"
    );
    assert_eq!(base.len(), wohd.len(), "an override adds no global");

    // The other template is a different style, and selecting it shows the rule
    // is by name: Fury is black.
    let fury = globals(&xml, Some("Fury_Colours"));
    let value = |all: &[(String, String)], name: &str| {
        all.iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.clone())
    };
    assert_eq!(value(&fury, "HD_BG").as_deref(), Some("0xff000000"));
    assert_eq!(value(&fury, "HD_Grey").as_deref(), Some("0xff969696"));
    // A name the file does not carry changes nothing.
    assert_eq!(globals(&xml, Some("No_Such_Colours")), base);
}

/// The boot resolves through the title's own template, and the Ship Select
/// statistics bar's two halves are two colours.
#[test]
#[ignore = "needs the decrypted PS4 package pair in data/extracted/ps4/"]
fn the_boot_reads_wohd_and_the_ship_select_bar_has_two_colours() {
    let Some(source) = omega() else { return };
    let options = boot::Options {
        source: source.display().to_string(),
        language: None,
        dlc: Vec::new(),
        leg: oag_ui::frontend::Leg::LogoFmv,
        movie: None,
        cache: std::env::temp_dir().join("oag-omega-colour-template-ground-truth"),
        audio_cache: oag_source::cache::default_audio_cache_dir(),
        extent: oag_game::movie::Extent::Frames(1),
        no_video: true,
        refresh_video: false,
        prefer_av1_cache: false,
    };
    let shell = boot::load_shell(&options).expect("the shell loads").0;
    let resolve = |name: &str| {
        shell
            .screens
            .resolve(&format!("FEGlobals->{name}"))
            .and_then(parse_argb)
    };
    assert_eq!(resolve("HD_Grey"), Some(0xFF01_0001));
    assert_eq!(resolve("HD_BG"), Some(0xFFFF_FFFF));
    assert_eq!(resolve("HD_Blue"), Some(0xFFEE_0707));
    // Registered by all three blocks and equal in the two that matter.
    assert_eq!(resolve("HD_SubGrey"), Some(0xFF64_6464));

    let ship = shell.ship_select.as_ref().expect("Team Selection reads");
    // Loyalty and the four ratings: `Slide_0`..`Slide_4`, each `color="FEGlobals->HD_Grey"`
    // over an `AlwaysSolidColor` of `0xff646464`.
    let bars: Vec<u32> = ship
        .screen
        .blocks
        .iter()
        .filter(|block| {
            block
                .name
                .as_deref()
                .is_some_and(|n| n.starts_with("Slide_"))
        })
        .map(|block| block.color)
        .collect();
    assert_eq!(bars.len(), 5, "{:?}", ship.screen.blocks);
    assert!(
        bars.iter().all(|&color| color == 0xFF01_0001),
        "the bars' left halves are WOHD's HD_Grey, not the grey they are laid over: {bars:x?}"
    );
}
