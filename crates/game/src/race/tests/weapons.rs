//! Weapon pads, the pickup they hand out, and what firing one does: the
//! turbo, the shield and the rocket.
//!
//! Split out of `race.rs`'s inline `#[cfg(test)] mod tests` under the 200-line
//! rule in `scripts/check-file-size.py`. Shared fixtures live in the parent
//! `tests.rs`; the ones only these tests read are here.

use super::*;

/// How long [`one_turbo_table`]'s Turbo runs for, in seconds.
const FIXTURE_TURBO_TIME: f32 = 0.75;

/// How long [`one_shield_table`]'s Shield runs for, in seconds.
const FIXTURE_SHIELD_TIME: f32 = 1.25;

/// What [`one_shield_table`]'s Shield pays back when absorbed.
const FIXTURE_SHIELD_ABSORB: f32 = 7.0;

/// The Turbo fixture's twin for the Shield, and the same rules apply: every
/// number invented per ADR-0006, and `absorb` and `time` deliberately unlike
/// each other and unlike the Turbo table's, so a test that confused any two
/// of the four fails rather than passes by coincidence.
fn one_shield_table() -> oag_formats::weapons::WeaponStats {
    oag_formats::weapons::parse(
        r#"<WeaponStats>
             <Weapon type="Global"><Stats slowdown_limit="0"/></Weapon>
             <Weapon type="Shield"><Stats absorb="7" time="1.25"/></Weapon>
             <Pickupodds class="Venom">
               <Weapon type="Shield"><Stats ai="1" back="1" front="1" human="1"/></Weapon>
             </Pickupodds>
           </WeaponStats>"#,
    )
    .expect("the fixture table must parse")
}

/// The whole grant chain: containment in `oag_formats::pads`, the entry
/// edge, the weighted draw out of the authored odds, and the inventory. One
/// test for the same reason the speed pad's is one - every link is worthless
/// without the others.
#[test]
fn crossing_a_weapon_pad_grants_the_pickup_its_class_weights() {
    let mut race = race_with_weapon_pads(Mode::SingleRace, enveloping_pad(), 1.0);
    assert!(
        race.ship_pickup().is_none(),
        "a race must start empty-handed"
    );
    race.tick(&InputSnapshot::default());
    assert_eq!(
        race.ship_pickup(),
        Some(oag_formats::weapons::Weapon::Turbo)
    );
}

/// The mode gate, and it is the one thing about a weapon pad that *is*
/// wholly recovered: a weapons-off race in the original does not ignore a
/// crossing, it empties the trigger list at track load. So the three
/// single-ship modes must grant nothing while standing on a pad forever.
#[test]
fn a_weapons_off_mode_never_grants_anything() {
    for mode in [Mode::TimeTrial, Mode::SpeedLap, Mode::Zone] {
        let mut race = race_with_weapon_pads(mode, enveloping_pad(), 1.0);
        for _ in 0..120 {
            race.tick(&InputSnapshot::default());
        }
        assert!(
            race.ship_pickup().is_none(),
            "{mode:?} handed out a pickup, and the original arms no pads for it"
        );
    }
}

/// One crossing is one pickup. The ship sits inside an enveloping pad for
/// two seconds, spending what it is given every tick; without the entry
/// edge and the refresh stamp it would be handed one on every tick it is
/// inside.
#[test]
fn sitting_on_a_weapon_pad_does_not_refill_the_slot() {
    let mut race = race_with_weapon_pads(Mode::SingleRace, enveloping_pad(), 1.0);
    let mut granted = 0;
    for _ in 0..120 {
        race.tick(&InputSnapshot::default());
        if race.ship_pickup().is_some() {
            granted += 1;
            race.world.ships[0].pickup.weapon = None;
        }
    }
    assert_eq!(granted, 1, "the pad fired {granted} times for one crossing");
}

/// **Leaving a pad and coming back before its timer expires gets nothing**,
/// and coming back after it does gets a second pickup.
///
/// The case `sitting_on_a_weapon_pad_does_not_refill_the_slot` cannot
/// reach: an enveloping pad never lets the ship out, so that test exercises
/// the entry edge and never the cooldown.
///
/// **The pad is moved rather than the ship**, because there is no public way
/// to teleport a craft mid-race and driving one out of an enveloping volume
/// is not possible by construction. Two consequences the first draft of this
/// test got wrong, both worth knowing:
///
/// - **Emptying the pad list is not the same as leaving a pad.**
///   [`Race::test_weapon_pads`] returns before the countdown when there is
///   nothing to test, which is right - the original runs the timer as the
///   pad class's own `update` method, so no pads means no timers to run -
///   but it means a test that clears the list freezes every cooldown.
/// - **The broadphase assumes a pad does not move.** `weapon_pad_distance`
///   caches how far the craft was from each pad and spends that distance
///   before testing again, so a pad teleported back under the ship is
///   skipped for as many ticks as the fake distance bought. `Pad_Bind`
///   zeroes the same cache at load; a test that moves a pad has to do the
///   same, which is what the assignment below is.
#[test]
fn a_pad_re_entered_within_its_cooldown_hands_out_nothing() {
    // A whole second, so a handful of ticks cannot run it out by accident.
    let mut race = race_with_weapon_pads(Mode::SingleRace, enveloping_pad(), 1.0);
    let far_away = pad_at(Vec3::splat(1.0e6), 1.0);

    // Moves the pad under the ship or a long way from it, invalidating the
    // broadphase either way.
    let place = |race: &mut Race, pad: oag_formats::pads::PadVolume| {
        race.weapon_pads[0] = pad;
        // Slot 0's row: these tests fly the player.
        race.weapon_pad_distance[0][0] = 0.0;
    };

    race.tick(&InputSnapshot::default());
    assert!(race.ship_pickup().is_some(), "the first crossing must pay");
    race.world.ships[0].pickup.weapon = None;

    // Off the pad, then back on, well inside the cooldown.
    place(&mut race, far_away);
    race.tick(&InputSnapshot::default());
    place(&mut race, enveloping_pad()[0]);
    race.tick(&InputSnapshot::default());
    assert_eq!(
        race.ship_pickup(),
        None,
        "a pad re-entered inside its refresh time paid a second time"
    );

    // And once the timer has run out it pays again. Away while it expires,
    // so the return is a fresh entry edge rather than a craft that never
    // left - and the timer keeps running, because the list is not empty.
    place(&mut race, far_away);
    for _ in 0..90 {
        race.tick(&InputSnapshot::default());
    }
    place(&mut race, enveloping_pad()[0]);
    race.tick(&InputSnapshot::default());
    assert!(
        race.ship_pickup().is_some(),
        "the pad never became collectable again"
    );
}

/// Firing a Turbo sets the timer from the file's own `<Turbo time>` and the
/// engine adds the class's own `<Engine turbo>` while it runs. Read as a
/// *difference* against an identical race that never fired, because the
/// absolute thrust depends on the whole force law.
///
/// **The add is uncapped, and that is the point.** An earlier version of
/// this wired the `1.2` multiplier from the neighbouring branch instead and
/// asserted exactly that ratio - which passed, because a ratio is a ratio.
/// What it could not catch is that `1.2` applied to a thrust already clamped
/// to `0.5 * speed + accelcap` is a few units of force and is imperceptible
/// in a race. Asserting the *magnitude* against the authored tunable is what
/// makes this test able to tell a turbo from a nudge.
#[test]
fn a_fired_turbo_multiplies_thrust_for_its_authored_duration() {
    let mut fired = race_with_weapon_pads(Mode::SingleRace, enveloping_pad(), 1.0);
    let mut plain = race_with_weapon_pads(Mode::SingleRace, enveloping_pad(), 1.0);
    let mut fired_buttons = Buttons::new();
    let mut plain_buttons = Buttons::new();

    // Both take the pickup on tick 1 and only one of them fires it.
    fired.tick(&fired_buttons.tick(CROSS));
    plain.tick(&plain_buttons.tick(CROSS));
    assert_eq!(
        fired.ship_pickup(),
        Some(oag_formats::weapons::Weapon::Turbo)
    );

    let boosted = fired.tick(&fired_buttons.tick(CROSS | SQUARE));
    let ordinary = plain.tick(&plain_buttons.tick(CROSS));

    assert!(
        fired.ship().physics.turbo_timer > 0.0,
        "firing must arm the timer"
    );
    assert_eq!(fired.ship_pickup(), None, "firing must spend the pickup");
    assert!(
        boosted.engine.thrust > ordinary.engine.thrust,
        "a fired turbo must add thrust: {} against {}",
        boosted.engine.thrust,
        ordinary.engine.thrust
    );
    // Exactly the authored `<Engine turbo>`, doubled by the engine's own
    // fixed `* 2.0` - the two races differ in nothing else on this tick, so
    // the whole difference is the add.
    let turbo = fired.ship().handling.engine.turbo;
    assert!(turbo > 0.0, "the fixture must author a turbo to add");
    assert!(
        (boosted.engine.thrust
            - (ordinary.engine.thrust + turbo * oag_physics::engine::ENGINE_OUTPUT_DOUBLE))
            .abs()
            < 1e-3,
        "expected the authored turbo added uncapped: {} against {}",
        boosted.engine.thrust,
        ordinary.engine.thrust
    );

    // And it expires on the file's own duration rather than running forever.
    let mut ticks: u32 = 1;
    while fired.ship().physics.turbo_timer > 0.0 {
        fired.tick(&fired_buttons.tick(CROSS));
        ticks += 1;
        assert!(ticks < 600, "the turbo never expired");
    }
    // **Exact, not within one, and the exact answer is one *more* than the
    // arithmetic suggests.** A tolerance of one tick passes whether the
    // countdown runs before or after the force law reads it, which is the
    // thing `oag_physics::engine::advance_turbo` makes a claim about - so a
    // tolerant assertion would leave that claim untested. It ran 46 ticks
    // for 45 ticks' worth of seconds, and the extra one is real: the timer
    // is read before it is decremented, and 45 sequential `f32`
    // subtractions of `1/60` from `0.75` leave a residue above zero, so a
    // forty-sixth read still sees a live boost. Harmless - a sixtieth of a
    // second - and pinned rather than tolerated so that a change to the
    // ordering shows up here.
    let whole = (FIXTURE_TURBO_TIME / fired.dt()).round() as u32;
    assert_eq!(
        ticks,
        whole + 1,
        "the turbo ran {ticks} tick(s) against the authored {whole} plus the \
         float residue"
    );
}

/// The Shield's half of the same chain: a pad hands one out, `SQUARE` spends
/// it, and the timer comes from the file's own `<Shield time>` rather than
/// from anything invented in this crate.
///
/// **What a running shield then does is pinned in
/// `oag_physics::damage`'s own tests**, not here, and the split is
/// deliberate: that is where the wall contact and the energy pool live, and
/// reproducing a scrape in this fixture would test the collision geometry
/// rather than the wiring. What this test owns is the wiring - that the
/// button reaches the right field with the disc's number in it, and that the
/// slot empties.
#[test]
fn a_fired_shield_arms_the_timer_for_its_authored_duration() {
    let mut race =
        race_with_weapon_table(Mode::SingleRace, enveloping_pad(), 1.0, one_shield_table());
    let mut buttons = Buttons::new();

    race.tick(&buttons.tick(CROSS));
    assert_eq!(
        race.ship_pickup(),
        Some(oag_formats::weapons::Weapon::Shield),
        "a table weighting Shield alone must hand out a Shield"
    );

    race.tick(&buttons.tick(CROSS | SQUARE));
    assert_eq!(race.ship_pickup(), None, "firing must spend the pickup");
    // The authored `time`, less the one tick `crate::step` has already
    // counted off - the countdown runs after the damage path reads it, which
    // is what protects the tick a shield is fired on.
    assert!(
        (race.ship().physics.shield_pickup_timer - (FIXTURE_SHIELD_TIME - race.dt())).abs() < 1e-5,
        "expected the authored {FIXTURE_SHIELD_TIME} less one tick, got {}",
        race.ship().physics.shield_pickup_timer
    );

    // And it expires on the file's own duration, one tick longer than the
    // arithmetic, for the reason `oag_physics::damage::advance_shield_pickup`
    // gives and `a_fired_turbo_multiplies_thrust_for_its_authored_duration`
    // spells out at length.
    let mut ticks: u32 = 1;
    while race.ship().physics.shield_pickup_timer > 0.0 {
        race.tick(&buttons.tick(CROSS));
        ticks += 1;
        assert!(ticks < 600, "the shield never expired");
    }
    let whole = (FIXTURE_SHIELD_TIME / race.dt()).round() as u32;
    assert_eq!(
        ticks,
        whole + 1,
        "the shield ran {ticks} against {whole} + 1"
    );
}

/// Absorbing a Shield pays its own `absorb` rather than the Turbo's, which
/// is the one thing that could quietly cross over now that two weapons are
/// authored with two different pairs.
#[test]
fn absorbing_a_shield_pays_the_shields_own_absorb() {
    let mut race =
        race_with_weapon_table(Mode::SingleRace, enveloping_pad(), 1.0, one_shield_table());
    let mut buttons = Buttons::new();

    race.tick(&buttons.tick(0));
    assert_eq!(
        race.ship_pickup(),
        Some(oag_formats::weapons::Weapon::Shield)
    );
    // Spend the pool first, or the payment lands against a full one and the
    // recovered clamp hides it.
    race.world.ships[0].physics.shield = 10.0;
    race.tick(&buttons.tick(CIRCLE));

    assert_eq!(race.ship_pickup(), None, "absorbing must spend the pickup");
    assert!(
        (race.ship().physics.shield - (10.0 + FIXTURE_SHIELD_ABSORB)).abs() < 1e-4,
        "expected the shield's own absorb of {FIXTURE_SHIELD_ABSORB}, pool is {}",
        race.ship().physics.shield
    );
    assert_eq!(
        race.ship().physics.shield_pickup_timer,
        0.0,
        "absorbing must not also raise the shield"
    );
}

/// Firing a Rocket puts **three** in the air at once, ahead of the craft,
/// at the class's own speed - and spends the pickup.
///
/// Three is recovered, not chosen: `Weapon_FireRocket` (`0x0886e104`) makes
/// three literal spawn calls in one invocation. See
/// `docs/ghidra/functions/psp-pulse-usa/weapon-fire.md`.
///
/// **The fan's geometry is pinned in `oag_gameplay::projectile`'s own
/// tests**, the same split the Shield's wiring test uses. What this owns is
/// the wiring: that the button reaches the array, three times, with the
/// disc's numbers, and that the slot empties.
#[test]
fn a_fired_rocket_puts_three_projectiles_in_the_air() {
    let mut race =
        race_with_weapon_table(Mode::SingleRace, enveloping_pad(), 1.0, one_rocket_table());
    let mut buttons = Buttons::new();

    race.tick(&buttons.tick(0));
    assert_eq!(
        race.ship_pickup(),
        Some(oag_formats::weapons::Weapon::Rocket)
    );
    assert_eq!(race.world.projectiles.live(), 0, "nothing before firing");

    let before = race.ship().physics.body.position;
    let forward = race.ship().physics.body.forward();
    race.tick(&buttons.tick(SQUARE));

    assert_eq!(race.ship_pickup(), None, "firing must spend the pickup");
    assert_eq!(
        race.world.projectiles.live(),
        oag_gameplay::projectile::ROCKET_SHOTS,
        "one press is a volley of three"
    );

    for slot in 0..oag_gameplay::projectile::ROCKET_SHOTS {
        let rocket = race.world.projectiles.slots[slot];
        assert_eq!(rocket.kind, Some(oag_formats::weapons::Weapon::Rocket));
        assert_eq!(rocket.owner, 0);
        assert!(
            (rocket.position - before).dot(forward) > 0.0,
            "rocket {slot} spawned behind the craft: {:?}",
            rocket.position
        );
        // Venom's authored speed plus `launchSpeed`, the same for all three:
        // the fan turns them, it does not slow them. Both figures are km/h
        // in the file, so the velocity is the sum over
        // `KMH_PER_UNIT_PER_SECOND` - spelled as the arithmetic so the unit
        // stays legible.
        let expected = (600.0 + 16.0) / oag_gameplay::projectile::KMH_PER_UNIT_PER_SECOND;
        assert!(
            (rocket.velocity.length() - expected).abs() < 1e-2,
            "rocket {slot}: expected (600 + 16) km/h as units per second, got {}",
            rocket.velocity.length()
        );
        assert!(
            rocket.velocity.dot(forward) > 0.0,
            "rocket {slot} must fly forwards"
        );
    }

    // And they really are fanned rather than three copies of one shot,
    // which the count alone would not catch.
    let right = race.ship().physics.body.right();
    let lateral: Vec<f32> = (0..oag_gameplay::projectile::ROCKET_SHOTS)
        .map(|slot| race.world.projectiles.slots[slot].velocity.dot(right))
        .collect();
    assert!(
        lateral.iter().any(|&l| l > 1.0) && lateral.iter().any(|&l| l < -1.0),
        "the volley did not fan: lateral components {lateral:?}"
    );
}

/// The free Turbo a time trial and a speed lap get once per lap, which the
/// disc records twice - in `MSC_EVENT_TT`/`MSC_EVENT_SL` and in
/// `TimeTrial_HUD.xml`'s lone `TurboIcon`. See [`Race::grant_free_turbo`].
///
/// Driven directly rather than by racing a lap: the *edge* it hangs off is
/// `oag_race::Outcome::lap_completed`, which `oag-race`'s own tests cover,
/// and what is worth pinning here is which modes it applies to and what it
/// does to a slot that is already full.
#[test]
fn only_the_two_solo_modes_are_given_a_free_turbo_and_never_two_at_once() {
    use oag_formats::weapons::Weapon;

    for mode in [Mode::TimeTrial, Mode::SpeedLap] {
        let mut race = race_with_weapon_pads(mode, Vec::new(), 1.0);
        race.grant_free_turbo();
        assert_eq!(
            race.ship_pickup(),
            Some(Weapon::Turbo),
            "{mode:?} should be given a free turbo"
        );
    }

    // Zone authors no pickup widget at all and its event text promises
    // nothing; a single race has pads instead.
    for mode in [Mode::Zone, Mode::SingleRace] {
        let mut race = race_with_weapon_pads(mode, Vec::new(), 1.0);
        race.grant_free_turbo();
        assert_eq!(race.ship_pickup(), None, "{mode:?} was given a free turbo");
    }

    // A held pickup is kept rather than replaced, so a player who saved last
    // lap's turbo does not lose this lap's for nothing.
    let mut race = race_with_weapon_pads(Mode::TimeTrial, Vec::new(), 1.0);
    race.grant_free_turbo();
    race.grant_free_turbo();
    assert_eq!(race.ship_pickup(), Some(Weapon::Turbo));

    // And a disc whose weapon table did not load hands out nothing rather
    // than a turbo with no duration behind it.
    let mut setup = setup(hulled_handling());
    setup.mode = Mode::TimeTrial;
    setup.weapons = None;
    let mut race = Race::start(setup);
    race.grant_free_turbo();
    assert_eq!(race.ship_pickup(), None);
}

/// Absorbing pays the weapon's own `absorb` into the energy pool, and the
/// clamp is the recovered one: a full pool cannot exceed its maximum.
#[test]
fn absorbing_pays_the_pool_and_never_past_its_maximum() {
    let mut race = race_with_weapon_pads(Mode::SingleRace, enveloping_pad(), 1.0);
    race.tick(&InputSnapshot::default());
    assert_eq!(
        race.ship_pickup(),
        Some(oag_formats::weapons::Weapon::Turbo)
    );

    // The pool starts full, so absorbing into it must add nothing at all -
    // which is the clamp under test rather than a missing absorb.
    let max = race.ship().handling.dimensions.shield;
    assert!(max > 0.0, "the fixture must have a pool to fill");
    let mut buttons = Buttons::new();
    buttons.tick(0);
    race.tick(&buttons.tick(CIRCLE));
    assert_eq!(race.ship_pickup(), None, "absorbing must spend the pickup");
    assert!(
        race.ship().physics.shield <= max,
        "the pool went past its maximum: {} against {max}",
        race.ship().physics.shield
    );

    // Now with room in the pool, so the payment itself is observable.
    let mut race = race_with_weapon_pads(Mode::SingleRace, enveloping_pad(), 1.0);
    race.world.ships[0].physics.shield = 1.0;
    race.tick(&InputSnapshot::default());
    let before = race.ship().physics.shield;
    let mut buttons = Buttons::new();
    buttons.tick(0);
    race.tick(&buttons.tick(CIRCLE));
    assert!(
        race.ship().physics.shield > before,
        "absorbing paid nothing: {} against {before}",
        race.ship().physics.shield
    );
}
