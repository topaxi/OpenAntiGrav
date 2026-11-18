//! A module that exports `oag_wants_race` is asked once, before the race is built, and a
//! module that declines is not loaded for that race at all: not ticked, no state, not in the
//! report as running, and the race is not modded on its account.

use std::sync::Arc;

use super::extensions::on::{module, prepared};
use super::extensions::{grid, run};
use crate::extensions::{Source, admit};
use oag_ext_abi::{
    RACE_CAMPAIGN, RACE_CLASS_AT, RACE_CRAFTS_AT, RACE_FLAGS_AT, RACE_MODE_AT, RACE_PEOPLE_AT,
    RACE_TITLE_AT, RACE_WEAPONS, RaceMode, channel_key,
};

const IO: i32 = 1024;

/// `source` with a module that answers `body` to `oag_wants_race` and traps if it is ever
/// ticked, so a tick of a module that declined shows as a fault.
fn picky(id: &str, body: &str) -> Source {
    guest(id, body, "unreachable")
}

/// A module answering `check` to `oag_wants_race` whose tick runs `tick`.
fn guest(id: &str, check: &str, tick: &str) -> Source {
    let wasm = wat::parse_str(format!(
        r#"(module
            (memory (export "memory") 1)
            (func (export "oag_abi") (result i32) i32.const {word})
            (func (export "oag_io") (result i32) i32.const {IO})
            (func (export "oag_wants_race") (result i32) {check})
            (func (export "tick") (result i32) {tick}))"#,
        word = oag_ext_abi::ABI_WORD,
    ))
    .unwrap();
    let mut source = module(id, "i32.const 0");
    source.read = Arc::new(move || Ok(wasm.clone()));
    source
}

/// Runs iff the `width` byte little-endian word at `at` of the description is `value`.
fn runs_if(id: &str, at: usize, width: u8, value: u32) -> Source {
    let load = if width == 1 { "load8_u" } else { "load" };
    picky(
        id,
        &format!(
            "(i32.eq (i32.{load} (i32.const {})) (i32.const {}))",
            IO as usize + at,
            value as i32
        ),
    )
}

fn options(mode: oag_race::Mode, weapons: bool) -> crate::Options {
    crate::Options {
        mode,
        weapons_override: Some(weapons),
        ..crate::Options::default()
    }
}

fn kept(sources: Vec<Source>, options: &crate::Options) -> Vec<String> {
    let mut report = Vec::new();
    admit(sources, "wipeout-pulse", None, options, &mut report)
        .into_iter()
        .map(|s| s.mod_id)
        .collect()
}

#[test]
fn a_module_that_declines_is_dropped_and_one_that_accepts_or_has_no_check_stays() {
    let options = options(oag_race::Mode::SingleRace, true);
    let mut report = Vec::new();
    let sources = vec![
        picky("test.declines", "i32.const 0"),
        picky("test.accepts", "i32.const 1"),
        module("test.no-check", "i32.const 0"),
    ];
    let left = admit(sources, "wipeout-pulse", None, &options, &mut report);
    let ids: Vec<&str> = left.iter().map(|s| s.mod_id.as_str()).collect();
    assert_eq!(ids, ["test.accepts", "test.no-check"]);
    assert!(
        report
            .iter()
            .any(|l| l.contains("test.declines") && l.contains("not run in this race")),
        "{report:?}"
    );
}

#[test]
fn a_decliner_is_not_ticked_costs_nothing_and_leaves_the_race_unmodded_and_vanilla() {
    let options = options(oag_race::Mode::SingleRace, true);
    let mut report = Vec::new();
    let left = admit(
        vec![picky("test.declines", "i32.const 0")],
        "wipeout-pulse",
        None,
        &options,
        &mut report,
    );
    assert!(left.is_empty());
    let (race, hashes) = run(grid(prepared(&left)), 600);
    let (vanilla, plain) = run(grid(crate::extensions::Prepared::default()), 600);
    assert_eq!(hashes, plain, "the race hashes as a race with no mod");
    assert!(!race.core.sim.is_modded());
    assert!(race.core.sim.extension_reports().is_empty());
    assert_eq!(race.core.sim.world.ships, vanilla.core.sim.world.ships);
}

#[test]
fn the_same_module_unfiltered_would_have_been_ticked_and_faulted() {
    // The control for the test above: its guest traps on its first tick, so a race that
    // loads it shows a fault in the report and is modded.
    let (race, _) = run(grid(prepared(&[picky("test.declines", "i32.const 0")])), 5);
    assert!(race.core.sim.is_modded());
    assert!(race.core.sim.extension_reports()[0].faulted.is_some());
}

#[test]
fn a_module_that_accepts_runs_as_it_always_did() {
    let options = options(oag_race::Mode::SingleRace, true);
    let mut report = Vec::new();
    let accepts = guest("test.accepts", "i32.const 1", "i32.const 0");
    let left = admit(vec![accepts], "wipeout-pulse", None, &options, &mut report);
    assert_eq!(left.len(), 1);
    let (race, _) = run(grid(prepared(&left)), 5);
    assert!(race.core.sim.is_modded());
    assert_eq!(race.core.sim.extension_reports()[0].faulted, None);
}

#[test]
fn a_check_that_traps_or_answers_neither_run_nor_skip_leaves_the_module_out_by_name() {
    let options = options(oag_race::Mode::SingleRace, true);
    for body in [
        "unreachable",
        "i32.const 2",
        "(loop $l (br $l)) i32.const 1",
    ] {
        let mut report = Vec::new();
        let left = admit(
            vec![picky("test.broken", body)],
            "wipeout-pulse",
            None,
            &options,
            &mut report,
        );
        assert!(left.is_empty(), "{body}");
        assert!(
            report.iter().any(|l| l.contains("test.broken")
                && l.contains("left out")
                && l.contains("race check")),
            "{body}: {report:?}"
        );
    }
}

#[test]
fn a_module_is_told_the_mode_and_whether_weapons_are_on() {
    use oag_race::Mode;
    let modes = [
        (Mode::TimeTrial, RaceMode::TimeTrial),
        (Mode::SpeedLap, RaceMode::SpeedLap),
        (Mode::Zone, RaceMode::Zone),
        (Mode::SingleRace, RaceMode::SingleRace),
        (Mode::Eliminator, RaceMode::Eliminator),
        (Mode::Tournament, RaceMode::Tournament),
        (Mode::Head2Head, RaceMode::Head2Head),
        (Mode::Detonator, RaceMode::Detonator),
    ];
    for (mode, coded) in modes {
        for weapons in [false, true] {
            let options = options(mode, weapons);
            let mode_is = runs_if("test.mode", RACE_MODE_AT, 1, u32::from(coded.code()));
            assert_eq!(kept(vec![mode_is], &options).len(), 1, "{mode:?} mode byte");
            let weapons_on = runs_if("test.weapons", RACE_FLAGS_AT, 1, u32::from(RACE_WEAPONS));
            assert_eq!(
                kept(vec![weapons_on], &options).len(),
                usize::from(weapons),
                "{mode:?} weapons {weapons}"
            );
        }
    }
}

#[test]
fn a_module_is_told_the_people_the_craft_the_class_the_title_and_the_campaign() {
    use oag_race::Mode;
    let crafts = |mode, people: u8, no_driver: bool| {
        let options = crate::Options {
            people,
            no_driver_for_class: no_driver,
            ..options(mode, true)
        };
        (1..=16_u32)
            .filter(|&n| {
                !kept(vec![runs_if("test.crafts", RACE_CRAFTS_AT, 1, n)], &options).is_empty()
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(crafts(Mode::SingleRace, 1, false), [8]);
    assert_eq!(crafts(Mode::TimeTrial, 1, false), [1]);
    assert_eq!(
        crafts(Mode::TimeTrial, 2, false),
        [2],
        "a multiplayer time trial"
    );
    assert_eq!(crafts(Mode::Head2Head, 1, false), [2]);
    assert_eq!(
        crafts(Mode::SingleRace, 1, true),
        [1],
        "a class with no driver"
    );

    let people = crate::Options {
        people: 2,
        ..options(Mode::SingleRace, true)
    };
    assert_eq!(
        kept(vec![runs_if("test.people", RACE_PEOPLE_AT, 1, 2)], &people).len(),
        1
    );

    let classed = crate::Options {
        class: "Venom".to_string(),
        ..options(Mode::SingleRace, true)
    };
    assert_eq!(
        kept(
            vec![runs_if(
                "test.class",
                RACE_CLASS_AT,
                4,
                channel_key("venom")
            )],
            &classed
        )
        .len(),
        1
    );
    assert_eq!(
        kept(
            vec![runs_if(
                "test.title",
                RACE_TITLE_AT,
                4,
                channel_key("wipeout-pulse")
            )],
            &classed
        )
        .len(),
        1
    );

    for campaign in [false, true] {
        let options = crate::Options {
            campaign,
            ..options(Mode::SingleRace, true)
        };
        let source = runs_if(
            "test.campaign",
            RACE_FLAGS_AT,
            1,
            u32::from(RACE_WEAPONS | RACE_CAMPAIGN),
        );
        assert_eq!(kept(vec![source], &options).len(), usize::from(campaign));
    }
}

#[test]
fn a_module_for_another_title_is_not_asked() {
    let options = options(oag_race::Mode::SingleRace, true);
    let mut other = picky("test.other", "unreachable");
    other.titles = vec!["wipeout-hd".to_string()];
    let mut report = Vec::new();
    let left = admit(vec![other], "wipeout-pulse", None, &options, &mut report);
    assert!(left.is_empty());
    assert!(report.is_empty(), "{report:?}");
}
