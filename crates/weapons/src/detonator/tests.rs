//! Invented layouts and a straight lap: the law, not the disc.

use super::*;
use oag_tables::detonator_layout::{Cluster, Pattern};

fn lap(entries: usize, lanes: u8) -> Survey {
    Survey {
        entries: (0..entries)
            .map(|i| Entry {
                usable: i >= 10,
                lanes,
                blocked: 0,
                centre: Vec3::new(0.0, 0.0, i as f32 * 10.0),
                lateral: Vec3::X,
                up: Vec3::Y,
            })
            .collect(),
    }
}

fn layout() -> Layout {
    let cluster = |across, from| Cluster {
        num_objects: 1,
        distance_between_objects: 3,
        percentage_across_track: across,
        distance_from_original_position: from,
        ..Cluster::default()
    };
    let mut values = GamePlayValues::default();
    values.mine_count_stage[0] = 12;
    values.mine_count_stage[1] = 20;
    Layout {
        patterns: PatternFile {
            patterns: vec![
                Pattern {
                    stage_to_appear_from: 1,
                    clusters: vec![cluster(0, 0), cluster(1, 1), cluster(-1, 2)],
                },
                Pattern {
                    stage_to_appear_from: 2,
                    clusters: vec![cluster(2, 0)],
                },
            ],
        },
        values,
    }
}

fn laid(seed: u64, stages: u32) -> Field {
    let (survey, layout) = (lap(300, 7), layout());
    let mut field = Field::new();
    let mut rng = Rng::new(seed);
    for old in 0..stages {
        place_stage(&mut field, &survey, &layout, &mut rng, old);
    }
    field
}

#[test]
fn a_stage_lays_up_to_its_mine_count_and_at_most_one_bomb() {
    let field = laid(1, 1);
    assert_eq!(field.live_mines, 12, "the stage's count, a total");
    assert!(field.live_bombs <= 1, "one attempt per stage");
    assert_eq!(field.stages_laid, 1);
}

#[test]
fn the_bomb_attempt_lands_on_some_seeds_and_misses_on_others() {
    let landed = (0..64)
        .filter(|&seed| laid(seed, 1).live_bombs == 1)
        .count();
    assert!(landed > 0 && landed < 64, "{landed} of 64");
}

#[test]
fn the_next_stage_tops_the_total_up_to_its_count() {
    let field = laid(1, 2);
    assert_eq!(field.live_mines, 20);
    assert!(field.live_bombs <= 2);
    assert_eq!(field.stages_laid, 2);
}

#[test]
fn the_bomb_pool_holds_two() {
    let field = laid(3, 6);
    assert!(usize::from(field.live_bombs) <= MAX_BOMBS);
}

#[test]
fn the_same_seed_lays_the_same_field() {
    assert_eq!(laid(9, 2), laid(9, 2));
    assert_ne!(laid(9, 2), laid(10, 2));
}

#[test]
fn nothing_is_laid_on_an_unusable_entry_and_no_two_objects_share_a_lane() {
    let field = laid(5, 2);
    let objects: Vec<&Placed> = field
        .live_mines()
        .iter()
        .chain(field.live_bombs())
        .collect();
    assert!(objects.iter().all(|o| o.entry >= 10));
    for (i, a) in objects.iter().enumerate() {
        for b in &objects[i + 1..] {
            assert!(!(a.entry == b.entry && a.lane == b.lane), "{a:?} {b:?}");
        }
    }
}

#[test]
fn a_mine_sits_on_its_lane_across_the_entry() {
    let (survey, layout) = (lap(300, 7), layout());
    let field = laid(2, 1);
    for mine in field.live_mines() {
        let (position, up) = survey
            .point(mine, layout.values.distance_offset_between_mines)
            .expect("the entry is in the survey");
        assert_eq!(position.z, f32::from(mine.entry) * 10.0);
        assert_eq!(position.x, 3.5 * f32::from(mine.lane));
        assert_eq!(up, Vec3::Y);
    }
    assert_eq!(
        survey.point(
            &Placed {
                entry: 999,
                lane: 0
            },
            3.5
        ),
        None
    );
}

#[test]
fn a_lap_with_no_free_lane_lays_nothing_and_ends() {
    let mut survey = lap(40, 3);
    for entry in &mut survey.entries {
        entry.usable = false;
    }
    let mut field = Field::new();
    place_stage(&mut field, &survey, &layout(), &mut Rng::new(1), 0);
    assert_eq!(field.live_mines, 0);
    assert_eq!(field.live_bombs, 0);
}

#[test]
fn a_stage_past_the_table_lays_nothing() {
    let mut field = Field::new();
    place_stage(&mut field, &lap(300, 7), &layout(), &mut Rng::new(1), 14);
    assert_eq!(field.live_mines, 0);
}

#[test]
fn an_empty_field_says_so() {
    assert!(Field::new().is_empty());
    assert!(!laid(1, 1).is_empty());
}
