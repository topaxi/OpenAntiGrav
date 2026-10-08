//! The "medal awarded" line, on Pulse's own HUD layout and string table.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content; run it with
//! `just test-data`, or only this file:
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
//!     -E 'binary(medal_message_ground_truth)'
//! ```
//!
//! # What only real data can say here
//!
//! That `ER_GMA`/`ER_SMA`/`ER_BMA` resolve in the table the HUD draws with, so
//! the line reads "Gold medal awarded" and not the bare id; that the Zone and
//! Speed Lap layouts author the `Info1`..`Info4` widgets it is drawn in; and that a message raised on a real `Race` reaches the drawn frame
//! and is gone four seconds on. The trigger - a medal earned in a campaign
//! Zone or Speed Lap - is chosen, not measured; see
//! `oag_hud::messages` for what is.

use oag_gameplay::PlayerInputs;
use oag_ui::frontend::Draw;

fn load(image: &std::path::Path, mode: oag_race::Mode) -> oag_raceplay::Loaded {
    oag_raceplay::load(&oag_raceplay::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode,
        track: Some(r"Data\Environments\16_Track\track.vex".to_string()),
        seed: Some(1),
        ..oag_raceplay::Options::default()
    })
    .expect("loading 16_Track")
}

fn drawn(loaded: &oag_raceplay::Loaded, readout: &oag_hud::Readout) -> Vec<String> {
    let context = loaded.hud.context().expect("Pulse's layout parses");
    let frame = oag_hud::draw_list(&context, readout);
    frame
        .hud_text
        .iter()
        .filter_map(|draw| match draw {
            Draw::Text { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect()
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_raised_medal_phrase_reads_as_the_disc_words_it_and_expires() {
    let Some(image) = oag_testdata::image("pulse-psp-usa.chd") else {
        return;
    };
    for mode in [oag_race::Mode::Zone, oag_race::Mode::SpeedLap] {
        let loaded = load(&image, mode);
        let mut race = oag_raceplay::Race::start(loaded.setup.clone());
        let context = loaded.hud.context().expect("Pulse's layout parses");
        for id in ["ER_GMA", "ER_SMA", "ER_BMA"] {
            assert_ne!(
                context.strings.get_or_id(id),
                id,
                "{id} resolves ({mode:?})"
            );
        }
        let line = |race: &oag_raceplay::Race| {
            drawn(&loaded, &race.readout())
                .into_iter()
                .find(|text| text.to_lowercase().contains("medal"))
        };
        assert_eq!(line(&race), None, "nothing before a medal ({mode:?})");

        race.raise_message("ER_GMA", true);
        for _ in 0..30 {
            race.tick(&PlayerInputs::none());
        }
        let shown = line(&race).unwrap_or_else(|| panic!("a raised medal draws ({mode:?})"));
        assert_eq!(shown, context.strings.get_or_id("ER_GMA"));

        for _ in 0..300 {
            race.tick(&PlayerInputs::none());
        }
        assert_eq!(line(&race), None, "gone after its four seconds ({mode:?})");
    }
}

/// HD authors the same four `Info` widgets in its Zone and Speed Lap layouts
/// (`InfoTextParent`), and its own English table carries the three phrases: the
/// line draws there in HD's own font and place, from HD's own strings.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn hd_zone_and_speed_lap_draw_the_phrase_from_hds_own_table() {
    let Some(image) = oag_testdata::image("hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let mut archives = oag_hd::open(&image.display().to_string()).expect("HD opens");
    let blob = archives
        .read_name(r"Data\Plugins\Languages\English\entries.xml")
        .expect("English entries.xml");
    let strings = oag_ui::language::StringTable::from_xml(
        &oag_tables::fexml::text(&blob).expect("entries.xml is text"),
    );
    for id in ["ER_GMA", "ER_SMA", "ER_BMA"] {
        assert!(strings.get(id).is_some(), "{id} is in HD's table");
    }
    for root in [oag_hd::hud::layouts::ZONE, oag_hd::hud::layouts::SPEED_LAP] {
        let mut read = |path: &str| archives.read_name(path).ok();
        let layout = oag_hud::compose(root, &mut read)
            .unwrap_or_else(|| panic!("composing {root}"))
            .layout;
        for widget in oag_hud::messages::WIDGETS {
            assert!(layout.label(widget).is_some(), "{root} authors {widget}");
        }
        let cx = oag_hud::Context {
            layout: &layout,
            strings: &strings,
            sheet: &oag_hud::sprite::Sheet::build(&[], &mut Vec::new()),
            art: oag_hd::hud::ART,
            hud_line_height: 92.0,
            small_line_height: 34.0,
            default_line_height: 34.0,
            default_border: layout.default_border(),
        };
        let mut board = oag_hud::messages::MessageBoard::default();
        board.push("ER_GMA", true);
        for _ in 0..30 {
            board.advance();
        }
        let readout = oag_hud::Readout {
            messages: board.lines(),
            ..Default::default()
        };
        let frame = oag_hud::draw_list(&cx, &readout);
        let phrase = strings.get("ER_GMA").expect("checked above");
        let drawn = frame
            .hud_text
            .iter()
            .chain(&frame.small_text)
            .chain(&frame.default_text)
            .any(|draw| matches!(draw, Draw::Text { text, .. } if text == phrase));
        assert!(drawn, "{root} draws {phrase:?}: {frame:?}");
    }
}

/// A real crossing, through the same `oag_game::medal_watch::tick` the windowed
/// session and the headless capture both call: Pulse's `grid8_3_2` Speed Lap
/// (`14_Track`, silver at 44 s), flown by the autopilot. The banner appears on
/// the lap that earns the tier, expires, and the standing line is still drawn
/// ten seconds on; a watch that stops calling either raises nothing.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_real_speed_lap_crossing_raises_the_banner_and_leaves_a_standing_line() {
    let Some(image) = oag_testdata::image("pulse-psp-usa.chd") else {
        return;
    };
    let mut opened =
        oag_source::title::open_source(&image.display().to_string(), Vec::new(), Vec::new())
            .expect("opening the source");
    let cell = oag_game::campaign::read_grids(
        &mut opened.archives,
        oag_pulse::campaign::DEFINITION_ENTRY,
        None,
    )
    .expect("the grids")
    .into_iter()
    .flat_map(|grid| grid.cells)
    .find(|cell| cell.name == "grid8_3_2")
    .expect("grid8_3_2");
    let loaded = oag_raceplay::load(&oag_raceplay::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SpeedLap,
        track: Some(r"Data\Environments\14_Track\track.vex".to_string()),
        seed: Some(1),
        ..oag_raceplay::Options::default()
    })
    .expect("loading 14_Track");
    let mut race = oag_raceplay::Race::start(loaded.setup.clone());
    race.set_autopilot(true);
    let silver = loaded
        .hud
        .context()
        .expect("Pulse's layout parses")
        .strings
        .get_or_id("ER_SMA");
    let mut earned = None;
    let mut raised_at = None;
    for tick in 0..4200u32 {
        race.tick(&PlayerInputs::none());
        let was = earned;
        oag_game::medal_watch::tick(
            &cell,
            oag_tables::race_campaign::Difficulty::Medium,
            &mut earned,
            &mut race,
        );
        if earned != was && raised_at.is_none() {
            raised_at = Some(tick);
        }
    }
    let raised_at = raised_at.expect("the autopilot's lap earns a medal");
    assert_eq!(
        earned,
        Some(oag_tables::race_campaign::Medal::Silver),
        "raised at tick {raised_at}"
    );
    assert!(
        4200 - raised_at > 600,
        "the banner (4 s) is long gone by the last tick"
    );
    let saying = drawn(&loaded, &race.readout());
    assert_eq!(
        saying.iter().filter(|text| **text == silver).count(),
        1,
        "only the standing line is up: {saying:?}"
    );
}
