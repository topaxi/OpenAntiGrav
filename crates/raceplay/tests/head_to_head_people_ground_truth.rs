//! Head to Head with two people is those two craft and nothing else, and each person's gap
//! readout is measured against the other. Disc-backed: the grid is a circuit's own.
//!
//! The two-person rule is **chosen, not measured**: the original's own two-player Head to
//! Head is unknown (`docs/ghidra/functions/psp-pulse-usa/head2head.md`).
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-raceplay head_to_head_people_ground_truth:: --run-ignored all
//! ```

use oag_gameplay::{Controller, PlayerInputs};
use oag_raceplay::{self as race, Race};
use oag_render::viewport::Cut;

fn two_people(opponents: bool) -> Option<Race> {
    let image = oag_testdata::image("data/images/pulse-psp-usa.chd")?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::Head2Head,
        views: 2,
        // What `Session::prepare` sets for any race of two or more people.
        opponents,
        ..race::Options::default()
    })
    .expect("loading the race");
    let mut race = Race::start(loaded.setup);
    assert_eq!(race.set_viewport_count(2), 2);
    race.set_cut(Cut::Vertical);
    for slot in 0..2 {
        race.sim.world.controllers[slot] = Controller::Local;
    }
    Some(race)
}

/// Two people fly two craft. Fails on the build before this one, which fielded the full
/// grid of eight for any race of several people, and with the second person at slot 1
/// among seven AI craft.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn two_people_race_each_other_and_nobody_else() {
    for opponents in [false, true] {
        let Some(race) = two_people(opponents) else {
            return;
        };
        let world = &race.sim.world;
        assert_eq!(
            world.ship_count, 2,
            "two craft (opponents hatch: {opponents})"
        );
        let active: Vec<usize> = (0..8).filter(|&slot| world.ships[slot].active).collect();
        assert_eq!(
            active,
            vec![0, 1],
            "no third craft (opponents hatch: {opponents})"
        );
        for slot in active {
            assert_eq!(
                world.controllers[slot],
                Controller::Local,
                "slot {slot} is a person"
            );
        }
    }
}

/// One person still races one AI opponent, with or without the hatch: the field of two is
/// the mode's own.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn one_person_races_one_ai_opponent() {
    let Some(image) = oag_testdata::image("data/images/pulse-psp-usa.chd") else {
        return;
    };
    for opponents in [false, true] {
        let loaded = race::load(&race::Options {
            source: image.display().to_string(),
            class: "VENOM".to_string(),
            mode: oag_race::Mode::Head2Head,
            opponents,
            ..race::Options::default()
        })
        .expect("loading the race");
        let race = Race::start(loaded.setup);
        assert_eq!(race.sim.world.ship_count, 2, "hatch: {opponents}");
        assert_eq!(
            race.sim.world.controllers[1],
            Controller::Ai,
            "the opponent is AI"
        );
    }
}

/// Each person's readout is the gap to the other person: the same distance, and the
/// leader is whoever is ahead. The second person starts a row ahead of the first (people
/// take the back rows in order), then the first flies away from an idle second.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn each_person_reads_the_gap_to_the_other_person() {
    let Some(mut race) = two_people(true) else {
        return;
    };
    let read = |race: &Race| {
        let first = race
            .readout_for(0)
            .head_to_head
            .expect("person 1 has a readout");
        let second = race
            .readout_for(1)
            .head_to_head
            .expect("person 2 has a readout");
        (first, second)
    };
    // Standings are measured by the tick, so the first read waits for one.
    race.tick(&PlayerInputs::none());
    let (first, second) = read(&race);
    assert!(first.gap > 0.0, "the two start on different rows");
    assert!(
        (first.gap - second.gap).abs() < 1e-3,
        "one gap seen from both sides: {} and {}",
        first.gap,
        second.gap
    );
    assert_ne!(
        first.player_leads, second.player_leads,
        "exactly one of them leads"
    );
    // A title with no Head to Head readout (Pure, HD) draws the place instead, and a field
    // of two reads `1 / 2` and `2 / 2` from the first tick, not `8 / 2` for the back row.
    let mut places = [race.readout_for(0).place, race.readout_for(1).place];
    places.sort_unstable();
    assert_eq!(places, [1, 2], "two craft, two places");

    // The operator's autopilot flies every local seat, so the second person is handed
    // to the network seat, which sits still with no input: one drives, one waits.
    race.sim.world.controllers[1] = Controller::Remote;
    race.set_autopilot(true);
    let mut gaps = Vec::new();
    for tick in 1..=1200 {
        race.tick(&PlayerInputs::none());
        if tick == 300 || tick == 1200 {
            gaps.push(read(&race));
        }
    }
    for (first, second) in &gaps {
        assert!((first.gap - second.gap).abs() < 1e-3);
        assert_ne!(first.player_leads, second.player_leads);
    }
    let (early, late) = (&gaps[0], &gaps[1]);
    // Tick 300 is still the countdown and the first corner of the flight: the person who
    // started a row ahead is still ahead, and each side reads that from its own seat.
    assert!(
        !early.0.player_leads,
        "person 1 has not passed yet at tick 300"
    );
    assert!(early.1.player_leads, "person 2 still leads at tick 300");
    // By tick 1200 the one who flew has gone, and the two views swapped what they read.
    assert!(late.0.player_leads, "person 1 flew away");
    assert!(!late.1.player_leads);
    assert!(
        late.0.gap > 10.0 * early.0.gap,
        "the gap grows while one of them sits still: {} then {}",
        early.0.gap,
        late.0.gap
    );
}

/// One person's Head to Head is the same race whether or not the `--opponents` escape hatch
/// is set: the mode's field of two is fixed, so the hatch (which fielded seven AI here before
/// two people fielded two craft) leaves the state hash where it was. Relative on purpose: the
/// pilot roster feeds the hash and comes from the player's own config directory, so no constant
/// is pinned. The absolute number was also read on a tree without the two-person rule, equal.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn one_persons_head_to_head_is_the_same_race_with_or_without_the_hatch() {
    let Some(image) = oag_testdata::image("data/images/pulse-psp-usa.chd") else {
        return;
    };
    let hash = |opponents: bool| {
        let loaded = race::load(&race::Options {
            source: image.display().to_string(),
            class: "VENOM".to_string(),
            mode: oag_race::Mode::Head2Head,
            opponents,
            ..race::Options::default()
        })
        .expect("loading the race");
        let mut race = Race::start(loaded.setup);
        race.set_autopilot(true);
        for _ in 0..600 {
            race.tick(&PlayerInputs::none());
        }
        race.sim.state_hash()
    };
    assert_eq!(hash(false), hash(true));
}
