//! A race started on the real view and on a headless one flies the same simulation: the state
//! hash agrees at every tick of a full `tick`, with a weapon laid and fired along the way. Tests
//! that read only the simulation load `load_sim` and start on `ViewSetup::headless()`
//! (`docs/architecture/workspace-layout.md`, "Dependency decoders without debug assertions, and
//! races that read no picture"); this is the claim they rest on.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this project does not ship.

use oag_gameplay::PlayerInputs;
use oag_gameplay::input::{Button, Input};
use oag_raceplay as race;
use oag_tables::weapons::Weapon;

const TICKS: u32 = 400;

fn snapshot(bits: u32, previous: Option<&Input>) -> oag_gameplay::InputSnapshot {
    let mut buttons = previous.copied().unwrap_or_else(Input::new);
    buttons.begin_frame(bits);
    oag_gameplay::InputSnapshot {
        buttons,
        ..oag_gameplay::InputSnapshot::new()
    }
}

/// Ticks `race` with the autopilot flying, laying `weapon` on every other tick from tick 100, and
/// returns its state hash after each tick.
fn hashes(mut race: race::Race, weapon: Weapon) -> Vec<u64> {
    let mut input: Option<Input> = None;
    let mut out = Vec::new();
    for tick in 0..TICKS {
        race.core.sim.set_autopilot(true);
        if tick >= 100 {
            race.core.sim.world.ships[0].pickup.weapon = Some(weapon);
        }
        let bits = if tick >= 100 && tick % 2 == 0 {
            Button::Cross.bit() | Button::Square.bit()
        } else {
            Button::Cross.bit()
        };
        let snap = snapshot(bits, input.as_ref());
        input = Some(snap.buttons);
        race.tick(&PlayerInputs::single(snap));
        out.push(race.core.sim.state_hash());
    }
    out
}

fn assert_same(source: &std::path::Path, track: &str, weapon: Weapon) {
    let options = race::Options {
        source: source.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        track: Some(track.to_string()),
        ..race::Options::default()
    };
    let drawn = race::load(&options).expect("the full load");
    let drawn = hashes(
        race::Race::start(drawn.setup, drawn.view, drawn.modules),
        weapon,
    );
    let bare = race::load_sim(&options).expect("the simulation half");
    let bare = hashes(
        race::Race::start(
            bare.setup,
            oag_raceplay::ViewSetup::headless(),
            bare.modules,
        ),
        weapon,
    );
    assert_eq!(drawn, bare, "{track} {weapon:?}");
}

#[test]
#[ignore = "needs a disc image or an extraction"]
fn a_headless_view_flies_the_simulation_a_real_one_does_on_pulse() {
    let Some(image) = oag_testdata::image("pulse-psp-eu.chd") else {
        return;
    };
    for weapon in [Weapon::Bomb, Weapon::Mine, Weapon::Rocket] {
        assert_same(&image, r"Data\Environments\03_Track\track.vex", weapon);
    }
}

#[test]
#[ignore = "needs a disc image or an extraction"]
fn a_headless_view_flies_the_simulation_a_real_one_does_on_hd() {
    let Some(image) = oag_testdata::image("hdfury-ps3-eu-dec.iso") else {
        return;
    };
    for weapon in [Weapon::Bomb, Weapon::Mine, Weapon::Rocket] {
        assert_same(&image, "/data/environments/01_vineta_k/track.vex", weapon);
    }
}

#[test]
#[ignore = "needs a disc image or an extraction"]
fn a_headless_view_flies_the_simulation_a_real_one_does_on_omega() {
    let Some(source) = oag_testdata::exact("data/extracted/ps4") else {
        return;
    };
    for weapon in [Weapon::Bomb, Weapon::Mine, Weapon::Rocket] {
        assert_same(&source, r"data\environments2048\sol\track.vex", weapon);
    }
}
