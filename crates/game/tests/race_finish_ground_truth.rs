//! Races a full single race on a real circuit until the flag falls, and checks
//! the table it leaves behind.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
//!     race_finish_ground_truth
//! ```
//!
//! # Why this can only be a ground-truth test
//!
//! A race ends when the player crosses the line for the last time, and crossing
//! it takes a circuit: the closed ring the lap counter wraps on, the collision
//! soup the craft hovers over and the racing line the driver follows all come
//! off the disc, and no synthetic fixture has all three. `race::results`' own
//! unit tests cover the *rule* - when the board is taken and what goes on it -
//! against standings written by hand. This is the one that shows a craft
//! actually driving a single race and the game noticing how it ends.
//!
//! The player is flown by [`race::Race::set_autopilot`], which is a verification
//! aid and says so: nothing else can reach a finished race without a human at
//! the keyboard for four minutes.
//!
//! **The finishing tick here is not the one `--autopilot` prints from the
//! command line**, and the difference is the options rather than the
//! simulation: this builds `Options::default()`, where the CLI resolves the AI
//! difficulty out of the settings file - `elite` against `ace` on this machine,
//! which is a different field and so a different race. Repeated runs of this
//! file print the same tick every time, which is the property that matters.
//!
//! # A single race has two legitimate endings, not one
//!
//! Until 2026-08-26 the autopiloted player always reached the flag - the
//! recorded tick was 7,717 on `pulse-psp-usa.chd`, 2026-08-17. The AI gained
//! working weapons that day (the Mine, the Bomb, missile lock-on) and the
//! autopilot has no defence of its own - no shield, no dodge - so on `Options::default()`'s
//! full grid it is now shot down first, deterministically, on tick 3315: the
//! fixed seed (`crate::race::load`'s `SEED`) makes every run of this file kill
//! the same craft on the same tick. That is not a bug in the finish rule.
//! [`oag_race::RaceState::eliminate`] ending a single race is the disc's own
//! rule - a destroyed craft takes "Ship destroyed" instead of a place on the
//! results screen and does not respawn, confidence 75 off `0x08a82dfc` in
//! `psp-pulse-usa`'s `BOOT.BIN` - see `docs/gameplay/race-modes.md` for the full
//! argument. So both tests below check whichever ending actually happened
//! rather than assuming the player crossed the line.

use std::path::PathBuf;

use oag_gameplay::PlayerInputs;
use oag_raceplay as race;

/// Ticks to give the race before calling it stuck: a little over eight minutes
/// at the fixed 60 Hz, where a Venom-class lap of the default circuit runs
/// around a minute. Generous on purpose - this is a "did it ever finish" bound
/// rather than a claim about pace, and a slow lap is not what it is looking for.
const CAP: u64 = 30_000;

fn image() -> Option<PathBuf> {
    oag_testdata::image("data/images/pulse-psp-usa.chd")
}

/// A single race on the default circuit, everyone driven, the player included.
fn autopiloted_race() -> Option<race::Race> {
    let image = image()?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        ..race::Options::default()
    })
    .expect("loading the race");
    let mut race = race::Race::start(loaded.setup);
    race.set_autopilot(true);
    Some(race)
}

/// Runs until the race ends or [`CAP`] runs out, and says which.
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

#[test]
#[ignore = "needs a disc image under data/images/"]
fn a_single_race_ends_when_the_player_finishes_or_is_eliminated() {
    let Some(mut race) = autopiloted_race() else {
        return;
    };
    let target = race
        .sim
        .world
        .laps_target()
        .expect("a single race has laps");
    race_to_the_flag(&mut race);

    println!(
        "the race finished on tick {} ({:.1} s), the player {} of 8, craft state {:?}",
        race.sim.world.tick,
        race.sim.world.tick as f32 / 60.0,
        race.player_place(),
        race.sim.world.ships[0].physics.craft_state
    );
    match race.sim.world.ships[0].standing.finish_tick {
        Some(finish_tick) => {
            assert_eq!(
                finish_tick, race.sim.world.tick,
                "the player's own standing has to agree with the race's finish condition"
            );
            assert_eq!(
                race.sim.world.primary_race().lap,
                target + 1,
                "a finished race is one lap past its target"
            );
        }
        // The other legitimate ending, since the AI gained working weapons and
        // the autopilot has no defence of its own - see this file's module
        // doc. A craft destroyed in a single race is out, not respawned, so it
        // never earns a `finish_tick`.
        None => assert_eq!(
            race.sim.world.ships[0].physics.craft_state,
            oag_physics::CraftState::Eliminated,
            "the race ended without the player finishing or being eliminated"
        ),
    }
}

#[test]
#[ignore = "needs a disc image under data/images/"]
fn the_finished_race_leaves_a_board_with_the_whole_grid_on_it() {
    let Some(mut race) = autopiloted_race() else {
        return;
    };
    race_to_the_flag(&mut race);

    let board = race.results().expect("a finished race has results").clone();
    for row in &board.rows {
        println!(
            "{:>2}  slot {}  laps {}  {}",
            row.place,
            row.slot + 1,
            row.laps_completed,
            match row.finish_tick {
                Some(tick) => format!("finished on {tick}"),
                None => "still racing".to_string(),
            }
        );
    }

    assert_eq!(board.rows.len(), usize::from(race.sim.world.ship_count));
    assert_eq!(board.rows.len(), 8, "a single race grids eight");
    let mut places: Vec<u8> = board.rows.iter().map(|row| row.place).collect();
    places.sort_unstable();
    assert_eq!(places, (1..=8).collect::<Vec<_>>());

    let player = board.player().expect("the player is on the board");
    assert_eq!(player.place, race.player_place());

    if player.finished() {
        assert_eq!(player.laps_completed, board.laps_target.unwrap_or(0));
        // Everyone who finished did so before the player, because the player
        // finishing is what ended it.
        for row in board.rows.iter().filter(|row| row.finished()) {
            assert!(
                row.finish_tick <= player.finish_tick,
                "slot {} finished after the race ended",
                row.slot + 1
            );
        }
    } else {
        // The race's other legitimate ending - see this file's module doc.
        // The player never crosses, so none of the above holds for their own
        // row, but a craft destroyed in a single race is still out rather than
        // still racing.
        assert_eq!(
            race.sim.world.ships[0].physics.craft_state,
            oag_physics::CraftState::Eliminated,
            "the race ended without the player finishing or being eliminated"
        );
    }
    // Whichever ending it was, the finishers are ahead of everyone who did
    // not finish, in that order.
    let finishers = board.rows.iter().take_while(|row| row.finished()).count();
    assert!(
        board.rows[finishers..].iter().all(|row| !row.finished()),
        "a finisher was placed behind a craft still racing"
    );
}

/// The table is taken at the flag and never revised. Ticking on past the finish
/// must not rewrite the result - which the window does not do, but a trace run
/// or a capture with a long `--ticks` can.
#[test]
#[ignore = "needs a disc image under data/images/"]
fn ticking_past_the_flag_does_not_rewrite_the_result() {
    let Some(mut race) = autopiloted_race() else {
        return;
    };
    race_to_the_flag(&mut race);
    let taken = race.results().expect("a board").clone();

    for _ in 0..600 {
        race.tick(&PlayerInputs::none());
    }
    assert_eq!(race.results(), Some(&taken));
}
