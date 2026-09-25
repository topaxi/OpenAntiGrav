//! Races a full single race on a real Wipeout HD/Fury circuit until the flag
//! falls, then reads that title's own `EndRace Results`/`EndRace Menu` off
//! the same disc and draws them from the finished race's own board - the
//! end-to-end path `Session::build_endrace`/`EndRaceRuntime` drive live,
//! minus the GPU device neither needs to reach here.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
//!     hd_endrace_ground_truth
//! ```
//!
//! # Why this stops short of `EndRaceRuntime`
//!
//! `crate::main::race_stage::endrace::EndRaceRuntime` and
//! `crate::main::session::endrace::Session::build_endrace` are the binary
//! crate's own (`crates/game/src/main/`), not `[lib]` `oag_game`'s - a
//! `crates/game/tests/*.rs` integration test only ever sees the library, the
//! same reason every other ground-truth file in this directory reaches for
//! `oag_game::race`/`oag_game::endrace` rather than `crate::main::*`. What
//! those two hold beyond what this test exercises is thin, GPU-touching
//! glue: opening a renderer and copying `Board` rows into `FieldRow`, the
//! second of which has its own fast, disc-free unit tests
//! (`crate::main::race_stage::endrace::tests::hd_field_rows_*` in the binary
//! itself). This test is the disc-backed half: a real race, a real board, a
//! real `EndRace_Definition.xml`, and a real draw list built from both.

use std::path::PathBuf;

use oag_game::race;
use oag_gameplay::PlayerInputs;

/// The same generous "did it ever finish" bound
/// `race_finish_ground_truth.rs` uses, for the same reason.
const CAP: u64 = 30_000;

fn image() -> Option<PathBuf> {
    oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso")
}

/// A single race on HD's own default circuit and team - both resolved from
/// [`oag_title::RaceDefaults`] the same way a bare `--race hdfury...iso
/// --mode single_race` does, which is what a live cold walk into `RACE
/// CAMPAIGN` confirmed reaches a real, playing race (`docs/ui/campaign-screens.md`'s
/// 2026-09-21 live entry).
fn autopiloted_hd_race() -> Option<race::Race> {
    let image = image()?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        mode: oag_race::Mode::SingleRace,
        ..race::Options::default()
    })
    .expect("loading the race");
    let mut race = race::Race::start(loaded.setup);
    race.set_autopilot(true);
    Some(race)
}

fn race_to_the_flag(race: &mut race::Race) {
    while !race.finished() && race.sim.world.tick < CAP {
        race.tick(&PlayerInputs::none());
    }
    assert!(
        race.finished(),
        "the race was still running after {CAP} ticks; the player reached lap {} of {:?}",
        race.sim.world.primary_race().lap,
        race.sim.world.laps_target()
    );
}

/// Opens the disc fresh the way `Session::build_endrace` does - a second,
/// independent open from the one `race::load` already made, since that is
/// what the live flow actually does (the archives a race scene loaded
/// meshes from are not the ones `oag_game::endrace::load` reads screens
/// through).
fn open_hd_archives(source: &str) -> oag_assets::Archives {
    oag_game::title::open_source(source, Vec::new(), Vec::new())
        .expect("HD's own disc opens")
        .archives
}

/// The full path: a real HD race, driven to its own finish, reaches
/// `EndRace Results` with the field populated - the whole field, not only
/// the player, at real positions with real times, off the real disc's own
/// `EndRace_Definition.xml`.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn a_finished_hd_race_reaches_endrace_results_with_the_field_populated() {
    let Some(mut race) = autopiloted_hd_race() else {
        return;
    };
    race_to_the_flag(&mut race);

    let board = race.results().expect("a finished race has results").clone();
    assert!(!board.rows.is_empty(), "the board should carry the field");
    assert!(
        board.rows.iter().any(|row| row.player),
        "the player's own row should be on the board"
    );

    // `race_stage::endrace::hd_field_rows`'s own mapping, inlined - that
    // function lives in the binary crate and is unit-tested there; this is
    // the same three fields, fed real data, to confirm the shape survives
    // contact with a real board rather than only a hand-built one.
    let rows: Vec<oag_ui::endrace::FieldRow> = board
        .rows
        .iter()
        .map(|row| oag_ui::endrace::FieldRow {
            place: row.place,
            time_ticks: row.finish_tick,
            player: row.player,
        })
        .collect();
    let model = oag_ui::endrace::FieldResults {
        headline: oag_ui::endrace::Headline::Position(
            rows.iter()
                .find(|row| row.player)
                .map(|row| row.place)
                .unwrap_or(1),
        ),
        rows,
    };

    let source = image()
        .expect("checked by autopiloted_hd_race")
        .display()
        .to_string();
    let mut archives = open_hd_archives(&source);
    let strings = oag_ui::language::StringTable::default();
    let base = oag_game::sprite::Sheet::default();
    let screens = oag_game::endrace::load(
        &mut archives,
        &strings,
        oag_ui::picker::FaceScales::default(),
        oag_hd::endrace::AUTHORED_GRID,
        &base,
        &[],
        oag_hd::TITLE,
    )
    .expect("HD's own EndRace Results/Menu read off the real disc");
    // `DATA02`'s copy (the served one) authors `EndRace Rewards`, so the
    // layout reads - drawn only by `--menu-page`, never by the live flow,
    // since the original never enters it (see `EndRaceScreens::rewards`).
    assert!(
        screens.rewards.is_some(),
        "DATA02's own EndRace_Definition.xml authors EndRace Rewards"
    );

    let skin = oag_ui::menu::Skin::new(
        oag_hd::frontend::FRONT_END
            .menu
            .expect("HD authors a MenuSkin"),
        oag_display::space::Space::PSP,
        22.0,
    );
    let layers = oag_ui::endrace::hd::hd_results_draw_list(
        &model,
        &screens.results,
        &skin,
        &oag_ui::menu::Frame::default(),
        &strings,
        None,
        false,
        &|_| None,
    );
    let texts: Vec<String> = layers
        .body
        .iter()
        .filter_map(|draw| match draw {
            oag_ui::frontend::Draw::Text { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect();

    let player_place = board
        .rows
        .iter()
        .find(|row| row.player)
        .expect("the player's own row")
        .place;
    assert!(
        texts.contains(&player_place.to_string()),
        "the player's own place ({player_place}) should be drawn: {texts:?}"
    );
    // At least one other craft's own row drew too, if the field is more
    // than one - the whole-field grid, not only the player's line.
    if board.rows.len() > 1 {
        assert!(
            texts.len() > 3,
            "more than just the title/headline/POS/TIME captions should have drawn \
             for an {}-craft field: {texts:?}",
            board.rows.len()
        );
    }
}

/// HD's own English table, off the same disc - so the test sees the
/// strings a live session would, not bare idstrings.
fn english(archives: &mut oag_assets::Archives) -> oag_ui::language::StringTable {
    let blob = archives
        .read_name(r"Data\Plugins\Languages\English\entries.xml")
        .expect("HD's own English entries.xml reads");
    oag_ui::language::StringTable::from_xml(
        &oag_tables::fexml::text(&blob).expect("entries.xml is text"),
    )
}

/// `EndRace Rewards` off the real disc's own definition and string table:
/// `BigPos` carries the place it is fed (3, not its authored `"1"`), the
/// medal line resolves through HD's own table, and none of the loyalty
/// row's placeholders (`"test"`/`"points!"`/`"line 2"`) or its tile draw -
/// HD's loyalty law is not recovered. No race needed: the screen's own
/// widgets are what this pins.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn hd_endrace_rewards_draws_the_place_and_medal_off_the_real_definition() {
    let Some(image) = image() else {
        return;
    };
    let mut archives = open_hd_archives(&image.display().to_string());
    let strings = english(&mut archives);
    let screens = oag_game::endrace::load(
        &mut archives,
        &strings,
        oag_ui::picker::FaceScales::default(),
        oag_hd::endrace::AUTHORED_GRID,
        &oag_game::sprite::Sheet::default(),
        &[],
        oag_hd::TITLE,
    )
    .expect("HD's own EndRace screens read off the real disc");
    let rewards = screens
        .rewards
        .as_ref()
        .expect("DATA02 authors EndRace Rewards");

    let skin = oag_ui::menu::Skin::new(
        oag_hd::frontend::FRONT_END
            .menu
            .expect("HD authors a MenuSkin"),
        oag_display::space::Space::PSP,
        22.0,
    );
    let draw = |model: &oag_ui::endrace::HdRewards| {
        oag_ui::endrace::hd::hd_rewards_draw_list(
            model,
            rewards,
            &skin,
            &oag_ui::menu::Frame::default(),
            &strings,
            None,
            false,
            &|_| None,
        )
    };
    let texts = |layers: &oag_ui::menu::Layers| -> Vec<String> {
        layers
            .body
            .iter()
            .filter_map(|draw| match draw {
                oag_ui::frontend::Draw::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect()
    };
    let fill_at = |layers: &oag_ui::menu::Layers, x: f32, y: f32| {
        layers.body.iter().any(|draw| {
            matches!(draw, oag_ui::frontend::Draw::Fill { rect, .. }
                if (rect[0] - x).abs() < 0.5 && (rect[1] - y).abs() < 0.5)
        })
    };

    let layers = draw(&oag_ui::endrace::HdRewards {
        place: Some(3),
        medal: Some(oag_tables::race_campaign::Medal::Bronze),
        campaign: true,
    });
    let drawn = texts(&layers);
    assert!(drawn.contains(&"REWARDS".to_string()), "{drawn:?}");
    assert!(drawn.contains(&"3".to_string()), "{drawn:?}");
    assert!(!drawn.contains(&"1".to_string()), "{drawn:?}");
    assert!(
        drawn.contains(&"BRONZE MEDAL AWARDED".to_string()),
        "{drawn:?}"
    );
    for leak in ["test", "points!", "line 2", "TOTAL LOYALTY:"] {
        assert!(
            !drawn.contains(&leak.to_string()),
            "{leak:?} leaked: {drawn:?}"
        );
    }
    assert!(!fill_at(&layers, 600.0, 360.0), "MedalImg never draws");
    assert!(!fill_at(&layers, 600.0, 530.0), "LoyaltyImg never draws");

    // Not a campaign race and no place: only the authored labels.
    let bare = texts(&draw(&oag_ui::endrace::HdRewards {
        place: None,
        medal: None,
        campaign: false,
    }));
    assert!(
        !bare
            .iter()
            .any(|text| text.contains("MEDAL") || text == "1"),
        "{bare:?}"
    );
}
