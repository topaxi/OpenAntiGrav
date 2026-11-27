//! Invented numbers throughout: no shipped value is reproduced.

use super::*;

const PATTERNS: &str = r#"<?xml version="1.0"?>
<Pattern stageToAppearFrom="3">
	<Cluster1 numOfObjects="2" distanceBetweenObjects="4" type="0" percentageAcrossTrack="-2.9" angleUp="0.5" angleRight="0.25" distanceFromOriginalPosition="7" object1Type="1" object2Type="0"></Cluster1>
	<Cluster2 numOfObjects="1"></Cluster2>
</Pattern>
<Pattern>
</Pattern>"#;

#[test]
fn a_pattern_file_reads_groups_of_clusters_in_order() {
    let file = parse_patterns(PATTERNS);
    assert_eq!(file.patterns.len(), 2);
    let first = &file.patterns[0];
    assert_eq!(first.stage_to_appear_from, 3);
    assert_eq!(first.clusters.len(), 2);
    let c = first.clusters[0];
    assert_eq!(c.num_objects, 2);
    assert_eq!(c.distance_between_objects, 4);
    assert_eq!(c.kind, 0);
    assert_eq!(c.angle_up, 0.5);
    assert_eq!(c.angle_right, 0.25);
    assert_eq!(c.distance_from_original_position, 7);
    assert_eq!(c.object_types[..3], [1, 0, 0]);
}

#[test]
fn a_percentage_is_parsed_as_a_float_and_kept_truncated() {
    let c = parse_patterns(PATTERNS).patterns[0].clusters[0];
    assert_eq!(c.percentage_across_track, -2);
}

#[test]
fn a_missing_attribute_is_the_originals_zero_and_a_missing_stage_is_one() {
    let file = parse_patterns(PATTERNS);
    assert_eq!(file.patterns[0].clusters[1].distance_between_objects, 0);
    assert_eq!(file.patterns[1].stage_to_appear_from, 1);
    assert!(file.patterns[1].clusters.is_empty());
    assert_eq!(parse_patterns("<Nothing/>"), PatternFile::default());
}

#[test]
fn atoi_stops_at_the_first_non_digit() {
    assert_eq!(atoi(" 12abc"), 12);
    assert_eq!(atoi("-7"), -7);
    assert_eq!(atoi("x"), 0);
    assert_eq!(atoi("99999999999"), 0);
}

const VALUES: &str = r#"<GamePlayValues RemoveMinesEachStage="1" DistanceOffsetBetweenMines="2.5" NumStepsToIgnoreAfterJump="9" MineCountsRepresentMaxTotalToAppear="0" EnergyRechargePerStage="3" PointsForChainReactionMines="11" BombChainReactionDistance="42" CannonLockoutTimeFromManualReload="0.5" CannonLockoutTimeRunOutBullets="0.25" MineCountStage1="5" MineCountStage14="6"></GamePlayValues>"#;

#[test]
fn the_values_file_overwrites_what_it_names_and_the_rest_keeps_the_static_block() {
    let v = parse_values(VALUES);
    assert!(v.remove_mines_each_stage);
    assert!(!v.do_music_response_previous_stage_mines);
    assert_eq!(v.distance_offset_between_mines, 2.5);
    assert_eq!(v.num_steps_to_ignore_after_jump, 9);
    assert!(!v.mine_counts_represent_max_total_to_appear);
    assert_eq!(v.energy_recharge_per_stage, 3);
    assert_eq!(v.points_for_chain_reaction_mines, 11);
    assert_eq!(v.bomb_chain_reaction_distance, 42.0);
    assert_eq!(v.cannon_lockout_time_from_manual_reload, 0.5);
    assert_eq!(v.cannon_lockout_time_run_out_bullets, 0.25);
    assert_eq!(v.mine_count(1), Some(5));
    assert_eq!(v.mine_count(14), Some(6));
    assert_eq!(v.mine_count(2), GamePlayValues::default().mine_count(2));
}

#[test]
fn mines_scale_to_music_unless_the_file_says_otherwise() {
    assert!(parse_values(VALUES).mines_scale_to_music);
    let held = parse_values(r#"<GamePlayValues MinesScaleToMusic="0"></GamePlayValues>"#);
    assert!(!held.mines_scale_to_music);
}

#[test]
fn the_static_block_counts_run_seventy_to_two_hundred() {
    let d = GamePlayValues::default();
    assert_eq!(d.mine_count(1), Some(70));
    assert_eq!(d.mine_count(14), Some(200));
    assert_eq!(d.mine_count(0), None);
    assert_eq!(d.mine_count(15), None);
    assert_eq!(parse_values("<Other/>"), d);
}

const SCORING: &str = r#"<?xml version="1.0"?>
<DetonatorScoring>
	<Bullet multiplier="2.5" new_mine_score="10" old_mine_score="4" bomb_hit_score="1" bomb_score="70" bomb_increment="9"/>
	<Emp new_mine_score="3" old_mine_score="2" bomb_score="8"/>
</DetonatorScoring>"#;

#[test]
fn the_scoring_file_reads_the_bullet_and_emp_rows() {
    let s = parse_scoring(SCORING);
    assert_eq!(s.multiplier, 2.5);
    assert_eq!(
        (s.new_mine_score, s.old_mine_score, s.bomb_hit_score),
        (10, 4, 1)
    );
    assert_eq!((s.bomb_score, s.bomb_increment), (70, 9));
    assert_eq!(
        (s.emp_new_mine_score, s.emp_old_mine_score, s.emp_bomb_score),
        (3, 2, 8)
    );
    assert_eq!(parse_scoring("<Nothing/>"), Scoring::default());
}
