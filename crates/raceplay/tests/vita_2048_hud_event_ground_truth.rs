//! Wipeout 2048's `XP` and `OBJECTIVE` lines on the extracted package: a campaign event draws the
//! first and neither draws the second; a DLC circuit raced without one draws neither.
//!
//! **`#[ignore]`d and never run in CI.** It needs the extracted Vita packages, base, patch and DLC1,
//! under `data/extracted/vita/PCSF00007`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-raceplay --run-ignored all \
//!     -E 'test(/vita_2048_hud_event_ground_truth::/)'
//! ```
//!
//! The law is `Hud_BindWidgets` (`0x81192cd0`), which binds the current `SP.xml` event at `hud+4`:
//! `docs/ghidra/functions/vita-2048-eu-v104/race-hud-selection.md`. The originals this is checked
//! against are `data/reference/compare-page/2048-vita/empire-climb` (an event: `XP 0`, no
//! `OBJECTIVE`) and `dlc-vineta-k` (the HD Campaign's Uplift Race: neither).
//!
//! What this fails on: dropping `Setup::campaign_event` from `load_event`, dropping either of
//! `text_for`'s two gates, or a layout that stops authoring the widgets (the vacuity guards).
//!
//! The DLC load also checks the composite's exposure keys
//! ([`the_pack_copy_sets_the_exposure_held_for_a_comparison`]): one load serves both claims.

use std::path::{Path, PathBuf};

use oag_ui::frontend::Draw;

const VINETA_K: &str = r"Data\art\published\DLC1\environments\Vineta_K\track.vex";

fn source() -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/extracted/vita/PCSF00007");
    if path.join("base/PSP2/data.psarc").exists() && path.join("dlc1/PSP2/dlc1.psarc").exists() {
        return Some(path);
    }
    assert!(
        std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
        "OAG_REQUIRE_GAME_DATA is set but {} is missing",
        path.display()
    );
    println!("skipping: {} not complete", path.display());
    None
}

fn options(source: &Path) -> oag_raceplay::Options {
    oag_raceplay::Options {
        source: source.display().to_string(),
        seed: Some(1),
        ..oag_raceplay::Options::default()
    }
}

/// Every `HUD`-font string the loaded race's first frame draws, and whether the layout authors
/// the two widgets at all.
fn first_frame(loaded: oag_raceplay::Loaded) -> (Vec<String>, bool, bool) {
    let race = oag_raceplay::Race::start(loaded.setup);
    let readout = race.readout();
    let context = loaded.hud.context().expect("the layout parses");
    let authored = (
        context.layout.label("RaceXPTxt").is_some(),
        context.layout.label("ObjectiveTxt").is_some(),
    );
    let text = oag_hud::draw_list(&context, &readout)
        .hud_text
        .iter()
        .filter_map(|draw| match draw {
            Draw::Text { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect();
    (text, authored.0, authored.1)
}

/// `2048 - Event 1` authors a pass objective and no `m_options`: the original draws `XP` and no
/// `OBJECTIVE`, and so do we, because `load_event` says an event exists.
#[test]
#[ignore = "needs the extracted package under data/extracted/vita/"]
fn a_campaign_event_draws_xp_and_no_objective() {
    let Some(source) = source() else { return };
    let loaded = oag_raceplay::load_event(&options(&source), "2048 - Event 1").expect("it loads");
    assert!(loaded.campaign_2048_event.is_some());
    assert!(loaded.setup.campaign_event);
    let (text, xp_authored, objective_authored) = first_frame(loaded);
    assert!(xp_authored && objective_authored, "the layout authors both");
    assert!(text.iter().any(|text| text == "XP"), "{text:?}");
    assert!(
        !text.iter().any(|text| text.contains("OBJECTIVE")),
        "{text:?}"
    );
}

/// The DLC's Vineta K raced as the HD Campaign's Race is: no event object. Neither line draws,
/// as in the original's frame, and the rest of the HUD does.
#[test]
#[ignore = "needs the extracted package under data/extracted/vita/"]
fn a_dlc_circuit_raced_with_no_event_draws_neither() {
    let Some(source) = source() else { return };
    let loaded = oag_raceplay::load(&oag_raceplay::Options {
        track: Some(VINETA_K.to_string()),
        team: Some("Feisar".to_string()),
        mode: oag_race::Mode::SingleRace,
        exposure_hold: Some(1.15),
        ..options(&source)
    })
    .expect("the DLC circuit loads");
    the_pack_copy_sets_the_exposure_held_for_a_comparison(&loaded);
    assert!(loaded.campaign_2048_event.is_none());
    assert!(!loaded.setup.campaign_event);
    let (text, xp_authored, objective_authored) = first_frame(loaded);
    assert!(xp_authored && objective_authored, "the layout authors both");
    assert!(!text.iter().any(|text| text == "XP"), "{text:?}");
    assert!(
        !text.iter().any(|text| text.contains("OBJECTIVE")),
        "{text:?}"
    );
    assert!(
        text.iter().any(|text| text == "POS"),
        "the place still draws: {text:?}"
    );
}

/// Vineta K's exposure keys come from `dlc1.psarc`'s copy of its `track.EnvSettings` (scale 0.5,
/// max 8, gate 0.3), not the base `data.psarc`'s (0.6, 12, 0.2): the original's setters read the
/// pack's on that circuit (`docs/ghidra/functions/vita-2048-eu-v104/exposure.md`). The comparison
/// switch `Options::exposure_hold` lands as a held exposure.
fn the_pack_copy_sets_the_exposure_held_for_a_comparison(loaded: &oag_raceplay::Loaded) {
    use oag_post::wo_composite::{Exposure, Params};
    assert_eq!(
        loaded.wo_composite,
        Some(Params {
            exposure_scale: 0.5,
            exposure_max: 8.0,
            exposure_speed: 0.05,
            bloom_factor: 0.2,
            bloom_gate: 0.3,
            exposure: Exposure::Held(1.15),
        }),
        "{:#?}",
        loaded.report
    );
}

/// The HD Campaign's Zone event reads `Zone_HUD_HD.xml`, which authors the `SCORE` readout top
/// centre and `SpeedClass`/`Zone Txt`/`ZonePlus0` bottom left (the original's frames
/// `67-zone-grid`/`68-zone-5`); the native `Zone_HUD.xml` authors none of them. Fails if
/// `Options::add_on_campaign` stops selecting the layout.
#[test]
#[ignore = "needs the extracted package under data/extracted/vita/"]
fn the_hd_campaigns_zone_reads_its_hd_layout() {
    let Some(source) = source() else { return };
    let load = |campaign: Option<&str>| {
        oag_raceplay::load(&oag_raceplay::Options {
            track: Some(VINETA_K.to_string()),
            team: Some("Feisar".to_string()),
            mode: oag_race::Mode::Zone,
            add_on_campaign: campaign.map(str::to_string),
            ..options(&source)
        })
        .expect("the DLC circuit loads")
    };
    let authored = |loaded: &oag_raceplay::Loaded| {
        let layout = loaded.hud.context().expect("the layout parses").layout;
        ["Score", "SpeedClass", "Zone Txt", "ZonePlus0"].map(|name| layout.label(name).is_some())
    };
    let native = load(None);
    assert_eq!(authored(&native), [false; 4], "{:#?}", native.report);
    let hd = load(Some("HD"));
    assert!(
        hd.report
            .iter()
            .any(|line| line.contains("Zone_HUD_HD.xml")),
        "{:#?}",
        hd.report
    );
    assert_eq!(authored(&hd), [true; 4], "{:#?}", hd.report);
}
