//! A title's laws that its engine compiles in where Pulse reads them from `handlingstats.xml`.
//!
//! Wipeout Pure authors no `<pitch>` in any `<Class>`: its `Ship_UpdatePitch` (`0x0892caf8`)
//! and `Ship_ApplyAngularDamping` (`0x0892cc2c`) carry the numbers as literals, measured live in
//! `docs/physics/pure-physics.md`. A title whose table carries its own fills them here, in place
//! of `oag_gameplay::handling::PITCH_STAND_IN`, for the player and every opponent alike. Its
//! `<Airbrake>` authors no `sideshift` either, and its airbrake function pushes at `200.0`.
//!
//! The other way round too: Wipeout HD reads its drawn lean's stick gain and follower rate from
//! the team's `<Tilt>` where Pulse compiles them in, so a title whose engine does fills
//! `Handling::steer_lean` from the file here, and the cockpit camera's lean the same way
//! (`Handling::camera_lean`, `<Tilt internalAmount internalSpeed>`).

use oag_tables::handling;
use oag_title::craft_laws::CraftLaws;

/// Fills `handling` from the title's compiled-in laws wherever `class` authors nothing.
pub(super) fn apply(
    handling: &mut oag_physics::Handling,
    stats: &handling::Stats,
    class: &str,
    laws: Option<&CraftLaws>,
) {
    if let (Some(smooth), Some(tilt)) = (laws.and_then(|laws| laws.tilt_steer_lean), stats.tilt) {
        handling.steer_lean = oag_physics::SteerLean {
            stick_amount: tilt.ship_amount,
            follower_rate: tilt.ship_speed,
            smooth_rate: smooth.value,
        };
    }
    if let (Some(camera), Some(tilt)) = (laws.and_then(|laws| laws.tilt_camera_lean), stats.tilt) {
        handling.camera_lean = oag_physics::CameraLean {
            stick_scale: 0.015 * tilt.internal_amount,
            follower_rate: tilt.internal_speed,
            smooth_rate: camera.value.smooth_rate,
            sideshift_offset: camera.value.sideshift_offset,
            clamp: oag_physics::CameraClamp::Hd,
        };
    }
    if let Some(block) = stats.class_named(class) {
        fill(
            handling,
            block.pitch.is_some(),
            block.airbrake.sideshift.is_some(),
            laws,
        );
    }
}

/// [`apply`] once the class block has been read: whether it authors a `<pitch>` and an
/// `<Airbrake sideshift>`.
fn fill(
    handling: &mut oag_physics::Handling,
    authored_pitch: bool,
    authored_sideshift: bool,
    laws: Option<&CraftLaws>,
) {
    if !authored_sideshift && let Some(sideshift) = laws.and_then(|laws| laws.code_sideshift) {
        handling.airbrake.sideshift = sideshift.value;
    }
    if !authored_pitch && let Some(pitch) = laws.and_then(|laws| laws.code_pitch) {
        handling.pitch.pitch_air = pitch.value.air;
        handling.pitch.pitch_ground = pitch.value.ground;
        handling.pitch.pitch_damping = pitch.value.damping;
    }
}

#[cfg(test)]
mod tests {
    use super::fill;
    use oag_title::craft_laws::{CodePitch, CraftLaws};
    use oag_title::pre_race::Sourced;

    fn laws(code_pitch: Option<Sourced<CodePitch>>) -> CraftLaws {
        CraftLaws {
            inertia_mass: Sourced::chosen(0.9),
            steer_ramp_clamped: Sourced::chosen(false),
            air_pitch_bias: Sourced::chosen(false),
            class_gravity_on_flight: Sourced::chosen(false),
            engine_output_double: Sourced::chosen(true),
            drag_ground: Sourced::chosen(-0.005),
            bank_to_yaw: Sourced::chosen(30.0),
            bank_to_yaw_zone: Sourced::chosen(50.0),
            wall_one_sided: Sourced::chosen(false),
            wall_steer_away: Sourced::chosen(false),
            wall_point_rate_body_local: Sourced::chosen(false),
            cage_from_track_header: Sourced::chosen(true),
            code_pitch,
            code_sideshift: None,
            code_start_boost: None,
            tilt_steer_lean: None,
            tilt_camera_lean: None,
            pitch_gate: Sourced::chosen(true),
            eliminator_wrong_way_kill: Sourced::chosen(true),
            sideshift_grid_gate: Sourced::chosen(true),
            turbo_running_laws: Sourced::chosen(true),
            inertia_rotates: Sourced::chosen(true),
        }
    }

    /// A title's compiled-in pitch reaches a class that authors none, and only such a class.
    #[test]
    fn a_compiled_in_pitch_fills_an_unauthored_class_and_nothing_else() {
        let pure = laws(Some(Sourced::measured(CodePitch {
            air: 0.1,
            ground: 0.4,
            damping: 1.5,
            grid_damping: 0.6,
        })));
        let stand_in = oag_gameplay::handling::PITCH_STAND_IN;
        let mut handling = oag_physics::Handling {
            pitch: stand_in,
            ..oag_physics::Handling::ZERO
        };
        fill(&mut handling, true, true, Some(&pure));
        assert_eq!(handling.pitch, stand_in, "an authored block is the file's");
        fill(&mut handling, false, true, Some(&laws(None)));
        assert_eq!(
            handling.pitch, stand_in,
            "no compiled-in pitch, the stand-in"
        );
        fill(&mut handling, false, true, Some(&pure));
        let p = handling.pitch;
        assert_eq!(
            (p.pitch_air, p.pitch_ground, p.pitch_damping),
            (0.1, 0.4, 1.5)
        );
    }

    /// A title's compiled-in sideshift reaches a class that authors none, and only such a class.
    #[test]
    fn a_compiled_in_sideshift_fills_an_unauthored_class_and_nothing_else() {
        let pure = CraftLaws {
            code_sideshift: Some(Sourced::measured(200.0)),
            ..laws(None)
        };
        let mut handling = oag_physics::Handling::ZERO;
        handling.airbrake.sideshift = 7.0;
        fill(&mut handling, true, true, Some(&pure));
        assert_eq!(
            handling.airbrake.sideshift, 7.0,
            "an authored value is the file's"
        );
        fill(&mut handling, true, false, Some(&laws(None)));
        assert_eq!(
            handling.airbrake.sideshift, 7.0,
            "nothing compiled in, nothing changes"
        );
        fill(&mut handling, true, false, Some(&pure));
        assert_eq!(handling.airbrake.sideshift, 200.0);
    }
}
