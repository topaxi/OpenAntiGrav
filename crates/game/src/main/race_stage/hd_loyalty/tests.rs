use super::*;

fn race(mode: oag_race::Mode, difficulty: Option<Difficulty>) -> HdRace {
    HdRace {
        mode,
        laps: 3,
        kills: 0,
        zones: 0,
        difficulty,
    }
}

/// A single race with a full grid is `3 * 15` doubled, tripled or quadrupled by
/// the rung - HD's executable has no x1 for it, where Pulse's `None` is x1.
#[test]
fn a_single_race_is_multiplied_by_its_rung_and_a_missing_rung_is_easy() {
    let paid = |difficulty| hd_award(race(oag_race::Mode::SingleRace, difficulty));
    assert_eq!(paid(Some(Difficulty::Easy)), 90);
    assert_eq!(paid(Some(Difficulty::Medium)), 135);
    assert_eq!(paid(Some(Difficulty::Hard)), 180);
    assert_eq!(paid(None), 90);
}

#[test]
fn a_time_trial_pays_thirty_a_lap_whatever_the_rung() {
    let paid = |difficulty| hd_award(race(oag_race::Mode::TimeTrial, difficulty));
    assert_eq!(paid(Some(Difficulty::Hard)), 90);
    assert_eq!(paid(None), 90);
}

/// Pulse's `loyalty_award` pays an Eliminator 10 a lap and a zone 10; HD's mode
/// 8 pays 15 and its zones 5, so the two titles' awards for the same tallies
/// differ.
#[test]
fn the_hd_award_is_not_pulses() {
    let tallies = HdRace {
        mode: oag_race::Mode::Eliminator,
        laps: 2,
        kills: 1,
        zones: 3,
        difficulty: None,
    };
    // HD: (2 * 15 + 1 * 30 + 3 * 5) * 2 on the missing (easy) rung.
    assert_eq!(hd_award(tallies), 150);
    let pulse = super::super::endrace::loyalty_award(super::super::endrace::LoyaltyInputs {
        mode: oag_race::Mode::Eliminator,
        laps: 2,
        perfect_laps: 0,
        kills: 1,
        zones: 3,
        perfect_zones: 0,
        difficulty: None,
        suggested_ship: false,
    });
    assert_eq!(pulse, 2 * 10 + 30 + 3 * 10);
    assert_ne!(hd_award(tallies), pulse);
}

#[test]
fn every_mode_maps_to_the_id_the_dispatch_table_names() {
    assert_eq!(hd_mode_id(oag_race::Mode::SingleRace), 3);
    assert_eq!(hd_mode_id(oag_race::Mode::Tournament), 4);
    assert_eq!(hd_mode_id(oag_race::Mode::TimeTrial), 5);
    assert_eq!(hd_mode_id(oag_race::Mode::Eliminator), 8);
}
