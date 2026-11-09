//! Wipeout HD / Fury's Eliminator flip on a real race: the press turns the craft, the craft keeps
//! its velocity and its own engine stops it and drives it back up the circuit.
//!
//! **`#[ignore]`d and never run in CI.** It needs the decrypted HD disc image.
//!
//! The law and the original's run are on `docs/ghidra/functions/ps3-hdfury-eu/eliminator-flip.md`.

use oag_gameplay::PlayerInputs;
use oag_gameplay::input::{Button, Input, InputSnapshot};

fn inputs(held: u32, last: u32) -> PlayerInputs {
    let mut buttons = Input::EMPTY;
    buttons.begin_frame(last);
    buttons.begin_frame(held);
    PlayerInputs::single(InputSnapshot {
        buttons,
        ..InputSnapshot::EMPTY
    })
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_flip_stops_the_craft_and_sends_it_back_up_the_circuit() {
    let Some(image) = oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let loaded = oag_raceplay::load(&oag_raceplay::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::Eliminator,
        seed: Some(1),
        ..oag_raceplay::Options::default()
    })
    .expect("loading an HD Eliminator");
    let mut race = oag_raceplay::Race::start(loaded.setup);
    let thrust = Button::Cross.bit();
    let speed = |race: &oag_raceplay::Race| {
        race.sim.world.ships[0]
            .physics
            .body
            .linear_velocity
            .length()
    };
    let mut tick = 0;
    while speed(&race) < 115.0 && tick < 60 * 30 {
        race.tick(&inputs(thrust, thrust));
        tick += 1;
    }
    let before = race.sim.world.ships[0].physics.body.linear_velocity;
    let start = before.normalize();
    let forward = race.sim.world.ships[0].physics.body.forward();
    race.tick(&inputs(thrust | Button::Flip.bit(), thrust));
    let mut stopped = None;
    let mut reversed = None;
    for step in 1..360 {
        race.tick(&inputs(thrust, thrust));
        let body = &race.sim.world.ships[0].physics.body;
        let along = body.linear_velocity.dot(start);
        if std::env::var_os("OAG_SWEEP").is_some() && step % 15 == 0 {
            println!(
                "t={:.2} speed {:.1} along {along:.1} facing {:.2}",
                f64::from(step) / 60.0,
                body.linear_velocity.length(),
                body.forward().dot(forward)
            );
        }
        if stopped.is_none() && along <= 1.0 {
            stopped = Some(step);
        }
        if reversed.is_none() && along < -40.0 {
            reversed = Some(step);
            break;
        }
    }
    let body = &race.sim.world.ships[0].physics.body;
    assert!(body.forward().dot(forward) < -0.5, "still turned round");
    // The original stopped 0.80 s after the press, counted in its 30 Hz steps. Ours takes about
    // a second, and the page names the gap.
    let stopped = stopped.expect("the engine stops the craft");
    assert!((45..75).contains(&stopped), "stopped after {stopped} ticks");
    assert!(reversed.is_some(), "and drives it back up the circuit");
}
