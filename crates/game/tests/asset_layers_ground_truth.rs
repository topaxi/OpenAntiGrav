//! The two asset layers against real discs: the census that says the pre-HD
//! discs author no Pilot Assist indicator, an override replacing a disc entry by
//! name, the fallback never winning over one, and Pilot Assist's indicator
//! reaching a Pulse and a Pure race from the fallback.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content; run it with
//! `just test-data`, or only this file:
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
//!     -E 'binary(asset_layers_ground_truth)'
//! ```
//!
//! See `docs/architecture/asset-layers.md` for the decision and the census.

use oag_assets::layers::{Kind, Layer};
use oag_hud::assist::AssistReadout;
use oag_ui::frontend::Draw;

const IMAGES: [(&str, &oag_title::Title); 4] = [
    ("pulse-psp-eu.chd", oag_pulse::TITLE),
    ("pulse-ps2-eu.chd", oag_pulse::TITLE),
    ("pulse-psp-usa.chd", oag_pulse::TITLE),
    ("pure-psp-eu.chd", oag_pure::TITLE),
];

fn open(image: &std::path::Path, title: &'static oag_title::Title) -> oag_assets::Archives {
    oag_assets::Archives::open(&image.display().to_string(), title).expect("the disc opens")
}

/// The widget names HD authors, drawn by the same code; on a pre-HD title the
/// disc must not already carry any of them.
const INDICATOR: [&str; 6] = [
    "AssistIndicatorBG",
    "AssistIndicatorMain",
    "AssistIndicatorLeft",
    "AssistIndicatorLeftBig",
    "AssistIndicatorRight",
    "AssistIndicatorRightBig",
];

/// The census, executable: every HUD layout either title ships, composed as a
/// race composes it, has no widget of the indicator's names and none that says
/// "assist" or "pilot". (A WAD stores only name hashes, so a texture's own name
/// is covered by `scripts/census-assist-indicator.sh`, which hashes candidates.)
#[test]
#[ignore = "needs a disc image in data/images/"]
fn no_pre_hd_hud_layout_authors_an_assist_indicator() {
    for (name, title) in IMAGES {
        let Some(image) = oag_testdata::image(name) else {
            continue;
        };
        let mut archives = open(&image, title);
        let layouts = [
            title.hud.arcade,
            title.hud.time_trial,
            title.hud.speed_lap,
            title.hud.zone,
            title.hud.elimination,
        ];
        let mut read = 0;
        for entry in layouts {
            let Some(composed) = oag_hud::compose(entry, |path| archives.read_name(path).ok())
            else {
                continue;
            };
            read += 1;
            let layout = composed.layout;
            let names = layout
                .sprites
                .iter()
                .map(|s| s.name.as_str())
                .chain(layout.fills.iter().map(|f| f.name.as_str()))
                .chain(layout.labels.iter().map(|l| l.name.as_str()))
                .chain(layout.models.iter().map(|m| m.name.as_str()));
            for widget in names {
                let lower = widget.to_ascii_lowercase();
                assert!(
                    !INDICATOR.contains(&widget)
                        && !lower.contains("assist")
                        && !(lower.contains("pilot") && !lower.contains("autopilot")),
                    "{name}: {entry} authors {widget}"
                );
            }
        }
        assert!(read >= 3, "{name}: only {read} layout(s) read");
    }
}

/// An override replaces a disc entry by name, and the report says so.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn an_override_replaces_a_real_disc_entry_by_name() {
    let Some(image) = oag_testdata::image("pulse-psp-eu.chd") else {
        return;
    };
    let title = oag_pulse::TITLE;
    let entry = title.hud.arcade;
    let mut plain = open(&image, title);
    let disc = plain
        .read_name(entry)
        .expect("the arcade layout is on the disc");

    let dir = std::env::temp_dir().join(format!("oag-layers-gt-{}", std::process::id()));
    let file = dir.join(entry.replace('\\', "/"));
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(&file, b"the player's own layout").unwrap();
    let layer = Layer::from_dir(Kind::Override, &dir).expect("a layer");
    let mut modded = open(&image, title).with_override(layer);

    assert_eq!(modded.read_name(entry).unwrap(), b"the player's own layout");
    assert_ne!(disc, b"the player's own layout");
    let report = modded.take_layer_report();
    assert!(
        report
            .iter()
            .any(|l| l.contains(entry) && l.contains("override")),
        "{report:?}"
    );
    // A name the override does not hold is the disc's, and unreported.
    let other = title.hud.time_trial;
    assert_eq!(
        modded.read_name(other).unwrap(),
        plain.read_name(other).unwrap()
    );
    assert!(modded.take_layer_report().is_empty());
    std::fs::remove_dir_all(&dir).unwrap();
}

/// The fallback answers only what the disc cannot: a substitute filed under a
/// name the disc holds is never read, on a real Pulse and a real Pure.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_fallback_never_wins_over_a_real_disc_entry() {
    for (name, title) in IMAGES {
        let Some(image) = oag_testdata::image(name) else {
            continue;
        };
        let entry = title.hud.arcade;
        let disc = open(&image, title).read_name(entry).expect("on the disc");
        let mut layer = Layer::new(Kind::Fallback, "a substitute");
        layer.insert_bytes(entry, b"a substitute that must never be read");
        let mut archives = open(&image, title).with_fallback(layer);
        assert_eq!(archives.read_name(entry).unwrap(), disc, "{name}");
        assert!(archives.take_layer_report().is_empty(), "{name}");
    }
}

fn indicator_on() -> oag_hud::Readout {
    oag_hud::Readout {
        assist: AssistReadout {
            enabled: true,
            main: true,
            left: true,
            right: false,
        },
        ..oag_hud::Readout::default()
    }
}

fn sprites_of(loaded: &oag_raceplay::Loaded, readout: &oag_hud::Readout) -> usize {
    let context = loaded.hud.context().expect("the layout parses");
    let sprites = oag_hud::draw_list(&context, readout).sprites;
    sprites
        .iter()
        .filter(|draw| matches!(draw, Draw::Sprite { .. } | Draw::RotatedSprite { .. }))
        .count()
}

/// A Pulse and a Pure race load draws the indicator from the fallback, says so in
/// the loader report, and with the assist off draws exactly the HUD it drew before.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn pulse_and_pure_draw_the_indicator_from_the_fallback() {
    for (name, track) in [
        ("pulse-psp-eu.chd", r"Data\Environments\16_Track\track.vex"),
        ("pure-psp-eu.chd", ""),
    ] {
        let Some(image) = oag_testdata::image(name) else {
            continue;
        };
        let loaded = oag_raceplay::load(&oag_raceplay::Options {
            source: image.display().to_string(),
            track: (!track.is_empty()).then(|| track.to_string()),
            seed: Some(1),
            ..oag_raceplay::Options::default()
        })
        .expect("a race loads");

        let off = oag_hud::Readout::default();
        let base = sprites_of(&loaded, &off);
        let on = sprites_of(&loaded, &indicator_on());
        // BG, main, the small left arrow and its big one.
        assert_eq!(
            on,
            base + 4,
            "{name}: the indicator's four widgets are drawn"
        );

        let report = loaded.report.join("\n");
        assert!(
            report.contains("HUD_assist_indicator.xml")
                && report.contains("fallback")
                && report.contains("chosen, not measured"),
            "{name}: the report names the fallback\n{report}"
        );
        assert!(
            report.contains("AssistIndicator.pix"),
            "{name}: and the art it drew"
        );

        // Zone, where the assist never acts, shows no "assist on" plate.
        let zone = oag_raceplay::load(&oag_raceplay::Options {
            source: image.display().to_string(),
            track: (!track.is_empty()).then(|| track.to_string()),
            mode: oag_race::Mode::Zone,
            seed: Some(1),
            ..oag_raceplay::Options::default()
        })
        .expect("a Zone race loads");
        assert_eq!(
            sprites_of(&zone, &indicator_on()),
            sprites_of(&zone, &off),
            "{name}: Zone draws no indicator"
        );
    }
}

/// With the fallback absent (no layer mounted), the layout a race reads is the
/// disc's own: the splice finds nothing and changes nothing. Mounted, it adds the
/// six widgets, every one inside the source's own grid (which checks the PS2's
/// scaling from the PSP's 480x272), and Zone gets none.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn with_no_fallback_the_layout_is_the_discs_own() {
    use oag_race::Mode;
    for (name, title) in IMAGES {
        let Some(image) = oag_testdata::image(name) else {
            continue;
        };
        let mut archives = open(&image, title);
        let composed = oag_hud::compose(title.hud.arcade, |p| archives.read_name(p).ok()).unwrap();
        let before = composed.layout.clone();
        let space = oag_display::space::Space::of(archives.layout.platform);
        let after = oag_raceplay::splice_assist_indicator(
            &mut archives,
            title,
            composed.layout,
            space,
            Mode::SingleRace,
        );
        assert_eq!(before, after, "{name}");
        assert!(archives.take_layer_report().is_empty(), "{name}");

        let mut mounted = open(&image, title).with_fallback(oag_source::layers::fallback_layer());
        let again = oag_raceplay::splice_assist_indicator(
            &mut mounted,
            title,
            before.clone(),
            space,
            Mode::SingleRace,
        );
        assert_eq!(again.sprites.len(), before.sprites.len() + 6, "{name}");
        for sprite in &again.sprites[before.sprites.len()..] {
            let [x, y, w, h] = sprite.rect;
            assert!(
                x >= 0.0 && y >= 0.0 && x + w <= space.size.0 && y + h <= space.size.1,
                "{name}: {} at {:?} is off a {:?} grid",
                sprite.name,
                sprite.rect,
                space.size
            );
        }

        let zone = oag_raceplay::splice_assist_indicator(
            &mut mounted,
            title,
            before.clone(),
            space,
            Mode::Zone,
        );
        assert_eq!(
            zone, before,
            "{name}: Zone, where the assist never acts, draws no indicator"
        );
    }
}
