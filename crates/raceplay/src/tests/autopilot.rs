//! The Autopilot pickup: what it flies for, what it warns, and what cancels it.
//!
//! Split out of `race/tests/weapons.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change. The seam is
//! the obvious one - the Autopilot is the only pickup here that hands the craft
//! to a driver rather than putting something in the air or on the track, and it
//! carries its own fixture table because its `time` has to differ from the
//! Turbo's and the Shield's.

use super::*;
use oag_gameplay::PlayerInputs;

/// How long [`one_autopilot_table`]'s Autopilot flies for, in seconds.
const FIXTURE_AUTOPILOT_TIME: f32 = 3.5;

/// The Autopilot's own fixture, with `time` unlike the Turbo's and the
/// Shield's for the reason [`one_shield_table`] gives. A Rocket is authored but
/// never dealt, so a test that puts one in the slot has something to fire or
/// to pay out, and the pads still only hand out the Autopilot.
fn one_autopilot_table() -> oag_tables::weapons::WeaponStats {
    oag_tables::weapons::parse(
        r#"<WeaponStats>
             <Weapon type="Global"><Stats slowdown_limit="0"/></Weapon>
             <Weapon type="Autopilot"><Stats absorb="4" time="3.5"/></Weapon>
             <Weapon type="Rocket"><Stats absorb="11" blastforce="12" blastradius="13"
               damage="14" slowdown_time="15" venomspeed="600" flashspeed="700"
               rapierspeed="800" phantomspeed="900" launchSpeed="16" spread="0.2"/></Weapon>
             <Pickupodds class="Venom">
               <Weapon type="Autopilot"><Stats ai="1" back="1" front="1" human="1"/></Weapon>
             </Pickupodds>
           </WeaponStats>"#,
    )
    .expect("the fixture table must parse")
}

/// The Autopilot's whole life: armed off the disc's own `time`, flying the
/// craft, warning at one second, and letting go.
///
/// `Autopilot_Fire` (`0x088613bc`) and `Autopilot_Update` (`0x08861404`) are
/// what this pins - see `docs/ghidra/functions/psp-pulse-usa/autopilot.md`. What
/// it does **not** pin is the takeover itself, which is this project's reuse of
/// `oag_ai::Driver` rather than a reading; the assertion below is only that the
/// snapshot stops being consulted, not that the craft flies the way the
/// original's does.
#[test]
fn a_fired_autopilot_flies_the_craft_for_its_authored_duration() {
    let mut race = race_with_weapon_table(
        Mode::SingleRace,
        enveloping_pad(),
        1.0,
        one_autopilot_table(),
    );
    let mut buttons = Buttons::new();

    race.tick(&PlayerInputs::single(buttons.tick(CROSS)));
    assert_eq!(
        race.ship_pickup(),
        Some(oag_tables::weapons::Weapon::Autopilot)
    );
    assert!(
        !race.flown_for_the_player(0),
        "holding one is not the same as having spent it"
    );

    race.tick(&PlayerInputs::single(buttons.tick(CROSS | SQUARE)));
    assert_eq!(race.ship_pickup(), None, "firing must spend the pickup");
    assert!(
        race.flown_for_the_player(0),
        "the craft was not handed over"
    );
    // The authored `time` less the one tick the countdown has already run.
    let armed = race.sim.world.ships[0].autopilot_timer;
    assert!(
        (armed - (FIXTURE_AUTOPILOT_TIME - race.dt())).abs() < 1e-5,
        "expected {FIXTURE_AUTOPILOT_TIME} less one tick, got {armed}"
    );

    // It ends, and the operator's flag is untouched by any of it.
    let mut ticks: u32 = 1;
    while race.sim.world.ships[0].autopilot_timer > 0.0 {
        race.tick(&PlayerInputs::single(buttons.tick(CROSS)));
        ticks += 1;
        assert!(ticks < 600, "the autopilot never let go");
    }
    assert!(!race.flown_for_the_player(0));
    assert!(
        !race.autopilot(),
        "the pickup moved the operator's own switch"
    );
    // Measured in seconds rather than in ticks: the tick count is the file's
    // duration divided by `dt` and rounded up, plus the one the arm itself
    // consumed, and pinning that arithmetic tests the rounding rather than the
    // duration. What matters is that it ran for what the file says.
    let flown = ticks as f32 * race.dt();
    assert!(
        flown >= FIXTURE_AUTOPILOT_TIME && flown < FIXTURE_AUTOPILOT_TIME + 2.0 * race.dt(),
        "flew for {flown} s against the file's {FIXTURE_AUTOPILOT_TIME}"
    );
}

/// The one-second warning is an edge, and it is the only cue this raises.
#[test]
fn the_autopilot_warns_once_a_second_before_it_lets_go() {
    let mut race = race_with_weapon_table(
        Mode::SingleRace,
        enveloping_pad(),
        1.0,
        one_autopilot_table(),
    );
    let mut buttons = Buttons::new();
    race.tick(&PlayerInputs::single(buttons.tick(CROSS)));
    race.tick(&PlayerInputs::single(buttons.tick(CROSS | SQUARE)));

    let mut warnings = 0;
    let mut at = None;
    for tick in 0..600u32 {
        race.tick(&PlayerInputs::single(buttons.tick(CROSS)));
        let raised = race
            .drain_cues()
            .into_iter()
            .filter(|e| e.cue == oag_gameplay::cue::Cue::Disengaging)
            .count();
        if raised > 0 && at.is_none() {
            at = Some(tick);
        }
        warnings += raised;
    }
    // Once, not sixty times: `Autopilot_Update` keeps `craft+0x144` for exactly
    // this, and a level test would say "disengaging" for the whole last second.
    assert_eq!(warnings, 1, "the warning is not an edge");
    // And a second before the end, within the tick the crossing lands on.
    let at = at.expect("no warning at all");
    // The crossing is `now < 1.0 && now + dt > 1.0`, so the remainder at the
    // warning lands inside one tick below a second - never above it.
    let remaining = FIXTURE_AUTOPILOT_TIME - (at + 2) as f32 * race.dt();
    assert!(
        remaining > 1.0 - 2.0 * race.dt() && remaining <= 1.0,
        "warned with {remaining} s left, not one"
    );
}

/// A race one tick past the Autopilot's arming, with the craft handed over and
/// the slot empty: the state every stint is in from its first tick, because
/// `Autopilot_Fire` spends the pickup.
fn race_under_autopilot() -> (Race, Buttons) {
    let mut race = race_with_weapon_table(
        Mode::SingleRace,
        enveloping_pad(),
        1.0,
        one_autopilot_table(),
    );
    let mut buttons = Buttons::new();
    race.tick(&PlayerInputs::single(buttons.tick(CROSS)));
    race.tick(&PlayerInputs::single(buttons.tick(CROSS | SQUARE)));
    assert!(race.flown_for_the_player(0), "the Autopilot did not arm");
    assert_eq!(race.ship_pickup(), None, "arming must spend the pickup");
    // Released first: both buttons are edge-triggered, so a second press
    // without a gap is not a press at all.
    race.tick(&PlayerInputs::single(buttons.tick(CROSS)));
    (race, buttons)
}

/// The absorb press cancels a running Autopilot with nothing held, and the
/// craft goes back to its pilot on the tick after.
///
/// `Ship_AbsorbHeldPickup` tests the running bit before it reads the slot, so
/// the empty slot every stint is in does not stop it. Live on PPSSPP: the
/// timer went to `0.0` on the press frame and the craft was the pad's on the
/// next (`docs/ghidra/functions/psp-pulse-usa/autopilot.md`, "Absorb cancels
/// it, fire does not").
#[test]
fn absorbing_under_autopilot_cancels_it_with_an_empty_slot() {
    let (mut race, mut buttons) = race_under_autopilot();

    race.tick(&PlayerInputs::single(buttons.tick(CROSS | CIRCLE)));
    assert!(!race.flown_for_the_player(0), "absorb did not cancel it");
    assert_eq!(race.sim.world.ships[0].autopilot_timer, 0.0);
    assert_eq!(race.ship_pickup(), None, "the cancel invented a pickup");
}

/// Under the Autopilot the driver's controls fly the press tick and the
/// player's own fly the one after: the controls are chosen before the weapon
/// step, as the original's craft update runs before its weapon update.
#[test]
fn the_craft_is_handed_back_on_the_tick_after_the_absorb() {
    let (mut race, mut buttons) = race_under_autopilot();

    // A hard right on the stick throughout, which only a person's controls obey.
    let mut held = |mask| {
        let mut snapshot = buttons.tick(mask);
        snapshot.stick_x = 1.0;
        PlayerInputs::single(snapshot)
    };
    let pilot = race.last_controls[0].steer_x;
    race.tick(&held(CIRCLE));
    assert_eq!(
        race.last_controls[0].steer_x, pilot,
        "the press tick was already the person's"
    );
    race.tick(&held(0));
    assert!(
        race.last_controls[0].steer_x > pilot,
        "the person did not get the craft back"
    );
}

/// An absorb under the Autopilot only cancels it: a held pickup is neither
/// paid out nor spent, and no energy moves.
#[test]
fn absorbing_under_autopilot_keeps_a_held_pickup() {
    let (mut race, mut buttons) = race_under_autopilot();
    let held = Some(oag_tables::weapons::Weapon::Rocket);
    race.sim.world.ships[0].pickup.weapon = held;
    // Below the cap, so a pay-out would show: the Rocket absorbs for 11.
    race.sim.world.ships[0].physics.shield = 50.0;

    race.tick(&PlayerInputs::single(buttons.tick(CROSS | CIRCLE)));
    assert!(!race.flown_for_the_player(0), "absorb did not cancel it");
    assert_eq!(race.ship_pickup(), held, "the cancel spent the pickup");
    assert_eq!(
        race.sim.world.ships[0].physics.shield, 50.0,
        "the cancel paid the pickup out"
    );
}

/// Fire does nothing while the driver flies: the original passes only the
/// absorb byte into the driver's control record, so the player's fire never
/// reaches its fire path. Live: `SQUARE` for four frames changed nothing, with
/// the slot empty and with an Autopilot held.
#[test]
fn firing_under_autopilot_does_nothing() {
    let (mut race, mut buttons) = race_under_autopilot();
    let held = Some(oag_tables::weapons::Weapon::Rocket);
    race.sim.world.ships[0].pickup.weapon = held;
    let timer = race.sim.world.ships[0].autopilot_timer;

    race.tick(&PlayerInputs::single(buttons.tick(CROSS | SQUARE)));
    assert!(race.flown_for_the_player(0), "firing cancelled it");
    assert_eq!(race.ship_pickup(), held, "the press fired the pickup");
    assert_eq!(
        race.sim.world.projectiles.live(),
        0,
        "a projectile left the craft"
    );
    assert!(race.sim.world.ships[0].autopilot_timer < timer);
}

/// A held Autopilot cannot be fired into a running one: `Ship_FireHeldWeapon`
/// refuses held id `6` while the running bit is set (`0x08844b44`), and the
/// pickup stays held.
#[test]
fn a_held_autopilot_cannot_be_fired_into_a_running_one() {
    let (mut race, mut buttons) = race_under_autopilot();
    let held = Some(oag_tables::weapons::Weapon::Autopilot);
    race.sim.world.ships[0].pickup.weapon = held;
    let timer = race.sim.world.ships[0].autopilot_timer;

    race.tick(&PlayerInputs::single(buttons.tick(CROSS | SQUARE)));
    assert_eq!(race.ship_pickup(), held, "the press spent the held pickup");
    let now = race.sim.world.ships[0].autopilot_timer;
    assert!(
        (now - (timer - race.dt())).abs() < 1e-5,
        "the timer was re-armed or cancelled: {timer} then {now}"
    );
}

/// A pad crossed mid-stint grants nothing: `Ship_TestWeaponPads` raises the
/// grant request only with the running bit clear (`0x0883f0c0`), confidence
/// 80, static.
#[test]
fn no_pad_grant_while_the_autopilot_runs() {
    let (mut race, mut buttons) = race_under_autopilot();
    // Off the pad and back on it: the grant is an entry edge, and the pad has
    // to be ready to stamp again.
    race.sim.weapon_pad_current[0] = None;
    race.sim.weapon_pad_refresh_left[0] = 0.0;

    race.tick(&PlayerInputs::single(buttons.tick(CROSS)));
    assert!(race.flown_for_the_player(0));
    assert_eq!(race.ship_pickup(), None, "a pad granted mid-stint");
    assert!(
        race.sim.weapon_pad_refresh_left[0] > 0.0,
        "the pad was not stamped: the entry never happened"
    );
}

/// The player's engine bits under the Autopilot come from the title's own law,
/// `oag_title::autopilot_engine::AutopilotEngine`, not from a constant: a law
/// unlike any title's (x`1.5`, cap kept) reaches the flags unchanged, and the
/// measured one gives every title's `0xd` (`docs/ghidra/functions/ps3-hdfury-eu/autopilot.md`).
#[test]
fn the_autopilot_engine_bits_follow_the_titles_law() {
    use oag_title::autopilot_engine::{AutopilotEngine, MEASURED};
    use oag_title::pre_race::Sourced;
    let mut race = race_with_a_grid();
    race.sim.world.ships[0].autopilot_timer = 2.0;
    assert_eq!(
        race.sim.autopilot_engine, MEASURED,
        "the synthetic setup carries the measured law"
    );
    let flags = race.autopilot_engine_flags(0);
    assert_eq!(flags.speedup, Some(1.2));
    assert!(flags.uncapped);

    race.sim.autopilot_engine = AutopilotEngine {
        speedup: Sourced::chosen(1.5),
        uncapped: Sourced::chosen(false),
    };
    let flags = race.autopilot_engine_flags(0);
    assert_eq!(flags.speedup, Some(1.5));
    assert!(!flags.uncapped);

    race.sim.world.ships[0].autopilot_timer = 0.0;
    let flags = race.autopilot_engine_flags(0);
    assert_eq!(flags.speedup, None, "no pickup, no multiplier");
}

/// Two people on one grid, the Autopilot running out under slot 1 alone.
///
/// `seat` is the network client's seat (`None` for a machine that flies both).
fn two_people(seat: Option<usize>, remote: bool) -> Race {
    let mut setup = setup(Handling::default());
    setup.mode = Mode::SingleRace;
    setup.start_position = Some(oag_vex::track::StartPosition {
        position: [0.0, 0.0, 0.0],
        left: [0.0, 0.0, -1.0],
        up: [0.0, 1.0, 0.0],
        forward: [1.0, 0.0, 0.0],
    });
    let mut race = Race::start(setup);
    let controller = if remote {
        oag_gameplay::Controller::Remote
    } else {
        oag_gameplay::Controller::Local
    };
    race.sim.world.controllers[1] = controller;
    if remote {
        race.sim.world.controllers[0] = controller;
    }
    race.set_seat(seat);
    race
}

/// Runs `slot`'s Autopilot through its one-second warning and returns the slots
/// that raised [`Cue::Disengaging`].
fn disengaging_from(race: &mut Race, slot: usize) -> Vec<usize> {
    race.sim.world.ships[slot].autopilot_timer = 1.5;
    let mut raised = Vec::new();
    for _ in 0..120 {
        race.tick(&PlayerInputs::none());
        raised.extend(
            race.drain_cues()
                .into_iter()
                .filter(|e| e.cue == oag_gameplay::cue::Cue::Disengaging)
                .map(|e| usize::from(e.slot)),
        );
    }
    raised
}

/// A client seated at slot 0 hears its own call and not another person's: both
/// are person-flown on its copy of the world, and the call is dry, so a cue that
/// reached the mix would be in this player's ear.
#[test]
fn a_clients_seat_hears_only_its_own_disengaging_call() {
    let mut race = two_people(Some(0), true);
    assert_eq!(disengaging_from(&mut race, 1), Vec::<usize>::new());
    let mut race = two_people(Some(0), true);
    assert_eq!(disengaging_from(&mut race, 0), vec![0]);
    let mut race = two_people(Some(1), true);
    assert_eq!(disengaging_from(&mut race, 0), Vec::<usize>::new());
    let mut race = two_people(Some(1), true);
    assert_eq!(disengaging_from(&mut race, 1), vec![1]);
}

/// One machine flying both seats has one speaker: each person's call is heard.
/// Chosen, not measured - HD's `Autopilot_Update` has no seat test of its own.
#[test]
fn a_machine_flying_both_seats_hears_both_calls() {
    let mut race = two_people(None, false);
    assert_eq!(disengaging_from(&mut race, 1), vec![1]);
    let mut race = two_people(None, false);
    assert_eq!(disengaging_from(&mut race, 0), vec![0]);
}

/// The held `~AUTOPILOT` voice follows the same seat: another person's
/// Autopilot does not sound it on a client, and a second local seat's does.
#[test]
fn the_held_autopilot_voice_follows_the_seat() {
    let mut race = two_people(Some(0), true);
    race.sim.world.ships[1].autopilot_timer = 3.0;
    assert!(!race.autopilot_is_active(), "another person's Autopilot");
    race.sim.world.ships[0].autopilot_timer = 3.0;
    assert!(race.autopilot_is_active(), "this seat's own");

    let mut race = two_people(None, false);
    race.sim.world.ships[1].autopilot_timer = 3.0;
    assert!(race.autopilot_is_active(), "a second local seat's");
}
