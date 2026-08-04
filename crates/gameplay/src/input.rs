//! The abstract button layer, reproduced from `Input_BuildState`, and the
//! per-tick snapshot the simulation consumes.
//!
//! The original never uses PSP button masks downstream. `Input_BuildState`
//! (`0x0894eec4`) translates hardware bits into its own layout and everything
//! after that refers to **bit indices**, not masks, with `Input_ParseButtonName`
//! (`0x0894f2a0`) mapping the names the front-end XML uses. See
//! `docs/ghidra/functions/psp-pulse-usa/input.md`.
//!
//! The indirection is worth keeping rather than flattening: `activate` is cross
//! and `cancel` is circle *by convention*, so this layer is where a region
//! difference would live, and it is what the original's own XML expects.
//!
//! This module lives in `oag-gameplay` rather than in the input system because
//! of rule 1 of `docs/architecture/workspace-layout.md`: the simulation owns the
//! snapshot *type*, and whatever produces it - a keyboard, a pad, a replay, a
//! test - depends on this, not the reverse.

/// Abstract button indices. These are bit positions, not masks.
pub mod button {
    /// D-pad up.
    pub const UP: u8 = 0;
    /// D-pad down.
    pub const DOWN: u8 = 1;
    /// D-pad left.
    pub const LEFT: u8 = 2;
    /// D-pad right.
    pub const RIGHT: u8 = 3;
    /// Circle, which the front end also calls `cancel` and `backward`.
    pub const CIRCLE: u8 = 4;
    /// Cross, which the front end also calls `activate` and `forward`.
    pub const CROSS: u8 = 5;
    /// Triangle.
    pub const TRIANGLE: u8 = 6;
    /// Square.
    pub const SQUARE: u8 = 7;
    /// Left shoulder.
    pub const L: u8 = 8;
    /// Right shoulder.
    pub const R: u8 = 9;
    /// START. The intro skip tests this index, and it is `0xe`, not a mask.
    pub const START: u8 = 14;
    /// SELECT.
    pub const SELECT: u8 = 15;
    /// The synthetic "any button" index, `0x14`.
    pub const ANY: u8 = 0x14;
}

/// Every real button, for building the synthetic [`button::ANY`] bit.
const REAL_BUTTONS: [u8; 12] = [
    button::UP,
    button::DOWN,
    button::LEFT,
    button::RIGHT,
    button::CIRCLE,
    button::CROSS,
    button::TRIANGLE,
    button::SQUARE,
    button::L,
    button::R,
    button::START,
    button::SELECT,
];

/// Maps a front-end XML button name to its abstract index.
///
/// Mirrors `Input_ParseButtonName`, which is how the data-driven front end binds
/// `forward="start"` and `backward="circle"` to actual buttons.
#[must_use]
pub fn button_from_name(name: &str) -> Option<u8> {
    Some(match name.to_ascii_lowercase().as_str() {
        "up" => button::UP,
        "down" => button::DOWN,
        "left" => button::LEFT,
        "right" => button::RIGHT,
        "circle" | "cancel" | "backward" => button::CIRCLE,
        "cross" | "activate" | "forward" => button::CROSS,
        "triangle" => button::TRIANGLE,
        "square" => button::SQUARE,
        "l" => button::L,
        "r" => button::R,
        "start" => button::START,
        "select" => button::SELECT,
        "any" => button::ANY,
        // "none" is a real value in the XML and means "no button", so it is a
        // successful parse of nothing rather than an error.
        _ => return None,
    })
}

/// The four button masks the original keeps on its input object.
///
/// Offsets `+0x3c` held, `+0x40` held last frame, `+0x44` released this frame,
/// `+0x48` pressed this frame.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Input {
    held: u32,
    held_last: u32,
    released: u32,
    pressed: u32,
}

impl Input {
    /// A state with nothing held.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Feeds a new frame's held mask and recomputes the edges.
    ///
    /// The caller supplies abstract bits, not hardware ones: this layer is
    /// downstream of the translation, so a keyboard and a pad look identical
    /// from here.
    pub fn begin_frame(&mut self, held: u32) {
        let held = with_any_bit(held);
        self.held_last = self.held;
        self.held = held;
        self.pressed = held & !self.held_last;
        self.released = self.held_last & !held;
    }

    /// Whether `index` is held right now.
    #[must_use]
    pub fn is_held(&self, index: u8) -> bool {
        self.held & bit(index) != 0
    }

    /// Whether `index` went down this frame.
    ///
    /// `Input_IsPressed` tests `*(u32 *)(this + 0x48) & (1 << (index & 0x1f))`,
    /// so the masking of the index is reproduced rather than tidied away.
    #[must_use]
    pub fn is_pressed(&self, index: u8) -> bool {
        self.pressed & bit(index) != 0
    }

    /// Whether `index` came up this frame.
    #[must_use]
    pub fn is_released(&self, index: u8) -> bool {
        self.released & bit(index) != 0
    }

    /// Clears a pressed bit so one press cannot be handled twice.
    ///
    /// `Input_ConsumePress`. The intro skip calls it after firing the redirect,
    /// which is what stops the same START from also being seen by whatever the
    /// transition lands on.
    pub fn consume_press(&mut self, index: u8) {
        self.pressed &= !bit(index);
        // A consumed real press must also clear the synthetic "any" bit, or the
        // next screen sees a press nobody made.
        if index != button::ANY && self.pressed & real_mask() == 0 {
            self.pressed &= !bit(button::ANY);
        }
    }

    /// The raw held mask, for tests.
    #[must_use]
    pub fn held_mask(&self) -> u32 {
        self.held
    }

    /// The raw pressed mask, for tests.
    #[must_use]
    pub fn pressed_mask(&self) -> u32 {
        self.pressed
    }
}

/// One tick of pilot intent: the button edges plus the analog axes.
///
/// The whole of what the simulation is allowed to know about input. A replay
/// records these and nothing else, which is only sound because there is nothing
/// else: no key codes, no device identity, no timestamps.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct InputSnapshot {
    /// Held, pressed and released masks over the abstract button indices.
    pub buttons: Input,
    /// Analog stick X, `-1.0..=1.0`, positive right.
    pub stick_x: f32,
    /// Analog stick Y, `-1.0..=1.0`, positive up.
    pub stick_y: f32,
    /// Left airbrake, `0.0..=1.0`.
    ///
    /// An axis rather than the [`button::L`] bit because the recovered force law
    /// ramps each side toward "its analog input"; on a PSP pad that input can
    /// only ever be 0 or 1, and on anything else it need not be.
    pub airbrake_left: f32,
    /// Right airbrake, `0.0..=1.0`.
    pub airbrake_right: f32,
}

impl InputSnapshot {
    /// Nothing held, sticks centred.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Clamps the axes into range and drops NaN.
    ///
    /// Called on every snapshot before it reaches the simulation. An out-of-range
    /// axis from a miscalibrated pad would be a divergence between two machines
    /// running the same replay, and a NaN would poison the state hash for the
    /// rest of the race, so this is a determinism guard rather than politeness.
    #[must_use]
    pub fn sanitised(mut self) -> Self {
        self.stick_x = clamp_axis(self.stick_x, -1.0);
        self.stick_y = clamp_axis(self.stick_y, -1.0);
        self.airbrake_left = clamp_axis(self.airbrake_left, 0.0);
        self.airbrake_right = clamp_axis(self.airbrake_right, 0.0);
        self
    }
}

/// Clamps to `min..=1.0`, mapping NaN to `0.0`.
fn clamp_axis(value: f32, min: f32) -> f32 {
    if value.is_nan() {
        0.0
    } else {
        value.clamp(min, 1.0)
    }
}

fn bit(index: u8) -> u32 {
    1u32 << (index & 0x1f)
}

fn real_mask() -> u32 {
    REAL_BUTTONS.iter().fold(0, |acc, &b| acc | bit(b))
}

/// Adds the synthetic `0x14` bit when any real button is down.
fn with_any_bit(held: u32) -> u32 {
    if held & real_mask() != 0 {
        held | bit(button::ANY)
    } else {
        held & !bit(button::ANY)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn axes_are_clamped_and_nan_becomes_zero() {
        let snapshot = InputSnapshot {
            stick_x: 4.0,
            stick_y: -9.0,
            airbrake_left: -1.0,
            airbrake_right: f32::NAN,
            ..InputSnapshot::new()
        }
        .sanitised();
        assert_eq!(snapshot.stick_x, 1.0);
        assert_eq!(snapshot.stick_y, -1.0);
        assert_eq!(
            snapshot.airbrake_left, 0.0,
            "a trigger has no negative half"
        );
        assert_eq!(snapshot.airbrake_right, 0.0, "NaN must not reach the sim");
    }

    #[test]
    fn edges_are_computed_from_consecutive_frames() {
        let mut input = Input::new();
        input.begin_frame(bit(button::CROSS));
        assert!(input.is_pressed(button::CROSS));
        assert!(input.is_held(button::CROSS));

        input.begin_frame(bit(button::CROSS));
        assert!(!input.is_pressed(button::CROSS), "held is not pressed");
        assert!(input.is_held(button::CROSS));

        input.begin_frame(0);
        assert!(input.is_released(button::CROSS));
        assert!(!input.is_held(button::CROSS));
    }

    #[test]
    fn any_button_is_synthesised() {
        let mut input = Input::new();
        input.begin_frame(bit(button::START));
        assert!(input.is_pressed(button::ANY));
        input.begin_frame(0);
        assert!(!input.is_held(button::ANY));
    }

    #[test]
    fn consuming_a_press_clears_it_and_the_any_bit() {
        let mut input = Input::new();
        input.begin_frame(bit(button::START));
        assert!(input.is_pressed(button::START));
        input.consume_press(button::START);
        assert!(!input.is_pressed(button::START));
        assert!(
            !input.is_pressed(button::ANY),
            "the synthetic bit must go with the last real press"
        );
        assert!(input.is_held(button::START), "consuming is not releasing");
    }

    #[test]
    fn consuming_one_of_two_presses_keeps_the_any_bit() {
        let mut input = Input::new();
        input.begin_frame(bit(button::START) | bit(button::CROSS));
        input.consume_press(button::START);
        assert!(input.is_pressed(button::CROSS));
        assert!(input.is_pressed(button::ANY));
    }

    #[test]
    fn start_is_index_fourteen_not_a_mask() {
        // The intro skip reads `Input_IsPressed(g_input, 0xe, 0)`. Reading 0xe
        // as a mask would test up and down instead.
        assert_eq!(button::START, 0xe);
        let mut input = Input::new();
        input.begin_frame(1 << 14);
        assert!(input.is_pressed(button::START));
        assert!(!input.is_pressed(button::UP));
        assert!(!input.is_pressed(button::DOWN));
    }

    #[test]
    fn xml_names_map_to_indices() {
        assert_eq!(button_from_name("start"), Some(button::START));
        assert_eq!(button_from_name("circle"), Some(button::CIRCLE));
        assert_eq!(button_from_name("cancel"), Some(button::CIRCLE));
        assert_eq!(button_from_name("cross"), Some(button::CROSS));
        assert_eq!(button_from_name("activate"), Some(button::CROSS));
        assert_eq!(button_from_name("forward"), Some(button::CROSS));
        assert_eq!(button_from_name("backward"), Some(button::CIRCLE));
        assert_eq!(button_from_name("none"), None);
        assert_eq!(button_from_name("nonsense"), None);
    }

    #[test]
    fn activate_is_cross_and_cancel_is_circle() {
        // Stated as a test because getting it backwards is the single most
        // likely mistake in this file.
        assert_eq!(button_from_name("activate"), button_from_name("cross"));
        assert_eq!(button_from_name("cancel"), button_from_name("circle"));
        assert_ne!(button_from_name("activate"), button_from_name("cancel"));
    }
}
