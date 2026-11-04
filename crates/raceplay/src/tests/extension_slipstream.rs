//! The official mod Slipstream (`assets/mods/official/slipstream`) as the engine loads it:
//! its odds, its look per title, its committed module in the fuel
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

const TITLES: [(&str, &str); 5] = [
    ("wipeout-pulse", "pulse"),
    ("wipeout-pure", "pure"),
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
fn it_ships_off_and_each_title_has_its_own_icon() {
    let Some(slipstream) = open() else { return };
    let manifest = &slipstream.manifest;
    assert!(
        !manifest.default_enabled,
        "off until the player turns it on"
    );
    assert!(manifest.titles.iter().any(|t| t == "wipeout-pure"));
    let source = source(&slipstream);
    for (title, icon) in TITLES {
        let prepared = prepared_for(title, std::slice::from_ref(&source));
        let look = crate::extensions::Extensions::new(prepared)
            .pickup_look(0, oag_tables::weapons::ModKind::FIRST)
            .unwrap_or_else(|| panic!("{title}: a look"));
        assert_eq!(
            look.icon,
            Icon::Mod(crate::extensions::mod_icon_src(&manifest.id, icon)),
            "{title}"
        );
        let png = slipstream.read(&manifest.icons[icon]).unwrap().unwrap();
        assert!(png.starts_with(b"\x89PNG"), "{title}: a PNG");
    }
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
    let odds = extensions.pickup_odds("VENOM").first().expect("its row").1;
    assert_eq!(odds, slipstream.manifest.pickup_odds[0].odds);
    let plain = PickupTable {
        class: "VENOM".to_string(),
        // Pulse's real table sums to 86 with a Turbo at 14.
        odds: [
            (Weapon::Turbo, 14.0),
            (Weapon::Rocket, 20.0),
            (Weapon::Shield, 20.0),
            (Weapon::Mine, 32.0),
        ]
        .into_iter()
        .map(|(w, weight)| {
            (
                w,
                PickupOdds {
                    ai: weight,
                    human: weight,
                    front: 0.0,
                    back: 0.0,
                },
            )
        })
        .collect(),
    };
    let mut modded = plain.clone();
    modded
        .odds
        .push((Weapon::Extension(oag_tables::weapons::ModKind::FIRST), odds));
    for driver in [Driver::Ai, Driver::Human { place: 5, field: 8 }] {
        let share = |table: &PickupTable| {
            let mut rng = oag_core::Rng::new(11);
            let n = 20_000;
            let hits = (0..n)
                .filter(|_| {
                    draw(&mut rng, table, driver, None, None)
                        == Some(Weapon::Extension(oag_tables::weapons::ModKind::FIRST))
                })
                .count();
            hits as f32 / n as f32
        };
        let with = share(&modded);
        // The AI's column is flat; a person's blends `back` in by place.
        let low = odds.ai / (86.0 + odds.ai);
        let high = (odds.human + odds.back) / (86.0 + odds.human + odds.back);
        let (min, max) = if matches!(driver, Driver::Ai) {
            (low - 0.012, low + 0.012)
        } else {
            (low - 0.012, high + 0.012)
        };
        assert!(
            (min..max).contains(&with),
            "{driver:?}: {with} in {min}..{max}"
        );
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
        [
            "portal_2048",
            "portal_hd",
            "portal_omega",
            "portal_pulse",
            "portal_pure"
        ],
        "the guest's indices are this table's name order"
    );
    for (title, index) in [
        ("wipeout-2048", 0),
        ("wipeout-hd", 1),
        ("wipeout-omega-collection", 2),
        ("wipeout-pulse", 3),
        ("wipeout-pure", 4),
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

/// A module that stores the observation's title key, as the host writes it, in its state.
fn title_reader() -> crate::extensions::Source {
    module(
        "test.title",
        &format!(
            "(i32.store (i32.const {state}) (i32.load (i32.const {title})))
             i32.const 0",
            state = super::extensions::on::STATE,
            title = 1024 + oag_ext_abi::TITLE_AT,
        ),
    )
}

#[test]
fn a_race_hands_its_module_the_titles_key() {
    for title in [
        "wipeout-pulse",
        "wipeout-hd",
        "wipeout-omega-collection",
        "wipeout-2048",
    ] {
        let mut race = super::extensions::grid(prepared_for(title, &[title_reader()]));
        race.tick(&oag_gameplay::PlayerInputs::none());
        let blob = race.sim.extensions.blob_for_tests(0);
        assert_eq!(
            u32::from_le_bytes(blob[..4].try_into().unwrap()),
            oag_ext_abi::channel_key(title),
            "{title}"
        );
    }
}

#[cfg(feature = "view")]
#[test]
fn its_three_sounds_load_through_the_engines_loader_in_name_order() {
    let Some(slipstream) = open() else { return };
    let mut source = source(&slipstream);
    source.sounds = slipstream
        .manifest
        .sounds
        .iter()
        .map(|(name, path)| (name.clone(), path.clone()))
        .collect();
    let slipstream = std::sync::Arc::new(slipstream);
    source.files = std::sync::Arc::new({
        let slipstream = slipstream.clone();
        move |name| {
            slipstream
                .read(name)
                .map_err(|e| e.to_string())?
                .ok_or_else(|| format!("{name}: not in the mod"))
        }
    });
    let mut report = Vec::new();
    let loaded = crate::extension_sounds::load(&[source], "wipeout-pulse", None, &mut report);
    let names: Vec<&str> = slipstream
        .manifest
        .sounds
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(names, ["portal-exit", "portal-open", "transit"]);
    assert_eq!(report.len(), 3, "{report:?}");
    for (index, name) in names.iter().enumerate() {
        let sound = loaded
            .get(&slipstream.manifest.id, index as u16)
            .unwrap_or_else(|| panic!("{name} loads: {report:?}"));
        assert_eq!(
            (sound.channels(), sound.sample_rate()),
            (1, 22_050),
            "{name}"
        );
        assert!(sound.seconds() > 0.4 && sound.seconds() < 1.2, "{name}");
        assert!(
            report
                .iter()
                .any(|l| l.contains(name) && l.contains("loaded")),
            "{name} is in the report"
        );
    }
}

/// A module that stores the first option byte, as the host writes it, in its state.
fn option_reader() -> crate::extensions::Source {
    module(
        "test.option",
        &format!(
            "(i32.store8 (i32.const {state}) (i32.load8_u (i32.const {option})))
             i32.const 0",
            state = super::extensions::on::STATE,
            option = 1024 + oag_ext_abi::OPTIONS_AT,
        ),
    )
}

#[test]
fn a_race_hands_its_module_the_picks_its_source_carries() {
    for pick in [0u8, 1, 2] {
        let mut source = option_reader();
        source.options = crate::extensions::Picks::all_sim([pick, 0, 0, 0]);
        let mut race = super::extensions::grid(prepared_for("wipeout-pulse", &[source]));
        race.tick(&oag_gameplay::PlayerInputs::none());
        assert_eq!(race.sim.extensions.blob_for_tests(0)[0], pick);
    }
}

/// The real module's commands for the first `ticks` after a fire by a person at 360 km/h on
/// a 4000 unit lap from progress 3900, flying on, with the QUALITY pick `option`.
fn portal_commands(module: &Module, option: u8, ticks: usize) -> Vec<Command> {
    let crafts: Vec<Craft> = (0..8)
        .map(|slot| Craft {
            pickup: if slot == 0 {
                Pickup::Ours
            } else {
                Pickup::None
            },
            person: slot == 0,
            racing: true,
            speed_kmh: 360.0,
            ..Craft::default()
        })
        .collect();
    let slot = Slot::new(0).unwrap();
    let mut state = vec![0; module.limits().state_bytes as usize];
    let mut all = Vec::new();
    for tick in 0..ticks {
        // Five units a tick, so the craft reaches the entrance at 3930 on the sixth.
        let tracking: Vec<oag_ext_abi::Tracking> = (0..8)
            .map(|s| oag_ext_abi::Tracking {
                progress: (s == 0).then_some(3900.0 + 5.0 * tick as f32),
                place: u8::from(s == 0),
            })
            .collect();
        let events = if tick == 0 {
            vec![
                Event::PickupFire { slot },
                Event::Clearance {
                    slot,
                    fits: true,
                    ahead: 30.0,
                },
            ]
        } else {
            Vec::new()
        };
        let observation = Observation {
            tick: tick as u64,
            active: 0xff,
            events: &events,
            crafts: &crafts,
            course: oag_ext_abi::Course {
                length: 4000.0,
                tracking: &tracking,
                options: [option, 0, 0, 0],
                ..Default::default()
            },
            track: &[],
            speed_pads: 0,
            routes: 0,
        };
        let mut commands = Vec::new();
        module
            .tick(&observation, &mut state, &mut commands)
            .expect("no fault");
        all.extend(commands);
    }
    all
}

#[test]
fn the_real_module_draws_each_quality_as_specified_and_plays_the_same_race() {
    use oag_ext_abi::ViewUpdate;
    let Some(slipstream) = open() else { return };
    let module = compiled(&slipstream);
    let by = |option| portal_commands(&module, option, 30);
    let views = |commands: &[Command]| -> Vec<Command> {
        commands
            .iter()
            .filter(|c| matches!(c, Command::PortalView { .. }))
            .copied()
            .collect()
    };
    let rest = |commands: &[Command]| -> Vec<Command> {
        commands
            .iter()
            .filter(|c| {
                !matches!(
                    c,
                    Command::PortalView { .. } | Command::SetShaderParam { .. }
                )
            })
            .copied()
            .collect()
    };
    let (low, medium, high) = (by(0), by(1), by(2));
    assert!(views(&low).is_empty(), "Low asks for no scene");
    let kinds = |commands: &[Command]| -> Vec<ViewUpdate> {
        views(commands)
            .iter()
            .filter_map(|c| match c {
                Command::PortalView { update, .. } => Some(*update),
                _ => None,
            })
            .collect()
    };
    assert!(kinds(&medium).len() >= 24 && kinds(&medium).iter().all(|u| *u == ViewUpdate::Once));
    assert!(kinds(&high).len() >= 24 && kinds(&high).iter().all(|u| *u == ViewUpdate::Continuous));
    // The same generation every tick, so a held snapshot is taken once per portal.
    let generations: std::collections::BTreeSet<u8> = views(&medium)
        .iter()
        .filter_map(|c| match c {
            Command::PortalView { generation, .. } => Some(*generation),
            _ => None,
        })
        .collect();
    assert_eq!(generations.len(), 1);
    assert_eq!(
        rest(&low),
        rest(&medium),
        "transit, fill, FOV, sounds: unchanged"
    );
    assert_eq!(rest(&medium), rest(&high));
    assert!(
        rest(&high)
            .iter()
            .any(|c| matches!(c, Command::Transit { .. }))
    );
}
