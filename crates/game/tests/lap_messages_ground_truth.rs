//! The lap-crossing message lines and the lap-line time gap, on Pulse's own
//! HUD layout and string table.
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
//! That the autopilot's laps on `16_Track` raise `IG_HUD_FIN_LAP` at the start
//! of the final lap and nothing on the finishing crossing, that
//! `IG_HUD_NEWLAP_REC` needs a stored record to beat, that the time gap reaches
//! the drawn `TimeDiffText` with the original's `"+ "`/`"- "` format, and that
//! the ids resolve in Pulse's table. The laws are
//! `docs/ghidra/functions/psp-pulse-usa/hud-message-triggers.md`.

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
        count("IG_HUD_PLAP") <= laps - 1,
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
            assert_eq!(text, line.text);
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

/// Pure authors the lines under other keys, so the three lap lines stay off
/// there (no `MESSAGE` cue for words nothing draws) while the gap, which needs
/// no string, still runs.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn pure_lacks_the_lap_line_ids_so_only_the_gap_runs() {
    let Some(image) = oag_testdata::image("pure-psp-usa.chd") else {
        return;
    };
    let loaded = oag_raceplay::load(&oag_raceplay::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        seed: Some(1),
        ..oag_raceplay::Options::default()
    })
    .expect("loading Pure");
    let context = loaded.hud.context().expect("Pure's layout parses");
    for id in oag_raceplay::lap_messages::IDS {
        assert!(
            context.strings.get(id).is_none(),
            "{id} is in Pure's table now"
        );
    }
    let mut race = oag_raceplay::Race::start(loaded.setup.clone());
    race.set_autopilot(true);
    race.watch_laps(&loaded.hud, None);
    let mut gap = false;
    for _ in 0..7000 {
        race.tick(&PlayerInputs::none());
        let readout = race.readout();
        assert!(
            readout.messages.iter().all(Option::is_none),
            "no line raised"
        );
        gap |= readout.time_gap.is_some();
    }
    assert!(gap, "the gap runs on Pure");
}
