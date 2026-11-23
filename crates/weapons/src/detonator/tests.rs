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
        scoring: Scoring::default(),
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
                lane: 0,
                live: true,
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

mod contact {
    use super::*;
    use crate::test_craft::Ship;

    fn contact() -> Contact {
        Contact {
            mine: MineContact {
                trigger_radius: 3.0,
                blast_radius: 30.0,
                damage: 10.0,
                slowdown_time: 0.0,
                blast_force: 0.0,
                velocity_reduction: 0.5,
            },
            bomb: BombContact {
                trigger_radius: 6.0,
                blast_radius: 40.0,
                damage: 20.0,
                blast_force: 30.0,
            },
        }
    }

    fn field_with(mine: bool) -> Field {
        let mut field = Field::new();
        let at = Placed {
            entry: 20,
            lane: 0,
            live: true,
        };
        if mine {
            field.mines[0] = at;
            field.live_mines = 1;
        } else {
            field.bombs[0] = at;
            field.live_bombs = 1;
        }
        field
    }

    fn craft_at(z: f32) -> Ship {
        let mut ship = Ship {
            active: true,
            ..Ship::default()
        };
        // A mine stands `MINE_LIFT` above its lane point: the craft flies at that height.
        ship.physics.body.position = Vec3::new(0.0, MINE_LIFT, z);
        ship.physics.body.linear_velocity = Vec3::new(0.0, 0.0, 100.0);
        ship.physics.shield = 100.0;
        ship.handling.dimensions.shield = 100.0;
        ship.physics.craft_state = oag_physics::CraftState::Racing;
        ship
    }

    fn rules() -> oag_physics::DamageRules {
        oag_physics::DamageRules {
            weapons: true,
            ..oag_physics::DamageRules::default()
        }
    }

    #[test]
    fn a_mine_costs_ten_energy_halves_the_velocity_and_is_gone() {
        let survey = lap(300, 7);
        let mut field = field_with(true);
        let mut ships = [craft_at(201.0)];
        let went = step_contacts(&mut field, &survey, 3.5, &contact(), &mut ships, rules());
        assert_eq!(went.len(), 1);
        assert_eq!(went[0].kind, EventKind::MineTouched);
        assert_eq!(field.live_mines, 0);
        assert!((ships[0].physics.shield - 90.0).abs() < 1e-4);
        assert_eq!(ships[0].physics.body.linear_velocity.z, 50.0);
    }

    #[test]
    fn a_bomb_is_not_lifted_where_a_mine_is() {
        let (survey, layout) = (lap(300, 7), layout());
        let lane = layout.values.distance_offset_between_mines;
        let placed = Placed {
            entry: 5,
            lane: 1,
            live: true,
        };
        let at = Vec3::new(3.5, 0.0, 50.0);
        let contact = contact();
        // The bomb's trigger sphere is 6: 7 above its lane point is out of reach, on it is in.
        let mut field = Field::new();
        field.bombs[0] = placed;
        field.live_bombs = 1;
        let mut ship = craft_at(50.0);
        ship.physics.body.position = at + Vec3::new(0.0, 7.0, 0.0);
        let went = step_contacts(&mut field, &survey, lane, &contact, &mut [ship], rules());
        assert!(went.is_empty(), "a bomb is not lifted");
        let mut ship = craft_at(50.0);
        ship.physics.body.position = at;
        let went = step_contacts(&mut field, &survey, lane, &contact, &mut [ship], rules());
        assert_eq!(went.len(), 1, "a craft on the lane point touches the bomb");
        // The mine in the same place is the other way about: on the lane point is 3 off, out of reach.
        let mut field = Field::new();
        field.mines[0] = placed;
        field.live_mines = 1;
        let mut ship = craft_at(50.0);
        ship.physics.body.position = at;
        let went = step_contacts(&mut field, &survey, lane, &contact, &mut [ship], rules());
        assert!(went.is_empty(), "a mine stands 3 above its lane point");
    }

    #[test]
    fn a_craft_outside_the_trigger_sphere_touches_nothing() {
        let survey = lap(300, 7);
        let mut field = field_with(true);
        let mut ships = [craft_at(204.0)];
        let went = step_contacts(&mut field, &survey, 3.5, &contact(), &mut ships, rules());
        assert!(went.is_empty());
        assert_eq!(field.live_mines, 1);
        assert_eq!(ships[0].physics.shield, 100.0);
    }

    #[test]
    fn a_bomb_hurts_the_toucher_alone_and_pushes_the_craft_in_reach() {
        let survey = lap(300, 7);
        let mut field = field_with(false);
        let mut ships = [craft_at(203.0), craft_at(215.0)];
        let went = step_contacts(&mut field, &survey, 3.5, &contact(), &mut ships, rules());
        assert_eq!(went.len(), 1);
        assert_eq!(went[0].kind, EventKind::BombTouched);
        assert_eq!(field.live_bombs, 0);
        assert!((ships[0].physics.shield - 80.0).abs() < 1e-4);
        assert_eq!(ships[1].physics.shield, 100.0);
        assert_eq!(
            ships[0].physics.body.linear_velocity.z, 100.0,
            "a bomb cuts no velocity"
        );
        assert!(ships[1].physics.pending_impulse.length() > 0.0);
    }

    #[test]
    fn a_stage_step_sets_off_the_old_bomb_without_a_hit() {
        let mut field = field_with(false);
        let survey = lap(300, 7);
        let mut rng = Rng::new(1);
        let before: Vec<Vec3> = field
            .live_bombs()
            .iter()
            .filter_map(|b| survey.point(b, 3.5))
            .map(|(at, _)| at)
            .collect();
        let went = place_stage(&mut field, &survey, &layout(), &mut rng, 1);
        assert_eq!(
            went.iter().map(|e| (e.kind, e.at)).collect::<Vec<_>>(),
            before
                .iter()
                .map(|&at| (EventKind::BombReplaced, at))
                .collect::<Vec<_>>(),
            "each bomb the step replaces reports where it was"
        );
        assert!(field.live_bombs <= 1);
        assert!(
            field.live_bombs == 0
                || field.bombs[0]
                    != (Placed {
                        entry: 20,
                        lane: 0,
                        live: true,
                    }),
            "the stage-1 bomb is gone"
        );
    }
}

#[test]
fn a_mine_stands_its_lift_above_its_lane_point_and_a_bomb_on_it() {
    let (survey, layout) = (lap(300, 7), layout());
    let lane = layout.values.distance_offset_between_mines;
    let placed = Placed {
        entry: 5,
        lane: 1,
        live: true,
    };
    let at = Vec3::new(3.5, 0.0, 50.0);
    // The lane point is the survey's own, not a copy of the lift's arithmetic: entry 5 of the test lap,
    // one lane over.
    assert_eq!(survey.point(&placed, lane).expect("in the survey").0, at);
    assert_eq!(
        survey.mine_point(&placed, lane).expect("in the survey").0,
        at + Vec3::new(0.0, 3.0, 0.0),
        "the lift is the measured 3.0, along up"
    );
}
