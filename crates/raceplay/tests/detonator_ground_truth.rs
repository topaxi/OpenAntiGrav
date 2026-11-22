//! A Detonator race on HD/Fury runs its fourteen stages, one per lap, and ends on the fifteenth.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this project does not
//! ship.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-raceplay --run-ignored all \
//!     -E 'test(/^detonator_ground_truth::/)'
//! ```
//!
//! The law is `Detonator_UpdateRace`'s (`docs/ghidra/functions/ps3-hdfury-eu/detonator-mode.md`):
//! the stage opens at `1`, steps on each completed lap and the race is over at `15`. The
//! player flies alone on the autopilot's racing line (`RaceSim::set_autopilot`), and the run
//! stops on the tick the race finishes.

use oag_gameplay::PlayerInputs;
use oag_raceplay as race;

fn load() -> Option<race::Loaded> {
    let path = oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso")?;
    Some(
        race::load(&race::Options {
            source: path.display().to_string(),
            class: "VENOM".to_string(),
            mode: oag_race::Mode::Detonator,
            ..race::Options::default()
        })
        .expect("loading an HD Detonator race"),
    )
}

/// The autopilot flies the racing line through the mines and does not dodge; a test about the
/// stage counter keeps it alive so it reaches the lines.
fn keep_alive(race: &mut race::Race) {
    let ship = &mut race.core.sim.world.ships[0];
    ship.physics.shield = ship.handling.dimensions.shield;
}

/// [`keep_alive`], and the field swept clear so the craft is not slowed by every mine on the
/// line (each costs half its speed): a player dodges, the autopilot does not.
fn dodge(race: &mut race::Race) {
    keep_alive(race);
    race.core.sim.world.detonator.live_mines = 0;
}

/// The load takes Detonator's own grade table, opens on stage `1`, and fields nobody else.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn an_hd_detonator_race_loads_its_own_stage_table() {
    let Some(loaded) = load() else {
        return;
    };
    let grade = loaded
        .zone_grade
        .as_ref()
        .expect("HD ships /data/environments/detonatormode.effectsettings");
    assert!(
        grade.describe().contains("detonatormode"),
        "graded from {}",
        grade.describe()
    );
    assert_eq!(grade.last_stage(), 14);
    assert_eq!(grade.blend().current, 1);
    let (decoded, numbered) = grade.stage_art_counts();
    assert_eq!(
        (decoded, numbered),
        (15, 15),
        "detonatormodetrack0..14 decode"
    );
    // The EMP's cues decode: the pad's `WEAPONPICKUP` from the weapons bank and the two
    // announcer lines from `speech_det.bnk`, the one speech bank that names them.
    let report = loaded.report.join("\n");
    for line in [
        "sfx: WEAPONPICKUP -> ",
        "sfx: EMPREADY -> ",
        "sfx: EMPFULL -> ",
    ] {
        assert!(report.contains(line), "{line}...:\n{report}");
    }
}

/// **The stage counter.** Stage `1` until the first lap is completed, then one step per lap,
/// and the race finishes on the tick the fourteenth lap completes - stage `15` - not before.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn an_hd_detonator_race_steps_a_stage_per_lap_and_ends_at_fifteen() {
    const LONE: usize = 0;
    let Some(loaded) = load() else {
        return;
    };
    let mut race = race::Race::start(loaded.setup, loaded.view, loaded.modules);
    // The player flies their own racing line (`RaceSim::set_autopilot`); the rules are the race's.
    race.core.sim.set_autopilot(true);
    assert_eq!(
        race.core.sim.world.race[LONE].mode,
        oag_race::Mode::Detonator
    );
    assert_eq!(race.core.sim.world.race[LONE].laps_target, Some(14));

    let stage = |race: &race::Race| {
        oag_race::Mode::detonator_stage(race.core.sim.world.race[LONE].laps_completed())
    };
    assert_eq!(stage(&race), 1);
    let mut seen = vec![stage(&race)];
    let mut shield_per_lap = Vec::new();
    // Fourteen laps of an HD circuit at Venom are well under ten minutes of race.
    for _ in 0..60 * 60 * 10 {
        race.tick(&PlayerInputs::none());
        dodge(&mut race);
        let now = stage(&race);
        if now != *seen.last().expect("one stage at least") {
            shield_per_lap.push(race.core.sim.world.ships[LONE].physics.shield);
            seen.push(now);
        }
        if race.core.sim.world.race[LONE].finished {
            break;
        }
    }
    println!("stages seen: {seen:?}");
    println!("per-lap shield (at each line crossing): {shield_per_lap:?}");
    println!(
        "end-of-run shield: {}",
        race.core.sim.world.ships[LONE].physics.shield
    );
    assert!(race.core.sim.world.race[LONE].finished, "the race ends");
    assert_eq!(
        seen,
        (1..=15).collect::<Vec<u32>>(),
        "one stage per lap, 1 to 15"
    );
    assert_eq!(race.core.sim.world.race[LONE].laps_completed(), 14);
}

/// A title's own Detonator table reaches the cannon: holding Square past the green light spends a
/// volley from the clip. A table that lacks an attribute the cannon needs leaves the clip full.
fn assert_the_cannon_fires(loaded: &race::Loaded) {
    assert!(
        !loaded
            .report
            .iter()
            .any(|line| line.contains("no cannon_flash locators")),
        "the hull authors both cannon_flash locators, so the rounds leave from them: {:#?}",
        loaded.report
    );
    let mut race = race::Race::start(
        loaded.setup.clone(),
        loaded.view.clone(),
        loaded.modules.clone(),
    );
    start_racing(&mut race);
    for _ in 0..3 {
        race.tick(&square());
        keep_alive(&mut race);
    }
    let cannon = &race.core.sim.world.detonator.cannon;
    assert!(cannon.clip < 15, "no volley left the clip: {}", cannon.clip);
    assert!(cannon.rounds_in_flight() > 0);
}

/// 2048 on one of HD's ported Zone circuits: its own circuit table is absent, so the HD/Fury
/// DLC3 Detonator table out of the v1.04 patch grades it (`oag_2048::race::DETONATOR`).
#[test]
#[ignore = "needs the unpacked 2048 packages in data/extracted/"]
fn a_2048_detonator_race_on_zone_1_takes_the_hd_fury_dlc3_table() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/extracted/vita/PCSF00007");
    if !path.join("patch-v104").exists() {
        assert!(
            std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
            "OAG_REQUIRE_GAME_DATA is set but {} is missing",
            path.display()
        );
        return;
    }
    let loaded = race::load(&race::Options {
        source: path.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::Detonator,
        track: Some(r"data\art\published\DLC1\environments\zone_1\track.vex".to_string()),
        ..race::Options::default()
    })
    .expect("loading a 2048 Detonator race");
    assert_eq!(
        race::Race::start(
            loaded.setup.clone(),
            loaded.view.clone(),
            loaded.modules.clone()
        )
        .core
        .sim
        .world
        .race[0]
            .laps_target,
        Some(14)
    );
    let grade = loaded.zone_grade.as_ref().expect("a grade");
    assert!(
        grade
            .describe()
            .starts_with("data/ZoneEnvironmentHDFury/DetonatorModeHDFuryDLC3.effectSettings"),
        "graded from {}",
        grade.describe()
    );
    assert_eq!(grade.last_stage(), 14);
    assert_the_cannon_fires(&loaded);
}

/// Omega runs the mode's rules and draws no grade, which it does not draw for Zone either.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn an_omega_detonator_race_runs_fourteen_stages_without_a_grade() {
    let Some(path) = oag_testdata::image("data/images/omega-ps4-eu.pkg") else {
        return;
    };
    let loaded = race::load(&race::Options {
        source: path.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::Detonator,
        ..race::Options::default()
    })
    .expect("loading an Omega Detonator race");
    assert_eq!(
        race::Race::start(
            loaded.setup.clone(),
            loaded.view.clone(),
            loaded.modules.clone()
        )
        .core
        .sim
        .world
        .race[0]
            .laps_target,
        Some(14)
    );
    assert!(loaded.zone_grade.is_none());
    assert!(
        loaded
            .report
            .iter()
            .any(|line| line.starts_with("detonator: this title's stage grade is not drawn")),
    );
    assert_the_cannon_fires(&loaded);
}

/// **Mines and bombs.** The opening stage lays up to the first stage's count (the disc's
/// `MineCountStage1`, `80`) and at most one bomb; the first lap's crossing lays the second
/// stage's top-up on a total of at most `90`. Nothing is laid off a usable lane, and nothing
/// moves: the field only grows.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn an_hd_detonator_race_lays_its_mines_and_steps_the_total_a_stage() {
    let Some(loaded) = load() else {
        return;
    };
    assert!(
        loaded
            .report
            .iter()
            .any(|line| line.contains("PatternFile.xml") && line.contains("18 pattern group")),
        "the layout load is reported: {:?}",
        loaded.report
    );
    let mut race = race::Race::start(loaded.setup, loaded.view, loaded.modules);
    race.core.sim.set_autopilot(true);
    race.tick(&PlayerInputs::none());
    let first = race.core.sim.world.detonator;
    assert_eq!(first.stages_laid, 1);
    assert!(
        (40..=80).contains(&first.live_mines),
        "stage 1 aims for 80 and a lane the spawn refuses costs one: {}",
        first.live_mines
    );
    assert!(first.live_bombs <= 1);
    let laid = first.live_mines;
    let mut seen = std::collections::BTreeSet::new();
    assert!(
        first
            .live_mines()
            .iter()
            .all(|m| seen.insert((m.entry, m.lane))),
        "no two mines share a lane"
    );
    assert!(
        first.live_mines().iter().all(|m| m.lane.abs() <= 10),
        "a lane is at most ten either side of the centre"
    );

    for _ in 0..60 * 60 * 3 {
        race.tick(&PlayerInputs::none());
        keep_alive(&mut race);
        if race.core.sim.world.detonator.stages_laid >= 2 {
            break;
        }
    }
    let second = race.core.sim.world.detonator;
    assert_eq!(second.stages_laid, 2, "the first crossing lays stage 2");
    assert!(second.live_mines <= 90);
    assert!(
        second.live_mines >= laid.min(first.live_mines) / 2,
        "the top-up refills toward the stage's count"
    );
}

/// **A mine bites.** A craft set on a laid mine loses the Mine's `damage` (10, the Detonator
/// table's) and half its speed, and the mine is gone; mines keep costing until the craft is
/// destroyed, and the race is then over.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn an_hd_detonator_mine_costs_energy_and_destruction_ends_the_race() {
    const LONE: usize = 0;
    let Some(loaded) = load() else {
        return;
    };
    let mut race = race::Race::start(loaded.setup, loaded.view, loaded.modules);
    race.tick(&PlayerInputs::none());
    let before = race.core.sim.world.detonator.live_mines;
    assert!(before > 0, "stage 1 laid mines");

    let at = race.core.sim.detonator_mine_position(0).expect("a mine");
    let ship = &mut race.core.sim.world.ships[LONE].physics;
    let energy = ship.shield;
    ship.body.position = at;
    ship.body.linear_velocity = oag_core::math::Vec3::new(0.0, 0.0, 100.0);
    race.tick(&PlayerInputs::none());

    let after = &race.core.sim.world.ships[LONE].physics;
    assert_eq!(race.core.sim.world.detonator.live_mines, before - 1);
    assert!(
        (energy - after.shield - 10.0).abs() < 1e-3,
        "{energy} -> {}",
        after.shield
    );

    for _ in 0..200 {
        let Some(at) = race.core.sim.detonator_mine_position(0) else {
            break;
        };
        race.core.sim.world.ships[LONE].physics.body.position = at;
        race.tick(&PlayerInputs::none());
        if race.core.sim.world.race[LONE].finished {
            break;
        }
    }
    assert!(
        race.core.sim.world.race[LONE].finished,
        "a destroyed craft ends the race"
    );
}

/// Slot 0's pad with Square held.
fn square() -> PlayerInputs {
    let mut snapshot = oag_gameplay::InputSnapshot::new();
    snapshot
        .buttons
        .begin_frame(oag_gameplay::input::Button::Square.bit());
    PlayerInputs::single(snapshot)
}

/// Ticks past the countdown, the craft kept alive and the autopilot off.
fn start_racing(race: &mut race::Race) {
    while oag_race::RaceState::thrust_gated(race.core.sim.world.tick) {
        race.tick(&PlayerInputs::none());
        keep_alive(race);
    }
}

/// **The cannon.** A craft standing in the field with Square held shoots the mines in front of
/// it: the score never goes down and ends above zero, the destroyed count rises with the
/// mines the clip hits directly, the shot mines are gone from the field, and the percentage is
/// that count over the disc's `1860` mines. A craft the autopilot flies fires nothing.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn an_hd_detonator_cannon_shoots_mines_and_scores_them() {
    const LONE: usize = 0;
    let Some(loaded) = load() else {
        return;
    };
    let mut race = race::Race::start(loaded.setup, loaded.view, loaded.modules);
    start_racing(&mut race);

    race.core.sim.set_autopilot(true);
    for _ in 0..30 {
        race.tick(&square());
        keep_alive(&mut race);
    }
    race.core.sim.set_autopilot(false);
    let sim = &race.core.sim;
    assert_eq!(
        sim.world.detonator.cannon.clip, 15,
        "a flown craft's pad fires nothing"
    );
    assert_eq!(sim.world.detonator.cannon.rounds_in_flight(), 0);
    assert_eq!(sim.detonator_score(), Some(0));

    let laid = race.core.sim.world.detonator.live_mines;
    let mut last = 0;
    let mut destroyed = 0;
    'stand: for index in 0..usize::from(laid) {
        let Some((stand, forward, up)) = race.core.sim.detonator_mine_stand(index, 60.0) else {
            break;
        };
        let right = forward.cross(up).normalize();
        let orientation = oag_core::math::Quat::from_mat3(&oag_core::math::Mat3::from_cols(
            right,
            right.cross(forward).normalize(),
            -forward,
        ));
        let ship = &mut race.core.sim.world.ships[LONE].physics;
        ship.body.position = stand;
        ship.body.orientation = orientation;
        ship.body.linear_velocity = oag_core::math::Vec3::ZERO;
        ship.body.angular_velocity = oag_core::math::Vec3::ZERO;
        for _ in 0..40 {
            race.tick(&square());
            keep_alive(&mut race);
            let score = race.core.sim.detonator_score().expect("a Detonator race");
            assert!(
                score >= last,
                "the score never goes down: {last} -> {score}"
            );
            last = score;
            destroyed = race
                .core
                .sim
                .detonator_destroyed_mines()
                .expect("a Detonator race");
            if destroyed > 0 {
                break 'stand;
            }
        }
    }
    println!("destroyed {destroyed}, score {last}");
    assert!(destroyed > 0, "a clip down the lane destroys a mine");
    assert!(last >= 50, "and scores it: {last}");
    assert!(race.core.sim.world.detonator.live_mines < laid + 1);
    assert_eq!(
        race.core.sim.detonator_percentage(),
        Some(destroyed as f32 / 1860.0 * 100.0)
    );
}

/// The craft's speed.
fn speed(race: &race::Race) -> f32 {
    race.core.sim.world.ships[0]
        .physics
        .body
        .linear_velocity
        .length()
}

/// **Auto-speed.** A craft nobody throttles reaches speed after the green light on Zone's law
/// with the Detonator stage as the count (`detonator-speed.md`: live, `107.6` within 4 s of GO
/// with cross never pressed). The ramp is full by GO and the target on lap 1 is `start`, `34`.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn an_hd_detonator_craft_with_no_throttle_reaches_speed() {
    let Some(loaded) = load() else {
        return;
    };
    let mut race = race::Race::start(loaded.setup, loaded.view, loaded.modules);
    while oag_race::RaceState::thrust_gated(race.core.sim.world.tick) {
        race.tick(&PlayerInputs::none());
        dodge(&mut race);
    }
    assert_eq!(
        race.core.sim.world.detonator.ramp, 1.0,
        "the ramp is full by GO"
    );
    assert_eq!(race.core.sim.detonator_speed_target(), Some(34.0));
    // An unsteered craft meets a wall within seconds, as the live run did, so the claim is the
    // top speed it reaches before that.
    let at_go = speed(&race);
    let mut top: f32 = 0.0;
    for _ in 0..60 * 4 {
        race.tick(&PlayerInputs::none());
        dodge(&mut race);
        top = top.max(speed(&race));
    }
    println!("speed at GO {at_go}, top within 4 s {top}");
    assert!(top > 90.0, "the engine drives with no throttle: {top}");
}

/// The engine's target steps with the stage: `34 + 4.4 n`, `n = 0` on lap 1 and the HUD stage
/// from each later crossing (`detonator-speed.md`, "`n`: the Detonator stage").
#[test]
#[ignore = "needs a disc image in data/images/"]
fn an_hd_detonator_speed_target_steps_with_the_stage() {
    let Some(loaded) = load() else {
        return;
    };
    let mut race = race::Race::start(loaded.setup, loaded.view, loaded.modules);
    start_racing(&mut race);
    for laid in [1u8, 2, 3, 9, 14] {
        race.core.sim.world.detonator.stages_laid = laid;
        let n = if laid == 1 { 0.0 } else { f32::from(laid) };
        assert_eq!(
            race.core.sim.detonator_speed_target(),
            Some(34.0 + 4.4 * n),
            "stage {laid}"
        );
    }
}

/// **Pads charge.** Flying the lap, every speed-pad entry raises the EMP charge by one, plays
/// `WEAPONPICKUP`, and leaves no boost timer (`craft+0x2e8` read `0.0` over 37 live entries).
#[test]
#[ignore = "needs a disc image in data/images/"]
fn an_hd_detonator_speed_pad_charges_the_emp_and_gives_no_boost() {
    use oag_gameplay::cue::Cue;
    let Some(loaded) = load() else {
        return;
    };
    let mut race = race::Race::start(loaded.setup, loaded.view, loaded.modules);
    start_racing(&mut race);
    race.core.sim.set_autopilot(true);
    let mut charges = Vec::new();
    let mut pickups = 0;
    let mut last = 0;
    for _ in 0..60 * 60 {
        race.tick(&PlayerInputs::none());
        dodge(&mut race);
        let ship = &race.core.sim.world.ships[0].physics;
        assert_eq!(ship.pad_timer, 0.0, "a Detonator pad gives no boost");
        pickups += race
            .core
            .sim
            .drain_cues()
            .iter()
            .filter(|cue| cue.cue == Cue::WeaponPickup)
            .count();
        let now = race.core.sim.world.detonator.emp.charge;
        if now != last {
            assert_eq!(now, last + 1, "one charge per entry");
            charges.push(race.core.sim.world.tick);
            last = now;
        }
        if race.core.sim.world.race[0].laps_completed() >= 1 {
            break;
        }
    }
    println!("charges on ticks {charges:?}, WEAPONPICKUP {pickups}");
    assert!(!charges.is_empty(), "a lap crosses a speed pad");
    assert_eq!(pickups, charges.len(), "each charge plays WEAPONPICKUP");
    let (n, ready) = race.core.sim.detonator_emp().expect("a Detonator race");
    assert_eq!(n, last);
    assert_eq!(ready, n >= 4);
}

/// Slot 0's pad with L1 (the EMP button) pressed this tick.
fn l1() -> PlayerInputs {
    let mut snapshot = oag_gameplay::InputSnapshot::new();
    snapshot
        .buttons
        .begin_frame(oag_gameplay::input::Button::Flip.bit());
    PlayerInputs::single(snapshot)
}

/// A full EMP fired from a stand before a mine, every mine marked live or dark: returns the
/// mines it cleared and what they paid, after checking the charge and the clip.
fn emp_from_a_stand(live: bool) -> Option<(u32, i32)> {
    let loaded = load()?;
    let mut race = race::Race::start(loaded.setup, loaded.view, loaded.modules);
    start_racing(&mut race);
    let (stand, forward, up) = race.core.sim.detonator_mine_stand(0, 30.0)?;
    let right = forward.cross(up).normalize();
    let orientation = oag_core::math::Quat::from_mat3(&oag_core::math::Mat3::from_cols(
        right,
        right.cross(forward).normalize(),
        -forward,
    ));
    let world = &mut race.core.sim.world;
    world.detonator.live_bombs = 0;
    let count = usize::from(world.detonator.live_mines);
    for mine in &mut world.detonator.mines[..count] {
        mine.live = live;
    }
    world.detonator.emp.charge = 8;
    world.detonator.emp.gauge = 1.0;
    let hold = |race: &mut race::Race| {
        let ship = &mut race.core.sim.world.ships[0].physics;
        ship.body.position = stand;
        ship.body.orientation = orientation;
        ship.body.linear_velocity = oag_core::math::Vec3::ZERO;
        ship.body.angular_velocity = oag_core::math::Vec3::ZERO;
        keep_alive(race);
    };
    hold(&mut race);
    let clip = race.core.sim.world.detonator.cannon.clip;
    let (score, destroyed) = (
        race.core.sim.detonator_score()?,
        race.core.sim.detonator_destroyed_mines()?,
    );
    race.tick(&l1());
    hold(&mut race);
    assert_eq!(
        race.core.sim.detonator_emp(),
        Some((0, false)),
        "firing resets the charge"
    );
    assert_eq!(race.core.sim.world.detonator.emp.gauge, 0.0);
    for _ in 0..90 {
        race.tick(&PlayerInputs::none());
        hold(&mut race);
    }
    assert_eq!(
        race.core.sim.world.detonator.cannon.clip, clip,
        "the clip is untouched"
    );
    let cleared = race.core.sim.detonator_destroyed_mines()? - destroyed;
    let paid = race.core.sim.detonator_score()? - score;
    Some((cleared, paid))
}

/// **Fire.** L1 at a full charge resets it and clears the mines ahead, each counted and paid by
/// `DetonatorScoring`'s `<Emp>`: `+25` for a dark mine (the live run's value), `+50` for one laid
/// this stage.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn an_hd_detonator_emp_clears_the_mines_ahead_and_scores_the_emp_values() {
    let Some((dark, dark_paid)) = emp_from_a_stand(false) else {
        return;
    };
    let (live, live_paid) = emp_from_a_stand(true).expect("loaded once");
    println!("dark: {dark} cleared for {dark_paid}; live: {live} cleared for {live_paid}");
    assert!(dark > 0, "the wave clears a mine");
    assert_eq!(dark_paid, 25 * dark as i32);
    assert_eq!(live, dark, "the same field, the same wave");
    assert_eq!(live_paid, 50 * live as i32);
}
