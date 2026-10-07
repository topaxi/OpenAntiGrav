//! The abstract button layer, reproduced from `Input_BuildState`.
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
//! This vocabulary lives in `oag-core`, at the bottom of the graph, so that the
//! simulation's snapshot (`oag_gameplay::input`, which re-exports it) and the
//! front end's menus (`oag-ui`) share one `Button` without the menus depending
//! on the whole simulation. It is plain bit-twiddling: no floats, no hashing.

use std::fmt::Display;

/// Abstract button indices. These are bit positions, not masks.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Button {
    /// D-pad up.
    Up = 0,
    /// D-pad down.
    Down = 1,
    /// D-pad left.
    Left = 2,
    /// D-pad right.
    Right = 3,
    /// Circle, which the front end also calls `cancel` and `backward`.
    Circle = 4,
    /// Cross, which the front end also calls `activate` and `forward`.
    Cross = 5,
    /// Triangle.
    Triangle = 6,
    /// Square.
    Square = 7,
    /// Left shoulder.
    L = 8,
    /// Right shoulder.
    R = 9,
    /// START. The intro skip tests this index, and it is `0xe`, not a mask.
    Start = 14,
    /// SELECT.
    Select = 15,
    /// The synthetic "any button" index, `0x14`.
    Any = 0x14,
}

impl Button {
    #[must_use]
    pub fn from_index(index: u8) -> Self {
        match index {
            0 => Button::Up,
            1 => Button::Down,
            2 => Button::Left,
            3 => Button::Right,
            4 => Button::Circle,
            5 => Button::Cross,
            6 => Button::Triangle,
            7 => Button::Square,
            8 => Button::L,
            9 => Button::R,
            14 => Button::Start,
            15 => Button::Select,
            _ => Button::Any,
        }
    }

    #[must_use]
    pub const fn index(self) -> u8 {
        self as u8
    }

    /// The single-bit mask for this button, for building a held mask.
    ///
    /// `Input_IsPressed` tests `*(u32 *)(this + 0x48) & (1 << (index & 0x1f))`,
    /// so the masking of the index is reproduced here rather than tidied away.
    /// This is the one place that turns an index into a mask - the abstract
    /// layer is indices everywhere else, per `Input_BuildState`.
    #[must_use]
    pub const fn bit(self) -> u32 {
        1u32 << (self.index() & 0x1f)
    }
}

impl Display for Button {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Button::Up => "up",
            Button::Down => "down",
            Button::Left => "left",
            Button::Right => "right",
            Button::Circle => "circle",
            Button::Cross => "cross",
            Button::Triangle => "triangle",
            Button::Square => "square",
            Button::L => "l",
            Button::R => "r",
            Button::Start => "start",
            Button::Select => "select",
            Button::Any => "any",
        })
    }
}

/// Every real button, for building the synthetic [`Button::Any`] bit.
const REAL_BUTTONS: [Button; 12] = [
    Button::Up,
    Button::Down,
    Button::Left,
    Button::Right,
    Button::Circle,
    Button::Cross,
    Button::Triangle,
    Button::Square,
    Button::L,
    Button::R,
    Button::Start,
    Button::Select,
];

/// Maps a front-end XML button name to its abstract index.
///
/// Mirrors `Input_ParseButtonName`, which is how the data-driven front end binds
/// `forward="start"` and `backward="circle"` to actual buttons.
#[must_use]
pub fn button_from_name(name: &str) -> Option<Button> {
    Some(match name.to_ascii_lowercase().as_str() {
        "up" => Button::Up,
        "down" => Button::Down,
        "left" => Button::Left,
        "right" => Button::Right,
        "circle" | "cancel" | "backward" => Button::Circle,
        "cross" | "activate" | "forward" => Button::Cross,
        "triangle" => Button::Triangle,
        "square" => Button::Square,
        "l" => Button::L,
        "r" => Button::R,
        "start" => Button::Start,
        "select" => Button::Select,
        "any" => Button::Any,
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
    /// A state with nothing held, as a constant.
    ///
    /// [`Self::default`]'s value, reachable from a `const` context - which is
    /// what [`PlayerInputs`]'s out-of-range fallback needs and `Default` cannot
    /// give it.
    pub const EMPTY: Self = Self {
        held: 0,
        held_last: 0,
        released: 0,
        pressed: 0,
    };

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

    /// Makes `button` read as pressed this frame, as if the player had tapped it.
    ///
    /// For a screen that decides on the player's behalf to leave (the front end's
    /// movie legs with no picture), so the leave goes through the same handler a
    /// real press does and no second exit is written beside it.
    pub fn inject_press(&mut self, button: Button) {
        self.held |= button.bit();
        self.pressed |= button.bit();
    }

    /// Whether `button` is held right now.
    #[must_use]
    pub fn is_held(&self, button: Button) -> bool {
        self.held & button.bit() != 0
    }

    /// Whether `button` went down this frame.
    ///
    /// `Input_IsPressed`; see [`Button::bit`] for the mask it tests against.
    #[must_use]
    pub fn is_pressed(&self, button: Button) -> bool {
        self.pressed & button.bit() != 0
    }

    /// Whether `button` came up this frame.
    #[must_use]
    pub fn is_released(&self, button: Button) -> bool {
        self.released & button.bit() != 0
    }

    /// Clears a pressed bit so one press cannot be handled twice.
    ///
    /// `Input_ConsumePress`. The intro skip calls it after firing the redirect,
    /// which is what stops the same START from also being seen by whatever the
    /// transition lands on.
    pub fn consume_press(&mut self, button: Button) {
        self.pressed &= !button.bit();
        // A consumed real press must also clear the synthetic "any" bit, or the
        // next screen sees a press nobody made.
        if button != Button::Any && self.pressed & real_mask() == 0 {
            self.pressed &= !Button::Any.bit();
        }
    }

    /// Reads a button as an **edge** and consumes it in one step.
    ///
    /// Menus are not held down, and one press must not be seen twice - by two rows,
    /// or by the menus and then by whatever a transition lands on. The front end
    /// spells the same pair out at every site it needs it (`is_pressed` then
    /// `consume_press`); this is that pair with a name, because a menu tick needs
    /// six of them and the shape is what matters rather than each instance.
    pub fn take(&mut self, button: Button) -> bool {
        if self.is_pressed(button) {
            self.consume_press(button);
            true
        } else {
            false
        }
    }

    /// All four masks, in the original's offset order: held, held last
    /// frame, released, pressed.
    ///
    /// **What a replay stores**, and it stores all four rather than rebuilding
    /// the edges from `held` on playback: [`Self::consume_press`] can clear a
    /// pressed bit before the snapshot reaches the tick, so the edges the
    /// simulation saw are not always the ones `held` alone would derive. See
    /// `docs/architecture/adr/0055-replays-are-inputs-and-a-ghost-is-poses.md`.
    #[must_use]
    pub const fn masks(&self) -> [u32; 4] {
        [self.held, self.held_last, self.released, self.pressed]
    }

    /// The inverse of [`Self::masks`]: a state rebuilt from its four masks,
    /// exactly as stored.
    #[must_use]
    pub const fn from_masks(masks: [u32; 4]) -> Self {
        Self {
            held: masks[0],
            held_last: masks[1],
            released: masks[2],
            pressed: masks[3],
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

fn real_mask() -> u32 {
    REAL_BUTTONS.iter().fold(0, |acc, &b| acc | b.bit())
}

/// Adds the synthetic `0x14` bit when any real button is down.
fn with_any_bit(held: u32) -> u32 {
    if held & real_mask() != 0 {
        held | Button::Any.bit()
    } else {
        held & !Button::Any.bit()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edges_are_computed_from_consecutive_frames() {
        let mut input = Input::new();
        input.begin_frame(Button::Cross.bit());
        assert!(input.is_pressed(Button::Cross));
        assert!(input.is_held(Button::Cross));

        input.begin_frame(Button::Cross.bit());
        assert!(!input.is_pressed(Button::Cross), "held is not pressed");
        assert!(input.is_held(Button::Cross));

        input.begin_frame(0);
        assert!(input.is_released(Button::Cross));
        assert!(!input.is_held(Button::Cross));
    }

    #[test]
    fn any_button_is_synthesised() {
        let mut input = Input::new();
        input.begin_frame(Button::Start.bit());
        assert!(input.is_pressed(Button::Any));
        input.begin_frame(0);
        assert!(!input.is_held(Button::Any));
    }

    #[test]
    fn consuming_a_press_clears_it_and_the_any_bit() {
        let mut input = Input::new();
        input.begin_frame(Button::Start.bit());
        assert!(input.is_pressed(Button::Start));
        input.consume_press(Button::Start);
        assert!(!input.is_pressed(Button::Start));
        assert!(
            !input.is_pressed(Button::Any),
            "the synthetic bit must go with the last real press"
        );
        assert!(input.is_held(Button::Start), "consuming is not releasing");
    }

    #[test]
    fn consuming_one_of_two_presses_keeps_the_any_bit() {
        let mut input = Input::new();
        input.begin_frame(Button::Start.bit() | Button::Cross.bit());
        input.consume_press(Button::Start);
        assert!(input.is_pressed(Button::Cross));
        assert!(input.is_pressed(Button::Any));
    }

    #[test]
    fn start_is_index_fourteen_not_a_mask() {
        // The intro skip reads `Input_IsPressed(g_input, 0xe, 0)`. Reading 0xe
        // as a mask would test up and down instead.
        assert_eq!(Button::Start.index(), 0xe);
        let mut input = Input::new();
        input.begin_frame(1 << 14);
        assert!(input.is_pressed(Button::Start));
        assert!(!input.is_pressed(Button::Up));
        assert!(!input.is_pressed(Button::Down));
    }

    #[test]
    fn xml_names_map_to_indices() {
        assert_eq!(button_from_name("start"), Some(Button::Start));
        assert_eq!(button_from_name("circle"), Some(Button::Circle));
        assert_eq!(button_from_name("cancel"), Some(Button::Circle));
        assert_eq!(button_from_name("cross"), Some(Button::Cross));
        assert_eq!(button_from_name("activate"), Some(Button::Cross));
        assert_eq!(button_from_name("forward"), Some(Button::Cross));
        assert_eq!(button_from_name("backward"), Some(Button::Circle));
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
