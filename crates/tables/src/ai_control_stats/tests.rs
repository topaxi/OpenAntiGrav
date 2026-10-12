//! Invented fixtures only; shipped values are checked in
//! `crates/tables/tests/ai_control_stats_ground_truth.rs`.

use super::*;

const FIXTURE: &str = r#"
<AIStats>
<VenomStats>
<Controller SteerMul="1.5" LookAheadSecs="0.25" xtrackMul="0.5" SteerDamp="2.0"/>
</VenomStats>
<FlashStats>
<ControllerPO SteerMul="1.0"/>
</FlashStats>
<RapierStats>
<Controller SteerMul="1.0" Bogus="3"/>
</RapierStats>
</AIStats>
"#;

#[test]
fn absent_attributes_stay_absent() {
    let c = parse(FIXTURE, SpeedClass::VENOM).expect("parses");
    assert_eq!(c.steer_mul, Some(1.5));
    assert_eq!(c.look_ahead_secs, Some(0.25));
    assert_eq!(c.xtrack_max, None);
    assert_eq!(c.xtrack_damp, None);
}

#[test]
fn another_controller_family_is_refused_by_name() {
    assert_eq!(
        parse(FIXTURE, SpeedClass::FLASH),
        Err(Error::UnknownElement {
            element: "ControllerPO".to_string()
        })
    );
}

#[test]
fn an_unknown_attribute_is_refused_by_name() {
    assert!(matches!(
        parse(FIXTURE, SpeedClass::RAPIER),
        Err(Error::UnknownAttribute { attribute, .. }) if attribute == "Bogus"
    ));
}
