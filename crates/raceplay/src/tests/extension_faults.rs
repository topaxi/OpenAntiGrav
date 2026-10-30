//! What a module's fault leaves behind (`docs/architecture/wasm-extensions/hooks.md`,
//! "Faults"): everything it held for the race goes back to the title's, and a fault found
//! while its commands are applied lands none of them that tick.

use std::sync::Arc;

use oag_ext_abi::{Command, Slot};
use oag_tables::weapons::Weapon;
use oag_vex::pads::PadVolume;

use super::extensions::on::{REPLY, STATE, prepared};
use super::extensions::{grid, run};
use crate::extensions::{Declared, Prepared, Scope, Source, Track, TrackProps, prepare};
use oag_gameplay::PlayerInputs;

fn slot(index: usize) -> Slot {
    Slot::new(index).unwrap()
}

/// A module that runs `guard` (WAT, may `return`) and then writes `commands` as its
/// reply. The guard sees the blob's first word.
fn scripted(id: &str, guard: &str, commands: &[Command], publishes: &[&str]) -> Source {
    let mut bytes = (commands.len() as u32).to_le_bytes().to_vec();
    for command in commands {
        bytes.extend(command.encode());
    }
    let data: String = bytes.iter().map(|b| format!("\\{b:02x}")).collect();
    let wasm = wat::parse_str(format!(
        r#"(module
            (memory (export "memory") 1)
            (data $reply "{data}")
            (func (export "oag_abi") (result i32) i32.const {word})
            (func (export "oag_io") (result i32) i32.const 1024)
            (func (export "tick") (result i32)
              {guard}
              (memory.init $reply (i32.const {REPLY}) (i32.const 0) (i32.const {len}))
              i32.const 0))"#,
        word = oag_ext_abi::ABI_WORD,
        len = bytes.len(),
    ))
    .unwrap();
    Source {
        mod_id: id.to_string(),
        module: "test.wasm".to_string(),
        titles: Vec::new(),
        api: 1,
        memory_pages: 1,
        state_bytes: 8,
        scope: Scope::Rules,
        read: Arc::new(move || Ok(wasm.clone())),
        strings: Arc::new(|_| Default::default()),
        icons: Vec::new(),
        effects: Vec::new(),
        files: Arc::new(|_| Err("no files".into())),
        pickup: Some(crate::extension_pickup::a_look()),
        channels: Declared {
            publishes: publishes.iter().map(|s| (*s).to_string()).collect(),
            listens: Vec::new(),
        },
    }
}

/// Sends `commands` on its first call and traps on its second.
fn once_then_traps(id: &str, commands: &[Command]) -> Source {
    scripted(
        id,
        &format!(
            "(if (i32.load (i32.const {STATE})) (then unreachable))
             (i32.store (i32.const {STATE}) (i32.const 1))"
        ),
        commands,
        &[],
    )
}

fn roll_every_turbo() -> Command {
    Command::PickupRoll {
        weapon: oag_ext_abi::Weapon::Turbo,
        every: 1,
    }
}

#[test]
fn a_faulted_owner_hands_a_rolled_pickup_back_as_the_weapon_the_pad_drew_and_frees_the_kind() {
    let giver = Command::GivePickup { slot: slot(1) };
    // Its third call: after the roller has rolled (call 1) and trapped (call 2).
    let late = scripted(
        "test.b",
        &format!(
            "(i32.store (i32.const {STATE}) (i32.add (i32.load (i32.const {STATE})) (i32.const 1)))
             (if (i32.ne (i32.load (i32.const {STATE})) (i32.const 3)) (then (return (i32.const 0))))"
        ),
        &[giver],
        &[],
    );
    let mut race = grid(prepared(&[
        once_then_traps("test.a", &[roll_every_turbo()]),
        late,
    ]));
    race.tick(&PlayerInputs::none());
    // A pad draws a Turbo for slot 0 and the roll turns it into the mod's own pickup.
    let held = &mut race.sim.world.ships[0].pickup;
    held.grant(Weapon::Turbo);
    assert!(race.sim.extensions.rolls(0, Weapon::Turbo));
    race.sim.world.ships[0].pickup.weapon = Some(Weapon::Extension);

    race.tick(&PlayerInputs::none());
    let reports = race.extension_reports();
    assert!(reports[0].faulted.is_some(), "the roller trapped");
    assert_eq!(
        race.sim.world.ships[0].pickup.weapon,
        Some(Weapon::Turbo),
        "the rolled pickup is the vanilla grant it replaced"
    );
    assert!(
        !race.sim.extensions.rolls(1, Weapon::Turbo),
        "pads hand out vanilla weapons again"
    );

    race.tick(&PlayerInputs::none());
    let reports = race.extension_reports();
    assert_eq!(
        reports[1].faulted, None,
        "the dead owner no longer holds the mod pickup kind"
    );
    assert_eq!(
        race.sim.world.ships[1].pickup.weapon,
        Some(Weapon::Extension),
        "the second module gave its own"
    );
}

#[test]
fn a_faulted_owners_given_pickup_is_taken_away() {
    let give = Command::GivePickup { slot: slot(1) };
    let mut race = grid(prepared(&[once_then_traps("test.a", &[give])]));
    race.tick(&PlayerInputs::none());
    assert_eq!(
        race.sim.world.ships[1].pickup.weapon,
        Some(Weapon::Extension)
    );
    race.tick(&PlayerInputs::none());
    assert!(race.extension_reports()[0].faulted.is_some());
    assert_eq!(
        race.sim.world.ships[1].pickup.weapon, None,
        "a given pickup replaced no weapon: nothing to hand back, and nothing nobody can fire"
    );
}

fn with_one_pad(sources: &[Source]) -> Prepared {
    let pad = PadVolume {
        to_world: oag_ext_abi::track::IDENTITY,
        min: [0.0; 3],
        max: [1.0; 3],
        disabled: 0.0,
    };
    let props = TrackProps::default();
    let track = Track {
        name: "test",
        props: &props,
        speedup_pads: &[pad],
        weapon_pads: &[],
        paths: &[],
        routes: &[],
    };
    let mut report = Vec::new();
    let prepared = prepare(sources, "wipeout-pulse", None, track, None, &mut report);
    assert!(report.iter().all(|l| !l.contains("left out")), "{report:?}");
    prepared
}

#[test]
fn a_faulted_modules_held_scale_pad_goes_back_to_one() {
    let dead = Command::ScalePad {
        pad: 0,
        factor: 0.0,
    };
    let mut race = grid(with_one_pad(&[once_then_traps("test.a", &[dead])]));
    race.tick(&PlayerInputs::none());
    assert_eq!(
        race.sim.extensions.pad_factor(1, 0),
        0.0,
        "held while it runs"
    );
    race.tick(&PlayerInputs::none());
    assert!(race.extension_reports()[0].faulted.is_some());
    assert_eq!(
        race.sim.extensions.pad_factor(1, 0),
        1.0,
        "the pad is the title's again"
    );
}

#[test]
fn a_fault_found_while_applying_lands_none_of_the_modules_commands_that_tick() {
    let ping = oag_ext_abi::channel_key("test.e/ping");
    let emit = Command::Emit {
        slot: slot(1),
        channel: ping,
        value: 1.0,
    };
    let around = |emits: usize| {
        let mut commands = vec![Command::ScaleThrust {
            slot: slot(1),
            factor: 1.5,
        }];
        commands.extend(std::iter::repeat_n(emit, emits));
        commands.push(Command::ScaleThrust {
            slot: slot(0),
            factor: 1.5,
        });
        scripted(
            "test.e",
            &format!("(i32.store (i32.const {STATE}) (i32.const 7))"),
            &commands,
            &["test.e/ping"],
        )
    };
    let ticks = 600;
    let (vanilla, _) = run(grid(Prepared::default()), ticks);
    // Within the cap, the thrust commands land and the blob is kept.
    let (fine, _) = run(
        grid(prepared(&[around(oag_ext_abi::MAX_EMITS_PER_TICK)])),
        ticks,
    );
    assert_eq!(fine.extension_reports()[0].faulted, None);
    assert_eq!(fine.extension_state(0).unwrap()[0], 7);
    assert_ne!(fine.sim.world.ships, vanilla.sim.world.ships);
    // One emit past it: the module faults and nothing of it lands, before or after.
    let mut bad = grid(prepared(&[around(oag_ext_abi::MAX_EMITS_PER_TICK + 1)]));
    bad.tick(&PlayerInputs::none());
    let (_, why) = bad.extension_reports()[0].faulted.clone().expect("faulted");
    assert!(why.contains("emit past the cap"), "{why}");
    assert!(
        bad.sim.extensions.fired().is_empty(),
        "no emit of it was delivered or noted"
    );
    assert_eq!(
        bad.extension_state(0).unwrap()[0],
        0,
        "the blob keeps its value from before the call"
    );
    for _ in 1..ticks {
        bad.tick(&PlayerInputs::none());
    }
    assert_eq!(
        bad.sim.world.ships, vanilla.sim.world.ships,
        "no thrust command of the faulted reply landed, so the race is the vanilla one"
    );
}
