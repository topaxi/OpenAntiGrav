//! The LeachBeam's `WO_LEACHBEAM_ENERGY` instance is *killed*, not detached,
//! when the beam retires (and at each re-spawn, which this cannot see: the
//! effect holds about one particle at a time, so a detached old instance and a
//! killed one read the same at the re-spawn tick - measured, 1 before, 1 after,
//! with `detach` swapped in). `#[ignore]`d: it needs a disc
//! image (`just test-data`).
//!
//! `LeachBeam_Advance` and `FUN_08872e64` (the teardown `LeachBeam_UpdatePool`
//! calls on retire) hand the handle to `Psys_ReleaseHandle` with `now == 0`,
//! which destroys the instance and every particle in it - see
//! `docs/ghidra/functions/psp-pulse-usa/particle-system.md`, "Releasing an
//! instance by handle". A detach would leave the old instance's particles to
//! finish their lives, which is what these tests see.

use oag_gameplay::PlayerInputs;
use oag_gameplay::input::{Button, Input};
use oag_raceplay as race;

const WARM_UP_TICKS: u64 = oag_race::COUNTDOWN_TICKS + 60;

fn throttle() -> oag_gameplay::InputSnapshot {
    let mut buttons = Input::new();
    buttons.begin_frame(Button::Cross.bit());
    buttons.begin_frame(Button::Cross.bit());
    oag_gameplay::InputSnapshot {
        buttons,
        ..oag_gameplay::InputSnapshot::new()
    }
}

/// A Pulse race with the player's beam locked on craft 1, `ticks` into it.
fn beamed(ticks: usize) -> Option<race::Race> {
    let image = oag_testdata::image("data/images/pulse-psp-usa.chd")?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        team: Some("Assegai".to_string()),
        ..race::Options::default()
    })
    .expect("loading the race");
    let mut race = race::Race::start(loaded.setup);
    for _ in 0..WARM_UP_TICKS {
        race.tick(&PlayerInputs::single(throttle()));
    }
    assert!(race.force_leach_lock(0, 1), "Pulse authors a LeachBeam");
    for _ in 0..ticks {
        race.tick(&PlayerInputs::single(throttle()));
    }
    Some(race)
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn a_retired_beam_takes_its_energy_particles_with_it() {
    let Some(mut race) = beamed(40) else {
        return;
    };
    let (handle, alive) = race.leach_energy_for_tests();
    assert!(handle.is_some(), "the beam never attached its ENERGY");
    assert!(alive > 0, "the ENERGY effect drew nothing");

    race.sim.world.leach_beam = None;
    race.tick(&PlayerInputs::single(throttle()));
    let (handle, alive) = race.leach_energy_for_tests();
    assert!(handle.is_none(), "the retired beam kept its handle");
    assert_eq!(alive, 0, "a detach would leave these to finish");
}
