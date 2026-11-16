//! The AI field and the race order on a real circuit, out of a real disc image.
//!
//! `#[ignore]`d and never run in CI; it needs a disc image. The shared loaders
//! live in `race_ground_truth.rs`.

use crate::race_ground_truth::{envelope, load_single_race};
use oag_core::math::Vec3;
use oag_gameplay::PlayerInputs;
use oag_raceplay::{self as race};

/// The whole grid lands on the track, in the formation measured off the
/// original, and every craft is over solid geometry.
///
/// The unit tests in `oag_gameplay::spawn` pin the *arithmetic* against an
/// identity node; this pins it against a real track, where each slot is dropped
/// onto its own footprint and the surface is neither flat nor straight over the
/// grid's 138 units. A layout that is right in the abstract and buries the front
/// row is still wrong.
/// The AI's own composition check: eight craft, driven, on a real circuit.
///
/// Everything under this has synthetic tests - `oag-ai`'s controller against a
/// circle, `race::tests` against a straight - and none of them can see the thing
/// that actually goes wrong: whether the racing line built from the *disc's* own
/// `racing_line` offsets runs where the collision geometry is, and whether a
/// craft flown along it stays on the track rather than understeering into the
/// first wall.
///
/// **Deliberately not a lap-time assertion.** Nothing here claims the field is
/// fast, or that it is as fast as the original's - there is no measurement of
/// the original's opponents to compare against, and the speed law is this
/// project's own rather than a recovery. What is asserted is that they left the
/// grid, went the right way, made progress and were still on the track after ten
/// seconds. See `docs/gameplay/ai.md`.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_ai_drives_the_field_along_the_track() {
    let Some(loaded) = load_single_race() else {
        return;
    };
    let bound = envelope(&loaded);
    let mut race = race::Race::start(loaded.setup);
    assert_eq!(
        race.ship_count(),
        8,
        "a single race should field a full grid without an override"
    );

    let start: Vec<_> = race
        .sim
        .world
        .ships
        .iter()
        .map(|ship| ship.physics.body.position)
        .collect();
    let heading: Vec<_> = race
        .sim
        .world
        .ships
        .iter()
        .map(|ship| ship.physics.body.forward())
        .collect();

    // Ten seconds, the same window `a_ship_stays_on_the_track_for_ten_seconds`
    // uses, and with the player released so that what moves is the AI.
    for _ in 0..600 {
        race.tick(&PlayerInputs::none());
    }

    for slot in 1..8 {
        let ship = &race.sim.world.ships[slot];
        let moved = ship.physics.body.position - start[slot];

        assert!(
            moved.length() > 10.0,
            "opponent {slot} moved {:.2} units in ten seconds, so it is not being driven",
            moved.length()
        );
        assert!(
            moved.dot(heading[slot]) > 0.0,
            "opponent {slot} went backwards along the grid's own forward"
        );
        assert!(
            ship.driver.index != 0,
            "opponent {slot}'s driver never left the line's first point"
        );

        let distance = race
            .spline()
            .distance_to(ship.physics.body.position)
            .expect("the track has samples");
        assert!(
            distance < bound,
            "opponent {slot} is {distance:.2} from the track, past the {bound:.2} envelope"
        );
        assert!(
            ship.physics.body.position.is_finite(),
            "opponent {slot} left the world"
        );
    }
}

/// Every craft on the disc's own track burns, trails and boosts on its own.
///
/// The real-data half of the per-craft exhaust; `race::tests`' synthetic half
/// runs on a hand-built straight with no ship model, and so with no nozzle and no
/// pads at all. What only the disc can say is that the effect survives the real
/// article: a real `Engine Flare` locator on a real hull, and the track's own
/// authored speed pads under a field that actually drives over them.
///
/// **Three separate claims**, and the third is the one that used to be false:
///
/// 1. every craft's engine is lit and its ribbon ring is full,
/// 2. no two craft share a ribbon - each one's newest sample is at its own
///    nozzle,
/// 3. an opponent that crosses a speed pad gets a *plume*, not just the force.
///
/// Before 2026-08-11 there was one `Exhaust` and it belonged to slot 0, so the
/// seven opponents drove the whole circuit dark.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_craft_burns_trails_and_boosts_on_its_own() {
    let Some(loaded) = load_single_race() else {
        return;
    };
    let mut race = race::Race::start(loaded.setup);
    assert_eq!(race.ship_count(), 8);
    assert!(
        race.nozzle().is_some(),
        "the disc's hull authors an Engine Flare node, so every craft has a nozzle"
    );
    assert!(
        !race.speedup_pads().is_empty(),
        "the third claim needs a track with authored speed pads"
    );

    // Twenty seconds. Ten is enough for the field to be driving and trailing;
    // reaching a speed pad takes longer, and this is the assertion that has to
    // observe one being crossed rather than assume it.
    //
    // **Watched every tick rather than sampled at the end**, and that is not
    // caution: the AI's throttle is bang-bang against a cornering-speed target,
    // so a craft braking into a corner on the final tick has its engine
    // legitimately off. Whether a craft's flare ever lit is the claim; what it
    // happens to be doing at tick 1,200 is the track's business.
    const TICKS: u32 = 1200;
    let mut lit = [0u32; 8];
    let mut boosted = [false; 8];
    for _ in 0..TICKS {
        race.tick(&PlayerInputs::none());
        for slot in 0..8 {
            lit[slot] += u32::from(race.exhaust_of(slot).engine_on());
            boosted[slot] |= race.exhaust_of(slot).boost_timer() > 0.0;
        }
    }

    let nozzles: Vec<Vec3> = (0..8)
        .map(|slot| race.nozzle_of(slot).expect("every craft has a nozzle"))
        .collect();
    for (slot, lit) in lit.iter().enumerate().skip(1) {
        let exhaust = race.exhaust_of(slot);
        // A quarter of the run, which separates "this craft is racing" from
        // both "its engine never came on" and "one stray tick". The AI holds
        // throttle down every straight and lifts for every corner, so the real
        // figure is far above this on any circuit - and the *final* tick's state
        // is deliberately not asserted at all: a craft braking into a corner at
        // tick 1,200 has its engine legitimately off and its intensity already
        // decayed to zero, which is what this counter replaced.
        assert!(
            *lit > TICKS / 4,
            "opponent {slot}'s engine was on for {lit} of {TICKS} ticks"
        );
        assert!(
            exhaust.trail_ready(),
            "opponent {slot} laid no ribbon in twenty seconds"
        );

        // The head sits on the rim of a cross centred on the craft's own nozzle,
        // so the nearest nozzle is that craft's; a shared `Exhaust` would fail.
        let law = race.trail_law();
        let head = Vec3::from_array(exhaust.trail_vertices(Vec3::X, Vec3::Y, law)[0].position);
        let nearest = (0..8)
            .min_by(|a, b| {
                head.distance(nozzles[*a])
                    .total_cmp(&head.distance(nozzles[*b]))
            })
            .expect("eight craft");
        assert_eq!(
            nearest, slot,
            "opponent {slot}'s ribbon starts nearest slot {nearest}'s craft"
        );
    }

    // At least one *opponent*, which is the claim. Asserted over the field
    // rather than per slot: which craft finds a pad in twenty seconds is a
    // property of the racing line and the track, not something this test gets to
    // decide, and pinning a particular slot would make it a test of the AI's
    // route. The player is excluded outright - it is parked on the grid.
    assert!(
        boosted[1..].iter().any(|seen| *seen),
        "no opponent's plume was ever armed, so a pad's force is still reaching \
         craft its visual does not"
    );
}

/// The field spreads across the track's own AI corridor rather than queueing up
/// on one line.
///
/// **This is the real-data half of the personality work**; the synthetic half is
/// `oag-ai`'s `a_field_of_seeded_drivers_does_not_drive_one_line`, on an oval
/// with an invented corridor. What only real data can say is that the corridor
/// *off the disc* is wide enough to be worth spending and that the bounds were
/// rebased onto the racing line the right way round - a sign error there would
/// put the whole field on one side of the line, which is the same queue moved
/// sideways.
///
/// Signed offsets, in the sample's own lateral axis, measured off each craft's
/// own place on the line. The bound is the corridor's, so a track whose artists
/// left less room asserts less.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_field_spreads_across_the_ai_corridor() {
    let Some(loaded) = load_single_race() else {
        return;
    };
    let mut race = race::Race::start(loaded.setup);
    assert_eq!(race.ship_count(), 8);

    // Ten seconds, the same window the driving test uses: long enough that each
    // craft has left the grid's own lateral stagger behind and settled onto the
    // line it chose.
    for _ in 0..600 {
        race.tick(&PlayerInputs::none());
    }

    let mut offsets = Vec::new();
    let mut room = f32::INFINITY;
    for slot in 1..8 {
        let ship = &race.sim.world.ships[slot];
        let sample = race
            .spline()
            .sample(ship.driver.index as usize)
            .expect("the driver stands on a sample");
        let lateral = Vec3::from_array(sample.lateral).normalize_or_zero();
        let line = Vec3::from_array(sample.pos)
            - oag_vex::track::HOVER_LIFT * Vec3::from_array(sample.down)
            + sample.racing_line * lateral;
        offsets.push((ship.physics.body.position - line).dot(lateral));
        room = room
            .min(sample.ai_bound_right - sample.racing_line)
            .min(sample.racing_line - sample.ai_bound_left);
    }

    let low = offsets.iter().copied().fold(f32::INFINITY, f32::min);
    let high = offsets.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    assert!(
        high - low > room,
        "the field spread {:.2} units across a corridor with {room:.2} to spare either side of \
         the line: {offsets:?}",
        high - low
    );
    // Both sides of it, or the rebasing has a sign error.
    assert!(
        low < 0.0 && high > 0.0,
        "the whole field sits on one side of the line: {offsets:?}"
    );
}

/// Two runs of one race are one race, opponents included.
///
/// The drivers hold their state on the ships and therefore inside the world
/// snapshot, which is the property that makes this true - see
/// `oag_gameplay::Ship::driver`. It is asserted on real geometry because that is
/// where a divergence would come from: the collision query, not the controller.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_driven_field_replays_identically() {
    let (Some(first), Some(second)) = (load_single_race(), load_single_race()) else {
        return;
    };
    let mut first = race::Race::start(first.setup);
    let mut second = race::Race::start(second.setup);

    for _ in 0..300 {
        first.tick(&PlayerInputs::none());
        second.tick(&PlayerInputs::none());
    }

    for slot in 0..8 {
        assert_eq!(
            first.sim.world.ships[slot].physics.body.position,
            second.sim.world.ships[slot].physics.body.position,
            "slot {slot} diverged between two runs of the same race"
        );
        assert_eq!(
            first.sim.world.ships[slot].driver, second.sim.world.ships[slot].driver,
            "slot {slot}'s driver diverged between two runs of the same race"
        );
    }
}

/// The field is placed, which is what makes it a race rather than a parade.
///
/// **The property, not a result.** Nothing here asserts who wins - that is the
/// AI's speed law and this project's own, so a pinned finishing order would be
/// pinning our own arithmetic. What is asserted is that the places form a valid
/// permutation, that the player's place agrees with the table, and that being
/// further round the circuit is what earns a better one.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_field_is_placed_by_how_far_round_it_is() {
    let Some(loaded) = load_single_race() else {
        return;
    };
    let mut race = race::Race::start(loaded.setup);
    for _ in 0..900 {
        race.tick(&PlayerInputs::none());
    }

    let places = race.places();
    let mut seen: Vec<u8> = places.to_vec();
    seen.sort_unstable();
    assert_eq!(
        seen,
        (1..=8).collect::<Vec<u8>>(),
        "the places are not a permutation of 1..=8: {places:?}"
    );
    assert_eq!(race.player_place(), places[0]);

    // The craft in first is the one furthest round, since nobody has finished a
    // three-lap race in fifteen seconds.
    let leader = places
        .iter()
        .position(|&place| place == 1)
        .expect("somebody is first");
    let course = race.course().expect("16_Track closes");
    let furthest = (0..8)
        .max_by(|&a, &b| {
            race.sim.world.ships[a]
                .standing
                .distance(course)
                .total_cmp(&race.sim.world.ships[b].standing.distance(course))
        })
        .expect("eight craft");
    assert_eq!(
        leader, furthest,
        "slot {leader} is placed first but slot {furthest} is further round"
    );
}

/// The place reaches the HUD, which is the only place a player can read it.
///
/// `Race::places` had no reader outside the tests until 2026-08-11 - the table was
/// computed every tick and shown to nobody - so this asserts the *wiring* rather
/// than the ordering: the readout's place is the player's place, the field size is
/// the grid, and both halves of the widget group have a value at the same time.
/// `the_field_is_placed_by_how_far_round_it_is` above is what checks the ordering.
///
/// A single race is the mode that has a field at all, and `Arcade_HUD.xml` is the
/// only shipped layout with the place widgets - see
/// `oag_hud::place_owns_the_anchor`.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_players_place_reaches_the_hud() {
    let Some(loaded) = load_single_race() else {
        return;
    };
    let mut race = race::Race::start(loaded.setup);
    assert_eq!(
        race.ship_count(),
        8,
        "a single race with no field cannot test a place"
    );

    for _ in 0..900 {
        race.tick(&PlayerInputs::none());
    }

    let readout = race.readout();
    let course = race.course().expect("16_Track closes");
    let places = race.places();
    for (slot, place) in places.iter().enumerate() {
        let standing = race.sim.world.ships[slot].standing;
        println!(
            "slot {slot}: place {place}, lap {}, progress {:?}, distance {:.1}",
            standing.lap,
            standing.progress.map(|p| p.round()),
            standing.distance(course)
        );
    }
    println!(
        "the player is {} of {} after {} tick(s)",
        readout.place, readout.ships, 900
    );
    assert_eq!(readout.place, u32::from(race.player_place()));
    assert!(
        (1..=8).contains(&readout.place),
        "the HUD would draw a place of {}, which is not a position in a field of eight",
        readout.place
    );
    assert_eq!(readout.ships, 8);

    // Both halves together or neither: a lone `8` beside the `POS` caption was
    // what this HUD drew before the place was wired up.
    let context = loaded
        .hud
        .context()
        .expect("Arcade_HUD.xml parses; the widget-count test pins it");
    let frame = oag_hud::draw_list(&context, &readout);
    let drawn: Vec<&str> = frame
        .hud_text
        .iter()
        .filter_map(|draw| match draw {
            oag_ui::frontend::Draw::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    println!("the HUD's own font pass draws {drawn:?}");
    assert!(
        drawn.contains(&readout.place.to_string().as_str()),
        "the place is not on screen: {drawn:?}"
    );
    assert!(
        drawn.contains(&"8"),
        "the field size is not on screen: {drawn:?}"
    );
    assert!(
        drawn.contains(&"/"),
        "the separator is not on screen: {drawn:?}"
    );
}

/// An opponent actually fires, on a real circuit.
///
/// **The gate this is really about is the curvature one.** `wants_to_fire`
/// refuses a shot where the road between the firer and its target bends more
/// than `WEAPON_CURVATURE`, and a threshold set too tight would be invisible in
/// every synthetic test - a straight fixture always passes it - while making
/// the whole feature dead on real geometry. Only a real track can say whether
/// the number is a filter or a wall. The synthetic half is `oag-ai`'s
/// `a_driver_fires_at_a_craft_ahead_and_inside_its_cone`.
///
/// It hands the field the trigger rather than waiting for the pickup draw:
/// what is measured is whether the *aiming* gates ever open. Pinned to
/// `FireLaw::Ours`; the law's own check is `opponent_fire_ground_truth.rs`.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn an_opponent_fires_at_a_craft_ahead_on_a_real_circuit() {
    let Some(loaded) = load_single_race() else {
        return;
    };
    let mut race = race::Race::start(loaded.setup);
    race.set_fire_law(race::FireLaw::Ours);
    assert_eq!(race.ship_count(), 8);

    let mut fired = 0usize;
    let mut owners = std::collections::BTreeSet::new();
    // Two minutes, and a rocket handed back every tick a craft is empty, so the
    // question is only ever whether the driver wants to use it.
    for _ in 0..7_200 {
        for slot in 1..8 {
            if race.sim.world.ships[slot].pickup.weapon.is_none() {
                race.sim.world.ships[slot].pickup.weapon =
                    Some(oag_tables::weapons::Weapon::Rocket);
            }
        }
        let before = race
            .sim
            .world
            .projectiles
            .slots
            .iter()
            .filter(|p| p.kind.is_some())
            .count();
        race.tick(&PlayerInputs::none());
        for projectile in race.sim.world.projectiles.slots.iter() {
            if projectile.kind.is_some() && projectile.owner != 0 {
                owners.insert(projectile.owner);
            }
        }
        let after = race
            .sim
            .world
            .projectiles
            .slots
            .iter()
            .filter(|p| p.kind.is_some())
            .count();
        if after > before {
            fired += after - before;
        }
    }

    assert!(
        fired > 0,
        "no opponent fired a rocket in two minutes of a real race, so the \
         aiming gates never open on real geometry"
    );
    assert!(
        !owners.is_empty(),
        "rockets were fired but none was owned by an opponent"
    );
    println!("opponents fired {fired} rockets; owners: {owners:?}");
}
