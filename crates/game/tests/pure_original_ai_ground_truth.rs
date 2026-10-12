//! Wipeout Pure's original opponent throttle (`oag_ai::original::cruise_thrust`
//! and its neighbours) on Pure's own tables, read off the disc.
//!
//! The law was reproduced on 1,688 of 1,689 PPSSPP samples
//! (`docs/ghidra/functions/psp-pure-usa/ai-controls.md`, "Measured"): Single
//! Race, Vineta K, Venom and Vector, the player parked. Nothing flies it yet -
//! this project has no Pure campaign - so what is asserted here is that the
//! disc's tables, read the way Pure's executable reads them, give the law the
//! shapes and the figures the capture logged:
//!
//! - every class's legacy block and `<Controller>` read (the `<NewStats>` half and
//!   `<ControllerPO>` skipped by name, as the executable never selects them);
//! - the launch holds each slot at its `StartSpeed`, the cruise lands inside its
//!   slot's band times the track's tweak, the zone multiplier holds its curve's
//!   ends;
//! - the figures the live race manager held: Vineta K's tweak ranges and the
//!   legacy `GridSpread` (which the `<NewStats>` half would have changed).
//!
//! One test a class, so the matrix is the test axis.
//!
//! Then the field: the sibling driver (`oag_ai::original`'s cruise throttle and
//! ringless planner, `oag_pure::race::ORIGINAL_DRIVER`) flown by a race the way
//! a Pure campaign event flies it, on two circuits, one test each.

use oag_ai::original::{CruiseDraws, CruiseState, cruise_thrust, zone_multiplier};
use oag_core::Rng;
use oag_gameplay::PlayerInputs;
use oag_tables::pure_ai_stats::{self, PureClass, PureClassRace};

const VINETA_STATS: &str = r"Data\Environments\01_Vineta_K\stats.xml";
const VINETA_TRACK: &str = r"Data\Environments\01_Vineta_K\track.vex";

struct Tables {
    race: PureClassRace,
    tweak: (f32, f32),
}

fn tables(class: PureClass) -> Option<Tables> {
    let image = oag_testdata::image("pure-psp-usa.chd")?;
    let mut archives = oag_pure::open(&image.display().to_string()).expect("opening Pure");
    let race_blob = archives
        .read_name(pure_ai_stats::race_entry_name())
        .expect("AIRaceStats.xml");
    let control_blob = archives
        .read_name(pure_ai_stats::control_entry_name())
        .expect("AIControlStats.xml");
    let stats_blob = archives
        .read_name(VINETA_STATS)
        .expect("Vineta K's stats.xml");
    let controller =
        pure_ai_stats::controller_from_blob(&control_blob, class).expect("<Controller>");
    assert!(
        controller.steer_mul.is_some() && controller.look_ahead_secs.is_some(),
        "{class:?}'s <Controller> authors the gains the steering reads"
    );
    Some(Tables {
        race: pure_ai_stats::race_from_blob(&race_blob, class).expect("the legacy block"),
        tweak: pure_ai_stats::tweak_from_blob(&stats_blob, class).expect("the tweak"),
    })
}

fn class_holds_the_laws_shapes(class: PureClass) {
    let Some(Tables { race, tweak }) = tables(class) else {
        return;
    };
    let draws = CruiseDraws::draw(&race, tweak, &mut Rng::new(1));
    let (lo, hi) = (tweak.0.min(tweak.1), tweak.0.max(tweak.1));
    for slot in 1..=8u8 {
        let k = usize::from(slot) - 1;
        assert_eq!(
            cruise_thrust(&race, &draws, slot, 1.0, 1.0),
            race.start_speed[k],
            "{class:?} slot {slot}: the launch holds StartSpeed"
        );
        let cruise = cruise_thrust(&race, &draws, slot, 1.0, 0.0);
        assert!(
            race.cruise_min[k] * lo - 1e-3 <= cruise && cruise <= race.cruise_max[k] * hi + 1e-3,
            "{class:?} slot {slot}: cruise {cruise} outside its band times the tweak"
        );
    }
    for curve in [&race.zone, &race.lead_zone] {
        assert!(
            curve.position.windows(2).all(|w| w[0] < w[1]),
            "{class:?}: a zone curve's Position rises point to point"
        );
    }
    let lap = 10_000.0;
    let far_behind = [-2_000.0];
    let far_ahead = [2_000.0];
    assert_eq!(
        zone_multiplier(&race, 0.0, false, far_behind, lap),
        race.zone.speed[0]
    );
    assert_eq!(
        zone_multiplier(&race, 0.0, false, far_ahead, lap),
        race.zone.speed[5]
    );
    assert_eq!(
        zone_multiplier(&race, 0.0, true, far_behind, lap),
        race.lead_zone.speed[0]
    );
    // Fatigue as shipped: the window fills and the factor is the multiplier the
    // file authors, applied to the zone value.
    let mut state = CruiseState::default();
    let mut out = 0.0;
    for _ in 0..3_600 {
        out = state.update(&race.fatigue, false, 0.0, 30.0, 0.5);
    }
    assert_eq!(out, 0.5 * race.fatigue.multiplier);
}

#[test]
#[ignore = "needs data/images/pure-psp-usa.chd"]
fn vector_holds_the_laws_shapes() {
    class_holds_the_laws_shapes(PureClass::Vector);
}

#[test]
#[ignore = "needs data/images/pure-psp-usa.chd"]
fn venom_holds_the_laws_shapes() {
    class_holds_the_laws_shapes(PureClass::Venom);
}

#[test]
#[ignore = "needs data/images/pure-psp-usa.chd"]
fn flash_holds_the_laws_shapes() {
    class_holds_the_laws_shapes(PureClass::Flash);
}

#[test]
#[ignore = "needs data/images/pure-psp-usa.chd"]
fn rapier_holds_the_laws_shapes() {
    class_holds_the_laws_shapes(PureClass::Rapier);
}

#[test]
#[ignore = "needs data/images/pure-psp-usa.chd"]
fn phantom_holds_the_laws_shapes() {
    class_holds_the_laws_shapes(PureClass::Phantom);
}

/// What the live race manager held on Vineta K (the record words and the
/// settings object read in the capture): the tweak range per class and the
/// legacy `GridSpread`, where `<NewStats>` authors a different one.
#[test]
#[ignore = "needs data/images/pure-psp-usa.chd"]
fn the_tables_read_what_the_live_race_held() {
    let Some(venom) = tables(PureClass::Venom) else {
        return;
    };
    let vector = tables(PureClass::Vector).expect("the same disc");
    assert_eq!(venom.tweak, (0.805, 0.795), "Venom's tweak, read live");
    assert_eq!(vector.tweak, (0.805, 0.795), "Vector's tweak, read live");
    assert_eq!(venom.race.grid_spread, 650.0, "Venom's live R+0x18");
    assert_eq!(vector.race.grid_spread, 450.0, "Vector's live R+0x18");
    // Every cruise speed the three races drew (`R+0x68`, logged per capture)
    // sits inside the disc's own band for its slot: redrawn each race, never
    // outside it.
    let drawn: [(&Tables, [u32; 8]); 3] = [
        (
            &venom,
            [
                0x42b4_11d9,
                0x42ae_eba6,
                0x42a2_cb1f,
                0x429a_0525,
                0x4297_c475,
                0x428d_a557,
                0x428d_b97a,
                0x42c8_0000,
            ],
        ),
        (
            &venom,
            [
                0x42b1_5c5a,
                0x42b1_5895,
                0x42a3_cb06,
                0x429a_2c6e,
                0x4299_639c,
                0x428e_d211,
                0x428c_2dde,
                0x42c8_0000,
            ],
        ),
        (
            &vector,
            [
                0x42a5_2c6f,
                0x42a4_23ec,
                0x42a3_f133,
                0x429e_9362,
                0x4299_b4cf,
                0x4291_3f1e,
                0x428a_004a,
                0x42c8_0000,
            ],
        ),
    ];
    for (tables, bits) in drawn {
        for (k, b) in bits.iter().enumerate() {
            let v = f32::from_bits(*b);
            assert!(
                tables.race.cruise_min[k] <= v && v <= tables.race.cruise_max[k],
                "slot {}: drawn {v} outside [{}, {}]",
                k + 1,
                tables.race.cruise_min[k],
                tables.race.cruise_max[k]
            );
        }
    }
}

fn seconds(race: &oag_raceplay::Race) -> f32 {
    race.sim
        .world
        .tick
        .saturating_sub(oag_race::COUNTDOWN_TICKS) as f32
        / 60.0
}

/// A full Pure grid with the player parked, its opponents on the original
/// driver as a campaign event switches it on: from the green light every
/// opponent holds its grid slot's `StartSpeed` (the launch blend, which only
/// the cruise throttle has), and every opponent finishes inside 300 s with no
/// respawn. Shield is read at each opponent's lap crossings (per lap) and at
/// the end, reported apart.
fn fly(track: &str, class: &str, start: PureClass) {
    let Some(image) = oag_testdata::image("pure-psp-usa.chd") else {
        return;
    };
    let loaded = oag_raceplay::load(&oag_raceplay::Options {
        source: image.display().to_string(),
        class: class.to_string(),
        mode: oag_race::Mode::SingleRace,
        track: Some(track.to_string()),
        seed: Some(1),
        ..oag_raceplay::Options::default()
    })
    .expect("loading the circuit");
    let race_table = tables(start).expect("the same disc").race;
    let mut race = oag_raceplay::Race::start(loaded.setup);
    assert!(
        race.use_original_ai(None, 1),
        "Pure flies its original driver"
    );
    let mut lap = [1u32; 8];
    let mut per_lap: [Vec<f32>; 8] = Default::default();
    let mut launch_checked = false;
    while seconds(&race) <= 300.0 && !(1..8).all(|s| race.sim.world.ships[s].standing.finished()) {
        race.tick(&PlayerInputs::none());
        if !launch_checked && seconds(&race) > 0.1 {
            launch_checked = true;
            let field = race.original_ai_field().expect("the field is on");
            for slot in 1..8 {
                let driver = &field.drivers[slot];
                assert_eq!(
                    driver.held_thrust,
                    race_table.start_speed[usize::from(driver.grid) - 1],
                    "{track}: slot {slot} (grid {}) launches at its StartSpeed",
                    driver.grid
                );
            }
        }
        for slot in 1..8 {
            let ship = &race.sim.world.ships[slot];
            if ship.standing.lap != lap[slot] {
                lap[slot] = ship.standing.lap;
                per_lap[slot].push(ship.physics.shield);
            }
        }
    }
    assert!(launch_checked);
    println!("{track} {class}: {:.1} s", seconds(&race));
    for (slot, shields) in per_lap.iter().enumerate().skip(1) {
        let ship = &race.sim.world.ships[slot];
        println!(
            "  slot {slot}: finished {} lap {} respawns {} per-lap shield {shields:?}, \
             end-of-run shield {:.1}",
            ship.standing.finished(),
            ship.standing.lap,
            race.respawns_of(slot),
            ship.physics.shield
        );
    }
    for slot in 1..8 {
        assert!(
            race.sim.world.ships[slot].standing.finished(),
            "{track}: slot {slot} did not finish"
        );
        assert_eq!(race.respawns_of(slot), 0, "{track}: slot {slot} respawned");
    }
}

#[test]
#[ignore = "needs data/images/pure-psp-usa.chd"]
fn the_field_finishes_vineta_k() {
    fly(VINETA_TRACK, "Venom", PureClass::Venom);
}

#[test]
#[ignore = "needs data/images/pure-psp-usa.chd"]
fn the_field_finishes_chenghou_project() {
    fly(
        r"Data\Environments\04_Chenghou_Project\track.vex",
        "Venom",
        PureClass::Venom,
    );
}
