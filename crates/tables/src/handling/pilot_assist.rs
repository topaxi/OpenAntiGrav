//! `<GlobalClass><PilotAssist/><PilotAssistPenalty/></GlobalClass>`: the
//! per-class tuning of HD's and 2048's Pilot Assist.
//!
//! HD's `Handling_ReadGlobalClasses` (`0x000c6af0`) and 2048's
//! `Handling_ReadGlobalSettings` (`0x811ace10`) open **one** attribute loop
//! for either element, so an attribute may sit in either one; this reads both
//! and takes each attribute from whichever carries it. Pulse and Pure author
//! neither element, which is [`None`] rather than zeroes. Evidence:
//! `docs/ghidra/functions/ps3-hdfury-eu/pilot-assist.md`.

use super::{Error, Node, Result, SpeedClass, number};

/// One speed class's Pilot Assist numbers, verbatim from the file.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PilotAssist {
    /// `laDistConst`: the look-ahead distance at rest.
    pub la_dist_const: f32,
    /// `laDistVelMul`: look-ahead added per unit of speed.
    pub la_dist_vel_mul: f32,
    /// `laDistMax`: the look-ahead's ceiling.
    pub la_dist_max: f32,
    /// `springMul`: the lateral push per unit of edge intrusion (negative).
    pub spring_mul: f32,
    /// `torqueMul`: the yaw torque per unit of edge intrusion ahead.
    pub torque_mul: f32,
    /// `maxTorque`: the yaw torque's clamp.
    pub max_torque: f32,
    /// `maxAngVel`: no yaw torque while the craft already spins faster.
    pub max_ang_vel: f32,
    /// `generalThrustPercentWhenEnabled`: the throttle scale while enabled.
    pub general_thrust_percent: f32,
    /// `thrustPercentOnUse`: the throttle scale while the penalty runs.
    pub thrust_percent_on_use: f32,
    /// `penaltyDuration`: seconds the penalty runs after a correction.
    pub penalty_duration: f32,
}

const ELEMENTS: [&str; 2] = ["PilotAssist", "PilotAssistPenalty"];

impl PilotAssist {
    /// Reads one `<GlobalClass>`'s two elements; `Ok(None)` when it authors
    /// neither, an error when it authors one but leaves an attribute out.
    pub(super) fn from_class(class: &Node) -> Result<Option<Self>> {
        let nodes: Vec<&Node> = ELEMENTS
            .iter()
            .flat_map(|name| class.children_named(name))
            .collect();
        if nodes.is_empty() {
            return Ok(None);
        }
        let read = |attribute: &'static str| -> Result<f32> {
            match nodes.iter().find(|node| node.value(attribute).is_some()) {
                Some(node) => number(node, ELEMENTS[0], attribute),
                None => Err(Error::MissingAttribute {
                    element: ELEMENTS[0],
                    attribute,
                }),
            }
        };
        Ok(Some(Self {
            la_dist_const: read("laDistConst")?,
            la_dist_vel_mul: read("laDistVelMul")?,
            la_dist_max: read("laDistMax")?,
            spring_mul: read("springMul")?,
            torque_mul: read("torqueMul")?,
            max_torque: read("maxTorque")?,
            max_ang_vel: read("maxAngVel")?,
            general_thrust_percent: read("generalThrustPercentWhenEnabled")?,
            thrust_percent_on_use: read("thrustPercentOnUse")?,
            penalty_duration: read("penaltyDuration")?,
        }))
    }
}

/// Every recognised `<GlobalClass>`'s [`PilotAssist`], indexed by [`SpeedClass`].
pub(super) fn per_class(global: &Node) -> Result<[Option<PilotAssist>; 4]> {
    let mut out = [None; 4];
    for node in global.children_named("GlobalClass") {
        let Some(class) = node
            .value("name")
            .and_then(|name| SpeedClass::from_name(name.trim()))
        else {
            continue;
        };
        out[class as usize] = PilotAssist::from_class(node)?;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handling::parse_global;

    /// Invented numbers in HD's shape: the attributes split over both elements.
    fn document(class_body: &str) -> String {
        let block = |name: &str| {
            format!(
                r#"<GlobalClass name="{name}"><SpeedupPads amount="1" time="1"/>
                <GravityMul airborne="1"/><WeaponPad refresh_time="1"/>{class_body}</GlobalClass>"#
            )
        };
        format!(
            r#"<Handling><Global><Special roll_cost="1" roll_speed="1" roll_turbotime="1" speedpad_jump="1" turbo_jump="1"/>
            <Zone increment="1" recharge="1" start="1"/>{}{}{}{}</Global></Handling>"#,
            block("VENOM"),
            block("FLASH"),
            block("RAPIER"),
            block("PHANTOM")
        )
    }

    const SPLIT: &str = r#"<PilotAssist laDistConst="1" laDistVelMul="2" laDistMax="3" springMul="-4" torqueMul="5" maxTorque="6" maxAngVel="7"/>
        <PilotAssistPenalty generalThrustPercentWhenEnabled="98" thrustPercentOnUse="90" penaltyDuration="2"/>"#;

    #[test]
    fn both_elements_fill_one_record() {
        let global = parse_global(&document(SPLIT)).unwrap().unwrap();
        let assist = global.pilot_assist(SpeedClass::Rapier).unwrap();
        assert_eq!(assist.la_dist_const, 1.0);
        assert_eq!(assist.spring_mul, -4.0);
        assert_eq!(assist.max_ang_vel, 7.0);
        assert_eq!(assist.general_thrust_percent, 98.0);
        assert_eq!(assist.thrust_percent_on_use, 90.0);
        assert_eq!(assist.penalty_duration, 2.0);
    }

    #[test]
    fn a_title_without_the_elements_has_no_assist() {
        let global = parse_global(&document("")).unwrap().unwrap();
        for class in SpeedClass::ALL {
            assert_eq!(global.pilot_assist(class), None, "{class:?}");
        }
    }

    #[test]
    fn a_half_authored_block_is_an_error_not_a_zero() {
        let body = r#"<PilotAssist laDistConst="1"/>"#;
        assert!(parse_global(&document(body)).is_err());
    }
}
