//! Which law each Pilot Assist level runs for the player's craft, and where its
//! numbers came from.
//!
//! The rule is one: **play what the disc authors, and hold a labelled stand-in only
//! where it authors nothing**, per level.
//!
//! - **Extreme** is the global `<GlobalClass><PilotAssist/></GlobalClass>` rung
//!   (HD, 2048, Omega). A title that authors the element for no class at all
//!   (Pulse, Pure) runs [`EXTREME_CHOSEN`].
//! - **Normal** is the ship's own `<Class><Assist/></Class>` block with the global
//!   `<SteerAssist/>` ramp (2048, Omega). A ship that authors no `<Assist>` (HD,
//!   Pulse, Pure, and 2048's twelve HD-derived guest teams) runs [`NORMAL_CHOSEN`],
//!   and a title with no ramp runs [`RAMP_CHOSEN`].
//!
//! **Chosen, not measured**, all of it: the maintainer's ruling of 2026-10-10 that
//! every title offers 2048's three levels. The numbers are 2048's own, copied from
//! its disc, so they are re-derivable: `pilot_assist_levels_ground_truth` reads them
//! back and fails if either set drifts. Speed units need no scaling: a Venom's top
//! speed under thrust is 116-125 units/s on Pulse, Pure, HD, 2048 and Omega alike
//! (same test), so 2048's `min_speed 50` and `ramp_up_range 25` mean the same thing
//! on all five.

use oag_physics::pilot_assist::{Law, Laws, Params, Ramp};

use super::*;

/// 2048's per-ship `<Assist>`, identical on all 100 class blocks of its 20 native
/// craft and on all 100 of Omega's 20: `data/HandlingStats/<team>2048/<n>/handlingstats.xml`.
/// **Chosen, not measured**, wherever a ship authors none.
pub(crate) const NORMAL_CHOSEN: handling::Assist = handling::Assist {
    la_dist_const: 10.0,
    la_dist_vel_mul: 0.4,
    la_dist_max: 75.0,
    spring_mul: -12.0,
    torque_mul: 10.0,
    max_torque: 50.0,
    max_ang_vel: 2.0,
    thrust_percent_on_use: 95.0,
    penalty_duration: 0.25,
    not_in_use_strength: 0.1,
};

/// 2048's `<SteerAssist min_speed ramp_up_range>`, authored on every one of its rungs.
/// **Chosen, not measured**, wherever a title authors none.
pub(crate) const RAMP_CHOSEN: handling::SteerAssist = handling::SteerAssist {
    min_speed: 50.0,
    ramp_up_range: 25.0,
};

/// One rung of 2048's global `<PilotAssist>`/`<PilotAssistPenalty>`, as it names it.
const fn rung(
    la_dist_vel_mul: f32,
    la_dist_max: f32,
    spring_mul: f32,
    torque_mul: f32,
    max_torque: f32,
    max_ang_vel: f32,
    penalty: (f32, f32),
) -> handling::PilotAssist {
    handling::PilotAssist {
        la_dist_const: 10.0,
        la_dist_vel_mul,
        la_dist_max,
        spring_mul,
        torque_mul,
        max_torque,
        max_ang_vel,
        general_thrust_percent: penalty.0,
        thrust_percent_on_use: penalty.1,
        penalty_duration: 3.0,
    }
}

/// 2048's Extreme ("Super") rungs by the name a class carries, `data/xml/handlingstats.xml`.
/// `VECTOR` is Pure's fifth class. **Chosen, not measured**, wherever a title authors
/// no `<PilotAssist>`.
pub(crate) const EXTREME_CHOSEN: [(&str, handling::PilotAssist); 5] = [
    (
        "VECTOR",
        rung(0.3, 50.0, -100.0, 60.0, 1000.0, 3.0, (99.0, 92.0)),
    ),
    (
        "VENOM",
        rung(0.4, 75.0, -15.0, 50.0, 600.0, 2.0, (99.0, 92.0)),
    ),
    (
        "FLASH",
        rung(0.4, 75.0, -15.0, 50.0, 600.0, 2.0, (99.0, 92.0)),
    ),
    (
        "RAPIER",
        rung(0.4, 75.0, -15.0, 50.0, 600.0, 2.0, (98.0, 90.0)),
    ),
    (
        "PHANTOM",
        rung(0.4, 75.0, -15.0, 50.0, 600.0, 2.0, (97.0, 88.0)),
    ),
];

fn extreme_params(p: &handling::PilotAssist) -> Params {
    Params {
        la_dist_const: p.la_dist_const,
        la_dist_vel_mul: p.la_dist_vel_mul,
        la_dist_max: p.la_dist_max,
        spring_mul: p.spring_mul,
        torque_mul: p.torque_mul,
        max_torque: p.max_torque,
        max_ang_vel: p.max_ang_vel,
        general_thrust_percent: p.general_thrust_percent,
        thrust_percent_on_use: p.thrust_percent_on_use,
        penalty_duration: p.penalty_duration,
    }
}

/// Normal pays only its `thrustPercentOnUse`: `<Assist>` authors no general figure,
/// so the throttle is untouched (100) while the penalty is not running.
fn normal_params(a: &handling::Assist) -> Params {
    Params {
        la_dist_const: a.la_dist_const,
        la_dist_vel_mul: a.la_dist_vel_mul,
        la_dist_max: a.la_dist_max,
        spring_mul: a.spring_mul,
        torque_mul: a.torque_mul,
        max_torque: a.max_torque,
        max_ang_vel: a.max_ang_vel,
        general_thrust_percent: 100.0,
        thrust_percent_on_use: a.thrust_percent_on_use,
        penalty_duration: a.penalty_duration,
    }
}

/// Both levels' laws for the player's `ship` class `class`, reporting the source of each.
pub(super) fn resolve(
    global: Option<&handling::Global>,
    ship: Option<&handling::Class>,
    class: &str,
    report: &mut Vec<String>,
) -> Laws {
    let Some(global) = global else {
        report.push("Pilot Assist: no <Global> block, so no Pilot Assist this run".to_string());
        return Laws::default();
    };
    let rung_class = handling::SpeedClass::from_name(class);
    let authored_extreme = rung_class.and_then(|c| global.pilot_assist(c));
    let title_authors_extreme = global.pilot_assist.iter().any(Option::is_some);
    let extreme = match authored_extreme {
        Some(p) => {
            report.push(format!("Pilot Assist Extreme: authored <PilotAssist> for the {class} class"));
            Some(Law { params: extreme_params(&p), ramp: None })
        }
        None if title_authors_extreme => {
            report.push(format!("Pilot Assist Extreme: no <PilotAssist> for the {class} class, so none"));
            None
        }
        None => EXTREME_CHOSEN
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case(class))
            .map(|(name, p)| {
                report.push(format!(
                    "Pilot Assist Extreme: this title authors no <PilotAssist>; running 2048's {name} \
                     rung (chosen, not measured)"
                ));
                Law { params: extreme_params(p), ramp: None }
            }),
    };

    let (assist, assist_label) = match ship.and_then(|c| c.assist) {
        Some(a) => (a, "authored <Assist>"),
        None => (
            NORMAL_CHOSEN,
            "2048's native <Assist> (chosen, not measured)",
        ),
    };
    let (steer, steer_label) = match rung_class.and_then(|c| global.steer_assist(c)) {
        Some(s) => (s, "authored <SteerAssist>"),
        None => (RAMP_CHOSEN, "2048's <SteerAssist> (chosen, not measured)"),
    };
    report.push(format!(
        "Pilot Assist Normal: {assist_label}, ramp from {} over {} units/s: {steer_label}",
        steer.min_speed, steer.ramp_up_range
    ));
    let normal = Some(Law {
        params: normal_params(&assist),
        ramp: Some(Ramp {
            min_speed: steer.min_speed,
            ramp_up_range: steer.ramp_up_range,
            full: assist.not_in_use_strength,
        }),
    });
    Laws { normal, extreme }
}
