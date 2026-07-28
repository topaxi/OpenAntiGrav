//! Turning a snapshot of pilot intent into the controls the force law reads.
//!
//! Small enough to inline at the call site, and deliberately not: this is the
//! only place where "cross means thrust" is written down, and it is the sort of
//! binding that would otherwise end up asserted differently in three files. The
//! button *names* come from the original's own front-end XML vocabulary; see
//! `docs/ghidra/functions/psp-pulse/input.md`.

use oag_physics::ship::{ShipControls, Sideshift};

use crate::input::{InputSnapshot, button};

/// Maps one tick of input onto ship controls.
///
/// The snapshot's axes are used as they are, having already been clamped by
/// [`InputSnapshot::sanitised`]. Thrust and braking are buttons on a PSP pad and
/// so are read as 0 or 1 here; a pad with analog triggers would want them as
/// axes on the snapshot instead, which is a change to
/// [`InputSnapshot`](crate::input::InputSnapshot) and not to this function.
///
/// # What is not mapped
///
/// **Sideshift.** The original's binding is not recovered: it is a one-shot
/// impulse rather than a held axis, and which button combination fires it has
/// not been read out of the binary. Rather than guess, this returns
/// [`Sideshift::None`] always, so the manoeuvre is simply unavailable until
/// somebody establishes the binding. A plausible-looking guess here would be
/// worse than the gap, because it would look implemented.
#[must_use]
pub fn ship_controls(snapshot: &InputSnapshot) -> ShipControls {
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
        assert_eq!(ship_controls(&snapshot).thrust, 1.0);
    }

    /// Cross is `activate` in the front end and thrust in a race. Circle is
    /// neither, and mapping it to thrust is the single most likely slip here.
    #[test]
    fn circle_is_not_thrust() {
        let snapshot = InputSnapshot {
            buttons: held(button::CIRCLE),
            ..InputSnapshot::new()
        };
        assert_eq!(ship_controls(&snapshot).thrust, 0.0);
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
        let controls = ship_controls(&snapshot);
        assert_eq!(controls.steer_x, -0.5);
        assert_eq!(controls.steer_y, -0.25, "stick up is nose down");
        assert_eq!(controls.airbrake_left, 1.0);
        assert_eq!(controls.airbrake_right, 0.5);
    }

    /// Pins the gap rather than the behaviour: when somebody recovers the
    /// binding, this test is what tells them where to add it.
    #[test]
    fn sideshift_is_never_produced_because_its_binding_is_unknown() {
        let snapshot = InputSnapshot {
            buttons: held(button::L),
            airbrake_left: 1.0,
            ..InputSnapshot::new()
        };
        assert_eq!(ship_controls(&snapshot).sideshift, Sideshift::None);
    }

    #[test]
    fn no_input_produces_no_controls() {
        assert_eq!(
            ship_controls(&InputSnapshot::new()),
            ShipControls::default()
        );
    }
}
