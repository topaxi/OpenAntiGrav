//! Omega's Pilot Assist HUD indicator, off Omega's own layouts and `.gnf` atlases.
//!
//! **`#[ignore]`d and never run in CI.** It needs the decrypted PS4 packages in
//! `data/extracted/ps4/`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
//!     -E 'binary(omega_assist_hud_ground_truth)'
//! ```
//!
//! What this fails on: dropping `oag_omega::hud::ART.assist` (the sprites stop
//! appearing with the option on), wiring a fallback fragment over Omega's own
//! (the report would name `fallback`), or a layout that stops carrying the four
//! widgets.

use oag_hud::assist::AssistReadout;
use oag_ui::frontend::Draw;

const NAMES: [&str; 6] = [
    "AssistIndicatorBG",
    "AssistIndicatorMain",
    "AssistIndicatorLeft",
    "AssistIndicatorLeftBig",
    "AssistIndicatorRight",
    "AssistIndicatorRightBig",
];

fn open() -> Option<oag_assets::Archives> {
    let source = oag_testdata::exact("data/extracted/ps4")?;
    Some(oag_omega::open(&source.display().to_string()).expect("Omega opens"))
}

/// The four roots a race can use carry all six widgets themselves, the Zone root
/// none, and each widget's texture resolves in the archives.
#[test]
#[ignore = "needs the decrypted PS4 package pair in data/extracted/ps4/"]
fn the_roots_author_the_indicator_and_its_textures_resolve() {
    let Some(mut archives) = open() else { return };
    let hud = oag_omega::TITLE.hud;
    let assist = oag_omega::hud::ART
        .assist
        .expect("Omega draws an indicator");
    assert!(assist.fragment.is_none(), "Omega's layouts embed it");
    let wanted: Vec<&str> = [assist.background, assist.main]
        .into_iter()
        .chain(assist.left.iter().copied())
        .chain(assist.right.iter().copied())
        .collect();
    assert_eq!(wanted, NAMES);
    for (root, expect) in [
        (hud.arcade, true),
        (hud.time_trial, true),
        (hud.speed_lap, true),
        (hud.elimination, true),
        (hud.zone, false),
    ] {
        let composed = oag_hud::compose(root, |p| archives.read_name(p).ok()).expect(root);
        for name in NAMES {
            let sprite = composed.layout.sprite(name);
            assert_eq!(sprite.is_some(), expect, "{root}: {name}");
            if let Some(sprite) = sprite {
                let texture = format!(
                    "{}{}",
                    sprite.src.rsplit_once('.').map_or(&*sprite.src, |s| s.0),
                    oag_omega::hud::ART.texture_extension.expect("gnf"),
                );
                archives
                    .read_name(&texture)
                    .unwrap_or_else(|e| panic!("{root}: {name} texture {texture}: {e}"));
            }
        }
    }
}

fn readout(on: bool) -> oag_hud::Readout {
    oag_hud::Readout {
        assist: AssistReadout {
            enabled: on,
            main: on,
            left: on,
            right: false,
        },
        ..oag_hud::Readout::default()
    }
}

fn sprites_of(loaded: &oag_raceplay::Loaded, readout: &oag_hud::Readout) -> usize {
    let context = loaded.hud.context().expect("the layout parses");
    oag_hud::draw_list(&context, readout)
        .sprites
        .iter()
        .filter(|d| matches!(d, Draw::Sprite { .. } | Draw::RotatedSprite { .. }))
        .count()
}

/// With the option on the BG, the main icon and the left arrow with its big
/// partner join the draw list; off, the list is the HUD's own; Zone has none.
#[test]
#[ignore = "needs the decrypted PS4 package pair in data/extracted/ps4/"]
fn the_indicator_is_in_the_draw_list_with_the_assist_on_and_absent_off() {
    let Some(source) = oag_testdata::exact("data/extracted/ps4") else {
        return;
    };
    let load = |mode| {
        oag_raceplay::load(&oag_raceplay::Options {
            source: source.display().to_string(),
            mode,
            seed: Some(1),
            ..oag_raceplay::Options::default()
        })
        .expect("an Omega race loads")
    };
    let loaded = load(oag_race::Mode::TimeTrial);
    let base = sprites_of(&loaded, &readout(false));
    assert_eq!(sprites_of(&loaded, &readout(true)), base + 4);
    let report = loaded.report.join("\n");
    assert!(
        !report.contains("fallback"),
        "Omega uses its own art\n{report}"
    );

    let zone = load(oag_race::Mode::Zone);
    assert_eq!(
        sprites_of(&zone, &readout(true)),
        sprites_of(&zone, &readout(false)),
        "Zone draws no indicator"
    );
}
