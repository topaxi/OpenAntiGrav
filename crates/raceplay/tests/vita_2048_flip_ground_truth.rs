//! 2048's Eliminator flip on a real race: its own craft author the turn's time.
//!
//! **`#[ignore]`d and never run in CI.** It needs the extracted Vita package.
//!
//! `docs/ghidra/functions/ps3-hdfury-eu/eliminator-flip.md`, "Per title".

use oag_gameplay::PlayerInputs;
use oag_gameplay::input::{Button, Input, InputSnapshot};

fn press_flip() -> PlayerInputs {
    let mut buttons = Input::EMPTY;
    buttons.begin_frame(Button::Flip.bit());
    PlayerInputs::single(InputSnapshot {
        buttons,
        ..InputSnapshot::EMPTY
    })
}

/// `Auricom2048`'s prototype authors `eliminator_flip_time="0.5"`, so the half turn is thirty
/// ticks, not the default twelve.
#[test]
#[ignore = "needs the extracted package under data/extracted/vita/"]
fn a_2048_craft_turns_over_its_own_flip_time() {
    let Some(source) = oag_testdata::exact("data/extracted/vita/PCSF00007") else {
        return;
    };
    let loaded = oag_raceplay::load(&oag_raceplay::Options {
        source: source.display().to_string(),
        class: "VENOM".to_string(),
        team: Some(r"Auricom2048\4".to_string()),
        mode: oag_race::Mode::Eliminator,
        seed: Some(1),
        no_effects: true,
        ..oag_raceplay::Options::default()
    })
    .expect("loading a 2048 Eliminator");
    let mut race = oag_raceplay::Race::start(loaded.setup, loaded.view, loaded.modules);
    assert!(race.core.sim.l1_is_flip(), "2048's L1 is the flip");
    for _ in 0..60 * 6 {
        race.tick(&PlayerInputs::none());
    }
    race.tick(&press_flip());
    assert!(race.core.sim.world.ships[0].physics.flip.active);
    let mut ticks = 1;
    while race.core.sim.world.ships[0].physics.flip.active && ticks < 120 {
        race.tick(&PlayerInputs::none());
        ticks += 1;
    }
    assert_eq!(ticks, 30);
}
