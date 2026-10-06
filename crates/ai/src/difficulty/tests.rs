use super::*;

#[test]
fn from_str_matches_from_name() {
    use std::str::FromStr;

    for (name, level) in Difficulty::ALL {
        assert_eq!(Difficulty::from_str(name), Ok(level));
        assert_eq!(level.to_string(), name);
    }
    assert!(Difficulty::from_str("impossible").is_err());
}

#[test]
fn every_level_round_trips_through_its_name() {
    for (name, level) in Difficulty::ALL {
        assert_eq!(Difficulty::from_name(name), Some(level));
        assert_eq!(level.name(), name);
    }
    assert_eq!(Difficulty::from_name("impossible"), None);
}

/// The scales have to be ordered, or a "harder" setting is not harder.
#[test]
fn every_axis_rises_with_the_level() {
    let levels = Difficulty::ALL.map(|(_, level)| level);
    for pair in levels.windows(2) {
        let (easier, harder) = (pair[0], pair[1]);
        assert!(harder.grip_believed() > easier.grip_believed());
        assert!(harder.turn_allowed() > easier.turn_allowed());
        assert!(harder.aggression() > easier.aggression());
        assert!(harder.roll_appetite() > easier.roll_appetite());
        // Mistakes are one of the two that fall, and how much a level keeps
        // back from a roll is the other: a harder driver rolls more readily
        // and off less.
        assert!(harder.mistakes() < easier.mistakes());
        assert!(harder.roll_caution() < easier.roll_caution());
    }
}

/// The top level gives back the measurement untouched, mistakes included:
/// `Tuning::default()` does not err and neither does an ace. If it ever
/// scaled the pace, the hardest setting would be the least tested one.
#[test]
fn the_top_level_gives_back_the_measured_tuning_exactly() {
    let measured = Tuning::default();
    let ace = Difficulty::Ace.tune(&measured);
    assert_eq!(ace.lateral_accel, measured.lateral_accel);
    assert_eq!(ace.max_turn_rate, measured.max_turn_rate);
    assert_eq!(
        ace, measured,
        "the top level changed something, and it is the measured tuning"
    );
    assert_eq!(
        Difficulty::Ace.temper(&Pilot::AGGRESSIVE),
        Pilot::AGGRESSIVE
    );
    assert_eq!(Difficulty::Ace.mistakes(), 0.0);
}

#[test]
fn a_novice_field_is_slower_and_turns_less_hard() {
    let measured = Tuning::default();
    let easy = Difficulty::Novice.tune(&measured);
    assert!(easy.lateral_accel < measured.lateral_accel);
    assert!(easy.max_turn_rate < measured.max_turn_rate);
    // And everything else is left alone.
    assert_eq!(easy.look_min, measured.look_min);
    assert_eq!(easy.brake_floor, measured.brake_floor);
    assert_eq!(easy.trail_gain, measured.trail_gain);
    // And the novice is the one that errs.
    assert!(easy.mistake_rate > 0.0);
    assert_eq!(Difficulty::Ace.tune(&measured).mistake_rate, 0.0);
}

/// The three roll axes degrade the ways their meanings run, and `Ace` gets the
/// pilot's numbers untouched. **Degradation, never a boost** (the module's
/// rule): rolling *more* readily than the pilot asked is untested tuning on the
/// setting that matters most.
#[test]
fn a_lower_level_rolls_less_readily_off_more_shield_and_after_a_longer_jump() {
    let base = Pilot::BALANCED;
    let ace = Difficulty::Ace.temper(&base);
    assert_eq!(ace.roll_chance, base.roll_chance);
    assert_eq!(ace.roll_floor, base.roll_floor);
    assert_eq!(ace.roll_airtime, base.roll_airtime);

    let levels = Difficulty::ALL.map(|(_, level)| level);
    for pair in levels.windows(2) {
        let (easier, harder) = (pair[0].temper(&base), pair[1].temper(&base));
        assert!(harder.roll_chance.high > easier.roll_chance.high);
        assert!(harder.roll_floor.low < easier.roll_floor.low);
        assert!(harder.roll_airtime.low < easier.roll_airtime.low);
    }
}

/// A floor is a fraction of the pool, so raising it can only reach one -
/// which is a pilot that never rolls, and not a span that runs backwards.
#[test]
fn no_level_raises_a_roll_floor_past_the_whole_pool() {
    for (_, level) in Difficulty::ALL {
        for (name, pilot) in Pilot::BUILT_IN {
            let tempered = level.temper(&pilot);
            assert!(tempered.roll_floor.high <= 1.0, "{name}");
            assert!(tempered.is_well_formed(), "{name} at {}", level.name());
        }
    }
}

/// A slow opponent that still shoots you in the back is not an easy race.
#[test]
fn a_novice_field_leaves_the_player_alone() {
    let calm = Difficulty::Novice.temper(&Pilot::AGGRESSIVE);
    assert_eq!(calm.trigger.high, 0.0);
    assert_eq!(calm.ram.high, 0.0);
    assert_eq!(calm.defence.high, 0.0);
}

/// A pilot has to stay itself at every level, or four difficulties give one
/// generic slow driver and one generic fast one.
#[test]
fn a_level_does_not_change_which_pilot_a_craft_is() {
    for (_, level) in Difficulty::ALL {
        for (name, pilot) in Pilot::BUILT_IN {
            let tempered = level.temper(&pilot);
            assert_eq!(tempered.line_bias, pilot.line_bias, "{name}");
            assert_eq!(tempered.wander, pilot.wander, "{name}");
            assert_eq!(tempered.inside, pilot.inside, "{name}");
            assert_eq!(tempered.commitment, pilot.commitment, "{name}");
            assert_eq!(tempered.courtesy, pilot.courtesy, "{name}");
            assert!(tempered.is_well_formed(), "{name} at {}", level.name());
        }
    }
}

/// Scaling must not push a pilot past the ceiling that keeps a craft out of
/// the wall.
#[test]
fn no_level_lets_a_pilot_ask_for_more_grip_than_the_hull_has() {
    for (_, level) in Difficulty::ALL {
        for (name, pilot) in Pilot::BUILT_IN {
            assert!(
                level.temper(&pilot).validated().is_ok(),
                "{name} at {}",
                level.name()
            );
        }
    }
}

/// `1.0`/`2.0`/`3.0` must agree with `Novice`/`Skilled`/`Elite`'s own `tune`, or
/// the continuous path is not the discrete one generalised. **Within a ULP, not
/// bit-exact**: `lerp(a, b, 1.0)` is `a + (b - a)`, the ordinary lerp-endpoint
/// wart, not a determinism gap (same inputs, same output every run).
#[test]
fn tune_at_scale_agrees_with_the_named_levels_at_the_integers() {
    let measured = Tuning::default();
    for (scale, level) in [
        (1.0, Difficulty::Novice),
        (2.0, Difficulty::Skilled),
        (3.0, Difficulty::Elite),
    ] {
        let blended = Difficulty::tune_at_scale(scale, &measured);
        let named = level.tune(&measured);
        assert!(
            (blended.lateral_accel - named.lateral_accel).abs() <= f32::EPSILON,
            "{scale}: {blended:?} vs {named:?}"
        );
        assert!((blended.max_turn_rate - named.max_turn_rate).abs() <= f32::EPSILON);
        assert!((blended.mistake_rate - named.mistake_rate).abs() <= f32::EPSILON);
        assert_eq!(blended.reaction_ticks, named.reaction_ticks);
    }
}

/// A value strictly between two integers must sit strictly between their
/// tunings - the whole reason this exists over rounding to the nearer
/// level.
#[test]
fn tune_at_scale_blends_rather_than_snaps() {
    let measured = Tuning::default();
    let novice = Difficulty::Novice.tune(&measured);
    let skilled = Difficulty::Skilled.tune(&measured);
    let half = Difficulty::tune_at_scale(1.5, &measured);
    assert!(half.lateral_accel > novice.lateral_accel);
    assert!(half.lateral_accel < skilled.lateral_accel);
    assert!(half.max_turn_rate > novice.max_turn_rate);
    assert!(half.max_turn_rate < skilled.max_turn_rate);
}

/// A curve a track authors can run slightly outside `1.0..=3.0`; this
/// must not panic or extrapolate past `Novice`/`Elite`.
#[test]
fn tune_at_scale_clamps_outside_its_domain() {
    let measured = Tuning::default();
    let below = Difficulty::tune_at_scale(0.2, &measured);
    let novice = Difficulty::Novice.tune(&measured);
    assert!((below.lateral_accel - novice.lateral_accel).abs() <= f32::EPSILON);

    let above = Difficulty::tune_at_scale(9.0, &measured);
    let elite = Difficulty::Elite.tune(&measured);
    assert!((above.lateral_accel - elite.lateral_accel).abs() <= f32::EPSILON);
}
