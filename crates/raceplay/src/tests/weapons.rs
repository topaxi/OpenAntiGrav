//! Weapon pads, the pickup they hand out, and what firing one does: the
//! turbo, the shield and the rocket.
//!
//! Split out of `race.rs`'s inline `#[cfg(test)] mod tests` under the 200-line
//! rule in `scripts/check-file-size.py`. Shared fixtures live in the parent
//! `tests.rs`; the ones only these tests read are here.

use super::*;
use oag_gameplay::PlayerInputs;

mod projectiles;
mod visuals;

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
fn one_shield_table() -> oag_tables::weapons::WeaponStats {
    oag_tables::weapons::parse(
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

/// The whole grant chain: containment in `oag_vex::pads`, the entry
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
    race.tick(&PlayerInputs::none());
    assert_eq!(race.ship_pickup(), Some(oag_tables::weapons::Weapon::Turbo));
}

/// The mode gate, and it is the one thing about a weapon pad that *is*
/// wholly recovered: a weapons-off race in the original does not ignore a
/// crossing, it empties the trigger list at track load. So the three
/// single-ship modes must grant nothing while standing on a pad forever.
///
/// **About the pad only.** A time trial and a speed lap are handed a free
/// Turbo of their own on the line crossing - see [`Race::grant_free_turbo`] -
/// which this clears anyway so the 120 ticks that follow are a clean test of
/// the pad alone.
#[test]
fn a_weapons_off_mode_never_grants_anything() {
    for mode in [Mode::TimeTrial, Mode::SpeedLap, Mode::Zone] {
        let mut race = race_with_weapon_pads(mode, enveloping_pad(), 1.0);
        race.sim.world.ships[0].pickup.weapon = None;
        for _ in 0..120 {
            race.tick(&PlayerInputs::none());
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
        race.tick(&PlayerInputs::none());
        if race.ship_pickup().is_some() {
            granted += 1;
            race.sim.world.ships[0].pickup.weapon = None;
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
    let place = |race: &mut Race, pad: oag_vex::pads::PadVolume| {
        race.sim.weapon_pads[0] = pad;
        // Slot 0's row: these tests fly the player.
        race.sim.weapon_pad_distance[0][0] = 0.0;
    };

    race.tick(&PlayerInputs::none());
    assert!(race.ship_pickup().is_some(), "the first crossing must pay");
    race.sim.world.ships[0].pickup.weapon = None;

    // Off the pad, then back on, well inside the cooldown.
    place(&mut race, far_away);
    race.tick(&PlayerInputs::none());
    place(&mut race, enveloping_pad()[0]);
    race.tick(&PlayerInputs::none());
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
        race.tick(&PlayerInputs::none());
    }
    place(&mut race, enveloping_pad()[0]);
    race.tick(&PlayerInputs::none());
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
    fired.tick(&PlayerInputs::single(fired_buttons.tick(CROSS)));
    plain.tick(&PlayerInputs::single(plain_buttons.tick(CROSS)));
    assert_eq!(
        fired.ship_pickup(),
        Some(oag_tables::weapons::Weapon::Turbo)
    );

    let _ = fired.drain_cues();
    let boosted = fired.tick(&PlayerInputs::single(fired_buttons.tick(CROSS | SQUARE)));
    let ordinary = plain.tick(&PlayerInputs::single(plain_buttons.tick(CROSS)));
    // `Turbo_Fire` plays `TURBO` through the flare's boost call, as a pad does
    // `SPEEDUPPAD`.
    assert!(
        fired
            .drain_cues()
            .iter()
            .any(|event| event.cue == oag_sound::sfx::Cue::Turbo),
        "a fired Turbo raised no TURBO cue"
    );

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
        fired.tick(&PlayerInputs::single(fired_buttons.tick(CROSS)));
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

    race.tick(&PlayerInputs::single(buttons.tick(CROSS)));
    assert_eq!(
        race.ship_pickup(),
        Some(oag_tables::weapons::Weapon::Shield),
        "a table weighting Shield alone must hand out a Shield"
    );

    race.tick(&PlayerInputs::single(buttons.tick(CROSS | SQUARE)));
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
        race.tick(&PlayerInputs::single(buttons.tick(CROSS)));
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

/// A second Shield fired into a running one is **wasted**: the timer keeps the
/// remainder it had rather than being refreshed, and the pickup is spent
/// anyway.
///
/// Recovered, and the one shape of this that reads as a bug rather than a rule.
/// `Shield_Fire` (`0x08861568`) does its whole body inside
/// `if ((craft->0x1b8 & 0x10) == 0)` and its `else` arm clears only the fire
/// request; the grant is what clears the held slot, so the pickup is gone either
/// way. See `docs/ghidra/functions/psp-pulse-usa/shield-pickup.md`.
#[test]
fn a_shield_fired_into_a_running_one_is_wasted_rather_than_stacked() {
    let mut race =
        race_with_weapon_table(Mode::SingleRace, enveloping_pad(), 1.0, one_shield_table());
    let mut buttons = Buttons::new();

    race.tick(&PlayerInputs::single(buttons.tick(CROSS)));
    race.tick(&PlayerInputs::single(buttons.tick(CROSS | SQUARE)));
    let after_first = race.ship().physics.shield_pickup_timer;
    assert!(after_first > 0.0, "the first Shield never armed");

    // The slot is written directly rather than waiting for a second grant: the
    // grant refuses to hand out the same weapon twice running (`pickup::draw`),
    // so a pad-driven second Shield would take an unbounded number of ticks and
    // the timer would be most of the way down by then.
    race.tick(&PlayerInputs::single(buttons.tick(CROSS)));
    race.sim.world.ships[0].pickup.weapon = Some(oag_tables::weapons::Weapon::Shield);
    race.tick(&PlayerInputs::single(buttons.tick(CROSS | SQUARE)));

    assert_eq!(
        race.ship_pickup(),
        None,
        "the second Shield must be spent even though it did nothing"
    );
    // Two ticks have passed since the first arm, so the timer must have fallen
    // by exactly those two - not been reset to the authored duration.
    let expected = after_first - 2.0 * race.dt();
    assert!(
        (race.ship().physics.shield_pickup_timer - expected).abs() < 1e-5,
        "the second Shield refreshed the timer: expected {expected}, got {}",
        race.ship().physics.shield_pickup_timer
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

    race.tick(&PlayerInputs::single(buttons.tick(0)));
    assert_eq!(
        race.ship_pickup(),
        Some(oag_tables::weapons::Weapon::Shield)
    );
    // Spend the pool first, or the payment lands against a full one and the
    // recovered clamp hides it.
    race.sim.world.ships[0].physics.shield = 10.0;
    race.tick(&PlayerInputs::single(buttons.tick(CIRCLE)));

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
    use oag_tables::weapons::Weapon;

    for mode in [Mode::TimeTrial, Mode::SpeedLap] {
        let mut race = race_with_weapon_pads(mode, Vec::new(), 1.0);
        race.grant_free_turbo(0);
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
        race.grant_free_turbo(0);
        assert_eq!(race.ship_pickup(), None, "{mode:?} was given a free turbo");
    }

    // A held pickup is kept rather than replaced, so a player who saved last
    // lap's turbo does not lose this lap's for nothing.
    let mut race = race_with_weapon_pads(Mode::TimeTrial, Vec::new(), 1.0);
    race.grant_free_turbo(0);
    race.grant_free_turbo(0);
    assert_eq!(race.ship_pickup(), Some(Weapon::Turbo));

    // And a disc whose weapon table did not load hands out nothing rather
    // than a turbo with no duration behind it.
    let mut setup = setup(hulled_handling());
    setup.mode = Mode::TimeTrial;
    setup.weapons = None;
    let mut race = Race::start(setup);
    race.grant_free_turbo(0);
    assert_eq!(race.ship_pickup(), None);
}

/// The original never holds anything in the pickup slot through the
/// countdown (maintainer side-by-side, 2026-09-07), and nothing at the release
/// either: the grant is on the line crossing (`0x0882ddd8` and its siblings,
/// see [`Race::grant_free_turbo`]), so a craft left on the grid holds nothing
/// long after the release - the 2026-09-30 stationary capture shows no pickup
/// icon in 21 s. Earlier this asserted a Turbo on the release tick.
#[test]
fn a_craft_that_never_reaches_the_line_is_never_given_a_turbo() {
    for mode in [
        Mode::TimeTrial,
        Mode::SpeedLap,
        Mode::Zone,
        Mode::SingleRace,
    ] {
        let mut race = race_with_weapon_pads(mode, Vec::new(), 1.0);
        for _ in 0..oag_race::COUNTDOWN_TICKS + 600 {
            race.tick(&PlayerInputs::none());
            assert_eq!(
                race.ship_pickup(),
                None,
                "{mode:?} held a pickup on tick {}",
                race.sim.world.tick
            );
        }
    }
}

/// Absorbing pays the weapon's own `absorb` into the energy pool, and the
/// clamp is the recovered one: a full pool cannot exceed its maximum.
#[test]
fn absorbing_pays_the_pool_and_never_past_its_maximum() {
    let mut race = race_with_weapon_pads(Mode::SingleRace, enveloping_pad(), 1.0);
    race.tick(&PlayerInputs::none());
    assert_eq!(race.ship_pickup(), Some(oag_tables::weapons::Weapon::Turbo));

    // The pool starts full, so absorbing into it must add nothing at all -
    // which is the clamp under test rather than a missing absorb.
    let max = race.ship().handling.dimensions.shield;
    assert!(max > 0.0, "the fixture must have a pool to fill");
    let mut buttons = Buttons::new();
    buttons.tick(0);
    race.tick(&PlayerInputs::single(buttons.tick(CIRCLE)));
    assert_eq!(race.ship_pickup(), None, "absorbing must spend the pickup");
    assert!(
        race.ship().physics.shield <= max,
        "the pool went past its maximum: {} against {max}",
        race.ship().physics.shield
    );

    // Now with room in the pool, so the payment itself is observable.
    let mut race = race_with_weapon_pads(Mode::SingleRace, enveloping_pad(), 1.0);
    race.sim.world.ships[0].physics.shield = 1.0;
    race.tick(&PlayerInputs::none());
    let before = race.ship().physics.shield;
    let mut buttons = Buttons::new();
    buttons.tick(0);
    race.tick(&PlayerInputs::single(buttons.tick(CIRCLE)));
    assert!(
        race.ship().physics.shield > before,
        "absorbing paid nothing: {} against {before}",
        race.ship().physics.shield
    );
}

/// The HUD readout's own absorb flag - `Readout::shield_absorbing` - mirrors
/// [`Race::absorb_window_active`] the same tick the pickup is spent and
/// clears once [`oag_fx::hull_overlay::WINDOW`] has passed. See
/// `crates/hud/src/shield_tests.rs` for what the readout draws once
/// this is set.
#[test]
fn readout_flags_the_absorb_window_the_same_tick_it_opens() {
    let mut race = race_with_weapon_pads(Mode::SingleRace, enveloping_pad(), 1.0);
    race.tick(&PlayerInputs::none());
    assert!(
        !race.readout().shield_absorbing,
        "granted, not yet absorbed"
    );

    let mut buttons = Buttons::new();
    buttons.tick(0);
    race.tick(&PlayerInputs::single(buttons.tick(CIRCLE)));
    assert!(
        race.readout().shield_absorbing,
        "the tick that spends the pickup must open the window"
    );

    // A second and a bit of ticks at the sim's fixed 60 Hz - comfortably past
    // `hull_overlay::WINDOW`'s one second.
    for _ in 0..70 {
        race.tick(&PlayerInputs::none());
    }
    assert!(
        !race.readout().shield_absorbing,
        "a second later the window has closed"
    );
}

/// Every weapon a pad can hand out has a fire arm on **both** paths.
///
/// # Why this is a tripwire rather than a real assertion
///
/// Neither dispatch can be introspected from a test. The player's
/// (`Race::spend_pickup`) is a `match` whose last arm is `_ => return`, and the
/// opponent's (`Race::spend_opponent_pickup`) is an `==` chain whose fallthrough
/// is the *absorb* branch. So a weapon added to `oag_weapons::pickup::IMPLEMENTED`
/// and nowhere else compiles clean, passes every other test, and reaches a player
/// as a pickup that does nothing when fired and quietly turns into energy for an
/// opponent.
///
/// An earlier comment in `race/weapons.rs` claimed the `_` arm made that "a
/// compile-visible choice". It does not, and this test is what that comment
/// should have been: growing the list fails here, with the sites named.
#[test]
fn every_implemented_weapon_has_a_fire_arm_on_both_paths() {
    use oag_tables::weapons::Weapon;

    // The Autopilot's opponent answer is **absorption, deliberately**: the
    // pickup hands a craft to its driver and an opponent already has one, so
    // there is nothing else it could do. `Race::spend_opponent_pickup`'s doc
    // names it for exactly that reason, so this entry is not the guard being
    // waved through.
    const WIRED: &[Weapon] = &[
        Weapon::Turbo,
        Weapon::Shield,
        Weapon::Rocket,
        Weapon::Missile,
        Weapon::Autopilot,
        Weapon::Mine,
        Weapon::Bomb,
        Weapon::Plasma,
        Weapon::Shuriken,
        Weapon::Cannon,
        Weapon::Quake,
        Weapon::LeachBeam,
        Weapon::Disruptor,
        Weapon::Repulser,
    ];

    assert_eq!(
        oag_weapons::pickup::IMPLEMENTED,
        WIRED,
        "`pickup::IMPLEMENTED` and this list disagree. A pad can now hand out a \
         weapon that may have no effect. Three places grow together:\n  \
         1. `oag_weapons::pickup::IMPLEMENTED`\n  \
         2. the `match` in `Race::spend_pickup` (crates/raceplay/src/weapons.rs)\n  \
         3. the `==` chain in `Race::spend_opponent_pickup` (crates/raceplay/src/field.rs)\n\
         Then update this list."
    );
}

/// A missile with nothing to lock is fired anyway, unguided, and the pickup goes.
///
/// **This test used to assert the opposite**, and the opposite was ours:
/// `Ship_FireHeldWeapon` (`0x08844ae8`) calls `Weapon_RequestFire` on both arms
/// of its lock test - with the target's address when there is one and with a
/// null and an index of `-1` when there is not - and `Weapon_FireMissile`
/// (`0x088685cc`) empties the held-weapon slot before it so much as checks
/// whether the pool has room. See
/// `oag_weapons::projectile::missile::lock`'s "`None` is not a refusal to fire".
///
/// Built on an empty grid so there is provably nothing to lock, and on a table
/// that authors a Missile so the arm cannot pass for the *other* reason it
/// returns early.
#[test]
fn a_missile_with_nothing_to_lock_is_fired_unguided_and_spent() {
    let mut race =
        race_with_weapon_table(Mode::SingleRace, enveloping_pad(), 1.0, one_missile_table());
    race.tick(&PlayerInputs::none());
    race.sim.world.ships[0].pickup.weapon = Some(oag_tables::weapons::Weapon::Missile);

    let mut buttons = Buttons::new();
    buttons.tick(0);
    race.tick(&PlayerInputs::single(buttons.tick(SQUARE)));

    assert_eq!(
        race.sim.world.projectiles.live(),
        1,
        "the press with no lock put nothing in the air"
    );
    let missile = race
        .sim
        .world
        .projectiles
        .slots
        .iter()
        .find(|p| p.kind == Some(oag_tables::weapons::Weapon::Missile))
        .copied()
        .expect("a missile is in the air");
    assert_eq!(
        missile.target, None,
        "an empty grid handed the missile a target to chase"
    );
    assert_eq!(
        race.ship_pickup(),
        None,
        "the pickup survived a shot that did leave the rail"
    );
}

/// An unguided missile fired through the button path ends itself on time.
///
/// **The timer only**, and deliberately: that the detonation spends no blast is
/// a rule of `oag_weapons::projectile` and is asserted there, by
/// `a_self_detonating_missile_damages_nobody_standing_in_it`. What this adds is
/// that a missile fired the way a player fires one - a pad's pickup, a `SQUARE`
/// press, `Race::tick` driving the pool - reaches the same end.
///
/// Recovered from `MissilePool_Update` (`0x08869588`), whose second pass tests
/// `3.0 < age` on every live slot.
#[test]
fn an_unguided_missile_fired_by_hand_ends_itself_on_time() {
    use oag_weapons::projectile::missile::SELF_DETONATE_SECONDS;

    let mut race =
        race_with_weapon_table(Mode::SingleRace, enveloping_pad(), 1.0, one_missile_table());
    race.tick(&PlayerInputs::none());
    race.sim.world.ships[0].pickup.weapon = Some(oag_tables::weapons::Weapon::Missile);

    let mut buttons = Buttons::new();
    buttons.tick(0);
    race.tick(&PlayerInputs::single(buttons.tick(SQUARE)));
    assert_eq!(
        race.sim.world.projectiles.live(),
        1,
        "nothing left the rail"
    );

    // Flown until it goes off, and the tick it goes off on is the assertion.
    // Counted rather than sampled either side of a chosen instant: the age is
    // derived from `MAX_FLIGHT_SECONDS - lifetime` and accumulates a tick's
    // worth of rounding over three seconds, so an exact-tick assertion would be
    // a test of `f32` addition rather than of the rule.
    let mut ticks = 1_usize; // the firing tick already flew it once
    while race.sim.world.projectiles.live() > 0 {
        race.tick(&PlayerInputs::none());
        ticks += 1;
        assert!(
            ticks < 600,
            "the missile outlived the recovered self-detonate timer"
        );
    }

    let flown = ticks as f32 / 60.0;
    assert!(
        (flown - SELF_DETONATE_SECONDS).abs() <= 2.0 / 60.0,
        "the missile flew {flown}s, not the recovered {SELF_DETONATE_SECONDS}s"
    );
}

/// Holding a Missile behind a craft puts the reticle on it and then locks it.
///
/// **The whole lock-on chain through `Race::tick`**, which the unit tests in
/// `race::sight` cannot reach: the held weapon decides there is a lock to take,
/// `Ship_AcquireLock`'s window picks the craft, the camera projects it, and the
/// reticle spends [`sight::HOLD_SECONDS`] closing on it. The tone follows the
/// same three states.
#[test]
fn holding_a_missile_behind_a_craft_locks_it_after_the_recovered_hold() {
    use oag_race::sight;

    let mut race = race_with_a_grid();
    race.sim.weapons = Some(one_missile_table());

    // Slot 1 parked squarely down slot 0's nose, inside the fixture table's
    // 10..400 longitudinal window. Placed rather than driven: this is about the
    // reticle, and an opponent that drove away would be testing the lock's
    // window instead.
    let forward = race.sim.world.ships[0].physics.body.forward();
    let ahead = race.sim.world.ships[0].physics.body.position + forward * 60.0;

    race.sim.world.ships[0].pickup.weapon = Some(oag_tables::weapons::Weapon::Missile);

    let mut states = Vec::new();
    for _ in 0..180 {
        race.sim.world.ships[1].physics.body.position = ahead;
        race.sim.world.ships[1].physics.body.linear_velocity = Vec3::ZERO;
        race.tick(&PlayerInputs::none());
        states.push(race.sight_state());
    }

    assert!(
        states.contains(&sight::State::Seeking),
        "the reticle never found the craft parked down the nose"
    );
    let locked = states
        .iter()
        .position(|&s| s == sight::State::Locked)
        .expect("the reticle never locked");
    let seconds = locked as f32 / 60.0;
    assert!(
        seconds >= 0.8,
        "it locked after {seconds}s, inside the recovered 0.8s hold"
    );
    assert!(race.sight().locked(), "the lock did not stay taken");
}

/// And holding something that does not lock draws no reticle at all.
///
/// The Missile and the LeachBeam are the only two weapons `Ship_AcquireLock`
/// reads distances for; a Rocket has no lock and must not borrow one.
#[test]
fn holding_a_rocket_draws_no_reticle() {
    use oag_race::sight;

    let mut race = race_with_a_grid();
    race.sim.weapons = Some(one_missile_table());
    let forward = race.sim.world.ships[0].physics.body.forward();
    let ahead = race.sim.world.ships[0].physics.body.position + forward * 60.0;
    race.sim.world.ships[0].pickup.weapon = Some(oag_tables::weapons::Weapon::Rocket);

    for _ in 0..180 {
        race.sim.world.ships[1].physics.body.position = ahead;
        race.tick(&PlayerInputs::none());
        assert_eq!(
            race.sight_state(),
            sight::State::Absent,
            "a Rocket put a lock-on reticle on screen"
        );
    }
    assert!(!race.sight().locked());
}

/// The cockpit view swaps the shield's **model**, it does not merely hide the
/// player's hull.
///
/// `ShipShield_Update` (`0x0885e254`) branches on `craft+0x6d` and draws the
/// hull-shaped shell *or* the `vr_shield_cockpit.vex` sphere, clearing the
/// other's draw flag - so exactly one is up. `craft+0x6d` is the same byte
/// `CameraView::draws_own_ship` reads, which is what lets this be asserted
/// without a second source of truth.
///
/// Asserted against the two predicates the frame actually gates on rather than
/// against a rendered pixel, because a headless run has no window and the
/// swap is a branch rather than a look. See
/// `docs/ghidra/functions/psp-pulse-usa/shield-pickup.md`.
#[test]
fn the_cockpit_view_swaps_the_shield_shell_for_its_sphere() {
    use oag_display::display::CameraView;

    let mut race =
        race_with_weapon_table(Mode::SingleRace, enveloping_pad(), 1.0, one_shield_table());
    let mut buttons = Buttons::new();
    race.tick(&PlayerInputs::single(buttons.tick(CROSS)));
    race.tick(&PlayerInputs::single(buttons.tick(CROSS | SQUARE)));
    assert!(race.shield_of(0).visible(), "the shield never came up");

    // External: the hull is drawn and so is the shell around it.
    race.set_camera_view(CameraView::Far);
    assert!(race.draws_own_ship());

    // Internal: neither the hull nor the shell, and the sphere instead - at the
    // recovered `1.8` times the shell's own scale, which is what carries it past
    // a hull the camera is sitting inside.
    race.set_camera_view(CameraView::Internal);
    assert!(!race.draws_own_ship());
    let state = race.shield_of(0);
    assert!(
        (state.cockpit_scale() - state.scale() * oag_render::shield::COCKPIT_SCALE).abs() < 1e-5
    );
    assert!(
        state.cockpit_scale() > state.scale(),
        "the cockpit sphere is not drawn larger than the shell"
    );
}

/// **An opponent that declines a shot keeps the pickup.** It did not, and this
/// is the test that says so.
///
/// `Race::spend_opponent_pickup` is an `&&`-chain whose fallthrough is the
/// *absorb* branch, and every weapon arm in it reads
/// `weapon == X && self.fire_x(...)`. `fire_x` returns `false` for the ordinary
/// case of a driver choosing not to shoot this tick - `Driver::wants_to_fire`
/// rolls against a `TRIGGER_RATE` of `0.05`, so it declines about nineteen
/// ticks in twenty - and a `false` right-hand side falls straight through to
/// `else`, which pays the weapon's `absorb` into the energy pool and clears the
/// slot.
///
/// The comment beside the Rocket's arm claimed the opposite in as many words
/// ("the pickup is kept rather than spent - the same rule the player's path
/// follows"), which is what kept this hidden for so long: the player's path
/// *is* a `match` with early returns and does keep it. The consequence is that
/// an opponent cashes in a Rocket, a Missile or a Mine on the first tick it
/// does not fire, which is nearly always the tick it collected it.
///
/// The field is empty, so there is provably nothing to shoot at and the decline
/// is not a roll that might have gone the other way.
#[test]
fn an_opponent_that_declines_a_shot_does_not_absorb_the_pickup() {
    for weapon in [
        oag_tables::weapons::Weapon::Rocket,
        oag_tables::weapons::Weapon::Mine,
    ] {
        let mut race = race_with_a_grid();
        race.sim.weapons = Some(rear_and_forward_table());

        let slot = 1;
        assert!(race.sim.world.ships[slot].active, "no opponent on the grid");
        race.sim.world.ships[slot].physics.shield = 10.0;
        race.sim.world.ships[slot].pickup.weapon = Some(weapon);

        race.spend_opponent_pickup(
            slot,
            &oag_physics::ShipControls::default(),
            &oag_ai::Field::EMPTY,
        );

        assert_eq!(
            race.sim.world.ships[slot].physics.shield, 10.0,
            "{weapon:?}: the opponent absorbed a pickup it never chose to fire - \
             `spend_opponent_pickup`'s `&&`-chain falls through to the absorb \
             branch whenever a `fire_*` helper declines"
        );
        assert_eq!(
            race.sim.world.ships[slot].pickup.weapon,
            Some(weapon),
            "{weapon:?}: the pickup left the slot without anything being fired"
        );
    }
}

/// A table authoring one forward weapon and one rear one, both weighted, for
/// the test above - which needs a weapon of each shape to show that the
/// fallthrough is the chain's and not one arm's.
fn rear_and_forward_table() -> oag_tables::weapons::WeaponStats {
    oag_tables::weapons::parse(
        r#"<WeaponStats>
             <Weapon type="Global"><Stats slowdown_limit="0"/></Weapon>
             <Weapon type="Rocket"><Stats absorb="23" blastforce="3" blastradius="4" damage="5" slowdown_time="0.5" venomspeed="100" flashspeed="200" rapierspeed="300" phantomspeed="400" launchSpeed="7" spread="0.1"/></Weapon>
             <Weapon type="Mine"><Stats absorb="24" blastforce="5" blastradius="6" damage="7" slowdown_time="0.5" timetodie="8" trigger_radius="2"/></Weapon>
             <Pickupodds class="Venom">
               <Weapon type="Rocket"><Stats ai="1" back="1" front="1" human="1"/></Weapon>
             </Pickupodds>
           </WeaponStats>"#,
    )
    .expect("the fixture table must parse")
}
