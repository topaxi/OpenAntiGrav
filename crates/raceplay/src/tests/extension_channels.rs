//! Events between mods (`hooks.md`, "Events between mods"): a module's `emit` and a
//! track trigger's `oag_emit` reach every module as `Event::Mod` on the next tick, in an
//! order no module's place changes; the queue is hashed; the caps fault by name; and a
//! trigger raises its channel with no module at all (the view's `oag_on` animations).
//! The guests are hand-written WebAssembly, as in `extensions.rs`.

use super::extensions::{grid, run};
use crate::extensions::{Prepared, Track, TrackProps, prepare};

fn any(channel: u32) -> oag_mesh::mesh::Start {
    oag_mesh::mesh::Start {
        channel,
        value: None,
    }
}

fn key(name: &str) -> u32 {
    oag_ext_abi::channel_key(name)
}

fn prepared_on(sources: &[crate::extensions::Source], props: &TrackProps) -> Prepared {
    let mut report = Vec::new();
    let track = Track {
        name: "test",
        props,
        speedup_pads: &[],
        weapon_pads: &[],
        paths: &[],
        routes: &[],
    };
    let prepared = prepare(sources, "wipeout-pulse", None, track, None, &mut report);
    assert!(report.iter().all(|l| !l.contains("left out")), "{report:?}");
    prepared
}

/// A box straddling the road between `from` and `to` along x, sending `channel`.
fn gate(from: f32, to: f32, channel: &str) -> TrackProps {
    let mut to_world = oag_ext_abi::track::IDENTITY;
    to_world[12] = (from + to) * 0.5;
    TrackProps {
        triggers: vec![oag_trackfile::props::Trigger {
            name: "gate".into(),
            to_world,
            min: [(from - to) * 0.5, -100.0, -100.0],
            max: [(to - from) * 0.5, 100.0, 100.0],
            props: Default::default(),
            emits: Some(channel.into()),
        }],
        ..TrackProps::default()
    }
}

/// Where slot 1 is after `ticks` ticks of the fixture race, and the gate that straddles
/// the stretch it drives between 300 and 380 ticks: it is entered once.
fn gate_on_the_road(channel: &str) -> TrackProps {
    let x_at = |ticks| {
        run(grid(Prepared::default()), ticks).0.sim.world.ships[1]
            .physics
            .body
            .position
            .x
    };
    let (from, to) = (x_at(300), x_at(380));
    assert!(to > from + 10.0, "slot 1 drives along +x: {from} to {to}");
    gate(from + (to - from) * 0.25, from + (to - from) * 0.5, channel)
}

#[test]
fn a_trigger_raises_its_channel_with_no_module_and_a_race_without_one_hashes_as_before() {
    let props = gate_on_the_road("test.track/passed");
    let beacons = prepared_on(&[], &props);
    let (race, with) = run(grid(beacons), 400);
    let (_, without) = run(grid(Prepared::default()), 400);
    assert_eq!(with, without, "an emitting trigger moved a hash");
    assert!(!race.is_modded());
    let started = race.channel_started(any(key("test.track/passed")));
    assert!(
        started.is_some_and(|s| (300.0 / 60.0..400.0 / 60.0).contains(&s)),
        "the trigger fired while the craft crossed it: {started:?}"
    );
    assert_eq!(race.channel_started(any(key("test.track/other"))), None);
    let unreached = prepared_on(&[], &gate(1.0e6, 1.0e6 + 10.0, "test.track/passed"));
    let (race, _) = run(grid(unreached), 400);
    assert_eq!(race.channel_started(any(key("test.track/passed"))), None);
}

#[test]
fn a_position_far_from_every_trigger_fires_nothing() {
    let props = gate(1.0e6, 1.0e6 + 10.0, "test.track/passed");
    let (race, _) = run(grid(prepared_on(&[], &props)), 60);
    assert!(race.sim.extensions.fired().is_empty());
}

#[cfg(feature = "extensions")]
mod with_modules {
    use std::sync::Arc;

    use super::*;
    use crate::Race;
    use crate::extensions::{Declared, Scope, Source};
    use oag_gameplay::PlayerInputs;

    const IO: i32 = 1024;
    const EVENTS: i32 = IO + oag_ext_abi::EVENTS_AT as i32;
    const STATE: i32 = IO + oag_ext_abi::STATE_AT as i32;
    const REPLY: i32 = IO + oag_ext_abi::REPLY_AT as i32;
    const TICK: i32 = IO + oag_ext_abi::TICK_AT as i32;

    fn source(id: &str, body: &str, publishes: &[&str]) -> Source {
        let wasm = wat::parse_str(format!(
            r#"(module
                (memory (export "memory") 1)
                (func (export "oag_abi") (result i32) i32.const {word})
                (func (export "oag_io") (result i32) i32.const {IO})
                (func (export "tick") (result i32) (local $i i32) (local $p i32) {body}))"#,
            word = oag_ext_abi::ABI_WORD,
        ))
        .unwrap();
        Source {
            mod_id: id.to_string(),
            module: "test.wasm".to_string(),
            titles: Vec::new(),
            api: 1,
            memory_pages: 1,
            state_bytes: 32,
            scope: Scope::Rules,
            read: Arc::new(move || Ok(wasm.clone())),
            strings: Arc::new(|_| Default::default()),
            icons: Vec::new(),
            effects: Vec::new(),
            files: Arc::new(|_| Err("no files".into())),
            pickup: None,
            channels: Declared {
                publishes: publishes.iter().map(|s| (*s).to_string()).collect(),
                listens: Vec::new(),
            },
        }
    }

    /// At world tick `at`, emits `channels` (name, value) about slot 1, each a command.
    fn emitter(id: &str, at: u32, emits: &[(&str, f32)], publishes: &[&str]) -> Source {
        let mut stores = String::new();
        for (j, (name, value)) in emits.iter().enumerate() {
            let c = REPLY + 4 + (oag_ext_abi::COMMAND_LEN * j) as i32;
            stores += &format!(
                "(i32.store8 (i32.const {c}) (i32.const 23))
                 (i32.store8 (i32.const {slot}) (i32.const 1))
                 (f32.store (i32.const {value_at}) (f32.const {value}))
                 (i32.store (i32.const {word_at}) (i32.const {channel}))",
                slot = c + oag_ext_abi::COMMAND_SLOT as i32,
                value_at = c + oag_ext_abi::COMMAND_VALUE as i32,
                word_at = c + oag_ext_abi::COMMAND_WORD as i32,
                channel = key(name) as i32,
            );
        }
        let body = format!(
            "(if (i64.eq (i64.load (i32.const {TICK})) (i64.const {at}))
               (then (i32.store (i32.const {REPLY}) (i32.const {count})) {stores}))
             i32.const 0",
            count = emits.len(),
        );
        source(id, &body, publishes)
    }

    /// Records the `mod` events it is delivered: their count (word 0), the last one's
    /// slot (byte 4), value (word 2), sender (word 3), channel (word 4) and the tick it
    /// arrived on (word 5).
    fn listener(id: &str) -> Source {
        let body = format!(
            "(block $done (loop $next
               (br_if $done (i32.ge_u (local.get $i) (i32.load (i32.const {count}))))
               (local.set $p (i32.add (i32.const {EVENTS}) (i32.mul (local.get $i) (i32.const {len}))))
               (if (i32.eq (i32.load8_u (local.get $p)) (i32.const 10))
                 (then
                   (i32.store (i32.const {STATE}) (i32.add (i32.load (i32.const {STATE})) (i32.const 1)))
                   (i32.store8 (i32.const {slot}) (i32.load8_u (i32.add (local.get $p) (i32.const {event_slot}))))
                   (f32.store (i32.const {value}) (f32.load (i32.add (local.get $p) (i32.const {event_value}))))
                   (i32.store (i32.const {from}) (i32.load (i32.add (local.get $p) (i32.const {event_from}))))
                   (i32.store (i32.const {channel}) (i32.load (i32.add (local.get $p) (i32.const {event_channel}))))
                   (i32.store (i32.const {tick}) (i32.load (i32.const {TICK})))))
               (local.set $i (i32.add (local.get $i) (i32.const 1)))
               (br $next)))
             i32.const 0",
            count = IO + oag_ext_abi::EVENT_COUNT_AT as i32,
            len = oag_ext_abi::EVENT_LEN,
            slot = STATE + 4,
            value = STATE + 8,
            from = STATE + 12,
            channel = STATE + 16,
            tick = STATE + 20,
            event_slot = oag_ext_abi::EVENT_SLOT,
            event_value = oag_ext_abi::EVENT_VALUE,
            event_from = oag_ext_abi::EVENT_FROM,
            event_channel = oag_ext_abi::EVENT_CHANNEL,
        );
        source(id, &body, &[])
    }

    #[derive(Debug, PartialEq)]
    struct Heard {
        count: u32,
        slot: u8,
        value: f32,
        from: u32,
        channel: u32,
        tick: u32,
    }

    fn heard(race: &Race, module: usize) -> Heard {
        let blob = race.extension_state(module).unwrap();
        let word = |at: usize| u32::from_le_bytes(blob[at..at + 4].try_into().unwrap());
        Heard {
            count: word(0),
            slot: blob[4],
            value: f32::from_bits(word(8)),
            from: word(12),
            channel: word(16),
            tick: word(20),
        }
    }

    fn race_of(sources: &[Source]) -> Race {
        grid(prepared_on(sources, &TrackProps::default()))
    }

    #[test]
    fn an_emit_reaches_a_listener_on_the_next_tick_with_everything_intact() {
        let ping = "test.a/ping";
        let mut race = race_of(&[
            emitter("test.a", 3, &[(ping, 2.5)], &[ping]),
            listener("test.b"),
        ]);
        for _ in 0..4 {
            race.tick(&PlayerInputs::none());
        }
        assert_eq!(
            heard(&race, 1).count,
            0,
            "nothing yet: the emit is at tick 3"
        );
        race.tick(&PlayerInputs::none());
        assert_eq!(
            heard(&race, 1),
            Heard {
                count: 1,
                slot: 1,
                value: 2.5,
                from: key("test.a"),
                channel: key(ping),
                tick: 4,
            }
        );
        for _ in 0..4 {
            race.tick(&PlayerInputs::none());
        }
        assert_eq!(heard(&race, 1).count, 1, "an event is delivered once");
    }

    #[test]
    fn the_result_is_the_same_whichever_module_runs_first() {
        let heard_with = |emitter_id: &str, listener_id: &str| {
            let ping = format!("{emitter_id}/ping");
            let mut race = race_of(&[
                emitter(emitter_id, 3, &[(&ping, -1.25)], &[&ping]),
                listener(listener_id),
            ]);
            let at = if listener_id < emitter_id { 0 } else { 1 };
            for _ in 0..6 {
                race.tick(&PlayerInputs::none());
            }
            heard(&race, at)
        };
        let after = heard_with("test.a", "test.b");
        let before = heard_with("test.b", "test.a");
        assert_eq!(
            (after.count, after.slot, after.value, after.tick),
            (before.count, before.slot, before.value, before.tick)
        );
        assert_eq!((after.count, after.tick), (1, 4));
        assert_ne!(after.from, before.from, "each hears its own sender");
    }

    #[test]
    fn events_are_delivered_in_channel_order_whatever_order_they_were_sent_in() {
        let names: Vec<String> = ('a'..='z').map(|c| format!("x/{c}")).collect();
        let pair = |ascending: bool| {
            let mut found = None;
            for first in &names {
                for second in &names {
                    if (key(first) < key(second)) == ascending && first != second {
                        found = Some((first.clone(), second.clone()));
                    }
                }
            }
            found.expect("some pair of the 26 names is ordered each way")
        };
        let last_channel = |first: &str, second: &str| {
            let (a, b) = (
                format!("test.a/{}", &first[2..]),
                format!("test.b/{}", &second[2..]),
            );
            let mut race = race_of(&[
                emitter("test.a", 3, &[(&a, 1.0)], &[&a]),
                emitter("test.b", 3, &[(&b, 2.0)], &[&b]),
                listener("test.c"),
            ]);
            for _ in 0..5 {
                race.tick(&PlayerInputs::none());
            }
            let h = heard(&race, 2);
            assert_eq!(h.count, 2);
            (h.channel, key(&a), key(&b))
        };
        for ascending in [true, false] {
            let (first, second) = pair(ascending);
            let (last, a, b) = last_channel(&first, &second);
            assert_eq!(last, a.max(b), "the larger channel key arrives last");
        }
    }

    #[test]
    fn the_queue_is_in_the_hash_before_anyone_has_heard_it() {
        let ping = "test.a/ping";
        let hashes = |value: f32| {
            run(
                race_of(&[
                    emitter("test.a", 3, &[(ping, value)], &[ping]),
                    listener("test.b"),
                ]),
                6,
            )
            .1
        };
        let (one, two) = (hashes(1.0), hashes(2.0));
        assert_eq!(one[..3], two[..3], "identical until the emit");
        assert_ne!(
            one[3], two[3],
            "the end of tick 3: only the queued event differs, and it is hashed"
        );
    }

    #[test]
    fn a_race_with_no_module_hashes_as_before() {
        let (_, plain) = run(grid(Prepared::default()), 120);
        let (_, none) = run(race_of(&[]), 120);
        assert_eq!(plain, none);
    }

    #[test]
    fn emitting_past_the_cap_faults_the_module_by_name_and_none_of_its_emits_land() {
        let ping = "test.a/ping";
        let at_cap = ["a", "b", "c", "d"].map(|_| (ping, 1.0));
        let mut over = at_cap.to_vec();
        over.push((ping, 1.0));
        let run_with = |emits: &[(&str, f32)]| {
            let mut race = race_of(&[emitter("test.a", 3, emits, &[ping]), listener("test.b")]);
            for _ in 0..6 {
                race.tick(&PlayerInputs::none());
            }
            race
        };
        let fine = run_with(&at_cap);
        assert_eq!(fine.extension_reports()[0].faulted, None);
        assert_eq!(heard(&fine, 1).count, 4);
        let broken = run_with(&over);
        let (tick, why) = broken.extension_reports()[0].faulted.clone().unwrap();
        assert_eq!(tick, 3);
        assert!(why.contains("emit past the cap of 4"), "{why}");
        assert_eq!(
            heard(&broken, 1).count,
            0,
            "a reply that faults lands none of its emits, the four within the cap included"
        );
    }

    #[test]
    fn emitting_on_an_undeclared_channel_faults_the_module_naming_what_it_publishes() {
        let mut race = race_of(&[
            emitter("test.a", 3, &[("test.a/secret", 1.0)], &["test.a/open"]),
            listener("test.b"),
        ]);
        for _ in 0..6 {
            race.tick(&PlayerInputs::none());
        }
        let (tick, why) = race.extension_reports()[0].faulted.clone().unwrap();
        assert_eq!(tick, 3);
        assert!(
            why.contains("test.a/open") && why.contains("publishes"),
            "{why}"
        );
        assert_eq!(heard(&race, 1).count, 0, "nothing was queued");
    }

    #[test]
    fn a_trigger_reaches_a_listening_module_from_the_track_with_no_module_on_the_track() {
        let channel = "test.track/passed";
        let props = gate_on_the_road(channel);
        let mut race = grid(prepared_on(&[listener("test.b")], &props));
        for _ in 0..400 {
            race.tick(&PlayerInputs::none());
        }
        let h = heard(&race, 0);
        assert_eq!(h.count, 1, "the craft entered the gate once");
        assert_eq!(h.channel, key(channel));
        assert_eq!(h.from, oag_ext_abi::TRACK_SENDER);
        assert_eq!(h.slot, 1);
        assert!(h.tick > 300, "{h:?}");
        assert!(race.channel_started(any(key(channel))).is_some());
    }
}
