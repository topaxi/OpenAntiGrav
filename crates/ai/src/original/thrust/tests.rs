//! The thrust law against `scripts/psp-ai-thrust.py`'s own `predict_one`, run
//! in `float32` on a synthetic class table (no disc number appears here,
//! ADR-0006). Each case is one branch the live samples exercised, plus the
//! pieces they could not separate: non-unity multipliers, so the `AIThrust[0]`
//! pivot of the last line and the `SpreadMultiplier` scaling are pinned, the
//! `100` cap, and the floor of `1`. The expected bits are the script's output.

use super::*;
use oag_tables::ai_race_stats::{AiRaceStats, GridPlace, PackSwapping, RubberRow, SkillPoint};

/// Invented numbers, laid out where the script reads them in the class record.
pub(crate) fn table() -> ClassRace {
    ClassRace {
        thrust: AiRaceStats {
            ai_thrust: [60.0, 58.0, 56.0, 54.0, 52.0, 50.0, 48.0, 46.0],
            skill_points: [
                SkillPoint {
                    thrust_offset: -40.0,
                    thrust_multiplier: 0.9,
                },
                SkillPoint {
                    thrust_offset: 0.0,
                    thrust_multiplier: 1.0,
                },
                SkillPoint {
                    thrust_offset: 30.0,
                    thrust_multiplier: 1.25,
                },
            ],
        },
        spread_dist: [40.0, 42.0, 44.0, 46.0, 48.0, 50.0, 52.0, 54.0],
        grid: [GridPlace {
            start_boost: 0.0,
            base_start_thrust: 0.0,
        }; 8],
        when_leading: RubberRow {
            speed_mul: 0.2,
            min_speed_change_dist: 100.0,
            max_speed_change: 40.0,
        },
        when_behind: RubberRow {
            speed_mul: 0.04,
            min_speed_change_dist: 150.0,
            max_speed_change: 9.0,
        },
        spread_multiplier: [1.1, 1.0, 0.8],
        pack_swapping: [PackSwapping {
            num_ai_position_swaps: 3.0,
            num_intermediate_positions: 2.0,
            intermediate_offset_size: 0.5,
        }; 3],
        split_screen: None,
    }
}

macro_rules! case {
    ($name:ident, $skill:expr, $time:expr, $player:expr, $own:expr, $finished:expr,
     [$(($progress:expr, $place:expr, $spread:expr)),* $(,)?], $throttle:expr, $gap:expr) => {
        #[test]
        fn $name() {
            let racers = [$(Racer { progress: $progress, place: $place, spread: $spread }),*];
            let out = opponent_thrust(
                &table(),
                &ThrustInput {
                    racers: &racers,
                    own: $own,
                    player: $player,
                    finished: $finished,
                    rubber_band_off: false,
                    race_time: $time,
                    skill: $skill,
                },
            );
            assert_eq!(
                (out.throttle.to_bits(), out.gap.to_bits()),
                ($throttle, $gap),
                "throttle {} gap {}",
                out.throttle,
                out.gap,
            );
        }
    };
}

// when_behind_at_its_clamp: band -9.0000
case!(
    when_behind_at_its_clamp,
    0.9_f32,
    30.0_f32,
    7,
    2,
    false,
    [
        (900.0, 1, 8.0),
        (880.0, 2, 7.0),
        (860.0, 3, 6.0),
        (840.0, 4, 5.0),
        (820.0, 5, 4.0),
        (800.0, 6, 3.0),
        (780.0, 7, 2.0),
        (100.0, 8, 1.0)
    ],
    0x40f9fbe8,
    0x431fc290
);
// gap_on_the_upper_stop: band 0.0000
case!(
    gap_on_the_upper_stop,
    2.0_f32,
    5.0_f32,
    7,
    3,
    false,
    [
        (900.0, 1, 8.0),
        (880.0, 2, 7.0),
        (860.0, 3, 6.0),
        (600.0, 8, 5.0),
        (820.0, 4, 4.0),
        (800.0, 5, 3.0),
        (780.0, 6, 2.0),
        (700.0, 7, 1.0)
    ],
    0x4279999a,
    0x43a80000
);
// gap_on_the_lower_stop: band 0.0000
case!(
    gap_on_the_lower_stop,
    2.0_f32,
    5.0_f32,
    7,
    3,
    false,
    [
        (900.0, 2, 8.0),
        (880.0, 3, 7.0),
        (860.0, 4, 6.0),
        (1200.0, 1, 5.0),
        (820.0, 5, 4.0),
        (800.0, 6, 3.0),
        (780.0, 7, 2.0),
        (700.0, 8, 1.0)
    ],
    0x4200cccd,
    0xc3810000
);
// own_reference_zero_gap_unclamped: band 0.0000
case!(
    own_reference_zero_gap_unclamped,
    1.5_f32,
    12.0_f32,
    7,
    6,
    false,
    [
        (900.0, 1, 8.0),
        (880.0, 2, 7.0),
        (860.0, 3, 6.0),
        (840.0, 4, 5.0),
        (820.0, 5, 4.0),
        (800.0, 6, 3.0),
        (781.0, 7, 2.5),
        (779.5, 8, 1.0)
    ],
    0x41d5999a,
    0x00000000
);
// inside_both_dead_bands: band 0.0000
case!(
    inside_both_dead_bands,
    2.0_f32,
    10.0_f32,
    3,
    1,
    false,
    [
        (900.0, 1, 8.0),
        (880.0, 2, 7.0),
        (860.0, 4, 6.0),
        (870.0, 3, 5.0),
        (820.0, 6, 4.0),
        (800.0, 7, 3.0),
        (780.0, 8, 2.0),
        (850.0, 5, 1.0)
    ],
    0x42600000,
    0x00000000
);
// capped_at_100_near_the_player_after_20s: band 20.0000
case!(
    capped_at_100_near_the_player_after_20s,
    3.0_f32,
    40.0_f32,
    0,
    1,
    false,
    [
        (1300.0, 1, 8.0),
        (1100.0, 2, 7.0),
        (1000.0, 3, 6.0),
        (990.0, 4, 5.0),
        (980.0, 5, 4.0),
        (970.0, 6, 3.0),
        (960.0, 7, 2.0),
        (950.0, 8, 1.0)
    ],
    0x42c80000,
    0x00000000
);
// not_capped_before_20s: band 20.0000
case!(
    not_capped_before_20s,
    3.0_f32,
    19.0_f32,
    0,
    1,
    false,
    [
        (1300.0, 1, 8.0),
        (1100.0, 2, 7.0),
        (1000.0, 3, 6.0),
        (990.0, 4, 5.0),
        (980.0, 5, 4.0),
        (970.0, 6, 3.0),
        (960.0, 7, 2.0),
        (950.0, 8, 1.0)
    ],
    0x42e60000,
    0x00000000
);
// when_leading_added_and_ref_is_place_two: band 40.0000
case!(
    when_leading_added_and_ref_is_place_two,
    2.5_f32,
    60.0_f32,
    0,
    4,
    false,
    [
        (2000.0, 1, 8.0),
        (1500.0, 2, 7.0),
        (1480.0, 3, 6.0),
        (1460.0, 4, 5.0),
        (1440.0, 5, 4.0),
        (1420.0, 6, 3.0),
        (1400.0, 7, 2.0),
        (1380.0, 8, 1.0)
    ],
    0x42c78000,
    0xc2400000
);
// finished_reads_spread_fifty: band 0.0000
case!(
    finished_reads_spread_fifty,
    1.2_f32,
    100.0_f32,
    0,
    0,
    true,
    [
        (3000.0, 1, 8.0),
        (2900.0, 2, 7.0),
        (2880.0, 3, 6.0),
        (2860.0, 4, 5.0),
        (2840.0, 5, 4.0),
        (2820.0, 6, 3.0),
        (2800.0, 7, 2.0),
        (2780.0, 8, 1.0)
    ],
    0x41370a40,
    0xc2480000
);
// floor_of_one: band -9.0000
case!(
    floor_of_one,
    0.0_f32,
    30.0_f32,
    7,
    0,
    false,
    [
        (9000.0, 1, 8.0),
        (880.0, 2, 7.0),
        (860.0, 3, 6.0),
        (840.0, 4, 5.0),
        (820.0, 5, 4.0),
        (800.0, 6, 3.0),
        (780.0, 7, 2.0),
        (100.0, 8, 1.0)
    ],
    0x3f800000,
    0xc5f4b99a
);

/// The ship-state gate: with the player in state `4..=6` the rubber band is
/// not added, whatever the gap. Static reading only; never seen live.
#[test]
fn the_rubber_band_is_off_while_the_player_is_in_states_four_to_six() {
    let racers = [
        Racer {
            progress: 900.0,
            place: 1,
            spread: 8.0,
        },
        Racer {
            progress: 880.0,
            place: 2,
            spread: 7.0,
        },
        Racer {
            progress: 100.0,
            place: 3,
            spread: 1.0,
        },
    ];
    let input = |rubber_band_off| ThrustInput {
        racers: &racers,
        own: 1,
        player: 2,
        finished: false,
        rubber_band_off,
        race_time: 30.0,
        skill: 2.0,
    };
    let on = opponent_thrust(&table(), &input(false)).throttle;
    let off = opponent_thrust(&table(), &input(true)).throttle;
    assert_eq!(
        off - on,
        9.0,
        "the WhenBehind clamp is the whole difference"
    );
}
