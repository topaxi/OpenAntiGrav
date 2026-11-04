//! A module's picks on its menu rows, by scope: a `sim` pick is part of the race's identity,
//! a `view` pick reaches only what is drawn (`docs/architecture/wasm-extensions/hooks.md`,
//! "Picks: sim and view").

use oag_ext_abi::{Command, Rgba, Slot};
use oag_gameplay::PlayerInputs;

use super::extension_faults::scripted;
use super::extensions::grid;
use super::extensions::on::prepared;
use crate::extensions::Picks;

/// A module that, when its first option is not 0, washes slot 0's view and also scales
/// slot 0's thrust: one drawn output and one command that acts on the race.
fn picky(picks: Picks) -> crate::extensions::Source {
    let mut source = scripted(
        "test.picky",
        &format!(
            "(if (i32.eqz (i32.load8_u (i32.const {option}))) (then (return (i32.const 0))))",
            option = 1024 + oag_ext_abi::OPTIONS_AT,
        ),
        &[
            Command::ScreenFill {
                slot: Slot::new(0).unwrap(),
                rgba: Rgba::new(255, 0, 0, 128),
                shader: None,
            },
            Command::ScaleThrust {
                slot: Slot::new(0).unwrap(),
                factor: 0.5,
            },
        ],
        &[],
    );
    source.options = picks;
    source
}

/// The state hash after `ticks`, and the first tick's wash and thrust scale of slot 0.
fn run(picks: Picks, ticks: usize) -> (u64, Option<[f32; 4]>, f32) {
    let mut race = grid(prepared(&[picky(picks)]));
    race.tick(&PlayerInputs::none());
    let wash = race.screen_wash();
    let scale = race.sim.world.ships[0].pending_thrust_scale;
    for _ in 1..ticks {
        race.tick(&PlayerInputs::none());
    }
    (race.sim.state_hash(), wash, scale)
}

/// Row 0 a `view` row: the simulation call reads its default (0), the view call the pick.
fn view_row(pick: u8) -> Picks {
    Picks {
        sim: [0; 4],
        view: [pick, 0, 0, 0],
        view_rows: 1,
    }
}

#[test]
fn a_sim_pick_changes_the_race_and_its_identity() {
    let (off_hash, off_wash, _) = run(Picks::all_sim([0; 4]), 4);
    let (on_hash, on_wash, _) = run(Picks::all_sim([1, 0, 0, 0]), 4);
    assert_ne!(off_hash, on_hash, "a sim pick is in the state hash");
    assert_eq!((off_wash, on_wash.is_some()), (None, true));
}

#[test]
fn a_view_pick_is_drawn_but_leaves_the_state_hash_and_the_sim_commands_alone() {
    let (base_hash, base_wash, base_scale) = run(view_row(0), 4);
    let (hash, wash, scale) = run(view_row(1), 4);
    assert_eq!(
        hash, base_hash,
        "two devices on different view picks hash the same"
    );
    assert_eq!(scale, base_scale, "the view call's thrust scale is dropped");
    assert_eq!(base_wash, None);
    assert_eq!(
        wash,
        Some([1.0, 0.0, 0.0, 128.0 / 255.0]),
        "the view call's fill is drawn"
    );
}

#[test]
fn a_view_row_reads_its_default_on_the_simulation_call() {
    // A view pick that is the row's own default never calls twice and draws what the
    // simulation call drew.
    let picks = Picks {
        sim: [1, 0, 0, 0],
        view: [1, 0, 0, 0],
        view_rows: 1,
    };
    assert!(!picks.differs());
    let (_, wash, _) = run(picks, 1);
    assert!(wash.is_some());
    assert_eq!(
        picks.identity().count(),
        0,
        "a view row is no part of the identity"
    );
}

#[test]
fn a_view_calls_fault_never_faults_the_module() {
    // Traps whenever its first option is not 0: only the view call reads one.
    let mut source = scripted(
        "test.traps",
        &format!(
            "(if (i32.load8_u (i32.const {option})) (then unreachable))",
            option = 1024 + oag_ext_abi::OPTIONS_AT,
        ),
        &[],
        &[],
    );
    source.options = view_row(1);
    let mut race = grid(prepared(&[source]));
    for _ in 0..3 {
        race.tick(&PlayerInputs::none());
    }
    assert!(race.extension_reports()[0].faulted.is_none());
}
