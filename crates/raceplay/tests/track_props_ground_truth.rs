//! A mod's `scale-pad` on a real race of a track file: a property authored on one of
//! Hillside's speed pads reaches a module through the track region, the module's
//! race-wide `scale-pad` doubles that pad's push and leaves the others alone, and a
//! per-slot `scale-pad` of 0 takes every pad's push, plume and sound while `speed-pad`
//! still fires.
//!
//! **`#[ignore]`d and never run in CI**: it needs `data/images/pulse-psp-eu.chd`, the
//! `assets/mods/examples` submodule (Hillside's track file) and the `extensions`
//! feature:
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-raceplay --features extensions \
//!     --run-ignored only -E 'test(/track_props_ground_truth::/)' --no-capture
//! ```
//!
//! The modules are WAT written here. A lone craft flown by this project's driver,
//! throttle held, the same in every run until the first pad: the runs differ only by
//! the module.
#![cfg(feature = "extensions")]

use std::path::{Path, PathBuf};
use std::sync::Arc;

use oag_ext_abi::track::{
    ENTITIES_AT, ENTITY_FIRST_PROP, ENTITY_LEN, ENTITY_PROP_COUNT, PROP_LEN, PROPS_AT, prop_key,
};
use oag_ext_abi::{
    COMMAND_ARG, COMMAND_KIND, COMMAND_SLOT, COMMAND_VALUE, EVENT_COUNT_AT, EVENT_KIND, EVENT_LEN,
    EVENTS_AT, REPLY_AT, STATE_AT,
};
use oag_gameplay::PlayerInputs;
use oag_gameplay::input::Button;
use oag_raceplay as race;

const HILLSIDE: &str = "assets/mods/examples/hillside/tracks/hillside.track.toml";
/// Where the WAT guests keep their buffers.
const IO: usize = 1024;
const TRACK: usize = 8192;
const TICKS: u32 = 2400;
/// How far each speed pad's box is widened to either side, world units.
const WIDEN: f32 = 20.0;

/// Hillside with every speed pad widened across the road (the driver's line passes
/// beside them otherwise) and, with `boost`, `boost = 2.0` on that speed pad; written
/// beside a copy of its images in a directory of its own.
fn hillside(boost: Option<usize>, tag: &str) -> Option<PathBuf> {
    let source = oag_testdata::exact(HILLSIDE)?;
    let mut track = oag_trackfile::read(&source).expect("Hillside reads").track;
    for pad in &mut track.speedup_pads {
        pad.min[0] -= WIDEN;
        pad.max[0] += WIDEN;
    }
    if let Some(pad) = boost {
        track.speedup_pad_props =
            vec![oag_trackfile::props::Props::new(); track.speedup_pads.len()];
        track.speedup_pad_props[pad].insert("boost".into(), 2.0);
    }
    let dir = std::env::temp_dir().join(format!("oag-track-props-{}-{tag}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    for entry in std::fs::read_dir(source.parent().unwrap()).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|e| e == "png") {
            std::fs::copy(&path, dir.join(path.file_name().unwrap())).unwrap();
        }
    }
    let manifest = dir.join("hillside.track.toml");
    oag_trackfile::write(&track, &manifest).expect("writable");
    Some(manifest)
}

fn module(id: &str, body: &str) -> race::extensions::Source {
    let wasm = wat::parse_str(format!(
        r#"(module
            (memory (export "memory") 1)
            (func (export "oag_abi") (result i32) i32.const {word})
            (func (export "oag_io") (result i32) i32.const {IO})
            (func (export "oag_track") (result i32) i32.const {TRACK})
            (func (export "tick") (result i32) (local $p i32) (local $i i32) {body} i32.const 0))"#,
        word = oag_ext_abi::ABI_WORD,
    ))
    .unwrap();
    race::extensions::Source {
        mod_id: id.into(),
        module: "test.wasm".into(),
        titles: Vec::new(),
        api: oag_ext_abi::ABI,
        memory_pages: 1,
        state_bytes: 4,
        scope: race::extensions::Scope::Rules,
        read: Arc::new(move || Ok(wasm.clone())),
        strings: Arc::new(|_| Default::default()),
        icons: Vec::new(),
    }
}

/// Reads speed pad `pad`'s first property; when it is `boost`, sends the race-wide
/// `scale-pad{pad, boost}`. Nothing else on the track carries a property.
fn pad_reader(pad: usize) -> race::extensions::Source {
    let entity = TRACK + ENTITIES_AT + (1 + pad) * ENTITY_LEN;
    let reply = IO + REPLY_AT;
    let cmd = reply + 4;
    module(
        "test.pad-reader",
        &format!(
            "(if (i32.load16_u (i32.const {count}))
               (then
                 (local.set $p (i32.add (i32.const {props})
                   (i32.mul (i32.load16_u (i32.const {first})) (i32.const {PROP_LEN}))))
                 (if (i32.eq (i32.load (local.get $p)) (i32.const {key}))
                   (then
                     (i32.store (i32.const {reply}) (i32.const 1))
                     (i32.store8 (i32.const {kind}) (i32.const 13))
                     (i32.store8 (i32.const {slot}) (i32.const 0))
                     (i32.store16 (i32.const {arg}) (i32.const {pad}))
                     (f32.store (i32.const {value}) (f32.load (i32.add (local.get $p) (i32.const 4))))))))",
            count = entity + ENTITY_PROP_COUNT,
            first = entity + ENTITY_FIRST_PROP,
            props = TRACK + PROPS_AT,
            key = prop_key("boost") as i32,
            kind = cmd + COMMAND_KIND,
            slot = cmd + COMMAND_SLOT,
            arg = cmd + COMMAND_ARG,
            value = cmd + COMMAND_VALUE,
        ),
    )
}

/// Holds slot 0's speed pads at 0 every tick and counts the `speed-pad` events slot 0
/// raises in its state.
fn pad_holder() -> race::extensions::Source {
    let reply = IO + REPLY_AT;
    let cmd = reply + 4;
    let state = IO + STATE_AT;
    module(
        "test.pad-holder",
        &format!(
            "(local.set $i (i32.const 0))
             (block $done (loop $next
               (br_if $done (i32.ge_u (local.get $i) (i32.load (i32.const {count}))))
               (local.set $p (i32.add (i32.const {events}) (i32.mul (local.get $i) (i32.const {EVENT_LEN}))))
               (if (i32.and
                     (i32.eq (i32.load8_u (local.get $p)) (i32.const 5))
                     (i32.eqz (i32.load8_u (i32.add (local.get $p) (i32.const 1)))))
                 (then (i32.store (i32.const {state}) (i32.add (i32.load (i32.const {state})) (i32.const 1)))))
               (local.set $i (i32.add (local.get $i) (i32.const 1)))
               (br $next)))
             (i32.store (i32.const {reply}) (i32.const 1))
             (i32.store8 (i32.const {kind}) (i32.const 14))
             (i32.store8 (i32.const {slot}) (i32.const 0))
             (f32.store (i32.const {value}) (f32.const 0.0))",
            count = IO + EVENT_COUNT_AT,
            events = IO + EVENTS_AT + EVENT_KIND,
            kind = cmd + COMMAND_KIND,
            slot = cmd + COMMAND_SLOT,
            value = cmd + COMMAND_VALUE,
        ),
    )
}

/// One tick of the run: speed, the pad push armed and its length, the plume, the pad
/// sounds, the speed pad the craft is inside (by box), the hash.
#[derive(Debug, Clone, Copy)]
struct Tick {
    kmh: f32,
    pad_timer: f32,
    push: f32,
    plume: f32,
    pad_sounds: usize,
    inside: Option<usize>,
}

fn run(track: &Path, with: Vec<race::extensions::Source>) -> Option<(Vec<Tick>, race::Race)> {
    let image = oag_testdata::image("data/images/pulse-psp-eu.chd")?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        foreign_track: Some(track.to_path_buf()),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        no_driver_for_class: true,
        extensions: with,
        ..race::Options::default()
    })
    .expect("loading the race");
    let mut race = race::Race::start(loaded.setup);
    race.set_autopilot(true);
    let pads = race.speedup_pads().to_vec();
    let mut input = oag_gameplay::input::Input::new();
    let ticks = (0..TICKS)
        .map(|_| {
            input.begin_frame(Button::Cross.bit());
            race.tick(&PlayerInputs::single(oag_gameplay::InputSnapshot {
                buttons: input,
                ..oag_gameplay::InputSnapshot::new()
            }));
            let ship = &race.sim.world.ships[0];
            let at = ship.physics.body.position.to_array();
            Tick {
                kmh: ship.physics.body.linear_velocity.length() * oag_core::math::SPEED_TO_KMH,
                pad_timer: ship.physics.pad_timer,
                push: ship.physics.pad_direction.length(),
                plume: race.exhaust().boost_timer(),
                pad_sounds: race
                    .drain_cues()
                    .iter()
                    .filter(|c| c.cue == oag_gameplay::cue::Cue::SpeedupPad && c.slot == 0)
                    .count(),
                inside: pads.iter().position(|p| p.distance(at) == 0.0),
            }
        })
        .collect();
    Some((ticks, race))
}

/// The ticks a pad push arms on, with the pad the craft is inside then.
fn entries(ticks: &[Tick]) -> Vec<(usize, Option<usize>)> {
    ticks
        .windows(2)
        .enumerate()
        .filter(|(_, w)| w[1].pad_timer > w[0].pad_timer)
        .map(|(t, w)| (t + 1, w[1].inside.or(w[0].inside)))
        .collect()
}

/// The first pad a lone craft crosses on Hillside, from a vanilla run.
fn first_pad(vanilla: &[Tick]) -> (usize, usize) {
    let (tick, pad) = entries(vanilla)[0];
    (tick, pad.expect("the first push arms inside a pad's box"))
}

#[test]
#[ignore = "needs a Pulse disc image, the mod examples and --features extensions"]
fn a_pads_authored_boost_doubles_that_pad_alone() {
    let Some(plain) = hillside(None, "plain") else {
        return;
    };
    let Some((vanilla, _)) = run(&plain, Vec::new()) else {
        return;
    };
    let (entry, pad) = first_pad(&vanilla);
    let file = hillside(Some(pad), "double").unwrap();
    let (modded, race) = run(&file, vec![pad_reader(pad)]).unwrap();
    assert!(race.is_modded());
    let pads = race.speedup_pads();
    let unit = |p: usize| {
        let d = pads[p].direction().unwrap();
        (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()
    };
    let later = entries(&modded);
    println!("vanilla entries {:?}", entries(&vanilla));
    println!("modded entries {later:?}");
    println!(
        "pad {pad} entered at tick {entry}: push {} vanilla, {} modded",
        vanilla[entry].push, modded[entry].push
    );
    assert_eq!(
        modded[entry - 1].kmh,
        vanilla[entry - 1].kmh,
        "the same race up to the pad"
    );
    assert_eq!(vanilla[entry].push, unit(pad));
    assert_eq!(
        modded[entry].push,
        2.0 * unit(pad),
        "the authored boost doubles it"
    );
    let after = entry + 30;
    println!(
        "tick {after}: {:.1} km/h modded, {:.1} vanilla",
        modded[after].kmh, vanilla[after].kmh
    );
    assert!(
        modded[after].kmh > vanilla[after].kmh,
        "the doubled pad is faster"
    );
    let other = later
        .iter()
        .find(|(_, p)| p.is_some_and(|p| p != pad))
        .expect("the craft crosses another pad");
    let (tick, other_pad) = (other.0, other.1.unwrap());
    println!("pad {other_pad} at tick {tick}: push {}", modded[tick].push);
    assert_eq!(
        modded[tick].push,
        unit(other_pad),
        "another pad stays vanilla"
    );
}

#[test]
#[ignore = "needs a Pulse disc image, the mod examples and --features extensions"]
fn a_slot_held_at_zero_gets_no_push_plume_or_sound_and_still_hears_each_pad() {
    let Some(plain) = hillside(None, "held") else {
        return;
    };
    let Some((vanilla, _)) = run(&plain, Vec::new()) else {
        return;
    };
    let (held, race) = run(&plain, vec![pad_holder()]).unwrap();
    let vanilla_entries = entries(&vanilla).len();
    let heard = u32::from_le_bytes(
        race.extension_state(0).expect("the module ran")[..4]
            .try_into()
            .unwrap(),
    );
    // Entries by box, from before the first tick: the grid's slot sits on a widened pad.
    let inside: Vec<Option<usize>> = std::iter::once(None)
        .chain(held.iter().map(|t| t.inside))
        .collect();
    let crossed = inside
        .windows(2)
        .filter(|w| w[1].is_some() && w[1] != w[0])
        .count();
    println!(
        "vanilla: {vanilla_entries} pushes, {} pad sounds; held: {} pushes, {} sounds, plume max {}, {crossed} pads crossed, {heard} speed-pad events",
        vanilla.iter().map(|t| t.pad_sounds).sum::<usize>(),
        entries(&held).len(),
        held.iter().map(|t| t.pad_sounds).sum::<usize>(),
        held.iter().map(|t| t.plume).fold(0.0_f32, f32::max),
    );
    assert!(vanilla_entries > 0 && vanilla.iter().any(|t| t.plume > 0.0));
    assert!(held.iter().all(|t| t.pad_timer == 0.0), "no push ever arms");
    assert!(held.iter().all(|t| t.plume == 0.0), "no plume");
    assert_eq!(
        held.iter().map(|t| t.pad_sounds).sum::<usize>(),
        0,
        "no pad sound"
    );
    assert!(crossed > 0, "the craft crosses pads");
    assert_eq!(
        heard as usize, crossed,
        "the module hears every pad entered"
    );
}
