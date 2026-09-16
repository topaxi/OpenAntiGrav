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
/// Shield's for the reason [`one_shield_table`] gives.
fn one_autopilot_table() -> oag_tables::weapons::WeaponStats {
    oag_tables::weapons::parse(
        r#"<WeaponStats>
             <Weapon type="Global"><Stats slowdown_limit="0"/></Weapon>
             <Weapon type="Autopilot"><Stats absorb="4" time="3.5"/></Weapon>
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
            .filter(|e| e.cue == crate::audio::sfx::Cue::Disengaging)
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

/// Pressing fire while it runs cancels it and fires nothing.
///
/// `FUN_08844ec4` tests the running bit before its held-weapon jump table and
/// zeroes the timer instead of dispatching. The pickup survives, because that
/// path never touches `craft+0x1bc`.
#[test]
fn firing_under_autopilot_cancels_it_and_keeps_the_pickup() {
    let mut race = race_with_weapon_table(
        Mode::SingleRace,
        enveloping_pad(),
        1.0,
        one_autopilot_table(),
    );
    let mut buttons = Buttons::new();
    race.tick(&PlayerInputs::single(buttons.tick(CROSS)));
    race.tick(&PlayerInputs::single(buttons.tick(CROSS | SQUARE)));
    assert!(race.flown_for_the_player(0));

    // A second pickup put in the slot directly - the pad only grants on a *new*
    // entry and the craft is already standing on this one. A fixture, like the
    // shield timer two tests up, not a grant.
    let held = Some(oag_tables::weapons::Weapon::Rocket);
    race.sim.world.ships[0].pickup.weapon = held;
    assert!(
        race.flown_for_the_player(0),
        "it let go before the test began"
    );

    // Released first: the fire button is edge-triggered, so a second press
    // without a gap is not a press at all.
    race.tick(&PlayerInputs::single(buttons.tick(CROSS)));
    race.tick(&PlayerInputs::single(buttons.tick(CROSS | SQUARE)));
    assert!(!race.flown_for_the_player(0), "firing did not cancel it");
    assert_eq!(
        race.ship_pickup(),
        held,
        "the cancel spent the pickup, which the original's branch never touches"
    );
}
