//! Simulation mods in a race ([`crate::extensions`]): a vanilla race hashes the same
//! with the `extensions` feature on and off, a modded race is deterministic, and a
//! module that runs out of fuel is turned off at the same tick every time.
//!
//! The first test runs in both builds: `cargo nextest run -p oag-raceplay` and the
//! same with `--features extensions` must both pass it against one constant.

use super::*;
use oag_gameplay::PlayerInputs;

/// The fixture grid, with an engine on slot 1 so the race has something to scale.
fn grid(extensions: crate::extensions::Prepared) -> Race {
    let mut setup = setup(hulled_handling());
    setup.mode = Mode::SingleRace;
    setup.extensions = extensions;
    setup.start_position = Some(oag_vex::track::StartPosition {
        position: [0.0, 0.0, 0.0],
        left: [0.0, 0.0, -1.0],
        up: [0.0, 1.0, 0.0],
        forward: [1.0, 0.0, 0.0],
    });
    let mut race = Race::start(setup);
    let engine = &mut race.sim.world.ships[1].handling.engine;
    engine.amount = 20.0;
    engine.accelcap = 1000.0;
    race
}

/// Every tick's state hash over `ticks` ticks, folded into one, and the hashes.
fn run(mut race: Race, ticks: u32) -> (Race, Vec<u64>) {
    let hashes = (0..ticks)
        .map(|_| {
            race.tick(&PlayerInputs::none());
            race.sim.state_hash()
        })
        .collect();
    (race, hashes)
}

fn fold(hashes: &[u64]) -> u64 {
    let mut hasher = oag_core::hash::StateHasher::new();
    for hash in hashes {
        hasher.write_u64(*hash);
    }
    hasher.finish()
}

/// The fixture race's 600 tick hashes, folded. Recorded from the build **without**
/// the `extensions` feature on 2026-10-29; the build with it must reproduce it.
const VANILLA_FOLD: u64 = 0x0898_d286_7629_23f9;

#[test]
fn a_vanilla_race_hashes_the_same_with_the_feature_on_or_off_and_builds_no_engine() {
    let (race, hashes) = run(grid(crate::extensions::Prepared::default()), 600);
    assert!(!race.is_modded());
    assert_eq!(fold(&hashes), VANILLA_FOLD, "{:#018x}", fold(&hashes));
    #[cfg(feature = "extensions")]
    assert_eq!(
        oag_modhost::engines_created(),
        0,
        "a vanilla race built a wasm engine"
    );
}

#[cfg(feature = "extensions")]
mod on {
    use std::sync::Arc;

    use super::*;
    use crate::extensions::{Prepared, Source, prepare};

    const STATE: i32 = 1024 + oag_modhost::abi::STATE_AT as i32;
    const REPLY: i32 = 1024 + oag_modhost::abi::REPLY_AT as i32;

    fn module(id: &str, body: &str) -> Source {
        let wasm = wat::parse_str(format!(
            r#"(module
                (memory (export "memory") 1)
                (func (export "oag_abi") (result i32) i32.const 1)
                (func (export "oag_io") (result i32) i32.const 1024)
                (func (export "tick") (result i32) {body}))"#
        ))
        .unwrap();
        Source {
            mod_id: id.to_string(),
            module: "test.wasm".to_string(),
            titles: Vec::new(),
            api: 1,
            memory_pages: 1,
            state_bytes: 8,
            read: Arc::new(move || Ok(wasm.clone())),
        }
    }

    /// Asks for slot 1's thrust at `factor` every tick.
    fn scaler(id: &str, factor: f32) -> Source {
        module(
            id,
            &format!(
                "(i32.store (i32.const {REPLY}) (i32.const 1))
                 (i32.store8 (i32.const {cmd}) (i32.const 1))
                 (i32.store8 (i32.const {slot}) (i32.const 1))
                 (f32.store (i32.const {f}) (f32.const {factor}))
                 i32.const 0",
                cmd = REPLY + 4,
                slot = REPLY + 5,
                f = REPLY + 8,
            ),
        )
    }

    #[test]
    fn a_factor_above_one_makes_a_craft_faster_and_one_changes_nothing() {
        let speed = |race: &Race| {
            race.sim.world.ships[1]
                .physics
                .body
                .linear_velocity
                .length()
        };
        let (vanilla, plain) = run(grid(Prepared::default()), 600);
        let (boosted, _) = run(grid(prepared(&[scaler("test.boost", 1.5)])), 600);
        assert!(
            speed(&boosted) > speed(&vanilla),
            "{} against {}",
            speed(&boosted),
            speed(&vanilla)
        );
        // A factor of exactly 1.0 is no command at all for the craft: its whole
        // trajectory is the vanilla one (the race hash differs only by the module's
        // own state).
        let (neutral, _) = run(grid(prepared(&[scaler("test.neutral", 1.0)])), 600);
        assert_eq!(neutral.sim.world.ships, vanilla.sim.world.ships);
        assert_eq!(fold(&plain), VANILLA_FOLD);
    }

    const CRAFTS: i32 = 1024 + oag_modhost::abi::CRAFTS_AT as i32;

    /// On its first call takes slot 1's whole pool (`-shield_max`, read from its craft
    /// record), then nothing.
    fn drainer() -> Source {
        module(
            "test.drainer",
            &format!(
                "(if (i32.eqz (i32.load (i32.const {STATE})))
                   (then
                     (i32.store (i32.const {STATE}) (i32.const 1))
                     (i32.store (i32.const {REPLY}) (i32.const 1))
                     (i32.store8 (i32.const {cmd}) (i32.const 2))
                     (i32.store8 (i32.const {slot}) (i32.const 1))
                     (f32.store (i32.const {f}) (f32.neg (f32.load (i32.const {max}))))))
                 i32.const 0",
                cmd = REPLY + 4,
                slot = REPLY + 5,
                f = REPLY + 8,
                max = CRAFTS + 16 + 8,
            ),
        )
    }

    #[test]
    fn a_shield_delta_that_empties_the_pool_wrecks_the_craft_as_a_hit_does() {
        let mut race = grid(prepared(&[drainer()]));
        // The fixture's hulls carry no pool; give slot 1 a full one.
        race.sim.world.ships[1].handling.dimensions.shield = 100.0;
        race.sim.world.ships[1].physics.shield = 100.0;
        assert_eq!(
            race.sim.world.ships[1].physics.craft_state,
            oag_physics::damage::CraftState::Racing
        );
        race.tick(&PlayerInputs::none());
        let ship = &race.sim.world.ships[1];
        assert_eq!(ship.physics.shield, 0.0);
        assert_eq!(
            ship.physics.craft_state,
            oag_physics::damage::CraftState::Destroyed
        );
        assert_eq!(race.extension_reports()[0].faulted, None);
    }

    /// Counts its calls in the blob and halves slot 1's thrust every tick.
    fn halver() -> Source {
        module(
            "test.halver",
            &format!(
                "(i32.store (i32.const {STATE}) (i32.add (i32.load (i32.const {STATE})) (i32.const 1)))
                 (i32.store (i32.const {REPLY}) (i32.const 1))
                 (i32.store8 (i32.const {cmd}) (i32.const 1))
                 (i32.store8 (i32.const {slot}) (i32.const 1))
                 (f32.store (i32.const {f}) (f32.const 0.5))
                 i32.const 0",
                cmd = REPLY + 4,
                slot = REPLY + 5,
                f = REPLY + 8,
            ),
        )
    }

    /// Runs normally for 100 calls, then loops forever.
    fn runaway() -> Source {
        module(
            "test.runaway",
            &format!(
                "(i32.store (i32.const {STATE}) (i32.add (i32.load (i32.const {STATE})) (i32.const 1)))
                 (if (i32.gt_u (i32.load (i32.const {STATE})) (i32.const 100))
                     (then (loop $spin (br $spin))))
                 i32.const 0"
            ),
        )
    }

    fn prepared(sources: &[Source]) -> Prepared {
        let mut report = Vec::new();
        let prepared = prepare(sources, "wipeout-pulse", &mut report);
        assert!(
            report.iter().all(|line| !line.contains("left out")),
            "{report:?}"
        );
        prepared
    }

    #[test]
    fn a_modded_race_run_twice_hashes_the_same_every_tick_and_the_command_lands() {
        let (first, a) = run(grid(prepared(&[halver()])), 600);
        let (_, b) = run(grid(prepared(&[halver()])), 600);
        assert!(first.is_modded());
        assert_eq!(a, b);
        assert_ne!(fold(&a), VANILLA_FOLD, "the blob is in the hash");
        let (vanilla, _) = run(grid(Prepared::default()), 600);
        let speed = |race: &Race| {
            race.sim.world.ships[1]
                .physics
                .body
                .linear_velocity
                .length()
        };
        assert!(
            speed(&first) < speed(&vanilla),
            "halved thrust {} against {}",
            speed(&first),
            speed(&vanilla)
        );
        assert_eq!(oag_modhost::engines_created(), 1);
        let reports = first.extension_reports();
        assert_eq!(reports.len(), 1);
        assert!(reports[0].fuel_used > 0 && reports[0].faulted.is_none());
    }

    #[test]
    fn a_runaway_module_faults_at_the_same_tick_every_time_and_the_race_goes_on() {
        let (first, a) = run(grid(prepared(&[runaway(), halver()])), 600);
        let (second, b) = run(grid(prepared(&[runaway(), halver()])), 600);
        assert_eq!(a, b);
        let reports = first.extension_reports();
        // Id order: `test.halver` runs before `test.runaway`.
        assert_eq!(reports[0].mod_id, "test.halver");
        assert!(
            reports[0].faulted.is_none(),
            "the other module keeps running"
        );
        let (tick, why) = reports[1].faulted.clone().expect("the runaway faulted");
        assert_eq!(why, "out of fuel");
        assert_eq!(
            second.extension_reports()[1].faulted.as_ref().unwrap().0,
            tick
        );
        assert!(
            first.sim.world.tick > tick + 400,
            "the race went on past the fault"
        );
    }
}
