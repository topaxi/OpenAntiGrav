//! Speed pads: who they boost, for how long, what they arm and what they
//! score. The class gravity scale rides along, because it is measured against
//! the same settled ship.
//!
//! Split out of `race.rs`'s inline `#[cfg(test)] mod tests` under the 200-line
//! rule in `scripts/check-file-size.py`. Shared fixtures live in the parent
//! `tests.rs`.

use super::*;

/// [`race_with_a_grid`] with a speed pad big enough to hold the whole field.
fn grid_on_a_speed_pad() -> Race {
    let mut handling = hulled_handling();
    // Invented and large, the same argument `race_with_pads` makes: the
    // boost has to be unmistakable rather than lost in the rest of the force
    // law. Not a shipped value - ADR-0006.
    handling.speedup_pads = oag_physics::params::SpeedupPads {
        amount: 37.0,
        time: 0.5,
    };
    let mut setup = setup(handling);
    setup.mode = Mode::SingleRace;
    setup.speedup_pads = enveloping_pad();
    setup.start_position = Some(oag_formats::track::StartPosition {
        position: [0.0, 0.0, 0.0],
        left: [0.0, 0.0, -1.0],
        up: [0.0, 1.0, 0.0],
        forward: [1.0, 0.0, 0.0],
    });
    Race::start(setup)
}

/// A speed pad is a pad for everybody, which it was not until 2026-08-11:
/// the sweep was built around the player's own position and an opponent was
/// the one thing on the circuit a pad could not touch.
#[test]
fn a_speed_pad_boosts_every_craft_and_not_only_the_player() {
    let mut race = grid_on_a_speed_pad();
    race.tick(&InputSnapshot::default());

    for slot in 0..8 {
        assert!(
            race.world.ships[slot].physics.pad_timer > 0.0,
            "slot {slot} crossed the pad and was not boosted"
        );
    }
}

/// The pad state is per racer, and one craft's row does not move another's.
///
/// The structural half - a row per slot - is what stops a pad the player has
/// just measured being skipped for everybody else. The behavioural half
/// perturbs one row and shows the others are untouched, because on an
/// enveloping pad *every* craft reads `Some(0)` whether the rows are shared
/// or not, so "they all know they are on it" discriminates nothing.
#[test]
fn one_craft_s_pad_row_does_not_move_another_s() {
    let mut race = grid_on_a_speed_pad();
    race.tick(&InputSnapshot::default());
    assert_eq!(race.pad_distance.len(), MAX_SHIPS);

    let others: Vec<_> = (1..8).map(|slot| race.pad_distance[slot][0]).collect();
    race.pad_distance[0][0] += 1234.0;
    for (index, slot) in (1..8).enumerate() {
        assert_eq!(
            race.pad_distance[slot][0], others[index],
            "moving slot 0's row moved slot {slot}'s"
        );
    }
    for slot in 0..8 {
        assert_eq!(
            race.pad_current[slot],
            Some(0),
            "slot {slot} does not know it is on the pad"
        );
    }
}

/// A speed pad arms the plume of the craft that crossed it, all eight of them.
///
/// Until 2026-08-11 there was one `Exhaust` and it was slot 0's, so an
/// opponent got the pad's *force* and nothing on screen. This is the visual
/// half of `a_speed_pad_boosts_every_craft_and_not_only_the_player`, and it is
/// a separate assertion on purpose: the force reaching a craft and its flare
/// being armed are two writes at the same site, and the plume was the one that
/// used to be missing.
#[test]
fn a_speed_pad_arms_every_craft_s_own_plume() {
    let mut race = grid_on_a_speed_pad();
    race.tick(&InputSnapshot::default());

    for slot in 0..8 {
        assert!(
            race.exhaust_of(slot).boost_timer() > 0.0,
            "slot {slot} crossed the pad and its plume was not armed"
        );
    }
}

#[test]
fn a_track_with_no_pads_never_boosts() {
    let mut race = race_with_pads(Mode::TimeTrial, Vec::new());
    for _ in 0..120 {
        let evaluated = race.tick(&InputSnapshot::default());
        assert_eq!(evaluated.speedup_pad, Vec3::ZERO);
    }
    assert_eq!(race.ship().physics.pad_timer, 0.0);
}

/// The whole chain, end to end: containment in `oag_formats::pads`, the
/// direction off the pad's matrix, `Environment::pad_hit`, and the force term
/// in `oag_physics`. It is deliberately one test, because each link is
/// worthless without the others and a failure anywhere reads the same way.
#[test]
fn a_ship_inside_a_pad_is_pushed_along_the_pads_own_axis() {
    let mut race = race_with_pads(Mode::TimeTrial, enveloping_pad());
    let evaluated = race.tick(&InputSnapshot::default());

    assert_ne!(evaluated.speedup_pad, Vec3::ZERO, "no boost was applied");
    assert_eq!(
        evaluated.speedup_pad.normalize(),
        Vec3::Z,
        "the boost must follow row 2 of the pad's matrix"
    );
    assert!(race.ship().physics.pad_timer > 0.0);
}

/// Leaving the pad does not end the boost, it starts the countdown. This is
/// the re-arm semantics `ShipState::pad_timer` documents, seen from outside.
#[test]
fn the_boost_outlives_the_pad_and_then_expires() {
    let mut race = race_with_pads(Mode::TimeTrial, vec![pad_at(Vec3::ZERO, 1.0e6)]);
    race.tick(&InputSnapshot::default());
    assert!(race.ship().physics.pad_timer > 0.0);

    // Take the pad away, which is the same to the trigger as driving off it.
    race.speedup_pads.clear();
    let mut boosted_ticks = 0;
    for _ in 0..120 {
        if race.tick(&InputSnapshot::default()).speedup_pad != Vec3::ZERO {
            boosted_ticks += 1;
        }
    }
    // `time` is 0.5 s at 60 Hz, so about thirty ticks - asserted as a range
    // rather than a count, because the exact tick the timer crosses zero on
    // is float arithmetic and not the thing under test.
    assert!(
        (25..=35).contains(&boosted_ticks),
        "{boosted_ticks} ticks of boost after leaving the pad"
    );
    assert_eq!(
        race.ship().physics.pad_timer,
        0.0,
        "the timer never expired"
    );
}

/// `<GravityMul airborne>` must reach the gravity term, and it must reach the
/// **grounded** half of it.
///
/// The name says airborne and the VFPU pair chain puts it on the grounded
/// lane - see `oag_formats::handling::GravityMul`. So this asserts the
/// counter-intuitive half: a heavier scale changes a ship resting on the
/// ground, and the identity leaves the term exactly as it was before the
/// value was decoded.
#[test]
fn the_class_gravity_scale_reaches_the_grounded_half_of_gravity() {
    fn settled_gravity(scale: f32) -> f32 {
        let mut handling = hulled_handling();
        // A non-zero `normal_gravity`, or the scale has nothing to multiply.
        handling.physical.normal_gravity = 10.0;
        let mut setup = setup_with(
            handling,
            vec![plane(1, 0.0, oag_physics::Surface::Floor, 0)],
        );
        setup.class_gravity_scale = scale;
        let mut race = Race::start(setup);

        // Several ticks, not one: gravity reads the *previous* frame's
        // groundedness, so the first tick sees zero contacts however solidly
        // the ship is resting on the floor.
        let mut evaluated = race.tick(&InputSnapshot::default());
        for _ in 0..20 {
            evaluated = race.tick(&InputSnapshot::default());
        }
        assert!(
            race.ship().physics.grounded > 0.0,
            "the ship never found the floor, so the grounded lane is not under test"
        );
        evaluated.gravity.y
    }

    let identity = settled_gravity(1.0);
    let heavier = settled_gravity(2.0);
    assert!(identity < 0.0, "gravity must pull down");
    assert!(
        heavier < identity,
        "doubling the scale must pull harder: {heavier} against {identity}"
    );
    // And the term is linear in it, which is what a *scale* means.
    assert!((heavier - identity * 2.0).abs() < 1e-3);
}

/// The flare **outlives** the force, and it is armed with a fixed duration.
///
/// This is the opposite of what an earlier revision asserted. The original
/// arms the flare on the entry edge with a code literal
/// ([`exhaust::BOOST_SECONDS`]), while the force runs for the speed class's
/// own `<SpeedupPads time>` - a fraction of it on every shipped class. So a
/// pad is a short shove and a long look, and tying the two together is the
/// obvious-looking mistake this pins against.
#[test]
fn the_flare_outlives_the_force_and_ignores_the_class_tunable() {
    let mut race = race_with_pads(Mode::TimeTrial, enveloping_pad());
    // Well under the flare's fixed duration, so the two are distinguishable.
    assert!(
        race.ship().handling.speedup_pads.time < exhaust::BOOST_SECONDS,
        "the fixture must make the flare the longer of the two"
    );

    race.tick(&InputSnapshot::default());
    // Armed with the constant, not with the class's `time`.
    assert!(
        (race.exhaust().boost_timer() - (exhaust::BOOST_SECONDS - race.dt())).abs() < 1e-4,
        "the flare was armed with {} rather than {}",
        race.exhaust().boost_timer(),
        exhaust::BOOST_SECONDS
    );

    race.speedup_pads.clear();
    let mut force_ticks = 0;
    let mut flare_ticks = 0;
    for _ in 0..120 {
        let evaluated = race.tick(&InputSnapshot::default());
        force_ticks += u32::from(evaluated.speedup_pad != Vec3::ZERO);
        flare_ticks += u32::from(race.exhaust().boost_timer() > 0.0);
    }
    assert!(
        flare_ticks > force_ticks,
        "the flare ran {flare_ticks} tick(s) and the force {force_ticks}; the \
         flare must outlast it"
    );
}

/// Zone pays for a pad **once**, on entry, no matter how long the ship sits
/// on it - `Zone_Update` consumes and clears a flag rather than counting.
#[test]
fn zone_scores_once_for_entering_a_pad_and_not_again_while_inside() {
    const TICKS: u32 = 90;
    let mut padded = race_with_pads(Mode::Zone, enveloping_pad());
    let mut bare = race_with_pads(Mode::Zone, Vec::new());
    for _ in 0..TICKS {
        padded.tick(&InputSnapshot::default());
        bare.tick(&InputSnapshot::default());
    }
    assert_eq!(
        padded.world.race.score - bare.world.race.score,
        oag_race::zone::SPEEDUP_PAD_SCORE,
        "a pad held for {TICKS} ticks must pay exactly once"
    );
}

/// And the other modes pay nothing at all, which is the `DAT_08b31048 == 6`
/// gate on the flag's only writer.
#[test]
fn only_zone_mode_scores_for_a_speed_pad() {
    for mode in [Mode::TimeTrial, Mode::SpeedLap] {
        let mut race = race_with_pads(mode, enveloping_pad());
        for _ in 0..90 {
            race.tick(&InputSnapshot::default());
        }
        assert!(
            race.ship().physics.pad_timer > 0.0,
            "{mode:?}: the pad did not fire at all, so the score assertion proves nothing"
        );
        assert_eq!(race.world.race.score, 0, "{mode:?} scored for a pad");
    }
}
