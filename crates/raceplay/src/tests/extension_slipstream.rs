//! The official mod Slipstream (`assets/mods/official/slipstream`) as the engine loads it:
//! its odds, its look per title (and Pure left out), its committed module in the fuel
//! budget, and its portal's colour per title through the observation's title key.
//!
//! The official mods are a submodule; a checkout without it skips with a line saying so.

use super::extensions::on::{module, prepared_for};
use oag_ext_abi::{Command, Craft, Distance, Event, Pickup, Slot};
use oag_modhost::{Limits, Module, Observation};
use oag_tables::pickup_look::Icon;
use oag_tables::weapons::{PickupOdds, PickupTable, Weapon};

const SLIPSTREAM: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../assets/mods/official/slipstream"
);

const TITLES: [(&str, &str); 4] = [
    ("wipeout-pulse", "pulse"),
    ("wipeout-hd", "hd"),
    ("wipeout-omega-collection", "omega"),
    ("wipeout-2048", "2048"),
];

fn open() -> Option<oag_mod::Mod> {
    let root = std::path::Path::new(SLIPSTREAM);
    if !root.join("oag-mod.toml").is_file() {
        eprintln!("slipstream: the official-mods submodule is not checked out; skipped");
        return None;
    }
    Some(oag_mod::Mod::open(root).expect("the mod opens"))
}

/// The mod's source with its real manifest rows over a module that does nothing.
fn source(module_: &oag_mod::Mod) -> crate::extensions::Source {
    let manifest = &module_.manifest;
    let mut source = module(&manifest.id, "i32.const 0");
    source.titles = manifest.titles.clone();
    source.pickup = manifest.pickup.clone();
    source.pickup_titles = manifest.pickup_titles.clone();
    source.pickup_odds = manifest.pickup_odds.clone();
    source
}

#[test]
fn it_ships_off_and_each_title_has_its_own_icon_and_pure_is_left_out() {
    let Some(slipstream) = open() else { return };
    let manifest = &slipstream.manifest;
    assert!(
        !manifest.default_enabled,
        "off until the player turns it on"
    );
    assert!(!manifest.titles.iter().any(|t| t == "wipeout-pure"));
    let source = source(&slipstream);
    for (title, icon) in TITLES {
        let prepared = prepared_for(title, std::slice::from_ref(&source));
        let look = crate::extensions::Extensions::new(prepared)
            .pickup_look(0)
            .unwrap_or_else(|| panic!("{title}: a look"));
        assert_eq!(
            look.icon,
            Icon::Mod(crate::extensions::mod_icon_src(&manifest.id, icon)),
            "{title}"
        );
        let png = slipstream.read(&manifest.icons[icon]).unwrap().unwrap();
        assert!(png.starts_with(b"\x89PNG"), "{title}: a PNG");
    }
    // Pure: the module is left out of the race altogether.
    let mut report = Vec::new();
    let props = crate::extensions::TrackProps::default();
    let track = crate::extensions::Track {
        name: "t",
        props: &props,
        speedup_pads: &[],
        weapon_pads: &[],
        paths: &[],
        routes: &[],
    };
    let left_out =
        crate::extensions::prepare(&[source], "wipeout-pure", None, track, None, &mut report);
    assert!(!crate::extensions::Extensions::new(left_out).any());
}

#[test]
fn the_pads_draw_it_beside_the_titles_weapons_at_its_own_odds() {
    use oag_weapons::pickup::{Driver, draw};
    let Some(slipstream) = open() else { return };
    let source = source(&slipstream);
    let extensions = crate::extensions::Extensions::new(prepared_for(
        "wipeout-pulse",
        std::slice::from_ref(&source),
    ));
    let odds = extensions.pickup_odds("VENOM").expect("its row");
    assert_eq!(odds, slipstream.manifest.pickup_odds[0].odds);
    let plain = PickupTable {
        class: "VENOM".to_string(),
        odds: [Weapon::Turbo, Weapon::Rocket, Weapon::Shield, Weapon::Mine]
            .into_iter()
            .map(|w| {
                (
                    w,
                    PickupOdds {
                        ai: 10.0,
                        human: 10.0,
                        front: 0.0,
                        back: 0.0,
                    },
                )
            })
            .collect(),
    };
    let mut modded = plain.clone();
    modded.odds.push((Weapon::Extension, odds));
    for driver in [Driver::Ai, Driver::Human { place: 5, field: 8 }] {
        let share = |table: &PickupTable| {
            let mut rng = oag_core::Rng::new(11);
            let n = 20_000;
            let hits = (0..n)
                .filter(|_| draw(&mut rng, table, driver, None, None) == Some(Weapon::Extension))
                .count();
            hits as f32 / n as f32
        };
        let with = share(&modded);
        assert!((0.02..0.3).contains(&with), "{driver:?}: {with}");
        assert_eq!(share(&plain), 0.0, "never without its row");
    }
}

fn limits(manifest: &oag_mod::Manifest) -> Limits {
    let extension = manifest.extension.as_ref().expect("[extension]");
    Limits {
        memory_pages: extension.memory_pages,
        state_bytes: extension.state_bytes,
        fuel: oag_modhost::FUEL_PER_TICK,
    }
}

fn compiled(slipstream: &oag_mod::Mod) -> Module {
    let manifest = &slipstream.manifest;
    let name = &manifest
        .extension
        .as_ref()
        .unwrap()
        .rules
        .as_ref()
        .unwrap()
        .module;
    let wasm = slipstream
        .read(name)
        .unwrap()
        .expect("the committed module");
    Module::compile(&wasm, limits(manifest)).expect("it loads on this host")
}

/// One call with slot 0 a person holding the pickup, who pressed fire this tick on a
/// portal that fits, in a race of `title`.
fn fire_commands(module: &Module, title: u32) -> (Vec<Command>, u64) {
    let crafts: Vec<Craft> = (0..8)
        .map(|slot| Craft {
            pickup: if slot == 0 {
                Pickup::Ours
            } else {
                Pickup::None
            },
            person: slot == 0,
            racing: true,
            speed_kmh: 400.0,
            ..Craft::default()
        })
        .collect();
    let slot = Slot::new(0).unwrap();
    let events = [
        Event::PickupFire { slot },
        Event::Clearance {
            slot,
            fits: true,
            ahead: 30.0,
        },
    ];
    let mut state = vec![0; module.limits().state_bytes as usize];
    let mut commands = Vec::new();
    let observation = Observation {
        tick: 0,
        active: 0xff,
        events: &events,
        crafts: &crafts,
        course: oag_ext_abi::Course {
            title,
            ..Default::default()
        },
        track: &[],
        speed_pads: 0,
        routes: 0,
    };
    let cost = module
        .tick(&observation, &mut state, &mut commands)
        .expect("no fault");
    (commands, cost.fuel_used)
}

#[test]
fn its_portal_is_the_titles_own_and_a_fire_fits_the_fuel_budget() {
    let Some(slipstream) = open() else { return };
    let module = compiled(&slipstream);
    let effects: Vec<&String> = slipstream.manifest.effects.keys().collect();
    assert_eq!(
        effects,
        ["portal_2048", "portal_hd", "portal_omega", "portal_pulse"],
        "the guest's indices are this table's name order"
    );
    for (title, index) in [
        ("wipeout-2048", 0),
        ("wipeout-hd", 1),
        ("wipeout-omega-collection", 2),
        ("wipeout-pulse", 3),
    ] {
        let (commands, fuel) = fire_commands(&module, oag_ext_abi::channel_key(title));
        let portals: Vec<u16> = commands
            .iter()
            .filter_map(|c| match c {
                Command::FireModEffectAhead { index, .. } => Some(*index),
                _ => None,
            })
            .collect();
        assert_eq!(portals, [index, index], "{title}");
        assert!(commands.iter().any(|c| matches!(
            c,
            Command::Transit {
                distance: Distance::Seconds(_),
                ..
            }
        )));
        assert!(fuel < oag_modhost::FUEL_PER_TICK / 4, "{title}: {fuel}");
        eprintln!("slipstream {title}: fuel on a fire {fuel}");
    }
}
