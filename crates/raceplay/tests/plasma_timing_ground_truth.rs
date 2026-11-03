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

/// Distance of the original's drawn bolt from its craft at the end of each frame from the
/// release frame on, both read at one hook after every update of the frame
/// (`--probe pool` on `ParticleSystem_DrawEmitterPool`: the head's matrix, which
/// `Plasma_BuildDrawMatrices` copies from the bolt's own after its move, against
/// `craft_pos`; 82 frames, `plasma.md`, "the head after the release"). Entry 0 is the
/// release frame: the bolt sits on the craft's weapon node, 4.24 ahead on Assegai, where
/// it charged; it then gains 0.19 a frame on the craft until its first floor ride at
/// entry 4. A pre-move bolt against a post-move craft (the `plasma_fly` probe at
/// `Plasma_Update`'s entry) reads one craft step less, 2.22, 2.41 ...: not one instant.
const ORIGINAL_FROM_CRAFT: [f32; 10] = [
    4.240, 4.431, 4.620, 4.806, 5.788, 6.790, 7.808, 8.841, 9.890, 10.954,
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

/// A bolt's position and its distance from the craft.
type Sample = (oag_core::math::Vec3, f32);

/// The bolt's position and its distance from the craft on each tick after the press, `None`
/// once it is gone, up to `ticks`, plus the tick it was released on.
fn fly(ticks: usize) -> Option<(Vec<Option<Sample>>, Option<usize>)> {
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
        let craft = race.sim.world.ships[0].physics.body.position;
        positions.push(bolt.map(|p| (p.position, (p.position - craft).length())));
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

    let origin = positions[ORIGINAL_RELEASE]
        .expect("the bolt exists on its release tick")
        .0;
    for (update, expected) in ORIGINAL_DISTANCE.iter().enumerate() {
        let at = positions[ORIGINAL_RELEASE + update]
            .unwrap_or_else(|| panic!("the bolt is gone {update} updates after release"))
            .0;
        let flown = (at - origin).length();
        assert!(
            (flown - expected).abs() < 0.25,
            "after {update} updates the bolt has flown {flown:.2} units, the original's \
             {expected:.2}"
        );
    }

    // And where it is against the craft it left, which the head's draw shows: the bolt
    // leaves from the weapon node 4.24 ahead and gains on the craft by the difference of
    // their speeds, as the original's does. Fails with the release on the hold point
    // (2.23: two units short on every row). The first seven frames agree to 0.1; after
    // that the ride's pin is a little short per frame in ours and the gap opens.
    for (frame, expected) in ORIGINAL_FROM_CRAFT.iter().enumerate() {
        let (_, from_craft) = positions[ORIGINAL_RELEASE + frame]
            .unwrap_or_else(|| panic!("the bolt is gone {frame} frames after release"));
        assert!(
            (from_craft - expected).abs() < if frame < 7 { 0.1 } else { 0.2 },
            "{frame} frames after release the bolt is {from_craft:.3} from the craft, the \
             original's {expected:.3}"
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
