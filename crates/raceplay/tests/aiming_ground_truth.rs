//! The aiming record on a real circuit: Pulse PSP EU's `16_Track` with a full field on the
//! starting grid. A guest copies the whole matrix into its state; every pair it reads is the
//! one computed independently from the crafts' bodies, a craft with another straight ahead of
//! it in its column reads that one ahead and centred, and one across the track does not.
//!
//! **`#[ignore]`d and never run in CI**: it needs `data/images/pulse-psp-eu.chd` and the
//! `extensions` feature:
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-raceplay --features extensions \
//!     --run-ignored only -E 'test(/aiming_ground_truth::/)' --no-capture
//! ```
#![cfg(feature = "extensions")]

use std::sync::Arc;

use oag_ext_abi::{AIM_AT, AIM_LEN, AIM_SLOTS, Aim};
use oag_gameplay::PlayerInputs;
use oag_raceplay as race;

const TRACK: &str = "Data\\Environments\\16_Track\\track.vex";

/// A module that copies the aiming matrix into its state on every call.
fn copies_the_matrix() -> race::extensions::Source {
    let wasm = wat::parse_str(format!(
        r#"(module
            (memory (export "memory") 1)
            (func (export "oag_abi") (result i32) i32.const {word})
            (func (export "oag_io") (result i32) i32.const 1024)
            (func (export "tick") (result i32)
              (memory.copy (i32.const {state}) (i32.const {from}) (i32.const {len}))
              i32.const 0))"#,
        word = oag_ext_abi::ABI_WORD,
        state = 1024 + oag_ext_abi::STATE_AT,
        from = 1024 + AIM_AT,
        len = AIM_SLOTS * AIM_SLOTS * AIM_LEN,
    ))
    .unwrap();
    race::extensions::Source {
        mod_id: "test.aiming".to_string(),
        module: "aiming.wasm".to_string(),
        titles: Vec::new(),
        api: 1,
        memory_pages: 1,
        state_bytes: (AIM_SLOTS * AIM_SLOTS * AIM_LEN) as u32,
        drives_ai_pickup: false,
        scope: race::extensions::Scope::Rules,
        read: Arc::new(move || Ok(wasm.clone())),
        strings: Arc::new(|_| Default::default()),
        icons: Vec::new(),
        effects: Vec::new(),
        shaders: Vec::new(),
        sounds: Vec::new(),
        files: Arc::new(|_| Err("no files".into())),
        pickup: None,
        pickup_titles: Vec::new(),
        pickup_odds: Vec::new(),
        options: Default::default(),
        channels: race::extensions::Declared::default(),
    }
}

fn matrix(state: &[u8]) -> Vec<Vec<Aim>> {
    state
        .as_chunks::<{ AIM_SLOTS * AIM_LEN }>()
        .0
        .iter()
        .map(|row| {
            row.as_chunks::<AIM_LEN>()
                .0
                .iter()
                .map(Aim::decode)
                .collect()
        })
        .collect()
}

#[test]
#[ignore = "needs data/images/pulse-psp-eu.chd and --features extensions"]
fn on_the_starting_grid_a_craft_ahead_reads_ahead_and_centred_and_one_across_does_not() {
    let Some(image) = oag_testdata::image("data/images/pulse-psp-eu.chd") else {
        return;
    };
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        track: Some(TRACK.to_string()),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        extensions: vec![copies_the_matrix()],
        ..race::Options::default()
    })
    .expect("loading the race");
    let mut race = race::Race::start(loaded.setup);
    let ships = usize::from(race.sim.world.ship_count).min(AIM_SLOTS);
    assert!(ships >= 4, "a full field: {ships} crafts");

    // What the guest must read, from the bodies as the tick begins.
    let bodies: Vec<_> = race.sim.world.ships[..ships]
        .iter()
        .map(|ship| ship.physics.body)
        .collect();
    race.tick(&PlayerInputs::none());
    let read = matrix(race.extension_state(0).expect("the module ran"));

    let mut ahead_centred = 0;
    for (o, from) in bodies.iter().enumerate() {
        for (t, to) in bodies.iter().enumerate() {
            if o == t {
                assert_eq!(read[o][t], Aim::default(), "{o} to itself");
                continue;
            }
            let offset = to.position - from.position;
            let expected = Aim::from_offset(
                offset.dot(from.forward()),
                offset.dot(from.right()),
                offset.dot(from.up()),
            );
            assert_eq!(read[o][t], expected, "{o} looking at {t}");
            // The craft straight ahead in the column: in front, within the grid's length,
            // and close to the forward axis.
            if read[o][t].in_cone(200.0, 0.2) && read[o][t].right.abs() <= 2 {
                ahead_centred += 1;
            }
        }
    }
    println!("{ships} crafts: {ahead_centred} pairs ahead and centred on the grid");
    assert!(ahead_centred > 0, "no craft has another straight ahead");

    // The same circuit's own axes: slot 1 moved across slot 0 (40 right, 4 ahead), then
    // straight ahead of it (60).
    let place = |race: &mut race::Race, ahead: f32, right: f32| {
        // Posed from slot 0's body as the next tick will observe it.
        let from = race.sim.world.ships[0].physics.body;
        race.sim.world.ships[1].physics.body.position =
            from.position + from.forward() * ahead + from.right() * right;
        race.tick(&PlayerInputs::none());
        matrix(race.extension_state(0).unwrap())[0][1]
    };
    let across = place(&mut race, 4.0, 40.0);
    assert!(
        across.right > 0 && !across.in_cone(200.0, 0.2),
        "across the track reads sideways, not in front: {across:?}"
    );
    let ahead = place(&mut race, 60.0, 0.0);
    assert!(
        ahead.in_cone(200.0, 0.2) && ahead.right.abs() <= 1,
        "straight ahead reads ahead and centred: {ahead:?}"
    );
}
