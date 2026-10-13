//! Where two people start on a real title's grid.
//!
//! **`#[ignore]`d and never run in CI**: it needs disc images in `data/images/`.
//!
//! PS2 Pulse and Wipeout HD/Fury both spawn a two-person race with the first person on the
//! back row and the second on the row before it (`oag_gameplay::rank_from_back`, read off
//! `FUN_0012ad78` and the `SPArcade` race manager `FUN_00061770`; the AI take rows 1 to 6).
//! Pulse PSP and Pure borrow PS2 Pulse's rule, as Omega and 2048 would borrow HD's. A row is
//! read off the craft's distance along the first person's heading, which is the track's own
//! on every start straight.

use oag_raceplay as race;

/// The grid's ships ordered from the back row to the front, read off `race`.
fn back_to_front(race: &race::Race) -> Vec<usize> {
    let ships = &race.sim.world.ships;
    let forward = ships[0].physics.body.orientation * oag_core::math::Vec3::NEG_Z;
    let along = |ship: usize| ships[ship].physics.body.position.dot(forward);
    let mut order: Vec<usize> = (0..usize::from(race.sim.world.ship_count)).collect();
    order.sort_by(|&a, &b| along(a).total_cmp(&along(b)));
    order
}

fn grid_of(image: &str, people: u8) -> Option<Vec<usize>> {
    let path = oag_testdata::image(image)?;
    let loaded = race::load(&race::Options {
        source: path.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        views: people,
        ..race::Options::default()
    })
    .expect("loading the race");
    Some(back_to_front(&race::Race::start(loaded.setup)))
}

const TITLES: [&str; 4] = [
    "data/images/pulse-psp-usa.chd",
    "data/images/pulse-ps2-eu.chd",
    "data/images/pure-psp-eu.chd",
    "data/images/hdfury-ps3-eu-dec.iso",
];

/// One person starts last with the opponents ahead, on every title: the grid every earlier
/// ground truth was built on.
#[test]
#[ignore = "needs disc images in data/images/"]
fn one_person_starts_last_on_every_title() {
    for image in TITLES {
        let Some(order) = grid_of(image, 1) else {
            continue;
        };
        assert_eq!(order, [0, 7, 6, 5, 4, 3, 2, 1], "{image}");
    }
}

/// Two people take the last two rows, the first on the back. Fails if the second person is
/// left on an opponent's front row, which is where it started before 2026-10-13.
#[test]
#[ignore = "needs disc images in data/images/"]
fn two_people_start_on_the_last_two_rows_on_every_title() {
    for image in TITLES {
        let Some(order) = grid_of(image, 2) else {
            continue;
        };
        assert_eq!(order, [0, 1, 7, 6, 5, 4, 3, 2], "{image}");
    }
}
