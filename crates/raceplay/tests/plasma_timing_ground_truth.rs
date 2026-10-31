//! The Plasma's release, flight and detonation tick against the original, in the
//! matched setup of `docs/ghidra/functions/psp-pulse-usa/plasma.md` ("the detonation
//! leg").
//!
//! **The setup.** Talon's Junction (`16_Track`), Time Trial, Venom, Assegai, thrust
//! held from the countdown, one fire press on the 394th tick: the file
//! `verification/scenarios/weapon-after-go-matched.inputs` says (393 ticks of `cross`,
//! then `cross square`). The original side is `scripts/psp-weapon-pair.py --probe
//! plasma_fly` on PPSSPP v1.20.4 (three boots agree to 0.05 unit), fire 120 frames after
//! GO at 106.2 u/s.
//!
//! **Counting.** `k` is ticks after the press, the press tick being 0, as the original's
//! frames after the fire are: its `Plasma_Launch` runs on frame 59, its seventeen
//! `Plasma_Update`s on frames 60 to 76, and its `PlasmaBlast_Construct` on frame 76.
//!
//! **What is compared.** The distance of the bolt from its release point after each
//! update, not its absolute position: this engine's craft is 1.1 units ahead of the
//! original's at release (239.54 against 240.67 on x), a craft-start difference this
//! file does not own, and it would show as a constant offset on every row. The tick
//! the bolt ends on needs the craft to be level and is within a tick of it.

use oag_gameplay::PlayerInputs;
use oag_gameplay::input::{Button, Input};
use oag_raceplay as race;
use oag_tables::weapons::Weapon;

/// Distance of the original's bolt from its release point at the entry of each
/// `Plasma_Update` (`--probe plasma_fly`, `vel` field, 3D, one boot of three; the three
/// agree to 0.03). Entry 0 is the release point, entry 16 is after sixteen updates, and
/// the seventeenth update is the one that detonates.
const ORIGINAL_DISTANCE: [f32; 17] = [
    0.0, 2.19, 4.40, 6.60, 9.61, 12.63, 15.67, 18.73, 21.82, 24.89, 28.00, 31.15, 34.26, 37.42,
    40.63, 43.80, 47.01,
];

/// The original's `PlasmaBlast_Construct`, in frames after the press.
const ORIGINAL_DETONATION: usize = 76;
/// The original's `Plasma_Launch`.
const ORIGINAL_RELEASE: usize = 59;

fn snap(input: &mut Input, buttons: u32) -> oag_gameplay::InputSnapshot {
    input.begin_frame(buttons);
    oag_gameplay::InputSnapshot {
        buttons: *input,
        ..oag_gameplay::InputSnapshot::new()
    }
}

/// The bolt's position on each tick after the press, `None` once it is gone, up to
/// `ticks`, plus the tick it was released on.
fn fly(ticks: usize) -> Option<(Vec<Option<oag_core::math::Vec3>>, Option<usize>)> {
    let image = oag_testdata::image("data/images/pulse-psp-usa.chd")?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        team: Some("Assegai".to_string()),
        mode: oag_race::Mode::TimeTrial,
        weapons_override: Some(true),
        track: Some(r"Data\Environments\16_Track\track.vex".to_string()),
        ..race::Options::default()
    })
    .expect("loading Talon's Junction");
    let mut race = race::Race::start(loaded.setup);
    race.sim.world.ships[0].pickup.weapon = Some(Weapon::Plasma);
    let mut input = Input::new();
    let cross = Button::Cross.bit();
    for _ in 0..393 {
        race.tick(&PlayerInputs::single(snap(&mut input, cross)));
    }
    let mut positions = Vec::new();
    let mut released = None;
    for k in 0..ticks {
        let buttons = if k == 0 {
            cross | Button::Square.bit()
        } else {
            cross
        };
        race.tick(&PlayerInputs::single(snap(&mut input, buttons)));
        let bolt = race
            .sim
            .world
            .projectiles
            .slots
            .iter()
            .find(|p| p.kind == Some(Weapon::Plasma) && p.owner == 0);
        if released.is_none() && bolt.is_some_and(|p| p.launch_speed_kmh > 0.0) {
            released = Some(k);
        }
        positions.push(bolt.map(|p| p.position));
    }
    Some((positions, released))
}

/// One fly-through, every claim: the release tick, the distance after each update
/// (which carries the 0.75 x launch speed for three updates, the first ride's pin, the
/// steer toward the ride point and the Plasma's ride height), and the tick the bolt
/// ends on.
#[test]
#[ignore = "needs the Pulse PSP USA image"]
fn the_plasma_releases_flies_and_detonates_on_the_originals_ticks() {
    let Some((positions, released)) = fly(90) else {
        return;
    };
    assert_eq!(
        released,
        Some(ORIGINAL_RELEASE),
        "released on tick {released:?}, the original's Plasma_Launch is frame 59"
    );

    let origin = positions[ORIGINAL_RELEASE].expect("the bolt exists on its release tick");
    for (update, expected) in ORIGINAL_DISTANCE.iter().enumerate() {
        let at = positions[ORIGINAL_RELEASE + update]
            .unwrap_or_else(|| panic!("the bolt is gone {update} updates after release"));
        let flown = (at - origin).length();
        assert!(
            (flown - expected).abs() < 0.25,
            "after {update} updates the bolt has flown {flown:.2} units, the original's \
             {expected:.2}"
        );
    }

    let gone = positions.iter().position(Option::is_none);
    assert_eq!(
        gone,
        Some(ORIGINAL_DETONATION),
        "the bolt ends on tick {gone:?}; the original's PlasmaBlast_Construct is frame \
         76, after 17 updates"
    );
}
