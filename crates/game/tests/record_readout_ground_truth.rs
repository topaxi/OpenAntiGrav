//! Pulse's in-race HUD text that comes from the disc's own data: the Time Trial
//! clock cluster counting down to a `RECORD`, and the Eliminator's kill column.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. Run it with `just test-data`, or only this file:
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
//!     -E 'binary(record_readout_ground_truth)'
//! ```
//!
//! # What only real data can say here
//!
//! A plain Venom Time Trial on Talon's Junction reads `record` and `1.33.2`
//! on a live PPSSPP frame at 23.7 s (`PLAYER_HUD+0x30` = 8265, tier 3, read
//! off `DAT_08b310b4`'s `<RaceTimes>` 117.0 s). The figure comes out of
//! `16_Track`'s own `stats.xml`, and it is the file's `<code>` dictionary that
//! puts `Venom` on the 117, not the attribute order: a reader that took the
//! order the way the page reads would show 138 s.

use oag_game::records::Record;
use oag_gameplay::PlayerInputs;
use oag_hud::{RecordTarget, pace_for};
use oag_ui::frontend::Draw;

fn image() -> Option<std::path::PathBuf> {
    oag_testdata::image("pulse-psp-usa.chd")
}

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

/// The texts the frame draws, small font and HUD font.
fn texts(frame: &oag_hud::Frame) -> (Vec<String>, Vec<String>) {
    let of = |list: &[Draw]| {
        list.iter()
            .filter_map(|draw| match draw {
                Draw::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    (of(&frame.small_text), of(&frame.hud_text))
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_load_carries_venoms_authored_time_off_the_disc() {
    let Some(image) = image() else { return };
    let loaded = load(&image, oag_race::Mode::TimeTrial);
    let stats = loaded.track_stats.expect("16_Track's stats.xml reads");
    assert_eq!(stats.race_times[0], 117.0, "Venom's <RaceTimes>");
    assert_eq!(stats.lap_times[0], 38.0, "Venom's <LapTimes>");
    let target = RecordTarget::new(oag_race::Mode::TimeTrial, "VENOM", Some(&stats), None, None)
        .expect("a Time Trial has a record target");
    assert_eq!(target.authored_centis, Some(11_700));
}

/// The whole cluster, drawn from a real race's readout: the caption is the
/// disc's own `IG_HUD_RECORD` string, not `TOTAL`, and the number counts down
/// from the authored time, then from a stored best when that is faster.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_time_trial_draws_the_record_caption_and_counts_down_to_it() {
    let Some(image) = image() else { return };
    let loaded = load(&image, oag_race::Mode::TimeTrial);
    let context = loaded.hud.context().expect("Pulse's layout parses");
    let record_word = context.strings.get_or_id("IG_HUD_RECORD").to_string();
    assert_ne!(
        record_word, "IG_HUD_RECORD",
        "the disc's string table resolves it"
    );
    let total_word = context.strings.get_or_id("IG_HUD_TOTAL").to_string();

    let stats = loaded.track_stats.expect("stats.xml reads");
    let mut race = oag_raceplay::Race::start(loaded.setup);
    race.tick(&PlayerInputs::none());
    let mut readout = race.readout();
    assert_eq!(readout.mode, oag_race::Mode::TimeTrial);
    // Ticks in the countdown are not race time: the clock has not started.
    assert_eq!(readout.race_ticks, 0);

    let fresh = RecordTarget::new(oag_race::Mode::TimeTrial, "VENOM", Some(&stats), None, None);
    readout.time_trial_pace = pace_for(readout.mode, 0, 0, None, fresh.as_ref());
    let (small, hud) = texts(&oag_hud::draw_list(&context, &readout));
    assert!(small.contains(&record_word), "small text {small:?}");
    assert!(!small.contains(&total_word), "small text {small:?}");
    assert!(hud.contains(&"1.57.0".to_string()), "hud text {hud:?}");

    // A stored best of 100.0 s replaces the authored 117.0 s.
    let best = Record {
        best_total_ticks: Some(6_000),
        ..Record::default()
    };
    let faster = RecordTarget::new(
        oag_race::Mode::TimeTrial,
        "VENOM",
        Some(&stats),
        best.best_total_ticks,
        best.best_lap_ticks,
    );
    readout.time_trial_pace = pace_for(readout.mode, 0, 0, None, faster.as_ref());
    let (_, hud) = texts(&oag_hud::draw_list(&context, &readout));
    assert!(hud.contains(&"1.40.0".to_string()), "hud text {hud:?}");

    // Without the target the cluster falls back to the plain clock.
    readout.time_trial_pace = pace_for(readout.mode, 0, 0, None, None);
    let (small, _) = texts(&oag_hud::draw_list(&context, &readout));
    assert!(small.contains(&total_word), "small text {small:?}");
}

/// The Eliminator's kill column, from a real race's readout: eight rows in
/// the default face, the disc's own team names, `KILLS (5)` over them.
///
/// The live frame this is checked against (PPSSPP, Venom Eliminator on
/// Talon's Junction) reads `KILLS (5)` over `AG Systems 1`, `Feisar 1`,
/// `Qirex 1`, `AAA 0`, `Triakis 0`, `Goteki 45 0`, `Piranha 0`, `EG-X 0`.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn an_eliminator_draws_the_kill_column_in_the_default_face() {
    let Some(image) = image() else { return };
    let loaded = load(&image, oag_race::Mode::Eliminator);
    let context = loaded.hud.context().expect("Pulse's layout parses");
    let mut race = oag_raceplay::Race::start(loaded.setup);
    race.tick(&PlayerInputs::none());
    let mut readout = race.readout();
    assert_eq!(readout.kill_target, 5, "the race box's first kill row");
    assert_eq!(readout.kill_tags.len(), 8, "one row per craft on the grid");
    assert_eq!(
        readout.kill_tags.iter().filter(|tag| tag.player).count(),
        1,
        "exactly one row is the player's"
    );

    // Two kills for the craft in slot 3 lifts it to the top row.
    race.sim.world.ships[3].standing.kills = 2;
    readout = race.readout();
    assert_eq!(readout.kill_tags[0].kills, 2);

    let frame = oag_hud::draw_list(&context, &readout);
    assert!(
        frame.default_text.len() >= 8,
        "eight rows in the default face, drew {:?}",
        frame.default_text
    );
    let (small, _) = texts(&frame);
    let caption = context.strings.get_or_id("IG_HUD_KILLS").to_string();
    assert!(
        small.contains(&format!("{caption} (5)")),
        "the header carries the target: {small:?}"
    );
    // Every opponent row names its team from the disc's string table, not the
    // folder id: `AG_Systems` reads `AG Systems`.
    let rows: Vec<String> = frame
        .default_text
        .iter()
        .filter_map(|draw| match draw {
            Draw::Text { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect();
    assert!(
        rows.iter().any(|row| row.starts_with("AG Systems ")),
        "rows {rows:?}"
    );
    assert!(rows.iter().all(|row| !row.contains('_')), "rows {rows:?}");
}

/// A single race draws `pos` and no `TOTAL`, as the live frame does: the clock
/// cluster is hidden by the mode, and `POS` is not hidden by anything. Measured
/// on PPSSPP 2026-09-30 (`TotalTimeTxt` flag word `0xb082`, `PositionTxt` up).
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_single_race_draws_pos_and_no_total() {
    let Some(image) = image() else { return };
    let loaded = load(&image, oag_race::Mode::SingleRace);
    let context = loaded.hud.context().expect("Pulse's layout parses");
    let pos = context.strings.get_or_id("IG_HUD_POS").to_string();
    let total = context.strings.get_or_id("IG_HUD_TOTAL").to_string();
    let mut race = oag_raceplay::Race::start(loaded.setup);
    race.tick(&PlayerInputs::none());
    let readout = race.readout();
    assert_eq!(readout.place, 8, "the parked player is 8th of 8");
    let (small, _) = texts(&oag_hud::draw_list(&context, &readout));
    assert!(small.contains(&pos), "small text {small:?}");
    assert!(!small.contains(&total), "small text {small:?}");
}
