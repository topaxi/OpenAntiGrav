//! Wipeout HD's craft from rest under held throttle reaches the speeds the original does.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this project does not
//! ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-raceplay hd_thrust_ground_truth:: --run-ignored all
//! ```
//!
//! Measured on RPCS3 (`scripts/rpcs3-trace.py`, `verification/scenarios/hd-thrust.inputs`):
//! Racebox Time Trial, Talon's Junction grid slot 0, Venom, Feisar, Pilot Assist off, four runs
//! that agree to `0.01` units/s. The forward speed at game time since the first thrust frame is
//! `53.82` at 1.0 s, `100.48` at 2.0 s, `111.68` at 2.5 s, `118.04` at 3.0 s and `121.52` at
//! 3.5 s (`docs/physics/hd-handling-ground-truth.md`). The late rows are where the rigid body's
//! own `0.01` velocity damping shows: without it this engine reads `101.09`, `112.45` and
//! `118.89` (`docs/ghidra/functions/ps3-hdfury-eu/body-damping.md`).

use oag_gameplay::input::{Button, Input};
use oag_gameplay::{InputSnapshot, PlayerInputs};
use oag_raceplay as race;
use oag_raceplay::Race;

const TRACK: &str = r"Data\Environments\Talons_Junction\track.vex";
/// Thrust starts this many ticks in: past the countdown and the start-boost window, which the
/// original's craft, teleported to the grid after the start, never has.
const ONSET: u64 = 400;

fn cross(down: bool) -> PlayerInputs {
    let mut buttons = Input::EMPTY;
    buttons.begin_frame(if down { Button::Cross.bit() } else { 0 });
    PlayerInputs::single(InputSnapshot {
        buttons,
        ..InputSnapshot::EMPTY
    })
}

fn forward_speed(race: &Race) -> f32 {
    let body = &race.ship().physics.body;
    body.linear_velocity.dot(body.forward())
}

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn held_throttle_from_rest_follows_the_originals_speed_curve() {
    let Some(image) = oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::TimeTrial,
        track: Some(TRACK.to_string()),
        team: Some("Feisar".to_string()),
        ..race::Options::default()
    })
    .expect("loading the race");
    let mut race = Race::start(loaded.setup);
    while race.sim.world.tick < ONSET {
        race.tick(&cross(false));
    }
    // (ticks of thrust, the original's forward speed then).
    let measured = [
        (60, 53.82),
        (120, 100.48),
        (150, 111.68),
        (180, 118.04),
        (210, 121.52),
    ];
    for (ticks, original) in measured {
        while race.sim.world.tick < ONSET + ticks {
            race.tick(&cross(true));
        }
        let ours = forward_speed(&race);
        println!("{ticks} ticks of thrust: ours {ours:.2}, the original {original:.2}");
        // 0.2 units/s: a fifth of what dropping the body damping costs at 2 s and later, and
        // twice the fraction-of-a-frame offset the first second carries.
        assert!(
            (ours - original).abs() < 0.2,
            "after {ticks} ticks of thrust ours reads {ours}, the original {original}"
        );
    }
}
