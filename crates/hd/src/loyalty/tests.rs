use super::*;

fn single_race(tally: Tally, tier: Tier, ai_present: bool) -> u32 {
    award(Inputs {
        mode: mode::SINGLE_RACE,
        tally,
        tier,
        ai_present,
    })
}

fn laps(laps: u32) -> Tally {
    Tally {
        laps,
        ..Tally::default()
    }
}

/// A three-lap single race with a craft in it is `3 * 15 = 45`, doubled on easy.
#[test]
fn a_single_race_pays_fifteen_a_lap_and_the_easy_rung_doubles_it() {
    assert_eq!(single_race(laps(3), Tier::Easy, true), 90);
    assert_eq!(single_race(laps(3), Tier::Medium, true), 135);
    assert_eq!(single_race(laps(3), Tier::Hard, true), 180);
}

/// With nobody else on the grid there is no multiplier at all.
#[test]
fn a_grid_of_one_pays_no_multiplier() {
    assert_eq!(single_race(laps(3), Tier::Hard, false), 45);
}

#[test]
fn time_trial_and_speed_lap_pay_thirty_and_fifty_and_never_a_multiplier() {
    for id in [mode::TIME_TRIAL, mode::SPEED_LAP] {
        let tally = Tally {
            laps: 3,
            perfect_laps: 1,
            ..Tally::default()
        };
        let got = award(Inputs {
            mode: id,
            tally,
            tier: Tier::Hard,
            ai_present: true,
        });
        assert_eq!(got, 3 * 30 + 50, "mode {id}");
    }
}

/// Pulse's Eliminator pays 10/20 a lap; HD's mode 8 is in the 15/25 class.
#[test]
fn hd_eliminator_pays_the_race_lap_rates_not_pulses() {
    let tally = Tally {
        laps: 2,
        perfect_laps: 1,
        kills: 2,
        ..Tally::default()
    };
    let got = award(Inputs {
        mode: mode::ELIMINATION,
        tally,
        tier: Tier::Medium,
        ai_present: true,
    });
    // (2 * 15 + 25 + 2 * 30) * 3
    assert_eq!(got, 345);
}

/// Zones pay 5 and perfect zones 15 here; Pulse's are 10 and 20.
#[test]
fn zones_pay_five_and_perfect_zones_fifteen() {
    let tally = Tally {
        zones: 4,
        perfect_zones: 2,
        ..Tally::default()
    };
    let got = award(Inputs {
        mode: mode::ZONE,
        tally,
        tier: Tier::Easy,
        ai_present: true,
    });
    assert_eq!(got, 4 * 5 + 2 * 15);
}

/// Detonator pays 150 a stage, 20 a perfect one, and nothing for a kill.
#[test]
fn detonator_pays_a_stage_a_hundred_and_fifty_and_has_no_kill_term() {
    let tally = Tally {
        laps: 2,
        perfect_laps: 1,
        kills: 9,
        ..Tally::default()
    };
    let got = award(Inputs {
        mode: mode::DETONATOR,
        tally,
        tier: Tier::Hard,
        ai_present: true,
    });
    assert_eq!(got, 2 * 150 + 20);
}

/// A kill is worth 15 outside Eliminator.
#[test]
fn a_kill_is_worth_fifteen_in_a_race() {
    let tally = Tally {
        kills: 4,
        ..Tally::default()
    };
    assert_eq!(single_race(tally, Tier::Easy, false), 60);
}

#[test]
fn an_online_mode_pays_triple_whatever_the_rung() {
    let got = award(Inputs {
        mode: 16,
        tally: laps(2),
        tier: Tier::Easy,
        ai_present: false,
    });
    assert_eq!(got, 2 * 15 * 3);
}

#[test]
fn the_banked_total_is_capped() {
    assert_eq!(bank(0, 90), 90);
    assert_eq!(bank(90, 90), 180);
    assert_eq!(bank(99_990, 90), CAP);
    assert_eq!(bank(CAP, 90), CAP);
    assert_eq!(bank(u32::MAX, 5), CAP);
}
