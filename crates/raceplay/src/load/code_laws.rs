//! A title's laws that its engine compiles in where Pulse reads them from `handlingstats.xml`.
//!
//! Wipeout Pure authors no `<pitch>` in any `<Class>`: its `Ship_UpdatePitch` (`0x0892caf8`)
//! and `Ship_ApplyAngularDamping` (`0x0892cc2c`) carry the numbers as literals, measured live in
//! `docs/physics/pure-physics.md`. A title whose table carries its own fills them here, in place
//! of `oag_gameplay::handling::PITCH_STAND_IN`, for the player and every opponent alike.

use oag_tables::handling;
use oag_title::craft_laws::CraftLaws;

/// Fills `handling` from the title's compiled-in laws wherever `class` authors nothing.
pub(super) fn apply(
    handling: &mut oag_physics::Handling,
    stats: &handling::Stats,
    class: &str,
    laws: Option<&CraftLaws>,
) {
    if let Some(block) = stats.class_named(class) {
        fill(handling, block.pitch.is_some(), laws);
    }
}

/// [`apply`] once the class block has been read: `authored_pitch` is whether it carries one.
fn fill(handling: &mut oag_physics::Handling, authored_pitch: bool, laws: Option<&CraftLaws>) {
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
            code_pitch,
        }
    }

    /// A title's compiled-in pitch reaches a class that authors none, and only such a class.
    #[test]
    fn a_compiled_in_pitch_fills_an_unauthored_class_and_nothing_else() {
        let pure = laws(Some(Sourced::measured(CodePitch {
            air: 0.1,
            ground: 0.4,
            damping: 1.5,
        })));
        let stand_in = oag_gameplay::handling::PITCH_STAND_IN;
        let mut handling = oag_physics::Handling {
            pitch: stand_in,
            ..oag_physics::Handling::ZERO
        };
        fill(&mut handling, true, Some(&pure));
        assert_eq!(handling.pitch, stand_in, "an authored block is the file's");
        fill(&mut handling, false, Some(&laws(None)));
        assert_eq!(
            handling.pitch, stand_in,
            "no compiled-in pitch, the stand-in"
        );
        fill(&mut handling, false, Some(&pure));
        let p = handling.pitch;
        assert_eq!(
            (p.pitch_air, p.pitch_ground, p.pitch_damping),
            (0.1, 0.4, 1.5)
        );
    }
}
