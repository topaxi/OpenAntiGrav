//! The race hash: what it must see move, and what two identical races must
//! agree on.
//!
//! Split out of `race.rs`'s inline `#[cfg(test)] mod tests` under the 200-line
//! rule in `scripts/check-file-size.py`. Shared fixtures live in the parent
//! `tests.rs`.

use super::*;

/// The pad half of the determinism gate, and the reason it is here rather
/// than in `crates/gameplay/tests/determinism.rs`: the timers live on
/// `Race`, and this fixture is the only one that builds a `Race` **with no
/// disc**, so it is the only one that can run in CI on all three OSes.
///
/// The scenario is `docs/gameplay/pickups.md`'s own worked example of a
/// silent divergence: **a pad refresh timer one tick out**. Nothing about it
/// shows in the craft's dynamics until the pickup itself differs, thousands
/// of ticks later.
#[test]
fn a_pad_refresh_timer_one_tick_out_moves_the_race_hash() {
    let mut race = race_with_weapon_pads(Mode::SingleRace, enveloping_pad(), 1.0);
    let mut buttons = Buttons::new();
    for _ in 0..30 {
        race.tick(&buttons.tick(CROSS));
    }
    let before = race.state_hash();

    // The craft has not moved and nothing it carries has changed - only how
    // long the pad has left to cool down.
    race.weapon_pad_refresh_left[0] -= race.dt();
    assert_ne!(
        before,
        race.state_hash(),
        "a pad timer moved by one tick was invisible to the gate, which is \
         exactly the hole this hash exists to close"
    );
}

/// The broadphase cache is simulation state too: a stale entry changes which
/// tick a pad is next measured on, and therefore which tick it triggers.
#[test]
fn the_pad_distance_cache_moves_the_race_hash() {
    let mut race = race_with_weapon_pads(Mode::SingleRace, enveloping_pad(), 1.0);
    race.tick(&InputSnapshot::default());
    let before = race.state_hash();
    race.weapon_pad_distance[0][0] += 1.0;
    assert_ne!(before, race.state_hash());
}

/// Two races from one seed driven with identical input agree on the whole
/// hash, tick for tick - not only at the end. A run that diverged and
/// converged again would pass a final-state comparison.
#[test]
fn two_identical_races_agree_on_the_race_hash_every_tick() {
    let mut a = race_with_weapon_pads(Mode::SingleRace, enveloping_pad(), 1.0);
    let mut b = race_with_weapon_pads(Mode::SingleRace, enveloping_pad(), 1.0);
    let (mut a_buttons, mut b_buttons) = (Buttons::new(), Buttons::new());

    for tick in 0..240 {
        // A varied script, so the run is not one long coast: fire on tick
        // 5, absorb on 90, thrust throughout.
        let mask = match tick {
            5 => CROSS | SQUARE,
            90 => CROSS | CIRCLE,
            _ => CROSS,
        };
        a.tick(&a_buttons.tick(mask));
        b.tick(&b_buttons.tick(mask));
        assert_eq!(
            a.state_hash(),
            b.state_hash(),
            "two identical races diverged on tick {tick}"
        );
    }
}

/// Firing a pickup moves the hash. Without this the two tests above would
/// pass over a scenario where nothing happened.
#[test]
fn the_race_hash_sees_a_pickup_being_spent() {
    let mut race = race_with_weapon_pads(Mode::SingleRace, enveloping_pad(), 1.0);
    let mut buttons = Buttons::new();
    race.tick(&buttons.tick(CROSS));
    assert!(race.ship_pickup().is_some(), "nothing to spend");

    let held = race.state_hash();
    race.tick(&buttons.tick(CROSS | SQUARE));
    assert_ne!(held, race.state_hash());
}
