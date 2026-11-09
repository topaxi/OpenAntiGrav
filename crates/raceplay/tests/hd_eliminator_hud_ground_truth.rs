//! Wipeout HD / Fury's Eliminator HUD, from a real race's readout.
//!
//! **`#[ignore]`d and never run in CI.** It needs the decrypted HD disc image. Run it with
//! `just test-data`, or only this file:
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-raceplay --run-ignored all \
//!     -E 'test(/^hd_eliminator_hud_ground_truth::/)'
//! ```
//!
//! The frames this is checked against are RPCS3's (`docs/ui/hud.md`, "HD's Eliminator race
//! line"): a bar along the bottom of the screen with a marker per craft, `0` at its left end
//! and `7` at its right on a grid of eight.

use oag_gameplay::PlayerInputs;
use oag_ui::frontend::Draw;

fn image() -> Option<std::path::PathBuf> {
    oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso")
}

/// The Eliminator HUD's race line reaches the draw list of a real HD race: thirteen pieces (the
/// bar, two end panels with an arrow each, seven opponents and the player), `0` behind and
/// `7` in front for a player parked on the grid's back row.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn an_eliminator_draws_the_race_line_with_every_craft_on_it() {
    let Some(image) = image() else { return };
    let loaded = oag_raceplay::load(&oag_raceplay::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::Eliminator,
        seed: Some(1),
        ..oag_raceplay::Options::default()
    })
    .expect("loading an HD Eliminator");
    let context = loaded.hud.context().expect("HD's layout parses");
    let mut race = oag_raceplay::Race::start(loaded.setup);
    race.tick(&PlayerInputs::none());
    let readout = race.readout();

    let line = readout.race_line.clone().expect("an Eliminator has a line");
    assert_eq!(line.others.len(), 7, "the other seven craft on the grid");
    assert!(
        std::iter::once(&line.player)
            .chain(&line.others)
            .all(|fraction| (0.0..1.0).contains(fraction)),
        "every marker is a fraction of a lap: {line:?}"
    );
    assert!(
        line.others.iter().all(|&fraction| fraction > 0.9),
        "the pack waits just behind the line, so its markers sit at the bar's right end: {line:?}"
    );

    let with = oag_hud::draw_list(&context, &readout);
    let without = oag_hud::draw_list(
        &context,
        &oag_hud::Readout {
            race_line: None,
            ..readout.clone()
        },
    );
    assert_eq!(
        with.sprites.len() - without.sprites.len(),
        13,
        "the bar, its four end pieces and eight markers"
    );
    // The lap counter's hexagon is one of the always-on panels: an art without it draws one fewer.
    let others: &'static [&'static str] = Box::leak(
        oag_hd::hud::ALWAYS_ON
            .iter()
            .copied()
            .filter(|name| *name != "LapPanel2")
            .collect::<Vec<_>>()
            .into_boxed_slice(),
    );
    let art: &'static oag_title::HudArt = Box::leak(Box::new(oag_title::HudArt {
        always_on: others,
        ..*context.art
    }));
    let hexagonless = oag_hud::draw_list(&oag_hud::Context { art, ..context }, &readout);
    assert_eq!(
        with.sprites.len() - hexagonless.sprites.len(),
        1,
        "`LapPanel2`, the hexagon round the lap number"
    );
    // Every lit string is drawn twice, a black twin under it (`font_border_ground_truth`).
    let mut counts: Vec<&str> = with
        .small_text
        .iter()
        .filter_map(|draw| match draw {
            Draw::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .filter(|text| matches!(*text, "0" | "7"))
        .collect();
    counts.dedup();
    assert_eq!(counts, ["0", "7"], "a count at each end of the bar");
}

/// A race that is not an Eliminator carries no race line, so the layouts that author none and the
/// ones that do both stay as they were.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_single_race_carries_no_race_line() {
    let Some(image) = image() else { return };
    let loaded = oag_raceplay::load(&oag_raceplay::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        seed: Some(1),
        ..oag_raceplay::Options::default()
    })
    .expect("loading an HD single race");
    assert_eq!(
        loaded.setup.score_law, None,
        "the awards are the Eliminator's alone"
    );
    let mut race = oag_raceplay::Race::start(loaded.setup);
    race.tick(&PlayerInputs::none());
    assert_eq!(race.readout().race_line, None);
}

/// The widgets the Eliminator draws and nothing else does are authored by the Eliminator roots
/// alone, so listing them in `oag_hd::hud::ALWAYS_ON` (`LapPanel2`) or drawing them from a
/// readout (`ShipLinePos0`, the race line) moves no other mode's frame. Every root the disc
/// ships is composed and asked.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_eliminator_only_widgets_are_authored_by_the_eliminator_roots_alone() {
    let Some(image) = image() else { return };
    let mut archives = oag_hd::open(&image.display().to_string()).expect("opening the disc");
    let mut authors = |name: &str| -> Vec<&'static str> {
        oag_hd::hud::ROOTS
            .iter()
            .copied()
            .filter(|root| {
                let mut read = |path: &str| archives.read_name(path).ok();
                oag_hud::compose(root, &mut read)
                    .unwrap_or_else(|| panic!("composing {root}"))
                    .layout
                    .sprite(name)
                    .is_some()
            })
            .collect()
    };
    let default = ["/data/xml/elimination_hud.xml"];
    // The retro skins' Eliminator roots are not asked for these two widgets, and nothing loads
    // them (`oag_hd::hud::SKINS`).
    assert_eq!(authors("LapPanel2"), default);
    let line = authors("ShipLinePos0");
    assert!(
        line.iter()
            .all(|root| root.ends_with("elimination_hud.xml"))
            && line.contains(&default[0]),
        "{line:?}"
    );
}

/// A Racebox Eliminator is won on score: the target is the slider's authored minimum, an
/// opponent's scale is `DATA00`'s row for the difficulty, and the HUD carries the ranked rows, the
/// header's panel, eight brackets and the highlight (`docs/ghidra/functions/ps3-hdfury-eu/eliminator-score.md`).
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_racebox_eliminator_is_scored_to_the_slider_target() {
    let Some(image) = image() else { return };
    let loaded = oag_raceplay::load(&oag_raceplay::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::Eliminator,
        difficulty: oag_ai::Difficulty::Novice,
        seed: Some(1),
        ..oag_raceplay::Options::default()
    })
    .expect("loading an HD Eliminator");
    let law = loaded.setup.score_law.expect("HD's Eliminator is scored");
    assert_eq!(loaded.setup.eliminator_kill_target, Some(200));
    let row = loaded
        .setup
        .weapon_ai
        .as_ref()
        .and_then(|stats| stats.eliminator())
        .expect("DATA00's row");
    assert_eq!(law.opponent_scale, row.score_scale[0]);
    assert!(
        law.opponent_scale > 1.0,
        "an opponent earns more than a person"
    );

    let context = loaded.hud.context().expect("HD's layout parses");
    let mut race = oag_raceplay::Race::start(loaded.setup);
    race.tick(&PlayerInputs::none());
    let readout = race.readout();
    assert!(readout.score_rows);
    assert_eq!(readout.kill_target, 200);
    assert_eq!(readout.kill_tags.len(), 8);
    assert_eq!(
        oag_hud::score_rows::sprites(context.layout, &readout).len(),
        10,
        "the header's panel, a bracket per row and the highlight"
    );
}

/// The board an Eliminator leaves is the score's: every row carries the points its craft ended on
/// and the rows run highest first, so `EndRace Results` can draw a `POINTS` column in that order.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_finished_eliminator_leaves_a_board_ranked_on_points() {
    let Some(image) = image() else { return };
    let loaded = oag_raceplay::load(&oag_raceplay::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::Eliminator,
        seed: Some(1),
        ..oag_raceplay::Options::default()
    })
    .expect("loading an HD Eliminator");
    let mut race = oag_raceplay::Race::start(loaded.setup);
    race.set_autopilot(true);
    for _ in 0..30_000 {
        race.tick(&PlayerInputs::none());
        if race.results().is_some() {
            break;
        }
    }
    let board = race.results().expect("the race reached its target");
    assert!(board.scored(), "a score race's board ranks on points");
    let world = &race.sim.world;
    for row in &board.rows {
        let standing = &world.ships[usize::from(row.slot)].standing;
        let points = row.points.expect("every row carries its points");
        assert_eq!(points.points, standing.score as u32, "slot {}", row.slot);
        assert_eq!(points.kills, standing.kills);
        assert_eq!(points.deaths, standing.deaths);
    }
    let ranked: Vec<u32> = board
        .rows
        .iter()
        .filter_map(|row| row.points.map(|points| points.points))
        .collect();
    assert!(
        ranked.windows(2).all(|pair| pair[0] >= pair[1]),
        "{ranked:?}"
    );
    let places: Vec<u8> = board.rows.iter().map(|row| row.place).collect();
    assert_eq!(places, (1..=8).collect::<Vec<u8>>());
}

/// The lap award reaches a craft through the race's own tick, at the opponents' scale.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn an_opponents_first_lap_pays_through_the_tick() {
    let Some(image) = image() else { return };
    let loaded = oag_raceplay::load(&oag_raceplay::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::Eliminator,
        seed: Some(1),
        ..oag_raceplay::Options::default()
    })
    .expect("loading an HD Eliminator");
    let law = loaded.setup.score_law.expect("HD's Eliminator is scored");
    let pay = law.lap * law.opponent_scale;
    let mut race = oag_raceplay::Race::start(loaded.setup);
    let snapshot = |race: &oag_raceplay::Race| {
        let world = &race.sim.world;
        std::array::from_fn::<_, 8, _>(|slot| {
            let standing = &world.ships[slot].standing;
            (standing.lap, standing.score)
        })
    };
    let mut before = snapshot(&race);
    for _ in 0..4000 {
        race.tick(&PlayerInputs::none());
        let after = snapshot(&race);
        for slot in 1..8 {
            if after[slot].0 > before[slot].0 {
                assert!(
                    after[slot].1 - before[slot].1 >= pay - 1e-3,
                    "slot {slot} crossed the line: {:?} to {:?}",
                    before[slot],
                    after[slot]
                );
                return;
            }
        }
        before = after;
    }
    panic!("no opponent finished a lap");
}

/// A score Eliminator takes a cell's target and ignores a `KILLS` pick stored by another title.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_score_race_ignores_a_stored_kill_pick() {
    let Some(image) = image() else { return };
    let options = oag_raceplay::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::Eliminator,
        eliminator_kill_target: Some(5),
        seed: Some(1),
        ..oag_raceplay::Options::default()
    };
    let picked = oag_raceplay::load(&options).expect("loading an HD Eliminator");
    assert_eq!(picked.setup.eliminator_kill_target, Some(200));
    let cell = oag_raceplay::load(&oag_raceplay::Options {
        eliminator_score_target: Some(260),
        ..options
    })
    .expect("loading an HD Eliminator");
    assert_eq!(cell.setup.eliminator_kill_target, Some(260));
}

/// Scratch sweep: one CSV row per change of a craft's score, kills, deaths or lap, in the columns
/// of `scripts/rpcs3-hd-eliminator-score.py`, so ours and the original's runs split by award kind
/// through one script. Runs only with `OAG_SWEEP` set; `OAG_SWEEP_OUT` names the file,
/// `OAG_SWEEP_SEED` and `OAG_SWEEP_DIFFICULTY` (`novice`, `skilled`, `elite`) the race.
#[test]
#[ignore = "a scratch sweep; set OAG_SWEEP"]
fn sweep_score_changes() {
    if std::env::var_os("OAG_SWEEP").is_none() {
        return;
    }
    let Some(image) = image() else { return };
    let seed: u64 = std::env::var("OAG_SWEEP_SEED").map_or(1, |s| s.parse().expect("a seed"));
    let difficulty = match std::env::var("OAG_SWEEP_DIFFICULTY").as_deref() {
        Ok("skilled") => oag_ai::Difficulty::Skilled,
        Ok("elite") => oag_ai::Difficulty::Elite,
        _ => oag_ai::Difficulty::Novice,
    };
    let loaded = oag_raceplay::load(&oag_raceplay::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::Eliminator,
        difficulty,
        seed: Some(seed),
        ..oag_raceplay::Options::default()
    })
    .expect("loading an HD Eliminator");
    let mut race = oag_raceplay::Race::start(loaded.setup);
    race.set_autopilot(true);
    let mut out =
        String::from("t,craft,score,kills,w6f78,shield,lap,lap2,state,attacker,dmgcause,lastdmg\n");
    let mut last = [(f32::NAN, 0u32, 0u32, 0u32); 8];
    let mut held = [None; 8];
    let mut events = String::new();
    for tick in 0..60_000u32 {
        race.tick(&PlayerInputs::none());
        for (slot, ship) in race.sim.world.ships.iter().enumerate().take(8) {
            let now = ship.pickup.weapon;
            if now != held[slot] {
                events.push_str(&format!(
                    "{:.3},{slot},{:?},{:?}\n",
                    f64::from(tick) / 60.0,
                    held[slot],
                    now
                ));
                held[slot] = now;
            }
        }
        for (slot, ship) in race.sim.world.ships.iter().enumerate().take(8) {
            let s = &ship.standing;
            let now = (s.score, s.kills, s.deaths, s.lap);
            if now.0.to_bits() != last[slot].0.to_bits()
                || now.1 != last[slot].1
                || now.2 != last[slot].2
                || now.3 != last[slot].3
            {
                last[slot] = now;
                out.push_str(&format!(
                    "{:.3},{slot},{},{},{},{},{},0,1,-1,0,0\n",
                    f64::from(tick) / 60.0,
                    s.score,
                    s.kills,
                    s.deaths,
                    ship.physics.shield,
                    s.lap
                ));
            }
        }
        if race.results().is_some() {
            break;
        }
    }
    let path = std::env::var("OAG_SWEEP_OUT").expect("OAG_SWEEP_OUT");
    std::fs::write(format!("{path}.pickups"), events).expect("writing the pickups");
    std::fs::write(path, out).expect("writing the sweep");
}

/// Runs an HD Racebox Eliminator to its target under autopilot and answers the tick it ended on
/// and the laps the leading opponent had completed, `None` if it never ended.
fn race_to_target(
    image: &std::path::Path,
    difficulty: oag_ai::Difficulty,
    seed: u64,
) -> Option<(u64, u32)> {
    let loaded = oag_raceplay::load(&oag_raceplay::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::Eliminator,
        difficulty,
        seed: Some(seed),
        ..oag_raceplay::Options::default()
    })
    .expect("loading an HD Eliminator");
    let mut race = oag_raceplay::Race::start(loaded.setup);
    assert_eq!(
        race.pad_seeking(),
        oag_raceplay::PadSeeking::Off,
        "a score Eliminator's opponents steer for no pad"
    );
    race.set_autopilot(true);
    for tick in 0..60_000u64 {
        race.tick(&PlayerInputs::none());
        if race.results().is_some() {
            let world = &race.sim.world;
            let leader = (1..world.ship_count as usize)
                .max_by(|&a, &b| {
                    world.ships[a]
                        .standing
                        .score
                        .total_cmp(&world.ships[b].standing.score)
                })
                .expect("an opponent");
            return Some((tick, world.ships[leader].standing.lap - 1));
        }
    }
    None
}

/// A score Eliminator lasts about as long as the original's (ten laps to 200 at Novice on Vineta K,
/// `docs/gameplay/race-modes.md`, "The pace of a score Eliminator"), on the seeds the pace was tuned
/// on. Novice reaches its target, which it never did while a Novice opponent held a Mine for good;
/// it takes more than eight laps, which it did not while opponents steered for pads and bunched;
/// and Elite, whose opponent keeps a pickup twenty seconds, takes more than three.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_score_eliminator_reaches_its_target_at_about_the_originals_pace() {
    let Some(image) = image() else { return };
    let (tick, laps) =
        race_to_target(&image, oag_ai::Difficulty::Novice, 1).expect("a Novice race reaches 200");
    assert!(tick < 40_000, "Novice ended at tick {tick}");
    assert!(laps >= 8, "Novice's leader had completed {laps} laps");
    let (_, laps) =
        race_to_target(&image, oag_ai::Difficulty::Elite, 1).expect("an Elite race reaches 200");
    assert!(laps >= 4, "Elite's leader had completed {laps} laps");
}
