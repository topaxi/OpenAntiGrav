//! `Race::set_trail_length`: the PSP ribbon's ring takes the length the profile names, on
//! every craft, and fills from the real tick path.

use super::*;
use oag_display::display::TrailLength;
use oag_fx::exhaust::{LONG_TRAIL_SAMPLES, TRAIL_SAMPLES};
use oag_gameplay::PlayerInputs;

/// A grid whose hulls author a nozzle, so every tick pushes a sample.
fn racing_grid() -> Race {
    let mut setup = setup(hulled_handling());
    setup.mode = Mode::SingleRace;
    setup.start_position = Some(oag_vex::track::StartPosition {
        position: [0.0, 0.0, 0.0],
        left: [0.0, 0.0, -1.0],
        up: [0.0, 1.0, 0.0],
        forward: [1.0, 0.0, 0.0],
    });
    setup.nozzles = vec![Some(Vec3::new(0.0, 0.0, -0.8)); MAX_SHIPS];
    let mut race = Race::start(setup);
    race.view.hd_trail_active = false;
    race
}

fn ready_slots(race: &Race) -> usize {
    (0..MAX_SHIPS)
        .filter(|&slot| race.exhaust_of(slot).trail_ready())
        .count()
}

#[test]
fn the_long_length_takes_54_ticks_to_fill_and_the_original_ten() {
    let mut race = racing_grid();
    race.set_trail_length(TrailLength::Hd);
    for tick in 1..=LONG_TRAIL_SAMPLES {
        race.tick(&PlayerInputs::none());
        let ready = ready_slots(&race);
        if tick < LONG_TRAIL_SAMPLES {
            assert_eq!(ready, 0, "a ring of 54 is not full at tick {tick}");
        } else {
            assert_eq!(ready, 8, "every ring is full at tick {tick}");
        }
    }
    // The frame loop sets the length every frame: the full rings stay.
    race.set_trail_length(TrailLength::Hd);
    assert_eq!(ready_slots(&race), 8);

    race.set_trail_length(TrailLength::Original);
    assert_eq!(
        ready_slots(&race),
        0,
        "a change of length restarts the rings"
    );
    for _ in 0..TRAIL_SAMPLES {
        race.tick(&PlayerInputs::none());
    }
    assert_eq!(ready_slots(&race), 8);
}

/// HD's tube has its own length: the profile's value must not resize the PSP rings behind it.
#[test]
fn hds_tube_ignores_the_trail_length() {
    let mut race = racing_grid();
    race.view.hd_trail_active = true;
    race.set_trail_length(TrailLength::Hd);
    for slot in 0..MAX_SHIPS {
        assert_eq!(race.exhaust_of(slot).trail_samples_kept(), TRAIL_SAMPLES);
    }
}
