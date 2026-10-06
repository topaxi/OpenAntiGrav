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
//! `oag_raceplay`/`oag_game::endrace` rather than `crate::main::*`. What
//! those two hold beyond what this test exercises is thin, GPU-touching
//! glue: opening a renderer and copying `Board` rows into `FieldRow`, the
//! second of which has its own fast, disc-free unit tests
//! (`crate::main::race_stage::endrace::tests::hd_field_rows_*` in the binary
//! itself). This test is the disc-backed half: a real race, a real board, a
//! real `EndRace_Definition.xml`, and a real draw list built from both.

use std::path::PathBuf;

use oag_gameplay::PlayerInputs;
use oag_raceplay as race;

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
    oag_source::title::open_source(source, Vec::new(), Vec::new())
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
    let rows: Vec<oag_ui_screens::endrace::FieldRow> = board
        .rows
        .iter()
        .map(|row| oag_ui_screens::endrace::FieldRow {
            place: row.place,
            time_ticks: row.finish_tick,
            player: row.player,
        })
        .collect();
    let model = oag_ui_screens::endrace::FieldResults {
        loyalty: None,
        headline: oag_ui_screens::endrace::Headline::Position(
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
    let base = oag_hud::sprite::Sheet::default();
    let screens = oag_game::endrace::load(
        &mut archives,
        &strings,
        oag_ui_screens::picker::FaceScales::default(),
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
    let layers = oag_ui_screens::endrace::hd::hd_results_draw_list(
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

/// Regression for `oag_tables::fexml`'s malformed-tag recovery
/// (`docs/formats/fexml.md`, `docs/formats/hd-endrace-screens.md`).
/// `DATA02`'s own `EndRace_Definition.xml` authors
/// `<Values Src="..." ... RotY="-0.5"</Values>` three times inside `EndRace
/// Results`' loyalty-block trophy widgets - a start tag missing its own `>`.
/// Before the fix, `fexml::parse`'s `tag_end` ran on into that literal
/// `</Values>` text and swallowed everything the screen authors afterwards
/// (`NavigationController` included) as a descendant of the wrongly-open
/// `Values` node, so `EndRace Results` never collected `ControlTextConfirm`
/// and drew no Confirm prompt at all - unlike `EndRace Menu`/`EndRace
/// Rewards`, which do not carry the malformed shape. This proves the real
/// disc file now reads through to its own `NavigationController` and draws
/// `CONFIRM`.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn hd_endrace_results_draws_its_confirm_prompt_despite_the_malformed_tag() {
    let Some(image) = image() else {
        return;
    };
    let mut archives = open_hd_archives(&image.display().to_string());
    let strings = english(&mut archives);
    let screens = oag_game::endrace::load(
        &mut archives,
        &strings,
        oag_ui_screens::picker::FaceScales::default(),
        oag_hd::endrace::AUTHORED_GRID,
        &oag_hud::sprite::Sheet::default(),
        &[],
        oag_hd::TITLE,
    )
    .expect("HD's own EndRace screens read off the real disc");

    let skin = oag_ui::menu::Skin::new(
        oag_hd::frontend::FRONT_END
            .menu
            .expect("HD authors a MenuSkin"),
        oag_display::space::Space::PSP,
        22.0,
    );
    let model = oag_ui_screens::endrace::FieldResults {
        loyalty: None,
        headline: oag_ui_screens::endrace::Headline::Position(1),
        rows: vec![oag_ui_screens::endrace::FieldRow {
            place: 1,
            time_ticks: Some(1000),
            player: true,
        }],
    };
    let layers = oag_ui_screens::endrace::hd::hd_results_draw_list(
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
    assert!(
        texts.iter().any(|t| t.eq_ignore_ascii_case("Confirm")),
        "EndRace Results should draw its own Confirm prompt once the \
         malformed-tag recovery reaches NavigationController: {texts:?}"
    );
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
/// HD shows its loyalty on `Results` instead. No race needed: the screen's own
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
        oag_ui_screens::picker::FaceScales::default(),
        oag_hd::endrace::AUTHORED_GRID,
        &oag_hud::sprite::Sheet::default(),
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
    let draw = |model: &oag_ui_screens::endrace::HdRewards| {
        oag_ui_screens::endrace::hd::hd_rewards_draw_list(
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

    let layers = draw(&oag_ui_screens::endrace::HdRewards {
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
    let bare = texts(&draw(&oag_ui_screens::endrace::HdRewards {
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

/// `EndRace Podium` off the real disc: the loader finds it in `DATA05` (the
/// copy `holder_of` does not serve, `DATA02`'s, has none) without disturbing
/// the other three screens, and the draw puts first place in the middle
/// column at the slot setter's own `x`, second to its left and third to its
/// right, each lower than the last, with the authored placeholders
/// (`BADGE NAME TEST`, `NAME OF PLAYER X`) never drawn.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn hd_endrace_podium_draws_three_places_off_the_copy_that_authors_it() {
    use oag_ui::frontend::Draw;
    use oag_ui_screens::endrace::hd::{HdPodium, PodiumSlot};
    let Some(image) = image() else {
        return;
    };
    let mut archives = open_hd_archives(&image.display().to_string());
    let strings = english(&mut archives);
    let screens = oag_game::endrace::load(
        &mut archives,
        &strings,
        oag_ui_screens::picker::FaceScales::default(),
        oag_hd::endrace::AUTHORED_GRID,
        &oag_hud::sprite::Sheet::default(),
        &[],
        oag_hd::TITLE,
    )
    .expect("HD's own EndRace screens read off the real disc");
    let podium = screens
        .podium
        .as_ref()
        .expect("DATA05 authors EndRace Podium, so the loader must find it");
    assert!(
        screens.rewards.is_some(),
        "the served copy still carries Rewards"
    );
    let slot = |name: &str, player| {
        Some(PodiumSlot {
            name: name.to_string(),
            player,
        })
    };
    let skin = oag_ui::menu::Skin::new(
        oag_hd::frontend::FRONT_END
            .menu
            .expect("HD authors a MenuSkin"),
        oag_display::space::Space::PSP,
        22.0,
    );
    let draw = |model: &HdPodium| {
        oag_ui_screens::endrace::hd::hd_podium_draw_list(
            model,
            podium,
            &skin,
            &oag_ui::menu::Frame::default(),
            &strings,
            None,
            false,
            &|_| None,
        )
    };
    let at = |layers: &oag_ui::menu::Layers, wanted: &str| -> (f32, f32) {
        layers
            .body
            .iter()
            .find_map(|d| match d {
                Draw::Text { text, x, y, .. } if text == wanted => Some((*x, *y)),
                _ => None,
            })
            .unwrap_or_else(|| panic!("{wanted:?} not drawn"))
    };
    let layers = draw(&HdPodium {
        places: [
            slot("ALPHA", true),
            slot("BRAVO", false),
            slot("CHARLIE", false),
        ],
    });
    let (first_x, first_y) = at(&layers, "ALPHA");
    let (second_x, second_y) = at(&layers, "BRAVO");
    let (third_x, third_y) = at(&layers, "CHARLIE");
    assert!((first_x - 790.0).abs() < 0.5, "winner is the middle column");
    assert!(second_x < first_x && first_x < third_x, "2nd, 1st, 3rd");
    assert!(first_y < second_y && second_y < third_y, "each place lower");
    for heading in ["1ST", "2ND", "3RD"] {
        at(&layers, heading);
    }
    assert!((at(&layers, "1ST").1 - 230.0).abs() < 0.5, "authored y");
    for placeholder in ["BADGE NAME TEST", "NAME OF PLAYER X"] {
        assert!(
            !layers
                .body
                .iter()
                .any(|d| matches!(d, Draw::Text { text, .. } if text == placeholder)),
            "{placeholder:?} leaked"
        );
    }
    let lone = draw(&HdPodium {
        places: [slot("ALPHA", true), None, None],
    });
    assert!(
        !lone
            .body
            .iter()
            .any(|d| matches!(d, Draw::Text { text, .. } if text == "2ND" || text == "3RD")),
        "an empty place draws no heading"
    );
}

/// A full eight-craft field on the real disc's own `EndRace Results`, laid
/// out as `0x0022c068`'s race branch and `0x0022b688` lay it out
/// (`docs/formats/hd-endrace-screens.md`): row `r` at the grid's `96 + 45 r`,
/// so the eighth row ends above the `487` bottom bar, and the file's own
/// Time-Trial footer block (`GridBottomBlock`, `y="357"` inside the grid)
/// is not drawn at all. The regression this pins: the eighth row sitting on
/// that footer, seen on a live HD campaign walk.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn an_eight_craft_hd_results_grid_clears_its_own_bottom_bar() {
    let Some(source) = image() else {
        return;
    };
    let source = source.display().to_string();
    let mut archives = open_hd_archives(&source);
    let strings = oag_ui::language::StringTable::default();
    let screens = oag_game::endrace::load(
        &mut archives,
        &strings,
        oag_ui_screens::picker::FaceScales::default(),
        oag_hd::endrace::AUTHORED_GRID,
        &oag_hud::sprite::Sheet::default(),
        &[],
        oag_hd::TITLE,
    )
    .expect("HD's own EndRace Results read off the real disc");
    let screen = &screens.results.screen;
    let fill = |name: &str| {
        screen
            .fills
            .iter()
            .find(|fill| fill.name.as_deref() == Some(name))
            .unwrap_or_else(|| panic!("{name} is on the real screen"))
            .clone()
    };
    let footer = fill("GridBottomBlock");
    let side = fill("GridSideBarL");
    // The grid's own origin: `GridSideBarL` sits at `y = 44` inside it.
    let origin_y = side.y - 44.0;

    let rows: Vec<oag_ui_screens::endrace::FieldRow> = (1..=8u8)
        .map(|place| oag_ui_screens::endrace::FieldRow {
            place,
            time_ticks: Some(6000 + u64::from(place) * 60),
            player: place == 5,
        })
        .collect();
    let model = oag_ui_screens::endrace::FieldResults {
        loyalty: None,
        headline: oag_ui_screens::endrace::Headline::Position(5),
        rows,
    };
    let skin = oag_ui::menu::Skin::new(
        oag_hd::frontend::FRONT_END
            .menu
            .expect("HD authors a MenuSkin"),
        oag_display::space::Space::PSP,
        22.0,
    );
    let layers = oag_ui_screens::endrace::hd::hd_results_draw_list(
        &model,
        &screens.results,
        &skin,
        &oag_ui::menu::Frame::default(),
        &strings,
        None,
        false,
        &|_| None,
    );
    let place_y = |place: &str| {
        layers
            .body
            .iter()
            .find_map(|draw| match draw {
                oag_ui::frontend::Draw::Text { text, y, .. } if text == place => Some(*y),
                _ => None,
            })
            .unwrap_or_else(|| panic!("place {place} drew"))
    };
    for place in 1..=8u8 {
        let expected = origin_y + 96.0 + f32::from(place - 1) * 45.0;
        assert!(
            (place_y(&place.to_string()) - expected).abs() < 0.01,
            "row {place} at {} not {expected}",
            place_y(&place.to_string())
        );
    }
    let bottom_bar = layers
        .body
        .iter()
        .filter_map(|draw| match draw {
            oag_ui::frontend::Draw::Fill { rect, .. } => Some(*rect),
            _ => None,
        })
        .find(|rect| (rect[1] - (origin_y + 487.0)).abs() < 0.01)
        .expect("GridBottomBar moves to the race layout's y = 487");
    // The eighth row's own 38-unit highlight band ends above the bar.
    assert!(place_y("8") - 2.0 + 38.0 <= bottom_bar[1], "{bottom_bar:?}");
    assert!(
        !layers.body.iter().any(|draw| matches!(
            draw,
            oag_ui::frontend::Draw::Fill { rect, .. }
                if (rect[1] - footer.y).abs() < 0.01 && (rect[3] - 38.0).abs() < 0.01
        )),
        "GridBottomBlock is hidden on a race"
    );
}
