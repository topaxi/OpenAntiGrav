//! Turning a snapshot of pilot intent into the controls the force law reads.
//!
//! Small enough to inline at the call site, and deliberately not: this is the
//! only place where "cross means thrust" is written down, and it is the sort of
//! binding that would otherwise end up asserted differently in three files. The
//! button *names* come from the original's own front-end XML vocabulary; see
//! `docs/ghidra/functions/psp-pulse-usa/input.md`.

use oag_physics::ship::{ShipControls, Sideshift};

use crate::input::{Button, InputSnapshot};

/// Which of the original's two control schemes the pilot is using.
///
/// Not a preference this project invented: the game binds eight abstract
/// *actions* rather than buttons, and the scheme decides which of them exist.
/// `Options_LoadControlMapping` (`0x08836a48`) picks it from the `Control_Type`
/// profile setting, and the row the options page shows changes with it. See
/// `docs/ghidra/functions/psp-pulse-usa/input-bindings.md`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ControlScheme {
    /// `R` is one airbrake button that picks its side off the steering, and
    /// `L` is a dedicated sideshift button: hold it and flick the stick.
    ///
    /// `R` with the stick left drives the left airbrake, with it right the
    /// right one, and with it centred both - see [`novice_airbrakes`].
    Novice,
    /// `L` and `R` are the two airbrakes separately, and there is no sideshift
    /// button - double-tap an airbrake instead.
    ///
    /// The default, at confidence **90**: two independent paths in
    /// `Options_LoadControlMapping` select it, and a 2026-08-27 capture off a
    /// genuinely never-configured PPSSPP profile (first-boot dialogs answered
    /// fresh, no `Control_Type` ever written) fired the veteran double-tap
    /// gesture correctly with no scheme set explicitly - see
    /// `docs/ghidra/functions/psp-pulse-usa/input-bindings.md`'s runtime
    /// section. If a measurement overturns that, this attribute moves and
    /// nothing else does.
    #[default]
    Veteran,
}

impl ControlScheme {
    /// Every scheme, in the order a menu should offer them.
    ///
    /// Two, not three. The original's `Control_Type` also takes `custom`, which
    /// is not a third scheme but a *rebinding* of whichever one is live -
    /// `Options_LoadControlMapping` copies a saved eight-entry table over the
    /// built-in one and leaves the gesture branch alone. Rebinding is not
    /// implemented, so offering the word would promise something that does
    /// nothing.
    pub const ALL: [Self; 2] = [Self::Veteran, Self::Novice];

    /// The token this scheme is stored and configured as.
    ///
    /// **The original's own words**, read out of the `strcasecmp` chains in
    /// `Options_LoadControlMapping` (`0x08836a48`) and
    /// `Options_BuildControlSchemeRows` (`0x0889b468`) - not names this project
    /// picked. See `docs/ghidra/functions/psp-pulse-usa/input-bindings.md`.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Novice => "novice",
            Self::Veteran => "veteran",
        }
    }

    /// The scheme a token names, or `None` if nothing does.
    ///
    /// `None` rather than a default, for the reason `oag_race::Mode::from_name`
    /// gives: a settings file naming a scheme this build does not have should be
    /// visible to the caller, not silently become the default.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|scheme| scheme.name() == name)
    }
}

impl std::fmt::Display for ControlScheme {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

impl std::str::FromStr for ControlScheme {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_name(&s.to_ascii_lowercase()).ok_or_else(|| {
            let names: Vec<_> = Self::ALL.iter().map(|s| s.name()).collect();
            format!("{s:?} is not a control scheme; try {}", names.join(" or "))
        })
    }
}

/// Maps one tick of input onto ship controls.
///
/// The snapshot's axes are used as they are, having already been clamped by
/// [`InputSnapshot::sanitised`]. **Thrust is still a button** - it is the cross
/// button on the original and the snapshot carries no throttle axis - while the
/// airbrakes are axes, which a PSP pad can only ever drive to 0 or 1 and a pad
/// with analog triggers can drive anywhere between. Which device produced the
/// value is not visible from here, and must not be: see
/// `oag_input::pad::TriggerMode` for where that decision lives.
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
/// The airbrake fields are scheme-filtered too, not just the gesture fields.
/// In novice, action 4 (`OPT_CTRL_AIRBRAKES`) is bound to `R` alone and action
/// 7 (`OPT_CTRL_SS`, sideshift) to `L` alone - `L` is a dedicated sideshift
/// button, not an airbrake at all. `R` picks its side off the steering, which
/// [`novice_airbrakes`] carries, and [`InputSnapshot::airbrake_left`] is
/// ignored for airbrake purposes: `L` only arms `shift_modifier`. See
/// `docs/ghidra/functions/psp-pulse-usa/input-bindings.md`.
///
/// [`ShipControls::sideshift`] stays [`Sideshift::None`] here always. It is the
/// *direct* request, for a test or a probe that wants a shift on a named tick;
/// a real pilot's shift arrives through the gesture.
#[must_use]
pub fn ship_controls(snapshot: &InputSnapshot, scheme: ControlScheme) -> ShipControls {
    let novice = scheme == ControlScheme::Novice;
    let (airbrake_left, airbrake_right) = if novice {
        novice_airbrakes(snapshot.airbrake_right, snapshot.stick_x)
    } else {
        (snapshot.airbrake_left, snapshot.airbrake_right)
    };
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
        thrust: f32::from(u8::from(snapshot.buttons.is_held(Button::Cross))),
        // Novice: action 4 on `R` picks a side off the steering, and `L`
        // (action 7, the sideshift) contributes none. Veteran keeps the
        // straight pass-through, actions 5/6 (`OPT_CTRL_LAB`/`OPT_CTRL_RAB`)
        // bound to `L` and `R` respectively.
        airbrake_left,
        airbrake_right,
        sideshift: Sideshift::None,
        // Action 7, `OPT_CTRL_SS`, bound to `L` by the shipped default mapping.
        shift_modifier: novice && snapshot.buttons.is_held(Button::L),
        // Actions 5 and 6, `OPT_CTRL_LAB`/`OPT_CTRL_RAB`, bound to `L` and `R`.
        // Read off the *pressed* mask rather than held, because the original
        // reads the pressed mask at `*(craft+0x78) + 0x20` for this branch and
        // a held airbrake must not repeat-fire a shift.
        shift_tap_left: !novice && snapshot.buttons.is_pressed(Button::L),
        shift_tap_right: !novice && snapshot.buttons.is_pressed(Button::R),
        // The barrel roll's d-pad leg, on the *pressed* mask for the same
        // reason the airbrake taps are: a held direction is one tap, not one a
        // tick. Filled for both schemes, because the roll is not a
        // scheme-dependent gesture - neither scheme spends the d-pad on
        // anything else. The gesture's other leg, the steering axis crossing
        // `+-90`, is read off `ShipControls::steer_x` inside
        // `oag_physics::barrel_roll::advance_gesture`, which is also where the
        // two are ORed into one tap; see that function for why they must be.
        roll_tap_left: snapshot.buttons.is_pressed(Button::Left),
        roll_tap_right: snapshot.buttons.is_pressed(Button::Right),
        // A real pad, so **never**: a human's roll arrives through the gesture
        // above, exactly as the original's does. `roll_request` is the direct
        // request `oag_ai::Driver` sets instead, which is a deliberate
        // deviation and documented as one - see
        // `oag_physics::ShipControls::roll_request`.
        roll_request: None,
        // And **zero**, which is the recovered behaviour exactly: the invented
        // shield floor that used to be
        // `oag_physics::barrel_roll::AI_ROLL_SHIELD_FLOOR` is a per-pilot
        // number now, and the human player's path does not carry one.
        roll_shield_floor: 0.0,
    }
}

/// How far the steering must lean, on `-1..=1`, before novice's airbrake
/// button picks a side: `10` on the original's `+/-100` steer record.
///
/// `PlayerInput_Update` (`0x0883c870`) compares the record's steer against the
/// literals `-10.0`/`10.0`, after its own `0.2` deadzone and response curve,
/// so on the PSP nub this is a raw deflection of `0.36`. The port's stick is
/// shaped by `oag_input::pad` instead and reaches here already standing for
/// the record over `100`, which is why the comparison is against `0.1`.
pub const NOVICE_AIRBRAKE_STEER_THRESHOLD: f32 = 0.1;

/// Novice's one airbrake button, split onto the two airbrakes by the steering.
///
/// The law of `PlayerInput_Update`'s novice branch (`0x0883c870`), static read
/// and PPSSPP-measured at confidence **95** (see
/// `docs/ghidra/functions/psp-pulse-usa/input-bindings.md`): with the button
/// held, steering past [`NOVICE_AIRBRAKE_STEER_THRESHOLD`] to the left drives
/// the left airbrake alone, past it to the right the right airbrake alone, and
/// anything within it, the threshold itself included, drives both - which is
/// the brake. Both are cleared every frame, so there is no memory of the last
/// side, and the d-pad steers the record to full lock, so it picks a side
/// exactly as a full stick does.
///
/// The original's button is all or nothing. `button` is carried onto the
/// chosen side or sides unscaled, which is the same thing on a digital button
/// and, for an analogue trigger, **chosen, not measured**.
#[must_use]
pub fn novice_airbrakes(button: f32, steer_x: f32) -> (f32, f32) {
    if steer_x < -NOVICE_AIRBRAKE_STEER_THRESHOLD {
        (button, 0.0)
    } else if steer_x > NOVICE_AIRBRAKE_STEER_THRESHOLD {
        (0.0, button)
    } else {
        (button, button)
    }
}

#[cfg(test)]
mod tests;
