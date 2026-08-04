//! Turning a snapshot of pilot intent into the controls the force law reads.
//!
//! Small enough to inline at the call site, and deliberately not: this is the
//! only place where "cross means thrust" is written down, and it is the sort of
//! binding that would otherwise end up asserted differently in three files. The
//! button *names* come from the original's own front-end XML vocabulary; see
//! `docs/ghidra/functions/psp-pulse/input.md`.

use oag_physics::ship::{ShipControls, Sideshift};

use crate::input::{InputSnapshot, button};

/// Which of the original's two control schemes the pilot is using.
///
/// Not a preference this project invented: the game binds eight abstract
/// *actions* rather than buttons, and the scheme decides which of them exist.
/// `Options_LoadControlMapping` (`0x08836a48`) picks it from the `Control_Type`
/// profile setting, and the row the options page shows changes with it. See
/// `docs/ghidra/functions/psp-pulse/input-bindings.md`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ControlScheme {
    /// `R` is both airbrakes at once and `L` is a dedicated sideshift button:
    /// hold it and flick the stick.
    Novice,
    /// `L` and `R` are the two airbrakes separately, and there is no sideshift
    /// button - double-tap an airbrake instead.
    ///
    /// The default, at confidence **75**: two independent paths in
    /// `Options_LoadControlMapping` select it and the value a never-configured
    /// profile holds was not read. If a measurement overturns that, this
    /// attribute moves and nothing else does.
    #[default]
    Veteran,
}

/// Maps one tick of input onto ship controls.
///
/// The snapshot's axes are used as they are, having already been clamped by
/// [`InputSnapshot::sanitised`]. Thrust and braking are buttons on a PSP pad and
/// so are read as 0 or 1 here; a pad with analog triggers would want them as
/// axes on the snapshot instead, which is a change to
/// [`InputSnapshot`](crate::input::InputSnapshot) and not to this function.
///
/// # Sideshift
///
/// This function does **not** decide that a sideshift happens - it reports which
/// buttons the gesture needs, and `oag_physics::airbrake::advance_sideshift`
/// runs the gesture. That split is the original's: the tap windows and the
/// flick's armed latch are per-craft state on the entity, not input state, so
/// putting them here would mean a second copy of them per input device.
///
/// Only the live scheme's fields are filled. The other scheme's stay `false`,
/// so the dormant gesture machine never sees an input - which is how one code
/// path in the physics crate can carry both schemes without knowing what a
/// scheme is.
///
/// [`ShipControls::sideshift`] stays [`Sideshift::None`] here always. It is the
/// *direct* request, for a test or a probe that wants a shift on a named tick;
/// a real pilot's shift arrives through the gesture.
#[must_use]
pub fn ship_controls(snapshot: &InputSnapshot, scheme: ControlScheme) -> ShipControls {
    let novice = scheme == ControlScheme::Novice;
    ShipControls {
        steer_x: snapshot.stick_x,
        // Up on the stick pitches the nose DOWN, which is a binding and not a
        // physics sign: `ShipControls::steer_y` is positive nose up, and the
        // original's pitch axis at `*(craft+0x78) + 0x10` is negative nose up.
        // Measured in PPSSPP, both directions held for 120 ticks - `up` on the
        // d-pad writes `-100` there and the craft's forward row drops, `down`
        // writes `+100` and the nose rises - and the analog stick's `y` feeds the
        // same field with the same sign. See `oag_physics::engine::pitch`.
        //
        // Inverting here rather than inside the force term is deliberate: the
        // term keeps the shape `Ship_UpdatePitch` has, and the fact that a
        // Wipeout pushes the nose down when the player pushes up stays at the
        // input boundary, which is where a player would also expect to find it if
        // it ever becomes an option.
        steer_y: -snapshot.stick_y,
        thrust: f32::from(u8::from(snapshot.buttons.is_held(button::CROSS))),
        airbrake_left: snapshot.airbrake_left,
        airbrake_right: snapshot.airbrake_right,
        sideshift: Sideshift::None,
        // Action 7, `OPT_CTRL_SS`, bound to `L` by the shipped default mapping.
        shift_modifier: novice && snapshot.buttons.is_held(button::L),
        // Actions 5 and 6, `OPT_CTRL_LAB`/`OPT_CTRL_RAB`, bound to `L` and `R`.
        // Read off the *pressed* mask rather than held, because the original
        // reads the pressed mask at `*(craft+0x78) + 0x20` for this branch and
        // a held airbrake must not repeat-fire a shift.
        shift_tap_left: !novice && snapshot.buttons.is_pressed(button::L),
        shift_tap_right: !novice && snapshot.buttons.is_pressed(button::R),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Input;

    fn held(index: u8) -> Input {
        let mut input = Input::new();
        input.begin_frame(1u32 << (index & 0x1f));
        input
    }

    #[test]
    fn cross_is_thrust() {
        let snapshot = InputSnapshot {
            buttons: held(button::CROSS),
            ..InputSnapshot::new()
        };
        assert_eq!(
            ship_controls(&snapshot, ControlScheme::default()).thrust,
            1.0
        );
    }

    /// Cross is `activate` in the front end and thrust in a race. Circle is
    /// neither, and mapping it to thrust is the single most likely slip here.
    #[test]
    fn circle_is_not_thrust() {
        let snapshot = InputSnapshot {
            buttons: held(button::CIRCLE),
            ..InputSnapshot::new()
        };
        assert_eq!(
            ship_controls(&snapshot, ControlScheme::default()).thrust,
            0.0
        );
    }

    /// Three of the four axes pass through, and the fourth is inverted.
    ///
    /// `stick_y` is the only one that is not a pass-through, and it is the whole
    /// point of this test: pushing the stick up pitches the nose down, measured
    /// off the original in PPSSPP. This test asserted a pass-through until that
    /// measurement existed.
    #[test]
    fn the_axes_pass_through_except_the_inverted_pitch() {
        let snapshot = InputSnapshot {
            stick_x: -0.5,
            stick_y: 0.25,
            airbrake_left: 1.0,
            airbrake_right: 0.5,
            ..InputSnapshot::new()
        };
        let controls = ship_controls(&snapshot, ControlScheme::default());
        assert_eq!(controls.steer_x, -0.5);
        assert_eq!(controls.steer_y, -0.25, "stick up is nose down");
        assert_eq!(controls.airbrake_left, 1.0);
        assert_eq!(controls.airbrake_right, 0.5);
    }

    /// The gesture fields are what a scheme selects, and they are exclusive.
    ///
    /// Holding `L` is a flick modifier on novice and a left-airbrake tap on
    /// veteran, and it must never be both: the two gesture machines run side by
    /// side in the physics crate and rely on this to stay dormant.
    #[test]
    fn a_scheme_fills_only_its_own_gesture_fields() {
        let snapshot = InputSnapshot {
            buttons: held(button::L),
            ..InputSnapshot::new()
        };

        let novice = ship_controls(&snapshot, ControlScheme::Novice);
        assert!(novice.shift_modifier, "L is the novice sideshift button");
        assert!(!novice.shift_tap_left);
        assert!(!novice.shift_tap_right);

        let veteran = ship_controls(&snapshot, ControlScheme::Veteran);
        assert!(!veteran.shift_modifier, "veteran has no sideshift button");
        assert!(veteran.shift_tap_left, "L is the veteran left airbrake");
        assert!(!veteran.shift_tap_right);
    }

    /// A held airbrake is one tap, not one per tick.
    ///
    /// `is_pressed` and not `is_held` is the whole of what stops a veteran
    /// pilot leaning on an airbrake from sideshifting continuously.
    #[test]
    fn a_veteran_tap_is_an_edge_and_not_a_level() {
        let mut buttons = Input::new();
        buttons.begin_frame(1 << button::L);
        let first = InputSnapshot {
            buttons,
            ..InputSnapshot::new()
        };
        assert!(ship_controls(&first, ControlScheme::Veteran).shift_tap_left);

        buttons.begin_frame(1 << button::L);
        let second = InputSnapshot {
            buttons,
            ..InputSnapshot::new()
        };
        assert!(!ship_controls(&second, ControlScheme::Veteran).shift_tap_left);
    }

    /// The direct request stays a caller's business, never a pilot's.
    ///
    /// A real shift arrives through the gesture machine in `oag_physics`, so
    /// this field staying `None` is the boundary between the two and not a gap.
    #[test]
    fn no_scheme_produces_a_direct_sideshift_request() {
        let snapshot = InputSnapshot {
            buttons: held(button::L),
            airbrake_left: 1.0,
            stick_x: 1.0,
            ..InputSnapshot::new()
        };
        for scheme in [ControlScheme::Novice, ControlScheme::Veteran] {
            assert_eq!(ship_controls(&snapshot, scheme).sideshift, Sideshift::None);
        }
    }

    #[test]
    fn no_input_produces_no_controls() {
        for scheme in [ControlScheme::Novice, ControlScheme::Veteran] {
            assert_eq!(
                ship_controls(&InputSnapshot::new(), scheme),
                ShipControls::default()
            );
        }
    }
}
