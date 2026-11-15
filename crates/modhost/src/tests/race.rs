use super::*;
use oag_ext_abi::{RACE_FLAGS_AT, RACE_MODE_AT, RaceMode};

const IO: i32 = 1024;

/// A guest whose `oag_wants_race` runs `$body`, with `tick` doing nothing.
fn picky(body: &str) -> Vec<u8> {
    wat::parse_str(format!(
        r#"(module
            (memory (export "memory") 1)
            (func (export "oag_abi") (result i32) i32.const {ABI_WORD})
            (func (export "oag_io") (result i32) i32.const {IO})
            (func (export "oag_wants_race") (result i32) {body})
            (func (export "tick") (result i32) i32.const 0))"#
    ))
    .unwrap()
}

/// Runs in a single race with weapons on, as the buffer's mode and flags say.
fn single_race_with_weapons() -> Vec<u8> {
    picky(&format!(
        "(i32.and
            (i32.eq (i32.load8_u (i32.const {mode})) (i32.const {single}))
            (i32.and (i32.load8_u (i32.const {flags})) (i32.const 1)))",
        mode = IO + RACE_MODE_AT as i32,
        flags = IO + RACE_FLAGS_AT as i32,
        single = RaceMode::SingleRace.code(),
    ))
}

fn race(mode: RaceMode, weapons: bool) -> RaceInfo {
    RaceInfo {
        title: 0,
        class: 0,
        mode,
        weapons,
        campaign: false,
        people: 1,
        crafts: 8,
        options: [0; 4],
    }
}

#[test]
fn a_guest_answers_for_the_race_it_is_shown() {
    let module = Module::compile(&single_race_with_weapons(), Limits::default()).unwrap();
    for mode in RaceMode::ALL {
        for weapons in [false, true] {
            let wanted = *mode == RaceMode::SingleRace && weapons;
            assert_eq!(
                module.wants_race(&race(*mode, weapons)),
                Ok(wanted),
                "{mode:?}, weapons {weapons}"
            );
        }
    }
}

#[test]
fn a_guest_with_no_check_runs_in_every_race() {
    let module = Module::compile(&counter("(f32.const 0.25)"), Limits::default()).unwrap();
    for mode in RaceMode::ALL {
        assert_eq!(module.wants_race(&race(*mode, false)), Ok(true));
    }
}

#[test]
fn an_answer_that_is_neither_run_nor_skip_is_a_fault() {
    let module = Module::compile(&picky("i32.const 2"), Limits::default()).unwrap();
    assert!(matches!(
        module.wants_race(&race(RaceMode::SingleRace, true)),
        Err(Fault::Reply(_))
    ));
}

#[test]
fn a_check_that_traps_or_runs_out_of_fuel_is_a_fault() {
    let traps = Module::compile(&picky("unreachable"), Limits::default()).unwrap();
    assert!(matches!(
        traps.wants_race(&race(RaceMode::SingleRace, true)),
        Err(Fault::Trap(_))
    ));
    let spins =
        Module::compile(&picky("(loop $l (br $l)) i32.const 1"), Limits::default()).unwrap();
    assert_eq!(
        spins.wants_race(&race(RaceMode::SingleRace, true)),
        Err(Fault::OutOfFuel)
    );
}

#[test]
fn a_check_with_another_signature_or_kind_is_refused_at_load() {
    let wrong_signature = wat::parse_str(format!(
        r#"(module
            (memory (export "memory") 1)
            (func (export "oag_abi") (result i32) i32.const {ABI_WORD})
            (func (export "oag_io") (result i32) i32.const {IO})
            (func (export "oag_wants_race") (param i32) (result i32) i32.const 1)
            (func (export "tick") (result i32) i32.const 0))"#
    ))
    .unwrap();
    assert!(matches!(
        Module::compile(&wrong_signature, Limits::default()),
        Err(LoadError::Export(why)) if why.contains("oag_wants_race")
    ));
    let wrong_kind = wat::parse_str(format!(
        r#"(module
            (memory (export "memory") 1)
            (global (export "oag_wants_race") i32 (i32.const 1))
            (func (export "oag_abi") (result i32) i32.const {ABI_WORD})
            (func (export "oag_io") (result i32) i32.const {IO})
            (func (export "tick") (result i32) i32.const 0))"#
    ))
    .unwrap();
    assert!(matches!(
        Module::compile(&wrong_kind, Limits::default()),
        Err(LoadError::Export(why)) if why.contains("oag_wants_race")
    ));
}
