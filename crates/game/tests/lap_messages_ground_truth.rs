//! The lap-crossing message lines and the lap-line time gap, on each title's
//! own HUD layout, string table and rule.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content; run it with
//! `just test-data`, or only this file:
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
//!     -E 'binary(lap_messages_ground_truth)'
//! ```
//!
//! # What only real data can say here
//!
//! That the autopilot's laps raise the final-lap line at the start of the last
//! lap and nothing on the finishing crossing, that the new-lap-record line needs
//! a stored record to beat, that the time gap reaches the drawn `TimeDiffText`
//! in each title's own wording, and that each title's ids resolve in its own
//! table. Pulse: `16_Track`, `IG_HUD_*`, `"+ 2.8"`. Pure: `01_Vineta_K`,
//! `HUD_<English>` keys, a sentence. HD: Talon's Junction, `IG_HUD_*`, no
//! record line in Time Trial. The laws are each binary's
//! `docs/ghidra/functions/<binary>/hud-message-triggers.md`.

use oag_gameplay::PlayerInputs;
use oag_ui::frontend::Draw;

fn load_track(image: &std::path::Path, mode: oag_race::Mode, track: &str) -> oag_raceplay::Loaded {
    oag_raceplay::load(&oag_raceplay::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode,
        track: Some(track.to_string()),
        seed: Some(1),
        ..oag_raceplay::Options::default()
    })
    .expect("loading the track")
}

fn load(image: &std::path::Path, mode: oag_race::Mode) -> oag_raceplay::Loaded {
    load_track(image, mode, oag_pulse::race::DEFAULT_TRACK)
}

fn drawn(loaded: &oag_raceplay::Loaded, readout: &oag_hud::Readout) -> Vec<String> {
    let context = loaded.hud.context().expect("Pulse's layout parses");
    let frame = oag_hud::draw_list(&context, readout);
    frame
        .hud_text
        .iter()
        .chain(&frame.small_text)
        .chain(&frame.default_text)
        .filter_map(|draw| match draw {
            Draw::Text { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect()
}

/// Every message id raised over a time trial the autopilot drives to the end,
/// in the order they came up, against `stored_best` ticks. A line is new when
/// its slot held something else (or nothing) on the tick before.
fn time_trial_ids(loaded: &oag_raceplay::Loaded, stored_best: Option<u32>) -> (Vec<String>, usize) {
    let mut race = oag_raceplay::Race::start(loaded.setup.clone());
    race.set_autopilot(true);
    race.watch_laps(&loaded.hud, stored_best);
    let mut ids: Vec<String> = Vec::new();
    let mut before: [Option<String>; 4] = Default::default();
    let mut after_finish = 0;
    for _ in 0..30_000 {
        race.tick(&PlayerInputs::none());
        for (slot, line) in race.readout().messages.iter().enumerate() {
            let now = line.as_ref().map(|line| line.text.clone());
            if now.is_some() && now != before[slot] {
                ids.extend(now.clone());
            }
            before[slot] = now;
        }
        if race.sim.world.primary_race().finished {
            after_finish += 1;
            if after_finish > 600 {
                break;
            }
        }
    }
    assert!(
        race.sim.world.primary_race().finished,
        "the autopilot finishes the time trial"
    );
    let laps = race
        .sim
        .world
        .primary_race()
        .laps_target
        .expect("a time trial has a lap count") as usize;
    (ids, laps)
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_final_lap_is_announced_once_and_the_finish_says_nothing() {
    let Some(image) = oag_testdata::image("pulse-psp-usa.chd") else {
        return;
    };
    let loaded = load(&image, oag_race::Mode::TimeTrial);
    let context = loaded.hud.context().expect("Pulse's layout parses");
    for id in ["IG_HUD_PLAP", "IG_HUD_NEWLAP_REC", "IG_HUD_FIN_LAP"] {
        assert_ne!(context.strings.get_or_id(id), id, "{id} resolves");
    }
    let (ids, laps) = time_trial_ids(&loaded, None);
    let count = |id: &str| ids.iter().filter(|seen| *seen == id).count();
    assert_eq!(count("IG_HUD_FIN_LAP"), 1, "{ids:?}");
    assert!(
        count("IG_HUD_PLAP") < laps,
        "the finishing lap raises no perfect lap: {ids:?}"
    );
    assert_eq!(
        count("IG_HUD_NEWLAP_REC"),
        0,
        "no stored record, no new one"
    );
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_lap_beating_the_stored_record_is_announced() {
    let Some(image) = oag_testdata::image("pulse-psp-usa.chd") else {
        return;
    };
    let loaded = load(&image, oag_race::Mode::TimeTrial);
    let (slow, _) = time_trial_ids(&loaded, Some(60 * 600));
    assert!(slow.iter().any(|id| id == "IG_HUD_NEWLAP_REC"), "{slow:?}");
    let (fast, _) = time_trial_ids(&loaded, Some(60));
    assert!(!fast.iter().any(|id| id == "IG_HUD_NEWLAP_REC"), "{fast:?}");
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_single_race_shows_the_lap_line_gap_in_the_originals_format() {
    let Some(image) = oag_testdata::image("pulse-psp-usa.chd") else {
        return;
    };
    let loaded = load(&image, oag_race::Mode::SingleRace);
    let mut race = oag_raceplay::Race::start(loaded.setup.clone());
    race.set_autopilot(true);
    race.watch_laps(&loaded.hud, None);
    let mut gaps = Vec::new();
    for _ in 0..7000 {
        race.tick(&PlayerInputs::none());
        let readout = race.readout();
        if let Some(line) = &readout.time_gap
            && line.icon
        {
            let text = drawn(&loaded, &readout)
                .into_iter()
                .find(|text| text.starts_with("+ ") || text.starts_with("- "))
                .expect("the held gap reaches TimeDiffText");
            let table = &loaded.hud.context().expect("Pulse's layout parses").strings;
            assert_eq!(Some(text.clone()), line.text(table));
            gaps.push(text);
        }
    }
    assert!(
        !gaps.is_empty(),
        "a lap line was crossed with the field about"
    );
}

#[test]
fn both_game_loops_still_switch_the_lap_lines_on() {
    for source in [
        include_str!("../src/main/stage.rs"),
        include_str!("../src/race_capture.rs"),
    ] {
        assert!(
            source.contains("race.watch_laps("),
            "a race built here must call Race::watch_laps"
        );
    }
}

/// The ids the title's own routine raises, in its own table.
fn ids_of(loaded: &oag_raceplay::Loaded) -> oag_title::lap_hud::LapHud {
    loaded.hud.art.lap_hud.expect("the title's routine is read")
}

/// Pure keys its phrases `HUD_<English>`, draws them white, and words the gap as
/// a sentence from its own table (`FUN_0881f2ec`, `FUN_0881f780`).
#[test]
#[ignore = "needs a disc image in data/images/"]
fn pure_raises_its_own_keys_white_and_words_the_gap_as_a_sentence() {
    let Some(image) = oag_testdata::image("pure-psp-usa.chd") else {
        return;
    };
    let loaded = load_track(
        &image,
        oag_race::Mode::TimeTrial,
        oag_pure::race::DEFAULT_TRACK,
    );
    let hud = ids_of(&loaded);
    let context = loaded.hud.context().expect("Pure's layout parses");
    for id in [hud.perfect_lap, hud.new_lap_record, hud.final_lap] {
        assert_ne!(context.strings.get_or_id(id), id, "{id} resolves");
    }
    let (ids, laps) = time_trial_ids(&loaded, Some(60 * 600));
    let count = |id: &str| ids.iter().filter(|seen| *seen == id).count();
    assert_eq!(count(hud.final_lap), 1, "{ids:?}");
    assert!(count(hud.perfect_lap) < laps, "{ids:?}");
    assert!(ids.iter().any(|id| id == hud.new_lap_record), "{ids:?}");

    let loaded = load_track(
        &image,
        oag_race::Mode::SingleRace,
        oag_pure::race::DEFAULT_TRACK,
    );
    let mut race = oag_raceplay::Race::start(loaded.setup.clone());
    race.set_autopilot(true);
    race.watch_laps(&loaded.hud, None);
    let mut sentences = Vec::new();
    let mut lines_seen = 0;
    for _ in 0..7000 {
        race.tick(&PlayerInputs::none());
        let readout = race.readout();
        for line in readout.messages.iter().flatten() {
            lines_seen += 1;
            assert_eq!(
                line.color[..3],
                [1.0, 1.0, 1.0],
                "Pure draws every line white"
            );
            assert!(
                !line.fades_border,
                "Pure's outline does not follow the text"
            );
        }
        if let Some(line) = &readout.time_gap
            && line.icon
        {
            let text = drawn(&loaded, &readout)
                .into_iter()
                .find(|text| text.starts_with("You are "))
                .expect("the held gap reaches TimeDiffText as a sentence");
            sentences.push(text);
        }
    }
    assert!(
        sentences
            .iter()
            .all(|text| text.ends_with("secs ahead") || text.ends_with("secs behind")),
        "{sentences:?}"
    );
    assert!(
        !sentences.is_empty(),
        "a lap line was crossed with the field about"
    );
    assert!(lines_seen > 0, "a lap line was raised in the single race");
}

/// HD raises the same ids as Pulse and no new-lap-record line in Time Trial
/// (`0x91d20`, mode 5 behind the clear profile byte), counts up alone in Time
/// Trial (`0x907b0`), and gaps three times a lap (`0xd06e0`).
#[test]
#[ignore = "needs a disc image in data/images/"]
fn hd_skips_the_record_line_in_time_trial_and_counts_up_alone() {
    let Some(image) = oag_testdata::image("hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let loaded = load_track(
        &image,
        oag_race::Mode::TimeTrial,
        oag_hd::race::DEFAULT_TRACK,
    );
    let hud = ids_of(&loaded);
    let context = loaded.hud.context().expect("HD's layout parses");
    for id in [hud.perfect_lap, hud.new_lap_record, hud.final_lap] {
        assert_ne!(context.strings.get_or_id(id), id, "{id} resolves");
    }
    let (ids, _) = time_trial_ids(&loaded, Some(60 * 600));
    assert!(
        !ids.iter().any(|id| id == hud.new_lap_record),
        "no record line in Time Trial: {ids:?}"
    );
    assert_eq!(
        ids.iter().filter(|id| *id == hud.final_lap).count(),
        1,
        "{ids:?}"
    );

    let mut race = oag_raceplay::Race::start(loaded.setup.clone());
    race.set_autopilot(true);
    race.watch_laps(&loaded.hud, None);
    let mut counted = false;
    for _ in 0..9000 {
        race.tick(&PlayerInputs::none());
        let readout = race.readout();
        if let Some(line) = &readout.time_gap {
            counted |= !line.icon && drawn(&loaded, &readout).iter().any(|t| t.contains('.'));
        }
    }
    assert!(counted, "the lone craft's count-up reaches TimeDiffText");
}

/// 2048 authors `Info1`-`Info4` and `TimeDiffText` in its own layouts and drains
/// the same flag word as HD (`FUN_81150730`), so its lines run on its own table:
/// the final-lap line once, no record line in Time Trial, the count-up alone.
#[test]
#[ignore = "needs data/extracted/vita/PCSF00007"]
fn the_2048_lap_lines_run_on_its_own_layout_and_table() {
    let Some(source) = oag_testdata::exact("data/extracted/vita/PCSF00007") else {
        return;
    };
    let loaded = oag_raceplay::load(&oag_raceplay::Options {
        source: source.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::TimeTrial,
        track: Some(oag_2048::race::DEFAULT_TRACK.to_string()),
        seed: Some(1),
        ..oag_raceplay::Options::default()
    })
    .expect("loading 2048");
    let hud = ids_of(&loaded);
    let context = loaded.hud.context().expect("2048's layout parses");
    for id in [hud.perfect_lap, hud.new_lap_record, hud.final_lap] {
        assert_ne!(context.strings.get_or_id(id), id, "{id} resolves");
    }
    let (ids, _) = time_trial_ids(&loaded, Some(60 * 600));
    assert!(
        !ids.iter().any(|id| id == hud.new_lap_record),
        "no record line in Time Trial: {ids:?}"
    );
    assert_eq!(
        ids.iter().filter(|id| *id == hud.final_lap).count(),
        1,
        "{ids:?}"
    );
}
