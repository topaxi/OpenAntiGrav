//! Unit tests against invented fixtures, per `docs/architecture/adr/0006-no-copyrighted-content.md`.
//! Shipped values are only ever checked in
//! `crates/tables/tests/ai_race_stats_ground_truth.rs`.

use super::*;

const FIXTURE: &str = r#"
<AIStats>
<FlashStats>
<RaceBalancing>
<PosBalancing>
<PlayerInPos1 AIThrust="80.0" SpreadDist="1"/>
<PlayerInPos2 AIThrust="79.0" SpreadDist="1"/>
<PlayerInPos3 AIThrust="78.0" SpreadDist="1"/>
<PlayerInPos4 AIThrust="70.0" SpreadDist="1"/>
<PlayerInPos5 AIThrust="76.0" SpreadDist="1"/>
<PlayerInPos6 AIThrust="75.0" SpreadDist="1"/>
<PlayerInPos7 AIThrust="74.0" SpreadDist="1"/>
<PlayerInPos8 AIThrust="73.0" SpreadDist="1"/>
</PosBalancing>
<SkillScale>
<SkillScalePoint1 ThrustOffset="-10.0" ThrustMultiplier="2.0" SpreadMultiplier="1.0"/>
<SkillScalePoint2 ThrustOffset="0.0" ThrustMultiplier="1.0" SpreadMultiplier="1.0"/>
<SkillScalePoint3 ThrustOffset="4.0" ThrustMultiplier="0.5" SpreadMultiplier="1.0"/>
</SkillScale>
</RaceBalancing>
</FlashStats>
</AIStats>
"#;

fn stats() -> AiRaceStats {
    parse(FIXTURE, SpeedClass::Flash).expect("fixture parses")
}

#[test]
fn reads_the_eight_places_and_three_skill_points() {
    let stats = stats();
    assert_eq!(stats.ai_thrust[0], 80.0);
    assert_eq!(stats.ai_thrust[3], 70.0);
    assert_eq!(stats.ai_thrust[7], 73.0);
    assert_eq!(stats.skill_points[0].thrust_offset, -10.0);
    assert_eq!(stats.skill_points[2].thrust_multiplier, 0.5);
}

#[test]
fn another_class_is_not_this_one() {
    assert!(matches!(
        parse(FIXTURE, SpeedClass::Venom),
        Err(Error::MissingElement { .. })
    ));
}

#[test]
fn the_skill_terms_lerp_below_two_and_above_it() {
    let stats = stats();
    assert_eq!(stats.skill_terms(1.0), (-10.0, 2.0));
    assert_eq!(stats.skill_terms(1.5), (-5.0, 1.5));
    assert_eq!(stats.skill_terms(2.0), (0.0, 1.0));
    assert_eq!(stats.skill_terms(2.5), (2.0, 0.75));
}

#[test]
fn a_skill_under_one_extrapolates_past_the_first_point() {
    // The live Venom Easy case's shape: 0.9 lies a tenth below Point1.
    let (offset, multiplier) = stats().skill_terms(0.9);
    assert!((offset - -11.0).abs() < 1e-5, "{offset}");
    assert!((multiplier - 2.1).abs() < 1e-5, "{multiplier}");
}

#[test]
fn the_finished_player_sits_on_the_lower_stop_of_its_own_place() {
    let stats = stats();
    // Skill 2: offset 0, multiplier 1, so the thrust is 0.7 of the place's own figure.
    assert!((stats.finished_player_thrust(1, 2.0) - 56.0).abs() < 1e-4);
    assert!((stats.finished_player_thrust(4, 2.0) - 49.0).abs() < 1e-4);
    // The place indexes the table: fourth's 70 is not third's 78.
    assert!(stats.finished_player_thrust(4, 2.0) < stats.finished_player_thrust(3, 2.0));
}

#[test]
fn the_multiplier_scales_the_distance_from_first_place() {
    // Skill 1: offset -10, multiplier 2. Place 4: 0.7 * 70 = 49, (49 - 80) * 2 + 80 - 10 = 8.
    let thrust = stats().finished_player_thrust(4, 1.0);
    assert!((thrust - 8.0).abs() < 1e-4, "{thrust}");
}

#[test]
fn the_thrust_is_clamped_to_one_and_to_a_hundred() {
    let stats = AiRaceStats {
        ai_thrust: [200.0; 8],
        skill_points: [SkillPoint {
            thrust_offset: 0.0,
            thrust_multiplier: 1.0,
        }; 3],
    };
    assert_eq!(stats.finished_player_thrust(1, 2.0), 100.0);
    let low = AiRaceStats {
        ai_thrust: [-5.0; 8],
        ..stats
    };
    assert_eq!(low.finished_player_thrust(1, 2.0), 1.0);
}

#[test]
fn a_place_out_of_range_reads_as_the_nearest() {
    let stats = stats();
    assert_eq!(
        stats.finished_player_thrust(0, 2.0),
        stats.finished_player_thrust(1, 2.0)
    );
    assert_eq!(
        stats.finished_player_thrust(12, 2.0),
        stats.finished_player_thrust(8, 2.0)
    );
}

#[test]
fn the_entry_name_is_the_lower_case_class() {
    assert_eq!(
        entry_name(SpeedClass::Phantom),
        r"Data\XML\AIRaceStats_phantom.xml"
    );
}

const FULL: &str = r#"
<AIStats>
<VenomStats>
<RaceBalancing>
<PosBalancing>
<PlayerInPos1 AIThrust="79.0" SpreadDist="101.0"/>
<PlayerInPos2 AIThrust="78.0" SpreadDist="102.0"/>
<PlayerInPos3 AIThrust="77.0" SpreadDist="103.0"/>
<PlayerInPos4 AIThrust="76.0" SpreadDist="104.0"/>
<PlayerInPos5 AIThrust="75.0" SpreadDist="105.0"/>
<PlayerInPos6 AIThrust="74.0" SpreadDist="106.0"/>
<PlayerInPos7 AIThrust="73.0" SpreadDist="107.0"/>
<PlayerInPos8 AIThrust="72.0" SpreadDist="108.0"/>
</PosBalancing>
<RubberBanding>
<WhenLeading SpeedMul="0.5" MinSpeedChangeDist="10" MaxSpeedChange="3"/>
<WhenBehind SpeedMul="0.25" MinSpeedChangeDist="20" MaxSpeedChange="6"/>
</RubberBanding>
<StartStats>
<GridPlace1 StartBoost="0.1" BaseStartThrust="71"/>
<GridPlace2 StartBoost="0.2" BaseStartThrust="72"/>
<GridPlace3 StartBoost="0.3" BaseStartThrust="73"/>
<GridPlace4 StartBoost="0.4" BaseStartThrust="74"/>
<GridPlace5 StartBoost="0.5" BaseStartThrust="75"/>
<GridPlace6 StartBoost="0.6" BaseStartThrust="76"/>
<GridPlace7 StartBoost="0.7" BaseStartThrust="77"/>
<GridPlace8 StartBoost="0.8" BaseStartThrust="78"/>
</StartStats>
<SkillScale>
<SkillScalePoint1 ThrustOffset="-1.0" ThrustMultiplier="1.0" SpreadMultiplier="0.5"/>
<SkillScalePoint2 ThrustOffset="0.0" ThrustMultiplier="1.0" SpreadMultiplier="1.0"/>
<SkillScalePoint3 ThrustOffset="1.0" ThrustMultiplier="1.0" SpreadMultiplier="1.5"/>
<AIPackSwapping1 numAIPositionSwaps="2" numIntermediatePositions="1" intermediateOffsetSize="0.1"/>
<AIPackSwapping2 numAIPositionSwaps="3" numIntermediatePositions="2" intermediateOffsetSize="0.2"/>
<AIPackSwapping3 numAIPositionSwaps="4" numIntermediatePositions="3" intermediateOffsetSize="0.3"/>
</SkillScale>
SPLIT</RaceBalancing>
</VenomStats>
</AIStats>
"#;

fn full(extra: &str) -> String {
    FULL.replace("SPLIT", extra)
}

#[test]
fn the_full_reader_carries_every_field_the_thrust_view_drops() {
    let race = parse_full(&full(""), SpeedClass::Venom).expect("parses");
    assert_eq!(race.spread_dist[0], 101.0);
    assert_eq!(race.spread_dist[7], 108.0);
    assert_eq!(race.grid[2].base_start_thrust, 73.0);
    assert_eq!(race.when_behind.max_speed_change, 6.0);
    assert_eq!(race.spread_multiplier[2], 1.5);
    assert_eq!(race.pack_swapping[1].num_intermediate_positions, 2.0);
    assert_eq!(race.split_screen, None);
    assert_eq!(race.thrust, parse(&full(""), SpeedClass::Venom).unwrap());
}

#[test]
fn split_screen_is_read_when_authored() {
    let extra = r#"<SplitScreenMultiplier Easy="0.9" Medium="0.8" Hard="0.7"/>"#;
    let race = parse_full(&full(extra), SpeedClass::Venom).expect("parses");
    assert_eq!(race.split_screen.map(|s| s.hard), Some(0.7));
}

#[test]
fn an_element_it_does_not_know_is_refused_by_name() {
    let extra = r#"<ShipSpacingVariation max="1" min="0"/>"#;
    assert_eq!(
        parse_full(&full(extra), SpeedClass::Venom),
        Err(Error::UnknownElement {
            element: "ShipSpacingVariation".to_string()
        })
    );
}

#[test]
fn an_attribute_it_does_not_know_is_refused_by_name() {
    let text = full("").replace(r#"MaxSpeedChange="3""#, r#"MaxSpeedChange="3" Extra="1""#);
    assert!(matches!(
        parse_full(&text, SpeedClass::Venom),
        Err(Error::UnknownAttribute { attribute, .. }) if attribute == "Extra"
    ));
}
