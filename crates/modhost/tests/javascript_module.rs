//! Wall Penalty written in JavaScript (Javy's QuickJS under a guest-side shim,
//! `assets/mods/examples/wall-penalty-js/`) against the Rust module
//! (`assets/mods/examples/wall-penalty/`): the same state and the same commands on every
//! observation, and what the JavaScript tick costs against the host's fuel budget.
//!
//! Needs the `assets/mods/examples` submodule (with its LFS objects); without the two
//! committed `.wasm` files the tests return early, so the gate passes where the
//! submodule is not checked out.

use oag_ext_abi::{Event, Slot};
use oag_modhost::{FUEL_PER_TICK, Limits, Module, Observation};

const EXAMPLES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/mods/examples");

fn load(path: &str, fuel: u64) -> Option<Module> {
    let wasm = std::fs::read(format!("{EXAMPLES}/{path}")).ok()?;
    if !wasm.starts_with(b"\0asm") {
        return None;
    }
    let limits = Limits {
        memory_pages: 24,
        state_bytes: 16,
        fuel,
    };
    Some(Module::compile(&wasm, limits).unwrap_or_else(|e| panic!("{path}: {e}")))
}

fn observe<'a>(tick: u64, active: u32, events: &'a [Event]) -> Observation<'a> {
    Observation {
        tick,
        active,
        events,
        crafts: &[],
        course: Default::default(),
        track: &[],
        speed_pads: 0,
        routes: 0,
    }
}

/// Wall contacts on `slots`, each with an event of another kind before it on a slot the
/// JavaScript module also reads, so a module that took any event for a wall contact differs.
fn mixed(slots: &[usize]) -> Vec<Event> {
    let mut events = Vec::new();
    for &s in slots {
        let slot = Slot::new(s).unwrap();
        events.push(Event::SpeedPad { slot, pad: 3 });
        events.push(Event::WallContact { slot });
    }
    events
}

fn walls(slots: &[usize]) -> Vec<Event> {
    slots
        .iter()
        .map(|&s| Event::WallContact {
            slot: Slot::new(s).unwrap(),
        })
        .collect()
}

#[test]
fn the_javascript_module_replies_as_the_rust_module_on_random_observations() {
    let (Some(rust), Some(js)) = (
        load("wall-penalty/ext/wall_penalty.wasm", FUEL_PER_TICK),
        load("wall-penalty-js/ext/wall_penalty_js.wasm", 100_000_000),
    ) else {
        return;
    };
    let mut seed = 0x9E37_79B9_u64;
    let mut next = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    let (mut state_rust, mut state_js) = ([0_u8; 16], [0_u8; 16]);
    let (mut commands_rust, mut commands_js) = (Vec::new(), Vec::new());
    let ticks = std::env::var("OAG_JS_DIFFERENTIAL_TICKS")
        .ok()
        .and_then(|n| n.parse().ok())
        .unwrap_or(3000);
    for tick in 0..ticks {
        let active = (next() & 0xfff) as u32;
        let slots: Vec<usize> = if next() % 4 == 0 {
            (0..next() % 16).map(|_| (next() % 12) as usize).collect()
        } else {
            Vec::new()
        };
        let events = if next() % 2 == 0 {
            walls(&slots)
        } else {
            mixed(&slots)
        };
        let observation = observe(tick, active, &events);
        rust.tick(&observation, &mut state_rust, &mut commands_rust)
            .unwrap();
        js.tick(&observation, &mut state_js, &mut commands_js)
            .unwrap();
        assert_eq!(state_js, state_rust, "state at tick {tick}");
        assert_eq!(commands_js, commands_rust, "commands at tick {tick}");
    }
}

#[test]
fn a_javascript_tick_with_one_cursed_craft_fits_the_hosts_fuel_budget() {
    let Some(js) = load("wall-penalty-js/ext/wall_penalty_js.wasm", FUEL_PER_TICK) else {
        return;
    };
    let mut state = [0_u8; 16];
    let mut commands = Vec::new();
    let empty = js
        .tick(&observe(0, 0b11, &[]), &mut state, &mut commands)
        .expect("a tick with nothing to do fits the budget");
    println!("JavaScript fuel: empty tick {}", empty.fuel_used);
    let events = walls(&[1]);
    let first = js
        .tick(&observe(0, 0b11, &events), &mut state, &mut commands)
        .expect("the first tick of a curse fits the budget");
    let idle = js
        .tick(&observe(1, 0b11, &[]), &mut state, &mut commands)
        .expect("a cursed tick fits the budget");
    println!(
        "JavaScript fuel: contact tick {}, cursed tick {} of {FUEL_PER_TICK}",
        first.fuel_used, idle.fuel_used
    );
    assert!(first.fuel_used < FUEL_PER_TICK && idle.fuel_used < FUEL_PER_TICK);
}

/// The most a tick can cost: all eight slots cursed with a puff due, two commands each. Printed
/// against the budget and held under it: the stress that shows how much room a JavaScript
/// mod has left.
#[test]
fn a_javascript_tick_with_every_slot_cursed_and_puffing_fits_the_hosts_fuel_budget() {
    let Some(js) = load("wall-penalty-js/ext/wall_penalty_js.wasm", 100_000_000) else {
        return;
    };
    let mut state = [0_u8; 16];
    for slot in 0..8 {
        state[slot * 2] = 91;
    }
    let mut commands = Vec::new();
    let worst = js
        .tick(&observe(0, 0xff, &[]), &mut state, &mut commands)
        .expect("the stress tick runs with fuel to spare");
    assert_eq!(commands.len(), 16, "every slot scales thrust and puffs");
    println!(
        "JavaScript fuel: every slot cursed and puffing {} of {FUEL_PER_TICK}",
        worst.fuel_used
    );
    assert!(
        worst.fuel_used < FUEL_PER_TICK,
        "the worst tick faults out of fuel"
    );
}
