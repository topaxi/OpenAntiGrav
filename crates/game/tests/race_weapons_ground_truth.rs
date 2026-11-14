//! Weapon pads, pickups and a rocket on a real circuit, out of a real disc image.
//!
//! `#[ignore]`d and never run in CI; it needs a disc image. The shared loaders
//! live in `race_ground_truth.rs`.

use crate::race_ground_truth::{image, load};
use oag_core::math::Vec3;
use oag_gameplay::{PlayerInputs, input::Button, input::Input};
use oag_raceplay::{self as race};

/// The weapon pads are drawn where they trigger.
///
/// Two independent decodes of the same nodes have to agree: `mesh::build_weapon_pads`
/// reads them as geometry through the mesh path, and `oag_vex::pads::volumes`
/// reads them as trigger boxes through the payload path. Neither knows about the
/// other, so a transform applied in one and not the other shows up here and
/// nowhere else - and it is exactly the mistake that is easy to make, because a
/// pad's payload is a *mesh* payload and its placement comes from the parent
/// chain rather than from itself.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_weapon_pads_are_drawn_where_they_trigger() {
    let Some(loaded) = load() else { return };
    let model = loaded
        .weapon_pad_model
        .as_ref()
        .expect("16_Track authors Weapon Pad geometry");
    assert!(!model.indices.is_empty());

    let volumes = &loaded.setup.weapon_pads;
    assert!(
        !volumes.is_empty(),
        "the track decodes no weapon pad volumes"
    );
    println!(
        "{} weapon pad volume(s), {} triangles drawn",
        volumes.len(),
        model.indices.len() / 3
    );

    // Every trigger volume has drawn geometry sitting in it. Ten units of slack
    // because a pad's mesh is a flat plate and its volume is a box around the
    // craft that crosses it, so the two are the same *place* rather than the
    // same extent.
    for (index, pad) in volumes.iter().enumerate() {
        let centre = Vec3::from_array(pad.centre());
        let nearest = model
            .vertices
            .iter()
            .map(|v| (Vec3::from_array(v.position) - centre).length())
            .fold(f32::INFINITY, f32::min);
        assert!(
            nearest < 10.0,
            "weapon pad {index} triggers at {centre:?} and the nearest drawn \
             vertex is {nearest:.2} away"
        );
    }
}

/// **The pickup chain, on the disc's own track and the disc's own weapon
/// table.** The unit tests in `oag_raceplay` run it against a synthetic pad
/// and a hand-written table, so they check the logic against this project's
/// reading of both formats. This checks it against the shipped bytes: a real
/// `Weapon Pad` volume out of `16_Track`, real `<Pickupodds>` out of
/// `WeaponStats_Race.xml`, and the real `<WeaponPad refresh_time>` out of
/// `HandlingStats.xml`.
///
/// Three things it pins, in order of how easily each could break silently:
///
/// 1. **A single race arms the pads and the three solo modes do not.** This is
///    the one part of the pickup system that is wholly recovered - a
///    weapons-off race in the original hides every pad and zeroes the trigger
///    list's count - so it is the part with a right answer to fail against.
/// 2. **Crossing one grants a pickup**, which is this project's reading rather
///    than a ported branch: no grant call site has been found. What is checked
///    here is that the chain reaches the inventory at all, not that the
///    original does the same.
/// 3. **The class's authored odds resolve.** `HandlingStats.xml` spells a speed
///    class `VENOM` and `WeaponStats_Race.xml` spells it `Venom`, so a lookup
///    that matched case-sensitively would find no table and hand out nothing -
///    a failure that looks exactly like "the pad did not fire".
///
/// See `docs/gameplay/pickups.md`.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_weapon_pad_on_the_disc_hands_out_a_pickup_in_a_single_race() {
    let Some(image) = image() else { return };

    // Where to put the craft: the first weapon pad the track authors, lifted to
    // the pad's own centre. Taken from a plain load so the position comes from
    // the same decode `the_weapon_pads_are_drawn_where_they_trigger` checks.
    let Some(scouted) = load() else { return };
    let pad = *scouted
        .setup
        .weapon_pads
        .first()
        .expect("16_Track authors weapon pads");
    let centre = Vec3::from_array(pad.centre());
    println!("first weapon pad at {centre:?}");

    let race_at_pad_seeded = |mode: oag_race::Mode, seed: Option<u64>| {
        let loaded = race::load(&race::Options {
            source: image.display().to_string(),
            class: "VENOM".to_string(),
            mode,
            seed,
            pose: Some(race::PoseRequest::SplineAligned {
                position: centre,
                yaw: 0.0,
            }),
            ..race::Options::default()
        })
        .expect("loading the race");
        race::Race::start(loaded.setup)
    };
    let race_at_pad = |mode: oag_race::Mode| race_at_pad_seeded(mode, None);

    // A single race: the craft starts inside the volume, so the first tick is
    // the entry edge.
    let mut single = race_at_pad(oag_race::Mode::SingleRace);
    single.tick(&Default::default());
    let granted = single.ship_pickup();
    let granted = granted.expect(
        "a craft standing on a real weapon pad was handed nothing - either the \
         trigger missed the volume or the class's <Pickupodds> did not resolve",
    );
    assert!(
        oag_weapons::pickup::IMPLEMENTED.contains(&granted),
        "the pad handed out {granted:?}, which nothing can fire or absorb"
    );
    println!("granted {granted:?} off the disc's own odds");

    // **The magnitude check below is about the Turbo specifically**, so it needs
    // a crossing that draws one. That used to be automatic - `IMPLEMENTED` held
    // a single weapon - and stopped being so the moment it held more. Scanning
    // seeds rather than pinning a magic one, because a magic constant would go
    // quietly wrong the next time `IMPLEMENTED` grows and the walk order shifts,
    // which is the failure this whole comment exists to prevent.
    let turbo_seed = (0..64u64).find(|&seed| {
        let mut race = race_at_pad_seeded(oag_race::Mode::SingleRace, Some(seed));
        race.tick(&Default::default());
        race.ship_pickup() == Some(oag_tables::weapons::Weapon::Turbo)
    });
    let turbo_seed = turbo_seed.expect(
        "no seed in 0..64 drew a Turbo from this class's authored odds - either \
         the table weights it at zero or the draw is broken",
    );
    println!("seed {turbo_seed} draws a Turbo");

    // **And firing it visibly accelerates the craft, on the disc's own
    // numbers.** This is the assertion the first version of this feature
    // needed and did not have: it wired the neighbouring `1.2` multiplier
    // instead of the turbo add, every unit test passed because a ratio is a
    // ratio, and the effect was imperceptible in a real race because `1.2`
    // applied to a thrust already clamped to `0.5 * speed + accelcap` is a few
    // units of force. Comparing *speed after a second* against a control makes
    // the magnitude the thing under test rather than the ratio.
    let mut control = race_at_pad_seeded(oag_race::Mode::SingleRace, Some(turbo_seed));
    let mut fired = race_at_pad_seeded(oag_race::Mode::SingleRace, Some(turbo_seed));
    let mut control_buttons = Input::new();
    let mut fired_buttons = Input::new();
    const CROSS: u32 = Button::Cross.bit();
    const SQUARE: u32 = Button::Square.bit();

    let snapshot = |buttons: &mut Input, mask: u32| {
        buttons.begin_frame(mask);
        oag_gameplay::InputSnapshot {
            buttons: *buttons,
            ..Default::default()
        }
    };

    // Both hold thrust; one of them also presses fire on the tick after the
    // pickup lands.
    control.tick(&PlayerInputs::single(snapshot(&mut control_buttons, CROSS)));
    fired.tick(&PlayerInputs::single(snapshot(&mut fired_buttons, CROSS)));
    control.tick(&PlayerInputs::single(snapshot(&mut control_buttons, CROSS)));
    fired.tick(&PlayerInputs::single(snapshot(
        &mut fired_buttons,
        CROSS | SQUARE,
    )));
    assert_eq!(fired.ship_pickup(), None, "firing must spend the pickup");
    assert!(
        fired.ship().physics.turbo_timer > 0.0,
        "firing must arm the turbo"
    );

    for _ in 0..60 {
        control.tick(&PlayerInputs::single(snapshot(&mut control_buttons, CROSS)));
        fired.tick(&PlayerInputs::single(snapshot(&mut fired_buttons, CROSS)));
    }
    let speed = |race: &race::Race| race.ship().physics.body.linear_velocity.length();
    let (boosted, plain) = (speed(&fired), speed(&control));
    println!("after one second: {boosted:.1} boosted against {plain:.1} plain");
    // Twice the control's speed. A deliberately coarse bar: what it is there to
    // reject is a boost that is technically applied and imperceptible, so it
    // has to be far above measurement noise rather than just above zero.
    assert!(
        boosted > plain * 2.0,
        "a fired turbo left the craft at {boosted:.1} against {plain:.1} - that \
         is not a turbo"
    );

    // And the three weapons-off modes hand out nothing from the same spot, for
    // any number of ticks. A time trial and a speed lap are handed a free
    // turbo at the countdown's release edge (`Race::tick`), cleared here first.
    for mode in [
        oag_race::Mode::TimeTrial,
        oag_race::Mode::SpeedLap,
        oag_race::Mode::Zone,
    ] {
        let mut race = race_at_pad(mode);
        race.sim.world.ships[0].pickup.weapon = None;
        for _ in 0..120 {
            race.tick(&Default::default());
        }
        assert_eq!(
            race.ship_pickup(),
            None,
            "{mode:?} handed out a pickup, and the original arms no pads for it"
        );
    }
}

/// A rocket fired on a real track flies and detonates on the track's own
/// geometry.
///
/// # What this is for, and what the unit tests cannot do
///
/// `oag_weapons::projectile`'s own tests fly rockets down a synthetic corridor:
/// one quad, authored by the test, at a distance the test chose. **Every
/// number in them is invented**, including the geometry. This is the only check
/// that a rocket at the disc's own authored speed, launched from the disc's own
/// hull dimensions, meets the disc's own collision soup at all: a launch offset
/// slightly inside the hull, a class speed read from the wrong attribute, or a
/// swept step long enough to tunnel through a real wall would all pass every
/// unit test and fail here.
///
/// The craft stands on the first weapon pad the track authors, as
/// `a_weapon_pad_on_the_disc_hands_out_a_pickup_in_a_single_race` does, and the
/// seed is scanned for a Rocket draw for the reason that test gives at length.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_rocket_fired_on_a_real_track_flies_and_detonates() {
    let Some(image) = image() else { return };
    let Some(scouted) = load() else { return };
    let pad = *scouted
        .setup
        .weapon_pads
        .first()
        .expect("16_Track authors weapon pads");
    let centre = Vec3::from_array(pad.centre());

    let race_at_pad = |seed: u64| {
        let loaded = race::load(&race::Options {
            source: image.display().to_string(),
            class: "VENOM".to_string(),
            mode: oag_race::Mode::SingleRace,
            seed: Some(seed),
            pose: Some(race::PoseRequest::SplineAligned {
                position: centre,
                yaw: 0.0,
            }),
            ..race::Options::default()
        })
        .expect("loading the race");
        race::Race::start(loaded.setup)
    };

    // The class's authored odds decide this, so scan rather than pin - see the
    // Turbo test's own comment.
    let rocket_seed = (0..64u64)
        .find(|&seed| {
            let mut race = race_at_pad(seed);
            race.tick(&Default::default());
            race.ship_pickup() == Some(oag_tables::weapons::Weapon::Rocket)
        })
        .expect(
            "no seed in 0..64 drew a Rocket from this class's authored odds - either the \
             table weights it at zero or the draw is broken",
        );
    println!("seed {rocket_seed} draws a Rocket");

    let mut race = race_at_pad(rocket_seed);
    let mut buttons = Input::new();
    let snapshot = |buttons: &mut Input, mask: u32| {
        buttons.begin_frame(mask);
        oag_gameplay::InputSnapshot {
            buttons: *buttons,
            ..Default::default()
        }
    };
    const SQUARE: u32 = Button::Square.bit();

    race.tick(&PlayerInputs::single(snapshot(&mut buttons, 0)));
    assert_eq!(
        race.ship_pickup(),
        Some(oag_tables::weapons::Weapon::Rocket)
    );

    let muzzle = race.ship().physics.body.position;
    race.tick(&PlayerInputs::single(snapshot(&mut buttons, SQUARE)));
    assert_eq!(race.ship_pickup(), None, "firing must spend the pickup");
    assert_eq!(
        race.sim.world.projectiles.live(),
        oag_weapons::projectile::ROCKET_SHOTS,
        "firing a rocket on a real track did not put a full volley in the air"
    );

    // **On the disc's own `spread`.** It authors `0.05` radians - under three
    // degrees - so the three fly very nearly parallel and diverge by only a few
    // units over the distance to a wall. That is the whole reason this is
    // asserted as a *fan* rather than eyeballed: at this angle a volley and
    // three copies of one shot look identical in a screenshot.
    let right = race.ship().physics.body.right();
    let lateral: Vec<f32> = (0..oag_weapons::projectile::ROCKET_SHOTS)
        .map(|slot| race.sim.world.projectiles.slots[slot].velocity.dot(right))
        .collect();
    println!("lateral velocity components off the disc's own spread: {lateral:?}");
    assert!(
        lateral.iter().any(|&l| l > 0.5) && lateral.iter().any(|&l| l < -0.5),
        "the volley did not fan on the disc's own numbers: {lateral:?}"
    );

    // It has to *travel*, which is what says the class speed was read at all,
    // and it has to stop, which is what says it met the real collision soup
    // rather than flying through it to the lifetime cap.
    //
    // **Measured per shot, not for the volley.** This used to run the clock
    // until the *last* rocket died and then assert that time was under the
    // reap, while only ever measuring slot 0 - two different rockets in one
    // assertion. On this circuit and at this firing point the disc's own
    // `spread` fans the third shot off the side, and it then falls for the
    // full five seconds because there is nothing under it to hit. That is
    // `Rocket_Update`'s own no-hit branch (`velocity.y -= dt * 50.0`, reaped
    // at `ROCKET_LIFETIME_SECONDS`), so the reap is the *correct* end for it -
    // and the failure it produced said "the rocket aged out instead of
    // hitting the track", which sent two passes looking for a speed bug that
    // was never there. The speed is exact: 222.22 units a second, the
    // authored `venomspeed` 800 alone over [`KMH_PER_UNIT_PER_SECOND`]
    // (277.78 with `launchSpeed` 200 added until 2026-10-01).
    let mut ticks = 1;
    let mut last_alive = [0usize; oag_weapons::projectile::ROCKET_SHOTS];
    let mut furthest = 0.0f32;
    let mut travelled = 0.0f32;
    let mut previous = race.sim.world.projectiles.slots[0].position;
    while race.sim.world.projectiles.live() > 0 {
        // Slot 0 is the middle rocket, the one that flies straight - the outer
        // two may detonate a tick either side of it.
        let flying = race.sim.world.projectiles.slots[0].position;
        if race.sim.world.projectiles.slots[0].kind.is_some() {
            furthest = furthest.max((flying - muzzle).length());
            travelled += (flying - previous).length();
            previous = flying;
        }
        for (slot, alive) in last_alive.iter_mut().enumerate() {
            if race.sim.world.projectiles.slots[slot].kind.is_some() {
                *alive = ticks;
            }
        }
        race.tick(&PlayerInputs::single(snapshot(&mut buttons, 0)));
        ticks += 1;
        assert!(ticks < 1200, "a rocket never stopped");
    }
    println!(
        "the middle rocket flew {travelled:.1} units of path, reaching \
         {furthest:.1} from the muzzle; last tick alive per shot: \
         {last_alive:?} of {ticks}"
    );
    assert!(
        furthest > 10.0,
        "the rocket travelled {furthest:.1} units, which is not flight"
    );

    // The reap in ticks, so a shot that ended on geometry is the one that
    // ended sooner than it.
    let reap = (oag_weapons::projectile::MAX_FLIGHT_SECONDS / race.dt()) as usize;
    let stopped = last_alive.iter().filter(|&&t| t < reap).count();

    // The middle shot is aimed down the craft's own forward axis, so it is
    // the one that must meet the circuit. If *this* ages out, either the
    // speed is wrong or the sweep is missing the soup - the two faults this
    // test exists to catch.
    assert!(
        last_alive[0] < reap,
        "the middle rocket aged out after {} tick(s) instead of hitting the \
         track; reap is {reap}",
        last_alive[0]
    );
    // And it must not be alone, or a volley that mostly flies through walls
    // would still pass on its one lucky shot.
    assert!(
        stopped >= 2,
        "only {stopped} of {} rockets met the collision soup: {last_alive:?}",
        oag_weapons::projectile::ROCKET_SHOTS
    );
}
