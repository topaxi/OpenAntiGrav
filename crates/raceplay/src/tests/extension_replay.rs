//! A modded race records its inputs and names its modules in the replay header
//! (`docs/architecture/wasm-extensions/determinism-and-state.md`, "Replays"), sets no ghost, and
//! is rebuilt with those modules or refused by name. These run without a disc, on the fixture
//! grid and test guests; `tests/energy_turbo_ground_truth.rs` does it on a real circuit.

use oag_gameplay::PlayerInputs;
use oag_replay::{Header, Verifier};

use super::extensions::{grid, grid_seen, on};
use crate::Race;
use crate::extensions::{Prepared, check_recorded, restrict_to};

fn header() -> Header {
    Header::new("pulse", "test", "single_race", "venom", "default", 7)
}

fn run_recorded(mut race: Race, ticks: u32) -> Race {
    race.start_recording(header());
    for _ in 0..ticks {
        race.tick(&PlayerInputs::none());
    }
    race
}

/// Crosses the line once and again: the second crossing completes lap 1, which a vanilla
/// recording keeps as its best lap.
fn two_crossings(mut race: Race) -> Race {
    race.start_recording(header());
    for lap_start in [5_u64, 30] {
        while race.core.sim.world.tick < lap_start {
            race.tick(&PlayerInputs::none());
        }
        let standing = &mut race.core.sim.world.ships[0].standing;
        standing.lap += 1;
        standing.lap_start_tick = Some(race.core.sim.world.tick);
        race.tick(&PlayerInputs::none());
    }
    race
}

#[test]
fn a_vanilla_race_keeps_the_lap_it_drove_as_a_ghost() {
    let mods = vec![oag_replay::ActiveMod::new("a.data", "1.0.0")];
    let view = crate::ViewSetup {
        active_mods: mods.clone(),
        ..crate::tests::fixture_view()
    };
    let mut race = two_crossings(grid_seen(Prepared::default(), view));
    let ghost = race.take_new_best_ghost().expect(
        "the control: the lap crossing yields a ghost, so the modded case below tests the gate",
    );
    assert_eq!(
        ghost.header.active_mods, mods,
        "a ghost file names the mods"
    );
}

#[test]
fn a_modded_race_sets_and_offers_no_ghost() {
    let source = on::module("test.ghost", "i32.const 0");
    let mut race = two_crossings(grid(on::prepared(&[source])));
    assert!(race.core.sim.is_modded() && race.recording());
    assert!(race.take_new_best_ghost().is_none());
    assert!(race.ghost().is_none(), "none raced either");
    assert!(race.recorded().expect("recording").ghost.is_none());
}

#[test]
fn a_modded_race_records_and_names_its_module() {
    let source = on::module("test.names", "i32.const 0");
    let wasm = (source.read)().unwrap();
    let race = run_recorded(grid(on::prepared(&[source])), 120);
    assert!(race.recording());
    let replay = race.recorded().expect("recording");
    assert_eq!(replay.ticks(), 120);
    let [used] = &replay.header.mods[..] else {
        panic!("{:?}", replay.header.mods);
    };
    assert_eq!(used.id, "test.names");
    assert_eq!(used.version, "1.0.0");
    assert_eq!(used.module, "test.wasm");
    assert_eq!(used.sha256.len(), 64);
    assert_eq!(
        used.sha256,
        oag_disc::ps4_pkg::crypto::sha256(&wasm)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    );
    assert_eq!(
        (used.abi, used.revision),
        (oag_ext_abi::ABI, oag_ext_abi::REVISION)
    );
    assert_eq!(used.state_bytes, 8);

    // The entry survives the file, and the file is verified by a fresh modded race.
    let read = oag_replay::Replay::from_bytes(&replay.to_bytes()).expect("reads");
    assert_eq!(read.header.mods, replay.header.mods);
    let mut again = grid(on::prepared(&[on::module("test.names", "i32.const 0")]));
    let found = Verifier::run(&read, again.core.sim.state_hash(), |events, inputs| {
        for event in events {
            again.core.sim.apply_event(event).unwrap();
        }
        again.tick(inputs);
        again.core.sim.state_hash()
    });
    assert!(found.is_ok(), "{found:?}");
}

#[test]
fn a_vanilla_races_header_has_no_mods_table() {
    let race = run_recorded(grid(Prepared::default()), 10);
    let replay = race.recorded().expect("recording");
    assert!(replay.header.mods.is_empty());
    let text = toml::to_string(&replay.header).unwrap();
    assert!(!text.contains("mods"), "{text}");
}

fn used(id: &str, sha: &str) -> oag_replay::header::ModUse {
    oag_replay::header::ModUse {
        id: id.to_string(),
        version: "1.0.0".to_string(),
        module: "test.wasm".to_string(),
        sha256: sha.repeat(64),
        abi: 1,
        revision: oag_ext_abi::REVISION,
        state_bytes: 8,
        picks: vec![0, 0, 0, 0],
    }
}

#[test]
fn the_check_names_what_differs() {
    let a = used("test.a", "a");
    assert_eq!(check_recorded(&[], &[]), Ok(()));
    assert_eq!(
        check_recorded(std::slice::from_ref(&a), std::slice::from_ref(&a)),
        Ok(())
    );

    let missing = check_recorded(std::slice::from_ref(&a), &[]).unwrap_err();
    assert!(
        missing.contains("test.a") && missing.contains("does not load it"),
        "{missing}"
    );

    let build = check_recorded(std::slice::from_ref(&a), &[used("test.a", "b")]).unwrap_err();
    assert!(
        build.contains("test.a") && build.contains("another build"),
        "{build}"
    );

    let mut version = a.clone();
    version.version = "1.1.0".to_string();
    let version = check_recorded(std::slice::from_ref(&a), &[version]).unwrap_err();
    assert!(version.contains("version 1.1.0"), "{version}");

    let mut layout = a.clone();
    layout.state_bytes = 16;
    let layout = check_recorded(std::slice::from_ref(&a), &[layout]).unwrap_err();
    assert!(layout.contains("layout differs"), "{layout}");

    let mut picks = a.clone();
    picks.picks = vec![1, 0, 0, 0];
    let picks = check_recorded(std::slice::from_ref(&a), &[picks]).unwrap_err();
    assert!(picks.contains("picks"), "{picks}");

    let extra = check_recorded(&[], std::slice::from_ref(&a)).unwrap_err();
    assert!(
        extra.contains("test.a") && extra.contains("did not have"),
        "{extra}"
    );

    let b = used("test.b", "c");
    let order = check_recorded(&[a.clone(), b.clone()], &[b, a]).unwrap_err();
    assert!(order.contains("another order"), "{order}");
}

#[test]
fn a_rebuild_runs_the_recorded_modules_with_the_recorded_picks_and_no_others() {
    let keep = on::module("test.keep", "i32.const 0");
    let other = on::module("test.other", "i32.const 0");
    let mut entry = used("test.keep", "a");
    entry.picks = vec![2, 0, 1, 0];
    let kept = restrict_to(vec![keep.clone(), other], &[entry]);
    let [kept] = &kept[..] else {
        panic!("{kept:?}");
    };
    assert_eq!(kept.mod_id, "test.keep");
    assert_eq!(kept.options.sim, [2, 0, 1, 0]);
    assert!(
        restrict_to(vec![keep], &[]).is_empty(),
        "a vanilla recording keeps none"
    );
}

/// The refusal a dropped screenshot gets before anything is loaded: the same differences
/// `check_recorded` names, read off the modules on offer and their bytes with nothing compiled.
/// Fails if a missing module or another build is let through to the load.
#[cfg(feature = "extensions")]
#[test]
fn the_preflight_refuses_a_missing_module_and_another_build_by_name() {
    use crate::Options;
    use crate::extensions::preflight_recorded;

    let source = on::module("test.keep", "i32.const 0");
    let race = grid(on::prepared(std::slice::from_ref(&source)));
    let recorded = race.core.sim.mods_used().to_vec();
    assert_eq!(recorded.len(), 1);
    let offered = |extensions: Vec<crate::extensions::Source>, recorded| Options {
        extensions,
        recorded_mods: Some(recorded),
        ..Options::default()
    };

    let unmodded = Options::default();
    assert_eq!(
        preflight_recorded(&unmodded),
        Ok(()),
        "no recording to hold it to"
    );
    assert_eq!(
        preflight_recorded(&offered(vec![source.clone()], recorded.clone())),
        Ok(()),
        "the recorded module, offered"
    );
    let missing = preflight_recorded(&offered(Vec::new(), recorded.clone())).unwrap_err();
    assert!(
        missing.contains("test.keep") && missing.contains("does not load it"),
        "{missing}"
    );
    let mut other = recorded;
    other[0].sha256 = "0".repeat(64);
    let build = preflight_recorded(&offered(vec![source], other)).unwrap_err();
    assert!(
        build.contains("test.keep") && build.contains("another build"),
        "{build}"
    );
}
