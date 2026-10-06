//! Gamepad to abstract button mapping, the pad's half of what
//! [`crate::keys`] does for a keyboard.
//!
//! Hardcoded, deliberately: there is no remapping UI and no Steam Input
//! integration yet (both stay planning, see
//! `docs/overview/modern-features.md`). The layout is the Steam Deck's, which
//! is an Xbox-shaped pad, and the mapping targets the same abstract button
//! layer the keyboard produces, so the simulation cannot tell which device a
//! snapshot came from.
//!
//! The shoulders are a 1:1 binding - L1 and R1 are the abstract `L` and `R`,
//! which are the two airbrakes. What the *analog triggers* do is a choice, and
//! [`TriggerMode`] is that choice; see its variants for the two mappings and
//! why the old one is still offered.
//!
//! Whichever mode is live, a trigger's travel is conditioned before it means
//! anything - [`condition`]. A pad's trigger does not rest at zero and often
//! does not reach one, and neither is a preference: it is a defect being
//! corrected, the way [`STICK_DEADZONE`] corrects a drifting stick.
//!
//! The mapping itself is [`map_button`] and [`resolve`], both free functions
//! over plain values, so every case is testable with no pad attached - which
//! is also why `just test` passes on a machine that has none.
//!
//! **And that is also the limit of what is verified here.** Every case is a
//! unit test, so the numbers a thumb would judge - [`TRIGGER_DEADZONE`],
//! [`TRIGGER_SATURATION`], and the spread
//! `oag_game::settings::TriggerSensitivity::OFFERED` puts on the menu row -
//! are asserted to be self-consistent and have never been felt. They are the
//! first thing to change if the brake reads wrong on real hardware, and
//! changing them breaks nothing but the tests that pin them to each other.

use log::warn;
use oag_gameplay::input::Button;

/// How far a trigger has to travel before it counts as a button press.
///
/// A resting trigger on a worn pad does not read exactly zero, and thrust that
/// engages itself is worse than thrust that needs a deliberate pull.
///
/// **Deliberately further than [`TRIGGER_DEADZONE`]**, and the gap is the
/// point: under [`TriggerMode::Airbrakes`] a trigger is braking from the
/// deadzone up but only counts as a *press* from here, so feathering the brake
/// through a corner cannot fire the veteran double-tap sideshift. Two
/// thresholds on one input reads as an oversight otherwise.
pub const TRIGGER_THRESHOLD: f32 = 0.25;

/// Trigger travel below this is treated as released.
///
/// The stick has [`STICK_DEADZONE`] and the triggers had nothing, which was
/// harmless only while a trigger's analog value went to thrust: a resting
/// trigger that reads `0.05` is 5% of an airbrake applied for a whole race.
pub const TRIGGER_DEADZONE: f32 = 0.10;

/// Trigger travel at or above this is treated as fully pulled.
///
/// The mirror of [`TRIGGER_DEADZONE`] at the other end. A pad whose trigger
/// tops out at `0.97` must still be able to ask for a full airbrake.
pub const TRIGGER_SATURATION: f32 = 0.95;

/// Stick movement below this is treated as centred.
///
/// [`oag_gameplay::InputSnapshot::sanitised`] clamps an axis into range; it
/// cannot tell drift from intent, which is what this is for.
pub const STICK_DEADZONE: f32 = 0.15;

/// The narrowest and widest response curve [`TriggerConfig`] will apply.
///
/// The exponent arrives as a bare `f32` from a settings file, and an exponent
/// of zero is a trigger that is fully pulled the instant it leaves the
/// deadzone. Bounded for the reason `oag_game::settings::TriggerSensitivity`
/// is bounded, one layer up, rather than trusting that it was.
pub const CURVE_RANGE: std::ops::RangeInclusive<f32> = 0.25..=4.0;

/// What the two analog triggers do.
///
/// The shoulders are the airbrakes on any pad; this is only about L2 and R2,
/// which the PSP does not have at all - so neither mapping is recovered
/// behaviour and neither is claimed to be.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TriggerMode {
    /// **The triggers are the two airbrakes**, analog: L2 is the left and R2
    /// the right, each feeding its own axis. Thrust is cross alone, which is
    /// the original's own convention.
    ///
    /// The default, because it is the only mapping that uses what the hardware
    /// offers: the force law ramps each side toward "its analog input", and on
    /// a PSP pad that input can only ever be 0 or 1. Here it need not be.
    ///
    /// A trigger past [`TRIGGER_THRESHOLD`] **also sets its shoulder's button
    /// bit**, exactly as L1 or R1 would. That is not decoration:
    /// `oag_gameplay::ship_controls` reads the veteran sideshift's double-tap
    /// off the *pressed* mask of `L` and `R`, so without the bit a player on
    /// the triggers could not sideshift at all. The analog value survives it -
    /// see the note on [`resolve`].
    #[default]
    Airbrakes,
    /// **R2 is thrust and L2 is "brake"**, which is what this file did before
    /// the airbrakes were analog.
    ///
    /// Kept rather than deleted because both halves were deliberate. The
    /// snapshot has no analog throttle - the original's thrust is the cross
    /// *button* - so R2 counts as cross once it is past [`TRIGGER_THRESHOLD`].
    /// And the original's action set has no brake at all: the closest thing an
    /// anti-gravity ship has is pulling both airbrakes, so L2 feeds *both*
    /// axes and deliberately sets no button bit.
    ThrustBrake,
}

impl TriggerMode {
    /// Every mode, in the order a menu should offer them.
    pub const ALL: [Self; 2] = [Self::Airbrakes, Self::ThrustBrake];

    /// The token this mode is stored and configured as.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Airbrakes => "airbrakes",
            Self::ThrustBrake => "thrust_brake",
        }
    }

    /// The mode a token names, or `None` if nothing does.
    ///
    /// `None` rather than a default, for the reason
    /// `oag_gameplay::ControlScheme::from_name` gives: a settings file naming a
    /// mode this build does not have should be visible to the caller, not
    /// silently become the default.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|mode| mode.name() == name)
    }
}

impl std::fmt::Display for TriggerMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

impl std::str::FromStr for TriggerMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_name(&s.to_ascii_lowercase()).ok_or_else(|| {
            let names: Vec<_> = Self::ALL.iter().map(|m| m.name()).collect();
            format!("{s:?} is not a trigger mode; try {}", names.join(" or "))
        })
    }
}

/// How the analog triggers are read: what they do, and how hard.
///
/// One value rather than two arguments to [`resolve`], so the mode and the
/// curve cannot be passed in the wrong order and a test can state a whole
/// trigger setup on one line.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TriggerConfig {
    /// What L2 and R2 are bound to.
    pub mode: TriggerMode,
    /// The exponent applied to conditioned travel. `1.0` is linear; below it
    /// the brake arrives sooner, above it the low end is finer.
    ///
    /// Held clamped to [`CURVE_RANGE`] by [`Self::with_curve`], which is the
    /// only way in from outside.
    curve: f32,
}

impl Default for TriggerConfig {
    fn default() -> Self {
        Self {
            mode: TriggerMode::default(),
            curve: 1.0,
        }
    }
}

impl TriggerConfig {
    /// The same config with a new response curve, clamped to [`CURVE_RANGE`].
    ///
    /// A NaN exponent becomes linear rather than propagating: it would reach
    /// the airbrake axis, and `InputSnapshot::sanitised` maps NaN to zero, so
    /// the failure would look like an airbrake that quietly stopped working.
    #[must_use]
    pub fn with_curve(self, curve: f32) -> Self {
        let curve = if curve.is_nan() {
            1.0
        } else {
            curve.clamp(*CURVE_RANGE.start(), *CURVE_RANGE.end())
        };
        Self { curve, ..self }
    }

    /// The exponent in effect.
    #[must_use]
    pub fn curve(self) -> f32 {
        self.curve
    }
}

/// One trigger's raw travel as an axis: deadzoned, saturated, curved.
///
/// Rescaled between the two ends rather than merely clipped at them, so a
/// trigger still reaches both `0.0` and `1.0` instead of jumping from zero to
/// [`TRIGGER_DEADZONE`] and never arriving at full.
#[must_use]
pub fn condition(raw: f32, curve: f32) -> f32 {
    let raw = if raw.is_nan() {
        0.0
    } else {
        raw.clamp(0.0, 1.0)
    };
    let travel =
        ((raw - TRIGGER_DEADZONE) / (TRIGGER_SATURATION - TRIGGER_DEADZONE)).clamp(0.0, 1.0);
    // Guarded rather than left to `powf`, which answers 0^0 with 1 - a resting
    // trigger asking for a full airbrake.
    if travel == 0.0 {
        0.0
    } else {
        travel.powf(curve)
    }
}

/// Maps a pad button to an abstract button index, or `None` if it is not bound.
///
/// The action pad follows the original's own naming rather than the host pad's
/// letters: the South button is cross whatever the pad prints on it.
#[must_use]
pub fn map_button(pad: gilrs::Button) -> Option<Button> {
    Some(match pad {
        gilrs::Button::DPadUp => Button::Up,
        gilrs::Button::DPadDown => Button::Down,
        gilrs::Button::DPadLeft => Button::Left,
        gilrs::Button::DPadRight => Button::Right,
        gilrs::Button::South => Button::Cross,
        gilrs::Button::East => Button::Circle,
        gilrs::Button::West => Button::Square,
        gilrs::Button::North => Button::Triangle,
        gilrs::Button::LeftTrigger => Button::L,
        gilrs::Button::RightTrigger => Button::R,
        gilrs::Button::Start => Button::Start,
        gilrs::Button::Select => Button::Select,
        _ => return None,
    })
}

/// Every button [`map_button`] binds, which is what a poll reads.
pub const BOUND_BUTTONS: [gilrs::Button; 12] = [
    gilrs::Button::DPadUp,
    gilrs::Button::DPadDown,
    gilrs::Button::DPadLeft,
    gilrs::Button::DPadRight,
    gilrs::Button::South,
    gilrs::Button::East,
    gilrs::Button::West,
    gilrs::Button::North,
    gilrs::Button::LeftTrigger,
    gilrs::Button::RightTrigger,
    gilrs::Button::Start,
    gilrs::Button::Select,
];

/// One tick's raw readings off a pad, before they mean anything.
///
/// Split out from the pad itself so [`resolve`] - where every decision in this
/// module actually lives - can be driven from a test with no device attached.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Reading {
    /// Abstract bits from the digital buttons, per [`map_button`].
    pub buttons: u32,
    /// Left stick, right positive.
    pub stick_x: f32,
    /// Left stick, up positive, which is the sign the keyboard's UP produces.
    pub stick_y: f32,
    /// R2, 0 to 1, raw. Named for what [`TriggerMode::ThrustBrake`] makes of
    /// it; under [`TriggerMode::Airbrakes`] it is the right airbrake.
    pub throttle: f32,
    /// L2, 0 to 1, raw. The left airbrake, or the brake - see
    /// [`Self::throttle`].
    pub brake: f32,
}

/// What a pad contributes to a snapshot.
///
/// Not a snapshot itself: a pad is one of the devices on the window, and the
/// buttons of all of them are merged into one [`crate::Input`] before any edge
/// is computed. See [`crate::Controls`].
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct PadState {
    /// Abstract buttons held, the d-pad and whatever the triggers synthesise
    /// included - see [`TriggerMode`].
    pub held: u32,
    /// Steering, deadzoned.
    pub stick_x: f32,
    /// Pitch, deadzoned.
    pub stick_y: f32,
    /// Left airbrake: L1, or a conditioned trigger.
    pub airbrake_left: f32,
    /// Right airbrake: R1, or a conditioned trigger.
    pub airbrake_right: f32,
}

/// Turns one tick's readings into what the pad contributes.
///
/// The whole mapping policy is here: the stick deadzone, and what
/// [`TriggerConfig`] says the triggers are.
///
/// # The shoulder term reads `reading.buttons`, not `held`
///
/// Not tidiness. Under [`TriggerMode::Airbrakes`] a pulled trigger *adds* its
/// shoulder's bit to `held`, so a `shoulder` closure over `held` would answer
/// `1.0` for a trigger at `0.4` and `.max` would quantise the pull straight
/// back to full travel - the exact granularity this mapping exists to keep.
/// The digital contribution is L1 and R1 and nothing else.
#[must_use]
pub fn resolve(reading: Reading, config: TriggerConfig) -> PadState {
    let mut held = reading.buttons;
    let shoulder = |button: Button| f32::from(u8::from(reading.buttons & button.bit() != 0));
    let mut airbrake_left = shoulder(Button::L);
    let mut airbrake_right = shoulder(Button::R);

    match config.mode {
        TriggerMode::Airbrakes => {
            for (raw, button, axis) in [
                (reading.brake, Button::L, &mut airbrake_left),
                (reading.throttle, Button::R, &mut airbrake_right),
            ] {
                *axis = axis.max(condition(raw, config.curve));
                if raw > TRIGGER_THRESHOLD {
                    held |= button.bit();
                }
            }
        }
        TriggerMode::ThrustBrake => {
            if reading.throttle > TRIGGER_THRESHOLD {
                held |= Button::Cross.bit();
            }
            let brake = condition(reading.brake, config.curve);
            airbrake_left = airbrake_left.max(brake);
            airbrake_right = airbrake_right.max(brake);
        }
    }

    PadState {
        held,
        stick_x: deadzone(reading.stick_x),
        stick_y: deadzone(reading.stick_y),
        airbrake_left,
        airbrake_right,
    }
}

/// Zero inside the deadzone, and rescaled outside it so the axis still reaches
/// its full travel rather than jumping from 0 to [`STICK_DEADZONE`].
fn deadzone(value: f32) -> f32 {
    let magnitude = value.abs();
    if magnitude <= STICK_DEADZONE {
        return 0.0;
    }
    let scaled = (magnitude - STICK_DEADZONE) / (1.0 - STICK_DEADZONE);
    scaled.min(1.0) * value.signum()
}

/// Which grid slot each device drives.
///
/// **This replaces "every pad is one logical stream", which is what this module
/// did until 2026-09-16.** The old rule was written down as a design decision
/// rather than a limitation - "pads are merged rather than assigned to players:
/// there is one ship, and a Deck with a pad plugged into its dock should steer
/// from either" - and it was the right call while there was one ship. Split
/// screen, a second window and a remote client each need two devices to mean
/// two craft, and a merge cannot express that at all.
///
/// **The default is the old behaviour exactly.** Every pad and the keyboard map
/// to slot 0, so a Deck with a pad in its dock still steers from either, and a
/// single-pad session is identical to what it was. Assigning a device anywhere
/// else is an explicit call; nothing does it yet.
///
/// **A `Vec` of pairs rather than a map**, for the reason
/// `docs/architecture/determinism.md` gives: which slot a device drives decides
/// which craft an input steers, so it reaches simulation state, and a
/// `HashMap`'s iteration order is not stable between processes. At most eight
/// devices matter, so a linear scan is not worth a hash anyway.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Assignment {
    /// Pads that have been given a slot of their own, in assignment order.
    ///
    /// A pad not in here drives [`Self::DEFAULT_SLOT`], which is what keeps an
    /// unconfigured session working the way it always did.
    pads: Vec<(gilrs::GamepadId, u8)>,
    /// Which slot the keyboard drives. Slot 0 until something says otherwise.
    keyboard: u8,
}

impl Assignment {
    /// Where a device with no assignment of its own goes.
    ///
    /// Slot 0: the human slot in `oag_gameplay::World::SINGLE_PLAYER`, and the
    /// one a race has until something enables a second.
    pub const DEFAULT_SLOT: usize = 0;

    /// Which slot `id` drives.
    #[must_use]
    pub fn slot_of(&self, id: gilrs::GamepadId) -> usize {
        self.pads
            .iter()
            .find(|(candidate, _)| *candidate == id)
            .map_or(Self::DEFAULT_SLOT, |(_, slot)| usize::from(*slot))
    }

    /// Puts `id` on `slot`, replacing whatever it drove before.
    ///
    /// A slot past the grid is ignored rather than clamped: clamping would
    /// silently put two people on one craft, which is harder to notice than a
    /// pad that did not take.
    pub fn assign_pad(&mut self, id: gilrs::GamepadId, slot: usize) {
        let Ok(slot) = u8::try_from(slot) else {
            return;
        };
        if usize::from(slot) >= oag_gameplay::MAX_PLAYERS {
            return;
        }
        match self.pads.iter_mut().find(|(candidate, _)| *candidate == id) {
            Some(entry) => entry.1 = slot,
            None => self.pads.push((id, slot)),
        }
    }

    /// Which slot the keyboard drives.
    #[must_use]
    pub fn keyboard_slot(&self) -> usize {
        usize::from(self.keyboard)
    }

    /// Puts the keyboard on `slot`, ignoring one past the grid.
    pub fn assign_keyboard(&mut self, slot: usize) {
        if slot < oag_gameplay::MAX_PLAYERS
            && let Ok(slot) = u8::try_from(slot)
        {
            self.keyboard = slot;
        }
    }

    /// Whether every device still drives [`Self::DEFAULT_SLOT`].
    ///
    /// What a caller asks to know it is in the single-player case - the one
    /// where a merged read and a per-slot read are the same thing.
    #[must_use]
    pub fn is_single_player(&self) -> bool {
        self.keyboard_slot() == Self::DEFAULT_SLOT
            && self
                .pads
                .iter()
                .all(|(_, slot)| usize::from(*slot) == Self::DEFAULT_SLOT)
    }
}

/// Every pad attached to the machine, read per player slot.
///
/// A machine with no pad, or a platform `gilrs` cannot open, is not an error -
/// it is a keyboard-only session, which is what every headless capture and CI
/// run is.
///
/// Which pad drives which craft is [`Assignment`]; by default all of them drive
/// slot 0, which is the single merged stream this type used to be able to
/// produce and nothing else.
pub struct Pad {
    gilrs: Option<gilrs::Gilrs>,
    /// What [`Self::poll`] makes of the analog triggers. Held here rather than
    /// passed in per tick because it is a pilot preference that outlives any
    /// one frame, and the frame has no business knowing about it.
    triggers: TriggerConfig,
    /// Which slot each attached pad drives. See [`Assignment`].
    assignment: Assignment,
    /// What Steam said about the controllers behind its virtual ones.
    launch: crate::prompt::Launch,
    /// The family (`None` when unknown) of the real pad last pressed since
    /// [`Self::take_activity`]; the outer `None` is no activity.
    active: Option<Option<crate::prompt::PromptFamily>>,
}

impl std::fmt::Debug for Pad {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Pad")
            .field("available", &self.gilrs.is_some())
            .finish()
    }
}

impl Default for Pad {
    fn default() -> Self {
        Self::new()
    }
}

impl Pad {
    /// Opens the pad subsystem, or reports why it could not and carries on.
    #[must_use]
    pub fn new() -> Self {
        let triggers = TriggerConfig::default();
        match gilrs::Gilrs::new() {
            Ok(gilrs) => Self {
                gilrs: Some(gilrs),
                triggers,
                assignment: Assignment::default(),
                launch: crate::prompt::Launch::from_process(),
                active: None,
            },
            Err(e) => {
                warn!("no gamepad support ({e}); keyboard only");
                Self {
                    gilrs: None,
                    triggers,
                    assignment: Assignment::default(),
                    launch: crate::prompt::Launch::default(),
                    active: None,
                }
            }
        }
    }

    /// A pad that was never opened, and so reads as nothing held.
    ///
    /// What a run that must not touch a device gets: a headless capture, a
    /// test, CI. Opening `gilrs` talks to udev, and a test that reads whatever
    /// pad the developer happens to have plugged in is a test that fails for
    /// one person.
    #[must_use]
    pub fn none() -> Self {
        Self {
            gilrs: None,
            triggers: TriggerConfig::default(),
            assignment: Assignment::default(),
            launch: crate::prompt::Launch::default(),
            active: None,
        }
    }

    /// Whether a pad subsystem was opened at all.
    #[must_use]
    pub fn is_available(&self) -> bool {
        self.gilrs.is_some()
    }

    /// Binds the analog triggers to something else.
    ///
    /// Takes effect on the next [`Self::poll`], which is why nothing has to be
    /// restarted for a menu row to change it: no state is carried across a
    /// tick by either mapping.
    pub fn set_trigger_mode(&mut self, mode: TriggerMode) {
        self.triggers.mode = mode;
    }

    /// Sets the trigger response curve, clamped by
    /// [`TriggerConfig::with_curve`].
    pub fn set_trigger_curve(&mut self, curve: f32) {
        self.triggers = self.triggers.with_curve(curve);
    }

    /// How the triggers are being read right now.
    #[must_use]
    pub fn triggers(&self) -> TriggerConfig {
        self.triggers
    }

    /// Names every connected pad, for the line the game prints at startup.
    #[must_use]
    pub fn names(&self) -> Vec<String> {
        let Some(gilrs) = &self.gilrs else {
            return Vec::new();
        };
        gilrs
            .gamepads()
            .filter(|(_, pad)| is_real_pad(pad))
            .map(|(_, pad)| pad.name().to_string())
            .collect()
    }

    /// Names every device the OS lists as a joystick that is not a gamepad
    /// (a keyboard's consumer-control interface, say), with its id, for the
    /// log. Nothing reads these.
    #[must_use]
    pub fn ignored(&self) -> Vec<String> {
        let Some(gilrs) = &self.gilrs else {
            return Vec::new();
        };
        gilrs
            .gamepads()
            .filter(|(_, pad)| !is_real_pad(pad))
            .map(|(_, pad)| {
                let info = pad_info(&pad);
                format!("{} ({})", info.name, crate::prompt::uuid_string(info.uuid))
            })
            .collect()
    }

    /// The prompt family of the first attached real pad: the outer `None`
    /// with none, the inner one when its family is unknown.
    #[must_use]
    pub fn first_family(&self) -> Option<Option<crate::prompt::PromptFamily>> {
        let gilrs = self.gilrs.as_ref()?;
        gilrs
            .gamepads()
            .find(|(_, pad)| is_real_pad(pad))
            .map(|(_, pad)| self.launch.family_of(&pad_info(&pad)))
    }

    /// The family of the real pad pressed or pushed since the last call, and
    /// clears it: the outer `None` is no activity, the inner one a pad of
    /// unknown family. Feeds [`crate::prompt::Detector::note_pad`].
    pub fn take_activity(&mut self) -> Option<Option<crate::prompt::PromptFamily>> {
        self.active.take()
    }

    /// Which slot each attached pad drives.
    #[must_use]
    pub fn assignment(&self) -> &Assignment {
        &self.assignment
    }

    /// The assignment, to change. See [`Assignment::assign_pad`].
    pub fn assignment_mut(&mut self) -> &mut Assignment {
        &mut self.assignment
    }

    /// Drains the event queue and reads every pad into the slot it drives.
    ///
    /// The events have to be drained for `gilrs` to update the state this then
    /// reads, so polling is not optional even though nothing here looks at an
    /// individual event.
    ///
    /// **Pads sharing a slot are still merged into one reading**, which is what
    /// makes the default assignment identical to the single merged stream this
    /// used to produce: with every pad on slot 0, slot 0's entry is the old
    /// `poll`'s answer and the other seven are [`PadState::default`].
    pub fn poll_players(&mut self) -> [PadState; oag_gameplay::MAX_PLAYERS] {
        let triggers = self.triggers;
        let mut readings = [Reading::default(); oag_gameplay::MAX_PLAYERS];
        if let Some(gilrs) = &mut self.gilrs {
            while gilrs.next_event().is_some() {}

            for (id, pad) in gilrs.gamepads() {
                if !is_real_pad(&pad) {
                    continue;
                }
                let slot = self.assignment.slot_of(id);
                let Some(reading) = readings.get_mut(slot) else {
                    continue;
                };
                let before = reading.buttons;
                for bound_button in BOUND_BUTTONS {
                    if pad.is_pressed(bound_button)
                        && let Some(button) = map_button(bound_button)
                    {
                        reading.buttons |= button.bit();
                    }
                }
                let pushed = pad.value(gilrs::Axis::LeftStickX).abs() > ACTIVITY_STICK
                    || pad.value(gilrs::Axis::LeftStickY).abs() > ACTIVITY_STICK;
                if pushed || reading.buttons != before {
                    self.active = Some(self.launch.family_of(&pad_info(&pad)));
                }
                reading.stick_x = larger(reading.stick_x, pad.value(gilrs::Axis::LeftStickX));
                reading.stick_y = larger(reading.stick_y, pad.value(gilrs::Axis::LeftStickY));
                reading.throttle = reading
                    .throttle
                    .max(analog(&pad, gilrs::Button::RightTrigger2));
                reading.brake = reading.brake.max(analog(&pad, gilrs::Button::LeftTrigger2));
            }
        }
        readings.map(|reading| resolve(reading, triggers))
    }

    /// What slot 0's pads contribute, which under the default assignment is
    /// every pad on the machine.
    ///
    /// [`Self::poll_players`]'s first entry, kept as its own call because the
    /// front end has one cursor and one menu no matter how many people are
    /// racing.
    pub fn poll(&mut self) -> PadState {
        self.poll_players()[Assignment::DEFAULT_SLOT]
    }
}

/// How far a stick must be pushed before it counts as the player using the
/// pad rather than drift, for the prompt family's last-used tracking.
const ACTIVITY_STICK: f32 = 0.5;

fn pad_info(pad: &gilrs::Gamepad<'_>) -> crate::prompt::PadInfo {
    crate::prompt::PadInfo {
        name: pad.name().to_string(),
        vendor_id: pad.vendor_id(),
        product_id: pad.product_id(),
        uuid: Some(pad.uuid()),
    }
}

/// What the backend knows of `pad`'s hardware, for [`crate::prompt::is_gamepad`].
fn pad_caps(pad: &gilrs::Gamepad<'_>) -> crate::prompt::PadCaps {
    let state = pad.state();
    let has_button = |button| {
        pad.button_code(button)
            .is_some_and(|code| state.button_data(code).is_some())
    };
    let has_axis = |axis| {
        pad.axis_code(axis)
            .is_some_and(|code| state.axis_data(code).is_some())
    };
    let count =
        |flags: &[bool]| u8::try_from(flags.iter().filter(|set| **set).count()).unwrap_or(0);
    crate::prompt::PadCaps {
        sdl_mapped: pad.mapping_source() == gilrs::MappingSource::SdlMappings,
        face_buttons: count(&[
            has_button(gilrs::Button::South),
            has_button(gilrs::Button::East),
            has_button(gilrs::Button::West),
            has_button(gilrs::Button::North),
        ]),
        stick_axes: count(&[
            has_axis(gilrs::Axis::LeftStickX),
            has_axis(gilrs::Axis::LeftStickY),
        ]),
        dpad: [
            gilrs::Button::DPadUp,
            gilrs::Button::DPadDown,
            gilrs::Button::DPadLeft,
            gilrs::Button::DPadRight,
        ]
        .into_iter()
        .all(has_button)
            || (has_axis(gilrs::Axis::DPadX) && has_axis(gilrs::Axis::DPadY)),
    }
}

/// Whether the OS's joystick is a gamepad at all, for the controls and the
/// prompts alike. A keyboard's system-control interface is listed as a
/// joystick and is neither.
fn is_real_pad(pad: &gilrs::Gamepad<'_>) -> bool {
    crate::prompt::is_gamepad(&pad_info(pad), pad_caps(pad))
}

/// A trigger's analog travel, falling back to its digital state on a pad whose
/// mapping does not report one.
fn analog(pad: &gilrs::Gamepad<'_>, button: gilrs::Button) -> f32 {
    pad.button_data(button).map_or_else(
        || f32::from(u8::from(pad.is_pressed(button))),
        |data| data.value(),
    )
}

/// Whichever reading is further from centre, sign kept.
pub(crate) fn larger(a: f32, b: f32) -> f32 {
    if b.abs() > a.abs() { b } else { a }
}

#[cfg(test)]
mod tests;
