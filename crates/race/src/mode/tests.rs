//! What [`super::Mode`]'s per-mode laws are asserted to do.
//!
//! Its own file rather than a `#[cfg(test)]` block at the end of `mode.rs`:
//! `scripts/check-file-size.py`'s 200-line ceiling on an inline test module,
//! crossed when `Mode::Head2Head` added its own five tests alongside
//! `Tournament`'s. A move, with no behaviour change - see
//! `crates/physics/src/airbrake.rs` for the same split.

use super::{Mode, SpeedClass};

#[test]
fn every_mode_round_trips_through_its_token() {
    for mode in Mode::ALL {
        assert_eq!(Mode::from_name(mode.name()), Some(mode));
    }
}

/// See [`Mode::ALL`]'s own doc for why: a one-track launch cannot run
/// what the disc's own `Tournament C` needs, a leg list.
#[test]
fn tournament_is_not_in_all() {
    assert!(!Mode::ALL.contains(&Mode::Tournament));
}

/// A leg races exactly like a single race - see [`Mode::Tournament`]'s
/// own doc for the evidence.
#[test]
fn tournament_mirrors_single_race() {
    for class in SpeedClass::ALL {
        assert_eq!(
            Mode::Tournament.laps_target(class),
            Mode::SingleRace.laps_target(class),
            "{class} disagrees with single race"
        );
    }
    assert!(Mode::Tournament.has_opponents());
    assert!(Mode::Tournament.weapons_enabled());
    assert!(Mode::Tournament.pickups_absorb());
    assert_eq!(Mode::Tournament.string_id(), "MSC_EVENT_TOURN");
}

#[test]
fn tokens_are_distinct() {
    let mut names: Vec<&str> = Mode::ALL.iter().map(|mode| mode.name()).collect();
    names.sort_unstable();
    let count = names.len();
    names.dedup();
    assert_eq!(names.len(), count, "two modes share a token");
}

#[test]
fn an_unknown_token_is_none_rather_than_a_default() {
    // `eliminator` is a real mode from 2026-09-08 and belongs in
    // `every_mode_round_trips_through_its_token` instead - kept here as
    // `elimination` (Eliminator's own descriptive token in the disc's
    // string table is `MSC_EVENT_ELIM`, not this crate's `name()`) so
    // the "unknown token" shape this test checks is not lost.
    assert_eq!(Mode::from_name("elimination"), None);
    assert_eq!(Mode::from_name(""), None);
    assert_eq!(Mode::from_name("TIME_TRIAL"), None);
}

#[test]
fn the_default_is_the_time_trial() {
    assert_eq!(Mode::default(), Mode::TimeTrial);
    assert_eq!(Mode::ALL[0], Mode::TimeTrial);
}

#[test]
fn only_the_two_unlimited_modes_never_end_on_laps() {
    assert_eq!(Mode::TimeTrial.laps_target(SpeedClass::Venom), Some(3));
    assert_eq!(Mode::SingleRace.laps_target(SpeedClass::Venom), Some(3));
    // Speed Lap's HUD does show a lap count (`7`, live-confirmed on
    // Venom) - it just never turns into an ending. See `laps_target`'s
    // own docs for the pause-menu evidence.
    assert_eq!(Mode::SpeedLap.laps_target(SpeedClass::Venom), None);
    assert_eq!(Mode::Zone.laps_target(SpeedClass::Venom), None);
    // Eliminator has no lap target either, and for a different reason
    // from Speed Lap's or Zone's: it has one, a kill count, that this
    // method does not report. See `Mode::ELIMINATOR_KILL_TARGET_DEFAULT`.
    assert_eq!(Mode::Eliminator.laps_target(SpeedClass::Venom), None);
}

/// The census, reproduced as an assertion: 3 Venom, 4 Flash, 4 Rapier, 5
/// Phantom across all 236 authored `PI_Cell` records. The whole point of
/// the change that introduced it is that three of these four are *not* 3.
#[test]
fn a_single_race_runs_the_campaigns_own_per_class_lap_count() {
    assert_eq!(Mode::SingleRace.laps_target(SpeedClass::Venom), Some(3));
    assert_eq!(Mode::SingleRace.laps_target(SpeedClass::Flash), Some(4));
    assert_eq!(Mode::SingleRace.laps_target(SpeedClass::Rapier), Some(4));
    assert_eq!(Mode::SingleRace.laps_target(SpeedClass::Phantom), Some(5));
}

/// The same table, live-confirmed rather than census-only: one Custom
/// Race Time Trial per rung under PPSSPP read `Lap 1 of 3`/`4`/`4`/`5`,
/// 2026-09-09, `pulse-psp-usa.chd`, Talon's Junction White.
#[test]
fn a_time_trial_runs_the_same_per_class_lap_count() {
    assert_eq!(Mode::TimeTrial.laps_target(SpeedClass::Venom), Some(3));
    assert_eq!(Mode::TimeTrial.laps_target(SpeedClass::Flash), Some(4));
    assert_eq!(Mode::TimeTrial.laps_target(SpeedClass::Rapier), Some(4));
    assert_eq!(Mode::TimeTrial.laps_target(SpeedClass::Phantom), Some(5));
}

/// Every rung the enum has must have a row, in both tables, or a class
/// would index past one. Cheap here, and the alternative is a panic
/// mid-race.
#[test]
fn every_speed_class_has_a_lap_count() {
    for (index, class) in SpeedClass::ALL.into_iter().enumerate() {
        assert_eq!(index, class as usize, "{class} is not at its own index");
        assert_eq!(
            Mode::SingleRace.laps_target(class),
            Some(Mode::SINGLE_RACE_LAPS_BY_CLASS[index]),
        );
        assert_eq!(
            Mode::TimeTrial.laps_target(class),
            Some(Mode::TIME_TRIAL_LAPS_BY_CLASS[index]),
        );
    }
}

/// The class parameter is taken and ignored by every mode except the two
/// that field a real per-class table - see `laps_target`'s own docs for
/// Speed Lap's `7`, which is real but does not end the race.
#[test]
fn only_time_trial_and_single_race_vary_with_the_speed_class() {
    for class in SpeedClass::ALL {
        assert_eq!(Mode::SpeedLap.laps_target(class), None);
        assert_eq!(Mode::Zone.laps_target(class), None);
        assert_eq!(Mode::Eliminator.laps_target(class), None);
    }
}

#[test]
fn only_zone_drives_its_own_throttle() {
    assert!(Mode::Zone.is_auto_throttle());
    assert!(!Mode::TimeTrial.is_auto_throttle());
    assert!(!Mode::SpeedLap.is_auto_throttle());
    assert!(!Mode::SingleRace.is_auto_throttle());
}

/// The three single-ship modes are the original's own answer, measured on
/// the running game; single race and Eliminator are the two that field a
/// grid.
#[test]
fn single_race_and_eliminator_field_a_grid() {
    assert!(Mode::SingleRace.has_opponents());
    assert!(Mode::Eliminator.has_opponents());
    assert!(!Mode::TimeTrial.has_opponents());
    assert!(!Mode::SpeedLap.has_opponents());
    assert!(!Mode::Zone.has_opponents());
}

/// The switch the pickup system hangs off. It is the *only* mode-dependent
/// thing about a weapon pad in the original: a weapons-off race hides the
/// pads and empties the trigger list rather than ignoring a crossing.
#[test]
fn single_race_and_eliminator_are_the_only_modes_that_arm_weapon_pads() {
    assert!(Mode::SingleRace.weapons_enabled());
    assert!(Mode::Eliminator.weapons_enabled());
    for mode in [Mode::TimeTrial, Mode::SpeedLap, Mode::Zone] {
        assert!(
            !mode.weapons_enabled(),
            "{mode:?} should race with weapons off"
        );
    }
}

/// The mechanic `MSC_EVENT_ELIM` states outright: Eliminator is the one
/// mode where a crossed pickup pad cannot be absorbed for health.
#[test]
fn only_eliminator_refuses_pickup_absorption() {
    assert!(!Mode::Eliminator.pickups_absorb());
    for mode in Mode::ALL {
        if mode != Mode::Eliminator {
            assert!(mode.pickups_absorb(), "{mode:?} should absorb pickups");
        }
    }
}

/// `docs/ghidra/functions/psp-pulse-usa/head2head.md`'s own census: all
/// 23 authored cells carry `AICount="1"`, against `7` for
/// `Race`/`Tournament`/`Eliminator`.
#[test]
fn head2head_fields_exactly_one_opponent() {
    assert_eq!(Mode::Head2Head.opponent_count(), 1);
    assert!(Mode::Head2Head.has_opponents());
    for mode in [Mode::SingleRace, Mode::Eliminator, Mode::Tournament] {
        assert_eq!(mode.opponent_count(), 7, "{mode:?} should field seven");
    }
    for mode in [Mode::TimeTrial, Mode::SpeedLap, Mode::Zone] {
        assert_eq!(mode.opponent_count(), 0, "{mode:?} should field none");
    }
}

/// `shield.md`'s `g_weapons_enabled` switch puts Head2Head in both its
/// default-off and its no-override set, and all 23 authored cells carry
/// `Weapons="off"` with no exception - locked, not merely defaulted off.
#[test]
fn head2head_races_with_weapons_locked_off() {
    assert!(!Mode::Head2Head.weapons_enabled());
}

/// Every authored `Head2Head` cell's own `laps` is `4` (Flash, Rapier)
/// or `5` (Phantom); no Venom-class cell exists, so that rung falls
/// back to the same table `Single Race` uses.
#[test]
fn head2head_shares_single_races_per_class_lap_count() {
    for class in SpeedClass::ALL {
        assert_eq!(
            Mode::Head2Head.laps_target(class),
            Mode::SingleRace.laps_target(class),
            "{class} disagrees with single race"
        );
    }
}

/// Like `Tournament`, reachable only through a campaign cell - see
/// `Mode::Head2Head`'s own doc comment for why.
#[test]
fn head2head_is_not_in_all() {
    assert!(!Mode::ALL.contains(&Mode::Head2Head));
}
