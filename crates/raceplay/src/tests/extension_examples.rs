//! The committed example modules (`assets/mods/examples/*/ext/*.wasm`) load on this
//! host, so a module built against another layout revision fails here, and stay inside
//! their fuel on a full grid.
//!
//! The examples are a submodule; a checkout without it skips with a line saying so.

use oag_ext_abi::{Craft, Event, Pickup, Slot};
use oag_modhost::{Limits, Module, Observation};

const EXAMPLES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/mods/examples");

/// Each example's module, with the limits its `oag-mod.toml` declares; `None` only
/// when the submodule is not checked out.
fn example(dir: &str) -> Option<Module> {
    let root = std::path::Path::new(EXAMPLES).join(dir);
    if !root.join("oag-mod.toml").is_file() {
        return None;
    }
    let module = oag_mod::Mod::open(&root).unwrap_or_else(|e| panic!("{dir}: {e}"));
    let extension = module.manifest.extension.clone().expect("[extension]");
    let name = extension
        .rules
        .map(|rules| rules.module)
        .or(extension.track.map(|track| track.module))
        .expect("a rules or a track module");
    let wasm = module
        .read(&name)
        .unwrap()
        .expect("the committed module is in the mod");
    let limits = Limits {
        memory_pages: extension.memory_pages,
        state_bytes: extension.state_bytes,
        fuel: oag_modhost::FUEL_PER_TICK,
    };
    Some(Module::compile(&wasm, limits).unwrap_or_else(|e| panic!("{dir}: {e}")))
}

/// Fuel per tick on a full grid: eight racing crafts carrying the mod pickup, slot 0 a
/// person holding fire, every craft hitting a wall, firing and crossing a pad on the
/// first tick, then two quiet ticks.
fn full_grid_fuel(module: &Module) -> Vec<u64> {
    let crafts: Vec<Craft> = (0..8)
        .map(|slot| Craft {
            pickup: Pickup::Ours,
            person: slot == 0,
            fire_held: slot == 0,
            racing: true,
            // One craft on its final lap, so a track module that starts there is measured
            // on its busy ticks.
            lap: if slot == 7 { 3 } else { 2 },
            laps_target: 3,
            shield: 100.0,
            shield_max: 100.0,
            speed_kmh: 400.0,
        })
        .collect();
    let loud: Vec<Event> = (0..8)
        .filter_map(Slot::new)
        .flat_map(|slot| {
            [
                Event::WallContact { slot },
                Event::PickupFire { slot },
                Event::PadEnergy { slot },
            ]
        })
        .collect();
    let mut state = vec![0; module.limits().state_bytes as usize];
    let mut commands = Vec::new();
    (0..3_u64)
        .map(|tick| {
            let events: &[Event] = if tick == 0 { &loud } else { &[] };
            let observation = Observation {
                tick,
                active: 0xff,
                events,
                crafts: &crafts,
            };
            module
                .tick(&observation, &mut state, &mut commands)
                .unwrap_or_else(|(fault, _)| panic!("tick {tick}: {fault}"))
                .fuel_used
        })
        .collect()
}

#[test]
fn the_committed_examples_load_and_run_a_full_grid_in_budget() {
    for dir in ["kiss-curse", "energy-turbo", "hillside"] {
        let Some(module) = example(dir) else {
            eprintln!("{dir}: the mod-examples submodule is not checked out; skipped");
            continue;
        };
        let fuel = full_grid_fuel(&module);
        eprintln!("{dir}: fuel per tick on a full grid {fuel:?}");
        assert!(fuel.iter().all(|f| *f < oag_modhost::FUEL_PER_TICK));
    }
}
