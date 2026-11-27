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
    // HD's visualiser strip is sRGB-decoded on fetch (`zone-vis-gamma.md`).
    assert_eq!(grade.zone_uniform().shape[2], 1.0);
    // Stage 1's `Lighting.Track Fog colour` and `density`: the orange-red a track chunk is fogged
    // with (`detonator-look.md`, "What the file says at stage 1").
    assert_eq!(
        grade.track_fog(),
        oag_mesh::mesh_render::TrackFog::authored([1.8, 0.141_176, 0.007_059], 0.001_5)
    );
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
    assert!(race.core.sim.l1_is_flip(), "HD's L1 is the EMP");
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
    // The end screen's stage row reads the scoreboard: a finish pads to the target.
    let board = race.core.results().expect("a finished race has a board");
    assert_eq!(board.player().map(|row| row.laps_completed), Some(14));
    // And the counters it prints exist on a Detonator race and nowhere else.
    assert_eq!(race.core.sim.detonator_destroyed_bombs(), Some(0));
    assert_eq!(race.core.sim.detonator_chains(), Some([0, 0, 0]));
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

/// The race-start placement call and the stage cap, read on HD (`Detonator_PlaceOpeningStage`) and
/// on Omega (`FUN_012b80a0(rm, stage - 1)` once at the start; the stage `rm+0x300d6`, capped `14`,
/// into `*(craft+0x6a18)+0x5f4`, v1.07; `+0x5e4` on v1.00): the first tick lays the opening
/// stage's mines and at most one bomb on distinct lanes, and the auto-speed's stage count is
/// `0` on stage 1, the stage after, and `14` at most (`detonator-mode.md`, "The same law on 2048
/// and Omega"). Returns the mines the first tick laid.
fn assert_the_opening_stage_and_the_stage_cap(loaded: &race::Loaded) -> u16 {
    assert!(
        loaded
            .report
            .iter()
            .any(|line| line.contains("PatternFile.xml") && line.contains("18 pattern group")),
        "the layout load is reported: {:?}",
        loaded.report
    );
    let mut race = race::Race::start(
        loaded.setup.clone(),
        loaded.view.clone(),
        loaded.modules.clone(),
    );
    assert_eq!(
        race.core.sim.world.detonator.stages_laid, 0,
        "nothing laid before tick 1"
    );
    race.core.sim.set_autopilot(true);
    race.tick(&PlayerInputs::none());
    let first = race.core.sim.world.detonator;
    assert_eq!(
        first.stages_laid, 1,
        "the opening stage is laid on the first tick"
    );
    assert!(
        (40..=80).contains(&first.live_mines),
        "stage 1 aims for 80: {}",
        first.live_mines
    );
    assert!(first.live_bombs <= 1);
    let mut seen = std::collections::BTreeSet::new();
    assert!(
        first
            .live_mines()
            .iter()
            .all(|m| seen.insert((m.entry, m.lane)) && m.lane.abs() <= 10),
        "no two mines share a lane, and a lane is at most ten either side of the centre"
    );
    // The ramp is full by GO (`assert_auto_speed_and_pads`); the target below is its full value.
    race.core.sim.world.detonator.ramp = 1.0;
    for laid in [1u8, 2, 13, 14] {
        race.core.sim.world.detonator.stages_laid = laid;
        let n = if laid == 1 { 0.0 } else { f32::from(laid) };
        assert_eq!(
            race.core.sim.detonator_speed_target(),
            Some(34.0 + 4.4 * n),
            "stage {laid}"
        );
    }
    race.core.sim.world.detonator.stages_laid = 14;
    let at_cap = race.core.sim.detonator_speed_target();
    for beyond in [15u8, 200] {
        race.core.sim.world.detonator.stages_laid = beyond;
        assert_eq!(
            race.core.sim.detonator_speed_target(),
            at_cap,
            "stage {beyond} reads the stage 14 target"
        );
    }
    first.live_mines
}

/// A title's Detonator craft flies the auto-speed (the ramp full by GO, the stage-1 target `34`),
/// and over a lap on the autopilot its speed pads charge the EMP and never boost; stood on a
/// speed pad by hand, the tick of entry charges once and leaves no boost timer. `line_crosses_a_pad`
/// is `false` where the autopilot's line misses every pad (2048's `zone_1`: none of its seven
/// entered over a 2120-tick lap), so the lap charges nothing there.
fn assert_auto_speed_and_pads(loaded: &race::Loaded, line_crosses_a_pad: bool) {
    let mut race = race::Race::start(
        loaded.setup.clone(),
        loaded.view.clone(),
        loaded.modules.clone(),
    );
    start_racing(&mut race);
    assert_eq!(
        race.core.sim.world.detonator.ramp, 1.0,
        "the ramp is full by GO"
    );
    assert_eq!(race.core.sim.detonator_speed_target(), Some(34.0));
    race.core.sim.set_autopilot(true);
    let mut boosted = false;
    for _ in 0..60 * 120 {
        race.tick(&PlayerInputs::none());
        dodge(&mut race);
        boosted |= race.core.sim.world.ships[0].physics.pad_timer > 0.0;
        if race.core.sim.world.race[0].laps_completed() >= 1 {
            break;
        }
    }
    let charge = race.core.sim.world.detonator.emp.charge;
    println!("charge after a lap {charge}, a pad boosted: {boosted}");
    assert_eq!(
        charge > 0,
        line_crosses_a_pad,
        "a lap's speed pads charge the EMP"
    );
    assert!(!boosted, "and never boost");
    // Stood on a speed pad by hand: the one tick of entry charges.
    race.core.sim.set_autopilot(false);
    let pad = loaded
        .setup
        .speedup_pads
        .first()
        .expect("the circuit has speed pads");
    let ship = &mut race.core.sim.world.ships[0].physics;
    ship.body.position = oag_core::math::Vec3::from_array(pad.centre());
    race.tick(&PlayerInputs::none());
    let ship = &race.core.sim.world.ships[0].physics;
    let after = race.core.sim.world.detonator.emp.charge;
    assert_eq!(
        (after, ship.pad_timer),
        (charge + 1, 0.0),
        "a charge, no boost"
    );
}

/// A weapon pad, flown by the player: one charge and no pickup; the same pad again charges
/// nothing until another pad or a crossing, a second pad charges again (`Ship_TestWeaponPad`'s
/// mode-14 arm, the pad latched against the last one taken).
fn assert_weapon_pads_charge(race: &mut race::Race, weapon_pads: &[oag_vex::pads::PadVolume]) {
    race.core.sim.set_autopilot(false);
    assert!(weapon_pads.len() >= 2, "the circuit has weapon pads");
    let visit = |race: &mut race::Race, at: Option<[f32; 3]>| {
        let ship = &mut race.core.sim.world.ships[0].physics;
        if let Some(at) = at {
            ship.body.position = oag_core::math::Vec3::from_array(at);
        } else {
            ship.body.position += ship.body.orientation * oag_core::math::Vec3::new(0.0, 40.0, 0.0);
        }
        ship.body.linear_velocity = oag_core::math::Vec3::ZERO;
        race.tick(&PlayerInputs::none());
        dodge(race);
        race.core.sim.world.detonator.emp.charge
    };
    let first = weapon_pads[0].centre();
    // A first hop to above the pad: the jump from wherever the craft stood sweeps across the
    // track and may cross a speed pad, and it is the tick the autopilot hands back.
    // The pad's memory is then cleared as a start-line crossing clears it.
    visit(race, Some(first));
    visit(race, None);
    race.core.sim.world.detonator.emp.weapon_pad = None;
    let before = race.core.sim.world.detonator.emp.charge;
    let got = visit(race, Some(first));
    assert_eq!(got, before + 1, "a weapon pad charges");
    assert!(
        race.core.sim.world.ships[0].pickup.is_empty(),
        "and hands nothing out"
    );
    assert_eq!(visit(race, None), before + 1);
    assert_eq!(
        visit(race, Some(first)),
        before + 1,
        "the same pad again: nothing"
    );
    let second = weapon_pads[1].centre();
    assert_eq!(visit(race, Some(second)), before + 2, "another pad charges");
    assert!(race.core.sim.world.ships[0].pickup.is_empty());
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
    let options = race::Options {
        no_effects: true,
        source: path.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::Detonator,
        track: Some(r"data\art\published\DLC1\environments\zone_1\track.vex".to_string()),
        ..race::Options::default()
    };
    let loaded = race::load(&options).expect("loading a 2048 Detonator race");
    let race = race::Race::start(
        loaded.setup.clone(),
        loaded.view.clone(),
        loaded.modules.clone(),
    );
    assert_eq!(race.core.sim.world.race[0].laps_target, Some(14));
    assert!(race.core.sim.l1_is_flip(), "Omega's L1 is the EMP");
    let grade = loaded.zone_grade.as_ref().expect("a grade");
    assert!(
        grade
            .describe()
            .starts_with("data/ZoneEnvironmentHDFury/DetonatorModeHDFuryDLC3.effectSettings"),
        "graded from {}",
        grade.describe()
    );
    assert_eq!(grade.last_stage(), 14);
    // The fallback's two per-stage texture sets, as `.gxt` (`Environment_LoadHDFuryContent`
    // loads `DetonatorModeTrack<n>`): the track set from the patch, the general set from the base.
    for set in ["detonatorModeTrack", "detonatorMode"] {
        let line = format!("data/Tex/{set}0.gxt..14: 15/15 per-stage Zone");
        assert!(
            loaded.report.iter().any(|l| l.contains(&line)),
            "{line}: {:#?}",
            loaded.report
        );
    }
    assert_the_cannon_fires(&loaded);
    assert_the_opening_stage_and_the_stage_cap(&loaded);
    assert_auto_speed_and_pads(&loaded, false);
    assert_the_pad_is_red_squares_on_a_slab_that_rests(&loaded);
    let plain = assert_the_circuits_own_pads_are_left_out(&options, &loaded);
    assert_a_single_race_speed_pad_still_boosts(&plain);
}

/// **2048's pads charge** (`docs/ghidra/functions/vita-2048-eu-v104/detonator-pads.md`): on its
/// HD-pack Vineta K, a lap on the autopilot charges the EMP from the speed pads with no boost, a
/// weapon pad charges as HD's does, and 2048's own Detonator table counts the pads
/// (`minNumPadsToCollectEMP`/`maxNumPadsToCollectEMP`, stored at `+0x234`/`+0x238` by
/// `FUN_8100f7c2` and read there by the charge, `FUN_8117e9c2`): the EMP is ready at the first
/// count and not one before.
#[test]
#[ignore = "needs the unpacked 2048 packages in data/extracted/"]
fn a_2048_detonator_pad_charges_the_emp_and_gives_no_boost_or_pickup() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/extracted/vita/PCSF00007");
    if !path.join("dlc1").exists() {
        assert!(
            std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
            "OAG_REQUIRE_GAME_DATA is set but {} is missing",
            path.display()
        );
        return;
    }
    let loaded = race::load(&race::Options {
        no_effects: true,
        source: path.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::Detonator,
        track: Some(r"Data\art\published\DLC1\environments\Vineta_K\track.vex".to_string()),
        ..race::Options::default()
    })
    .expect("loading a 2048 Detonator race on Vineta K");
    assert_auto_speed_and_pads(&loaded, true);
    let mut race = race::Race::start(
        loaded.setup.clone(),
        loaded.view.clone(),
        loaded.modules.clone(),
    );
    let emp = race
        .core
        .sim
        .weapon_stats()
        .and_then(|stats| stats.detonator())
        .and_then(|detonator| detonator.emp)
        .expect("2048's Detonator table has an EMP block");
    let (lo, hi) = (
        emp.min_num_pads_to_collect.expect("a ready count"),
        emp.max_num_pads_to_collect.expect("a full count"),
    );
    assert_eq!((lo, hi), (4.0, 8.0), "the counts 2048 reads");
    start_racing(&mut race);
    for n in [3, 4] {
        race.core.sim.world.detonator.emp.charge = n;
        assert_eq!(
            race.core.sim.detonator_emp(),
            Some((n, n >= 4)),
            "ready at {n}"
        );
    }
    race.core.sim.world.detonator.emp.charge = 0;
    let weapon_pads = loaded.setup.weapon_pads.clone();
    assert_weapon_pads_charge(&mut race, &weapon_pads);
}

/// Outside Detonator the same circuit's speed pad keeps its race boost: the charge is the mode-14
/// arm alone (`FUN_811ccdf4`).
fn assert_a_single_race_speed_pad_still_boosts(plain: &race::Loaded) {
    let mut race = race::Race::start(
        plain.setup.clone(),
        plain.view.clone(),
        plain.modules.clone(),
    );
    start_racing(&mut race);
    let pad = plain
        .setup
        .speedup_pads
        .first()
        .expect("the circuit has speed pads");
    let ship = &mut race.core.sim.world.ships[0].physics;
    ship.body.position = oag_core::math::Vec3::from_array(pad.centre());
    race.tick(&PlayerInputs::none());
    assert!(
        race.core.sim.world.ships[0].physics.pad_timer > 0.0,
        "a single race's speed pad boosts"
    );
    assert_eq!(race.core.sim.world.detonator.emp.charge, 0);
}

/// The same circuit in a single race draws the circuit's own pads; the Detonator race's track
/// model holds fewer triangles (`psp2::build_circuit`'s `hide_pads`).
fn assert_the_circuits_own_pads_are_left_out(
    options: &race::Options,
    loaded: &race::Loaded,
) -> race::Loaded {
    let plain = race::load(&race::Options {
        no_effects: true,
        mode: oag_race::Mode::SingleRace,
        ..options.clone()
    })
    .expect("loading the same circuit in a single race");
    let (with, without) = (
        plain.track_model.indices.len() / 3,
        loaded.track_model.indices.len() / 3,
    );
    assert!(
        with > without,
        "the Detonator race's track holds {without} triangles against {with} in a single race"
    );
    plain
}

/// The Detonator pad on the 2048 lineage: the `_ne` mask rides the third binding and the glow
/// entry is the unsquared-alpha program with the material's own red (`0x7611a2d8`), the squares the
/// original's pad shows; the slab is raised by the depth of its walls so it stands on the road.
fn assert_the_pad_is_red_squares_on_a_slab_that_rests(loaded: &race::Loaded) {
    assert!(
        loaded.report.iter().any(|l| l.contains(
            "detonator: the circuit's own speed and weapon pads are not drawn under the \
             Detonator pad"
        )),
        "the circuit's own pads are dropped under the Detonator pad: {:#?}",
        loaded.report
    );
    let parts = &loaded.bomb_blast_models.detonator;
    let pad = parts.pad.as_ref().expect("the pad model loaded");
    assert!(
        pad.emissive.iter().any(
            |e| e.program[0] == oag_mesh::mesh::PROGRAM_EMISSIVE_ALPHA_MASK
                && (e.tint[0] - 0.5957).abs() < 1e-3
                && e.tint[1] == 0.0
                && e.tint[2] == 0.0
        ),
        "a red unsquared-alpha entry: {:?}",
        pad.emissive
    );
    assert!(
        pad.pad_masks.iter().any(Option::is_some),
        "the _ne mask is bound third"
    );
    let lowest = pad
        .vertices
        .iter()
        .map(|v| v.position[1])
        .fold(f32::MAX, f32::min);
    assert!(lowest < -0.5, "walls lie below the model origin: {lowest}");
    let first = loaded
        .setup
        .speedup_pads
        .first()
        .or(loaded.setup.weapon_pads.first())
        .expect("a pad");
    let plain = oag_core::math::Mat4::from_cols_array(&first.to_world);
    let expected = plain.w_axis.truncate() + plain.y_axis.truncate() * -lowest;
    assert!(
        (parts.pads[0].w_axis.truncate() - expected).length() < 1e-3,
        "raised along the pad's up axis"
    );
}

/// Omega runs the mode's rules and grades an HD-lineage circuit (its default, `tech_de_ra`) the
/// way Zone does: HD/Fury's Detonator table and the `detonatorModeTrack<n>.gnf` set, the loader's
/// fork being the circuit path's (`FUN_0162a390`).
#[test]
#[ignore = "needs a disc image in data/images/"]
fn an_omega_detonator_race_runs_fourteen_stages_on_hd_furys_grade() {
    let Some(path) = oag_testdata::image("data/images/omega-ps4-eu.pkg") else {
        return;
    };
    let options = race::Options {
        no_effects: true,
        source: path.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::Detonator,
        ..race::Options::default()
    };
    let loaded = race::load(&options).expect("loading an Omega Detonator race");
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
            .starts_with("Data/zoneenvironmenthdfury/DetonatorModeHDFury.effectSettings"),
        "graded from {}",
        grade.describe()
    );
    assert_eq!(grade.last_stage(), 14);
    assert!(
        loaded.report.iter().any(|l| l
            .contains("Data/tex/detonatorModeTrack0.gnf..14: 15/15 per-stage Zone track textures")),
        "{:#?}",
        loaded.report
    );
    assert_the_cannon_fires(&loaded);
    assert_the_opening_stage_and_the_stage_cap(&loaded);
    assert_auto_speed_and_pads(&loaded, true);
    let mut race = race::Race::start(
        loaded.setup.clone(),
        loaded.view.clone(),
        loaded.modules.clone(),
    );
    start_racing(&mut race);
    assert_weapon_pads_charge(&mut race, &loaded.setup.weapon_pads);
    assert_the_pad_is_red_squares_on_a_slab_that_rests(&loaded);
    assert_the_circuits_own_pads_are_left_out(&options, &loaded);
}

/// Omega's `Environments2048` circuits take the loader's primary branch (their own
/// `ZoneMode2048` table, unwired for Detonator), so HD/Fury's pair does not grade them.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn an_omega_detonator_race_on_a_2048_circuit_keeps_its_own_colours() {
    let Some(path) = oag_testdata::image("data/images/omega-ps4-eu.pkg") else {
        return;
    };
    let loaded = race::load(&race::Options {
        no_effects: true,
        source: path.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::Detonator,
        track: Some(r"Data\Environments2048\tower\track.vex".to_string()),
        ..race::Options::default()
    })
    .expect("loading an Omega Detonator race on a 2048 circuit");
    assert!(loaded.zone_grade.is_none(), "{:#?}", loaded.report);
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
    let laid = assert_the_opening_stage_and_the_stage_cap(&loaded);
    let mut race = race::Race::start(loaded.setup, loaded.view, loaded.modules);
    race.core.sim.set_autopilot(true);
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
        second.live_mines >= laid / 2,
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

    // The blast, then the 5.5 s the end plays before the race is over.
    for _ in 0..600 {
        if race.core.sim.world.ships[LONE].physics.craft_state == oag_physics::CraftState::Racing
            && let Some(at) = race.core.sim.detonator_mine_position(0)
        {
            race.core.sim.world.ships[LONE].physics.body.position = at;
        }
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
/// with cross never pressed). The ramp is full by GO, the target on lap 1 is `start`, `34`, and
/// it steps with the stage.
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

    // The target steps with the stage: `34 + 4.4 n`, `n = 0` on lap 1 and the HUD stage from
    // each later crossing (`detonator-speed.md`, "`n`: the Detonator stage").
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
fn an_hd_detonator_pad_charges_the_emp_and_gives_no_boost_or_pickup() {
    use oag_gameplay::cue::Cue;
    let Some(loaded) = load() else {
        return;
    };
    let weapon_pads = loaded.setup.weapon_pads.clone();
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

    assert_weapon_pads_charge(&mut race, &weapon_pads);
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
fn emp_from_a_stand(loaded: &race::Loaded, live: bool) -> Option<(u32, i32)> {
    let mut race = race::Race::start(
        loaded.setup.clone(),
        loaded.view.clone(),
        loaded.modules.clone(),
    );
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
    let Some(loaded) = load() else {
        return;
    };
    let (dark, dark_paid) = emp_from_a_stand(&loaded, false).expect("a mine to stand before");
    let (live, live_paid) = emp_from_a_stand(&loaded, true).expect("the same field");
    println!("dark: {dark} cleared for {dark_paid}; live: {live} cleared for {live_paid}");
    assert!(dark > 0, "the wave clears a mine");
    assert_eq!(dark_paid, 25 * dark as i32);
    assert_eq!(live, dark, "the same field, the same wave");
    assert_eq!(live_paid, 50 * live as i32);
}

/// A pad in the person's seat `slot`, with `button` pressed this tick.
fn pad_in(slot: usize, button: oag_gameplay::input::Button) -> PlayerInputs {
    let mut snapshot = oag_gameplay::InputSnapshot::new();
    snapshot.buttons.begin_frame(button.bit());
    let mut inputs = PlayerInputs::none();
    inputs.set(slot, snapshot);
    inputs
}

/// **One slot rule.** With the person in seat 1 and slot 0 an opponent, `primary_slot()` is 1:
/// the cannon (Square) and the EMP's fire (L1) work off that seat's pad and craft, as the pad
/// charge already does.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn an_hd_detonator_cannon_and_emp_work_off_the_primary_slot() {
    use oag_gameplay::Controller;
    const SEAT: usize = 1;
    let Some(loaded) = load() else {
        return;
    };
    let mut race = race::Race::start(loaded.setup, loaded.view, loaded.modules);
    race.core.sim.world.controllers[0] = Controller::Ai;
    race.core.sim.world.controllers[SEAT] = Controller::Local;
    assert_eq!(race.core.sim.world.primary_slot(), SEAT);
    let alive = |race: &mut race::Race| {
        let ship = &mut race.core.sim.world.ships[SEAT];
        ship.physics.shield = ship.handling.dimensions.shield;
    };
    while oag_race::RaceState::thrust_gated(race.core.sim.world.tick) {
        race.tick(&PlayerInputs::none());
        alive(&mut race);
    }

    race.core.sim.world.detonator.emp.charge = 8;
    race.core.sim.world.detonator.emp.gauge = 1.0;
    race.tick(&pad_in(SEAT, oag_gameplay::input::Button::Flip));
    alive(&mut race);
    assert_eq!(
        race.core.sim.detonator_emp(),
        Some((0, false)),
        "L1 in the primary seat fires the EMP"
    );

    let clip = race.core.sim.world.detonator.cannon.clip;
    for _ in 0..10 {
        race.tick(&pad_in(SEAT, oag_gameplay::input::Button::Square));
        alive(&mut race);
    }
    assert!(
        race.core.sim.world.detonator.cannon.clip < clip,
        "Square in the primary seat fires the cannon"
    );
}

/// **A pair carries the original's mines by position.** Four mine objects read out of an HD
/// Detonator race on Talon's Junction (world position, RPCS3 pause 2.7 s after GO):
/// `--detonator-mines` puts each on ours' nearest survey lane, and the lane is where the
/// original's object stood across the section. The original's entry numbers run from the player's
/// start cursor and ours from the spline's origin, so only the positions are compared
/// (`docs/tools/compare-page.md`, "Detonator pairs").
#[test]
#[ignore = "needs a disc image in data/images/"]
fn carried_mines_stand_where_the_originals_objects_stood() {
    const READ: [[f32; 3]; 5] = [
        [217.6, -44.3, -192.3],
        [211.8, -43.3, -199.6],
        [216.9, -46.7, -178.6],
        [210.7, -48.0, -172.0],
        [386.7, -36.2, -110.0],
    ];
    let Some(path) = oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let loaded = race::load(&race::Options {
        no_effects: true,
        source: path.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::Detonator,
        track: Some(r"Data\Environments\Talons_Junction\track.vex".to_string()),
        ..race::Options::default()
    })
    .expect("loading an HD Detonator race on Talon's Junction");
    let mut race = race::Race::start(loaded.setup, loaded.view, loaded.modules);
    let positions: Vec<oag_core::math::Vec3> = READ
        .iter()
        .map(|&p| oag_core::math::Vec3::from_array(p))
        .collect();
    assert!(
        race.core
            .sim
            .carry_detonator_mines(&positions)
            .iter()
            .all(Option::is_some)
    );
    assert_eq!(race.core.sim.world.detonator.live_mines, 5);
    for (index, read) in positions.iter().enumerate() {
        let at = race
            .core
            .sim
            .detonator_mine_position(index)
            .expect("a carried mine");
        // The whole distance, height included: a mine left on its lane point is 3 units off.
        let apart = (at - *read).length();
        println!("ours {at:?}, read {read:?}, {apart:.2} apart");
        assert!(
            apart < 1.0,
            "{read:?} lands {apart:.2} units from the original's"
        );
    }
    // The same position twice (one lane holds one mine) and one at the right place across but 33
    // units too high (a lane on another deck): the twin and the high one are dropped.
    let first = positions[0];
    let high = first + oag_core::math::Vec3::new(0.0, 33.0, 0.0);
    let lanes = race.core.sim.carry_detonator_mines(&[first, first, high]);
    assert_eq!(lanes.iter().filter(|lane| lane.is_none()).count(), 2);
    assert_eq!(race.core.sim.world.detonator.live_mines, 1);
}

/// **Detonator on Pulse**, which ships no such mode (`docs/gameplay/detonator-pulse.md`): the race
/// flies the picked circuit's Zone twin, lays Pulse's own Mine and Bomb from this project's
/// layout, and every speed pad (a Zone circuit authors no weapon pad) is drawn as the race
/// circuit's weapon pad held red, charging the EMP and giving no boost.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_pulse_detonator_race_lays_pulse_mines_on_a_zone_circuit_with_red_charging_pads() {
    let Some(path) = oag_testdata::image("data/images/pulse-psp-usa.chd") else {
        return;
    };
    let loaded = race::load(&race::Options {
        no_effects: true,
        source: path.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::Detonator,
        ..race::Options::default()
    })
    .expect("loading a Pulse Detonator race");
    let report = loaded.report.join("\n");
    assert!(
        report.contains(r"zone: Data\Environments\16_Track\track.vex -> Data\Environments\16_Track\zone_track.vex"),
        "the Zone twin is flown:\n{report}"
    );
    assert!(
        report.contains(r"DetonatorLayouts_Pulse\PatternFile.xml: 6 pattern group(s)"),
        "this project's layout is read:\n{report}"
    );
    assert!(
        report.contains(
            r"DetonatorLayouts_Pulse\WeaponStats_Detonator.xml: 3 weapon(s) with an absorb value"
        ),
        "this project's own Detonator table is read in place of the race table:\n{report}"
    );
    // Its figures are Fury's: a mine costs 10 and neither pushes nor slows beyond the halving,
    // a bomb 20.
    let weapons = loaded.setup.weapons.as_ref().expect("the table");
    let contact = oag_weapons::detonator::Contact::from_stats(weapons).expect("mine and bomb");
    assert_eq!(
        (
            contact.mine.damage,
            contact.mine.blast_force,
            contact.mine.slowdown_time
        ),
        (10.0, 0.0, 0.0)
    );
    assert_eq!(
        (contact.bomb.damage, contact.bomb.blast_force),
        (20.0, 30.0)
    );
    // The Zone hull is flown, and wrecks as the Zone one.
    for entry in [r"\Zone.vex", r"\zonewreck.vex", r"\Zoneboost.vex"] {
        assert!(report.contains(entry), "{entry}:\n{report}");
    }
    // Pulse's own Mine body rides the mine pool: no Detonator object model is named.
    assert!(loaded.mine_model.is_some(), "Pulse_Mine.vex loads");
    assert!(loaded.bomb_blast_models.detonator.pad.is_none());
    // The pads: none of the circuit's own, every speed pad carrying the race circuit's weapon pad.
    assert!(
        loaded.setup.weapon_pads.is_empty(),
        "a Zone circuit authors no weapon pad"
    );
    let speed_pads = loaded.setup.speedup_pads.len();
    assert!(speed_pads > 0);
    assert!(
        report.contains(&format!(
            r"Detonator pads: {speed_pads} pad(s) carry Data\Environments\16_Track\track.vex's weapon pad"
        )),
        "every speed pad carries the weapon pad:\n{report}"
    );
    assert!(loaded.weapon_pad_model.is_none());
    let pads = loaded.pad_model.as_ref().expect("the pad model");
    assert_eq!(pads.node_vertex_ranges.len(), speed_pads);
    assert!(
        pads.vertices
            .iter()
            .all(|v| v.colour[0] > 0.79 && v.colour[1] < 0.41 && v.colour[2] < 0.41),
        "held at the ready cycle's red key"
    );

    let mut race = race::Race::start(
        loaded.setup.clone(),
        loaded.view.clone(),
        loaded.modules.clone(),
    );
    assert_eq!(race.core.sim.world.race[0].laps_target, Some(14));
    race.core.sim.set_autopilot(true);
    race.tick(&PlayerInputs::none());
    let first = race.core.sim.world.detonator;
    assert_eq!(first.stages_laid, 1, "the opening stage is laid");
    println!("stage 1 laid {} mines", first.live_mines);
    assert!(
        (40..=80).contains(&first.live_mines),
        "stage 1 aims for 80: {}",
        first.live_mines
    );
    // Stood on a speed pad: a charge and no boost, ready at Fury's count of 4.
    start_racing(&mut race);
    // Four pads apart from each other: two side by side are one entry.
    let mut apart: Vec<[f32; 3]> = Vec::new();
    for pad in &loaded.setup.speedup_pads {
        let c = pad.centre();
        let far = |o: &[f32; 3]| (0..3).map(|i| (c[i] - o[i]).powi(2)).sum::<f32>() > 50.0 * 50.0;
        if apart.iter().all(far) {
            apart.push(c);
        }
    }
    assert!(apart.len() >= 4, "four pads apart: {apart:?}");
    for (n, &pad) in apart.iter().take(4).enumerate() {
        race.core.sim.set_autopilot(false);
        let ship = &mut race.core.sim.world.ships[0].physics;
        ship.body.position = oag_core::math::Vec3::from_array(pad);
        race.tick(&PlayerInputs::none());
        assert_eq!(
            race.core.sim.world.ships[0].physics.pad_timer, 0.0,
            "no boost"
        );
        println!(
            "pad {n} at {pad:?}: charge {}",
            race.core.sim.world.detonator.emp.charge
        );
        dodge(&mut race);
    }
    assert_eq!(
        race.core.sim.detonator_emp(),
        Some((4, true)),
        "four pads ready the EMP"
    );
    assert!(race.core.sim.l1_is_flip(), "L1 is the EMP");
    // The EMP clears the mines ahead, paid by this project's `<Emp>` (50 a live mine).
    let (live, paid) = emp_from_a_stand(&loaded, true).expect("a mine to stand before");
    assert!(live > 0, "the wave clears a mine");
    assert_eq!(paid, 50 * live as i32);
    // Square spends Pulse's own Cannon clip.
    let mut race = race::Race::start(
        loaded.setup.clone(),
        loaded.view.clone(),
        loaded.modules.clone(),
    );
    start_racing(&mut race);
    let full = race.core.sim.world.detonator.cannon.clip;
    assert_eq!(full, 15, "fifteen volleys");
    for _ in 0..3 {
        race.tick(&square());
        keep_alive(&mut race);
    }
    assert!(
        race.core.sim.world.detonator.cannon.clip < full,
        "a volley left the clip"
    );
}

/// **Pulse's Detonator HUD is a layout of ours arranged as Fury's**: eight EMP pips bottom left, the
/// clip over the shield bar bottom right, and none of Time Trial's speed bar or lap times. The PS2
/// disc reads the copy in its own 640x448 grid.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_pulse_detonator_hud_shows_the_emp_gauge_and_the_clip() {
    for (image, size) in [
        ("data/images/pulse-psp-usa.chd", (480.0, 272.0)),
        ("data/images/pulse-ps2-eu.chd", (640.0, 448.0)),
    ] {
        let Some(path) = oag_testdata::image(image) else {
            return;
        };
        let loaded = race::load(&race::Options {
            source: path.display().to_string(),
            class: "VENOM".to_string(),
            mode: oag_race::Mode::Detonator,
            ..race::Options::default()
        })
        .expect("loading a Pulse Detonator race");
        assert_eq!(loaded.hud.space.size, size, "{image}");
        // Detonator's explosions and its pad charge sound: this build's flat cues load them by
        // name.
        for cue in oag_gameplay::cue::Cue::DETONATOR {
            assert!(
                loaded.view.sounds.pick_at(cue, 0).is_some(),
                "{image}: {cue:?} is loaded"
            );
        }
        let layout = loaded.hud.layout.as_ref().expect("a HUD layout");
        for gone in ["SpeedBarBg", "SpeedBar", "TimeIcon"] {
            assert!(layout.sprite(gone).is_none(), "{image}: {gone}");
        }
        let rect = |name: &str| {
            let sprite = layout
                .sprite(name)
                .unwrap_or_else(|| panic!("{image}: {name}"));
            assert!(
                oag_hud::inside_screen(sprite.rect, size),
                "{image}: {name} {:?}",
                sprite.rect
            );
            sprite.rect
        };
        for index in 0..8 {
            assert!(
                rect(&format!("Emp_{index}"))[0] < size.0 / 2.0,
                "{image}: left"
            );
        }
        let shield = rect("ShieldBarBg");
        for index in 0..15 {
            let bullet = rect(&format!("Bullet{index:02}"));
            assert!(
                bullet[0] > size.0 / 2.0 && bullet[1] + bullet[3] <= shield[1],
                "{image}: the clip sits over the shield bar: {bullet:?} against {shield:?}"
            );
        }
        for name in ["PulseBullets", "DetonatorScorePlain", "ScoreTxt"] {
            assert!(
                layout.labels.iter().any(|l| l.name == name),
                "{image}: {name}"
            );
        }
        let score = layout.labels.iter().find(|l| l.name == "ScoreTxt");
        let mark = rect("EmpReady");
        assert!(
            score.is_some_and(|score| mark[1] + mark[3] <= score.y),
            "{image}: the gauge clears the score"
        );
        let context = loaded.hud.context().expect("the layout parses");
        let drawn = |charge: u32, ready: bool| {
            let readout = oag_hud::Readout {
                detonator: Some(oag_hud::Detonator {
                    emp_charge: charge,
                    emp_ready: ready,
                    ..oag_hud::Detonator::default()
                }),
                ..oag_hud::Readout::default()
            };
            oag_hud::draw_list(&context, &readout).sprites.len()
        };
        assert_eq!(
            drawn(4, true),
            drawn(0, false) + 5,
            "{image}: 4 pips and the mark"
        );
        assert_eq!(drawn(8, true), drawn(0, false) + 9, "{image}: full");
        let clip = |clip: u8| {
            let readout = oag_hud::Readout {
                detonator: Some(oag_hud::Detonator {
                    clip,
                    ..oag_hud::Detonator::default()
                }),
                ..oag_hud::Readout::default()
            };
            oag_hud::draw_list(&context, &readout).sprites.len()
        };
        assert_eq!(clip(15), clip(0) + 15, "{image}: one icon a volley");
    }
}
