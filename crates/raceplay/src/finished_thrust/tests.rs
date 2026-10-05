//! The finished player's thrust cap, against an invented table (ADR-0006).

use super::*;
use crate::tests::race_with_a_grid;
use oag_tables::ai_race_stats::SkillPoint;

/// Invented: place `n`'s figure is `100 - n`, and skill 2 is offset 0,
/// multiplier 1, so the cap is `0.7 * (100 - n)` percent.
fn invented() -> FinishedThrust {
    FinishedThrust {
        stats: AiRaceStats {
            ai_thrust: std::array::from_fn(|i| 99.0 - i as f32),
            skill_points: [
                SkillPoint {
                    thrust_offset: -20.0,
                    thrust_multiplier: 1.0,
                },
                SkillPoint {
                    thrust_offset: 0.0,
                    thrust_multiplier: 1.0,
                },
                SkillPoint {
                    thrust_offset: 20.0,
                    thrust_multiplier: 1.0,
                },
            ],
        },
        skill: 2.0,
    }
}

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-5
}

#[test]
fn no_cap_before_the_line_or_without_a_table() {
    let mut race = race_with_a_grid();
    assert_eq!(race.finished_thrust_cap(0), None, "no table loaded");
    race.sim.finished_thrust = Some(invented());
    assert_eq!(race.finished_thrust_cap(0), None, "not finished yet");
}

#[test]
fn the_cap_reads_the_finishing_place() {
    let mut race = race_with_a_grid();
    race.sim.finished_thrust = Some(invented());
    race.sim.world.ships[0].standing.finish_tick = Some(100);
    let first = race.finished_thrust_cap(0).expect("finished");
    assert!(close(first, 0.7 * 99.0 / 100.0), "{first}");

    // Another craft over the line earlier puts the player second.
    race.sim.world.ships[1].standing.finish_tick = Some(50);
    let second = race.finished_thrust_cap(0).expect("finished");
    assert!(close(second, 0.7 * 98.0 / 100.0), "{second}");
}

#[test]
fn a_campaign_skill_replaces_the_ambient_one() {
    let mut race = race_with_a_grid();
    race.sim.finished_thrust = Some(invented());
    race.sim.world.ships[0].standing.finish_tick = Some(100);
    race.set_finished_thrust_skill(3.0);
    let cap = race.finished_thrust_cap(0).expect("finished");
    assert!(close(cap, (0.7 * 99.0 + 20.0) / 100.0), "{cap}");
}

#[test]
fn the_tiers_map_onto_the_three_rungs() {
    assert_eq!(rung(oag_ai::Difficulty::Novice), Difficulty::Easy);
    assert_eq!(rung(oag_ai::Difficulty::Skilled), Difficulty::Medium);
    assert_eq!(rung(oag_ai::Difficulty::Elite), Difficulty::Hard);
    assert_eq!(rung(oag_ai::Difficulty::Ace), Difficulty::Hard);
}

#[test]
fn only_a_single_race_and_head_to_head_add_a_mode_term() {
    assert_eq!(
        mode_term(oag_race::Mode::SingleRace, true, true),
        ModeTerm::Race {
            weapons: true,
            full_grid: true
        }
    );
    assert_eq!(
        mode_term(oag_race::Mode::Head2Head, true, false),
        ModeTerm::HeadToHead
    );
    assert_eq!(
        mode_term(oag_race::Mode::TimeTrial, false, false),
        ModeTerm::None
    );
}
