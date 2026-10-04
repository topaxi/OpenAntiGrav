//! Unit tests against invented fixtures, per `docs/architecture/adr/0006-no-copyrighted-content.md`.
//! Shipped values are only ever checked in
//! `crates/tables/tests/track_stats_ground_truth.rs`.

use super::*;

const FIXTURE: &str = r#"
<Stats>
  <RaceTimes Venom="200" Flash="180" Rapier="160" Phantom="140"/>
  <LapTimes Venom="50" Flash="45" Rapier="40" Phantom="35"/>
  <Targets Elimination="12" Zone="30"/>
  <Physical Length="4321"/>
  <SkillLevels>
    <Entry Difficulty="Easy" Class="Venom" SkillScaleValue="0.5"/>
    <Entry Difficulty="Easy" Class="Flash" SkillScaleValue="0.6"/>
    <Entry Difficulty="Easy" Class="Rapier" SkillScaleValue="0.7"/>
    <Entry Difficulty="Easy" Class="Phantom" SkillScaleValue="0.8"/>
    <Entry Difficulty="Medium" Class="Venom" SkillScaleValue="2.1"/>
    <Entry Difficulty="Medium" Class="Flash" SkillScaleValue="2.2"/>
    <Entry Difficulty="Medium" Class="Rapier" SkillScaleValue="2.3"/>
    <Entry Difficulty="Medium" Class="Phantom" SkillScaleValue="2.4"/>
    <Entry Difficulty="Hard" Class="Venom" SkillScaleValue="3.5"/>
    <Entry Difficulty="Hard" Class="Flash" SkillScaleValue="3.6"/>
    <Entry Difficulty="Hard" Class="Rapier" SkillScaleValue="3.7"/>
    <Entry Difficulty="Hard" Class="Phantom" SkillScaleValue="3.8"/>
    <ModeModifiers Class="Venom" HeadToHead="1.1" FullGridWithWeapons="0.0"
                   HalfGridWithWeapons="0.0" FullGridWithoutWeapons="0.2"
                   HalfGridWithoutWeapons="0.1"/>
    <ModeModifiers Class="Flash" HeadToHead="1.2" FullGridWithWeapons="0.0"
                   HalfGridWithWeapons="0.0" FullGridWithoutWeapons="0.3"
                   HalfGridWithoutWeapons="0.1"/>
    <ModeModifiers Class="Rapier" HeadToHead="1.3" FullGridWithWeapons="0.0"
                   HalfGridWithWeapons="0.0" FullGridWithoutWeapons="0.4"
                   HalfGridWithoutWeapons="0.1"/>
    <ModeModifiers Class="Phantom" HeadToHead="1.4" FullGridWithWeapons="0.0"
                   HalfGridWithWeapons="0.0" FullGridWithoutWeapons="0.5"
                   HalfGridWithoutWeapons="0.1"/>
  </SkillLevels>
</Stats>
"#;

#[test]
fn parses_the_flat_class_rows() {
    let stats = parse(FIXTURE).expect("parses");
    assert_eq!(stats.race_times, [200.0, 180.0, 160.0, 140.0]);
    assert_eq!(stats.lap_times, [50.0, 45.0, 40.0, 35.0]);
    assert_eq!(stats.elimination_target, Some(12));
    assert_eq!(stats.zone_target, Some(30));
    assert_eq!(stats.length, Some(4321));
}

#[test]
fn parses_the_skill_curve_per_class() {
    let stats = parse(FIXTURE).expect("parses");
    assert_eq!(stats.skill_curve(SpeedClass::Venom), [0.5, 2.1, 3.5]);
    assert_eq!(stats.skill_curve(SpeedClass::Flash), [0.6, 2.2, 3.6]);
    assert_eq!(stats.skill_curve(SpeedClass::Rapier), [0.7, 2.3, 3.7]);
    assert_eq!(stats.skill_curve(SpeedClass::Phantom), [0.8, 2.4, 3.8]);
}

#[test]
fn parses_mode_modifiers_per_class() {
    let stats = parse(FIXTURE).expect("parses");
    assert_eq!(
        stats.mode_modifiers[SpeedClass::Venom as usize],
        ModeModifiers {
            head_to_head: 1.1,
            full_grid_with_weapons: 0.0,
            half_grid_with_weapons: 0.0,
            full_grid_without_weapons: 0.2,
            half_grid_without_weapons: 0.1,
        }
    );
}

#[test]
fn a_missing_skill_levels_element_defaults_to_the_documented_curve() {
    let stats = parse(
        r#"<Stats>
             <RaceTimes Venom="1" Flash="1" Rapier="1" Phantom="1"/>
             <LapTimes Venom="1" Flash="1" Rapier="1" Phantom="1"/>
             <Physical Length="1"/>
           </Stats>"#,
    )
    .expect("parses");
    for class in SpeedClass::ALL {
        assert_eq!(stats.skill_curve(class), [1.0, 2.0, 3.0]);
    }
}

/// A minimal `PI_Cell` with `class="Venom"` and the given skill attributes,
/// built through [`crate::race_campaign::parse`] rather than a struct
/// literal - `Cell` carries no [`Default`], and this is the one shape this
/// module needs to hand [`resolve_skill_scale`].
fn cell(skill_easy: Option<f32>, skill: Option<f32>, skill_hard: Option<f32>) -> Cell {
    let attr = |name: &str, value: Option<f32>| {
        value.map_or(String::new(), |v| format!(r#" {name}="{v}""#))
    };
    let xml = format!(
        r#"<PI_Grid name="gridX">
             <Values RequiredPoints="1" Locked="false"></Values>
             <PI_Cell name="gridX_1_1">
               <Values track="99_Track" mode="Race" class="Venom" Weapons="on"
                       damage="on" AICount="7" laps="3" ship="None"
                       ShipChoice="Yes"{}{}{}></Values>
               <Gold Target="1"></Gold>
               <Silver Target="2"></Silver>
               <Bronze Target="3"></Bronze>
             </PI_Cell>
           </PI_Grid>"#,
        attr("skillEasy", skill_easy),
        attr("skill", skill),
        attr("skillHard", skill_hard),
    );
    crate::race_campaign::parse(&xml)
        .expect("parses")
        .cells
        .remove(0)
}

#[test]
fn resolve_skill_scale_interpolates_between_the_curve_points() {
    let stats = parse(FIXTURE).expect("parses");
    let cell = cell(Some(1.1), Some(1.75), Some(2.5));

    // Easy: t=1.1 < 2.0, lerp(curve[0]=0.5, curve[1]=2.1, 0.1)
    let easy = resolve_skill_scale(&cell, Difficulty::Easy, Some(&stats)).unwrap();
    assert!((easy - (0.5 + (2.1 - 0.5) * 0.1)).abs() < 1e-5, "{easy}");

    // Hard: t=2.5 >= 2.0, lerp(curve[1]=2.1, curve[2]=3.5, 0.5)
    let hard = resolve_skill_scale(&cell, Difficulty::Hard, Some(&stats)).unwrap();
    assert!((hard - (2.1 + (3.5 - 2.1) * 0.5)).abs() < 1e-5, "{hard}");
}

#[test]
fn resolve_skill_scale_falls_back_to_the_default_curve_with_no_stats() {
    let cell = cell(Some(1.1), Some(1.75), Some(2.5));
    let easy = resolve_skill_scale(&cell, Difficulty::Easy, None).unwrap();
    assert!((easy - (1.0 + (2.0 - 1.0) * 0.1)).abs() < 1e-5, "{easy}");
}

#[test]
fn resolve_skill_scale_is_none_for_a_solo_mode_cell() {
    let cell = cell(None, None, None);
    assert_eq!(resolve_skill_scale(&cell, Difficulty::Medium, None), None);
}

#[test]
fn ambient_skill_scale_adds_the_mode_term_to_the_plain_value() {
    let stats = parse(FIXTURE).expect("parses");
    // Venom Easy 0.5; a full grid without weapons adds 0.2, a half grid 0.1.
    let full = ModeTerm::Race {
        weapons: false,
        full_grid: true,
    };
    let half = ModeTerm::Race {
        weapons: false,
        full_grid: false,
    };
    let at = |mode| ambient_skill_scale(Some(&stats), SpeedClass::Venom, Difficulty::Easy, mode);
    assert!((at(full) - 0.7).abs() < 1e-6);
    assert!((at(half) - 0.6).abs() < 1e-6);
    assert!((at(ModeTerm::HeadToHead) - 1.6).abs() < 1e-6);
    assert_eq!(at(ModeTerm::None), 0.5);
    assert_eq!(
        ambient_skill_scale(
            Some(&stats),
            SpeedClass::Venom,
            Difficulty::Hard,
            ModeTerm::None
        ),
        3.5
    );
}

#[test]
fn ambient_skill_scale_without_a_table_is_two() {
    assert_eq!(
        ambient_skill_scale(
            None,
            SpeedClass::Rapier,
            Difficulty::Easy,
            ModeTerm::HeadToHead
        ),
        2.0
    );
}
