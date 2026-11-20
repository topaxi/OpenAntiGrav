//! Which codepoints a title's strings use for a controller button, and which
//! button each one is drawn as.
//!
//! The disc's strings embed a stand-in codepoint (`"Press ε to continue"`)
//! and its font draws the PlayStation glyph for it. A build that shows
//! another controller's glyph has to know what each stand-in *means*, and
//! that is a fact of the title's fonts, not of the engine: Pulse's `ε` and
//! HD's `ε` are both the cross button, but HD's `Δ` is a d-pad direction and
//! Pulse has no `Δ` at all. Each title carries its own table, read off the
//! rendered glyph cells, in the same way [`crate::flare::Flare`] carries its
//! texture name.
//!
//! The table is by **action**: the physical control the glyph depicts, as
//! the PlayStation names it. A confirm in a Japanese string table is a
//! circle because that table's text carries `γ`, not because anything here
//! knows a region; the pad layer maps a [`Prompt`] to whichever control the
//! player is holding.

/// The control a prompt glyph depicts, by the PlayStation's name for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Prompt {
    /// The bottom face button.
    Cross,
    /// The right face button.
    Circle,
    /// The left face button.
    Square,
    /// The top face button.
    Triangle,
    /// The left shoulder button.
    L,
    /// The right shoulder button.
    R,
    /// D-pad up.
    Up,
    /// D-pad down.
    Down,
    /// D-pad left.
    Left,
    /// D-pad right.
    Right,
}

/// Which text a title's stand-in codepoints are valid in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// Every face: the glyphs are embedded in the title's ordinary text
    /// faces, so any string may carry one (Pulse).
    AnyFace,
    /// Only text drawn in the title's `Buttons` face (HD). The same
    /// codepoints in another face are letters.
    ButtonsFace,
}

/// A title's button-prompt table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Prompts {
    /// Where the codepoints below are glyphs.
    pub scope: Scope,
    /// Each stand-in codepoint and the control its glyph depicts.
    ///
    /// Empty where the title's glyphs have not been read: then a build draws
    /// the disc's own glyphs unchanged, whatever the player's controller.
    pub glyphs: &'static [(char, Prompt)],
}

impl Prompts {
    /// A title none of whose prompt glyphs are read.
    pub const UNREAD: Self = Self {
        scope: Scope::AnyFace,
        glyphs: &[],
    };

    /// The control `codepoint` depicts, if this title draws it as a glyph.
    #[must_use]
    pub fn prompt_of(&self, codepoint: char) -> Option<Prompt> {
        self.glyphs
            .iter()
            .find(|(stand_in, _)| *stand_in == codepoint)
            .map(|&(_, prompt)| prompt)
    }
}

/// Which face button the front end leaves a screen on: the player's CONTROLS choice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MenuBack {
    /// Each title's and pressing's own button ([`crate::FrontEnd::back_button`]).
    #[default]
    Original,
    /// The circle, `B` on an Xbox-layout pad.
    Circle,
    /// The triangle, `Y` on an Xbox-layout pad.
    Triangle,
}

impl MenuBack {
    /// Every choice, in the order the CONTROLS row offers them.
    pub const ALL: [Self; 3] = [Self::Original, Self::Circle, Self::Triangle];

    /// The token stored in `settings.toml`.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Original => "original",
            Self::Circle => "circle",
            Self::Triangle => "triangle",
        }
    }

    /// The choice stored under `name`, `None` for a token this build does not know.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|choice| choice.name() == name)
    }
}

/// What a [`MenuBack`] choice means on one title and pressing: the one place that says which
/// physical button backs and which opens Help, and how a screen that does not ask follows.
///
/// A screen that takes its buttons as arguments (`Grid Selection`, `Cell Selection`) reads
/// [`Self::back`] and [`Self::help`] through [`Self::logical`]. Every other screen leaves on the
/// circle in its own code, so a choice of the triangle exchanges the two face buttons for the whole
/// front end instead ([`Self::input_swap`]), and a footer glyph follows the choice
/// ([`Self::glyph_swap`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MenuButtons {
    /// The physical button that leaves.
    pub back: Prompt,
    /// The physical button that opens Help: the screen's authored one, or the button `back`
    /// displaced from it.
    pub help: Prompt,
    /// Whether the front end reads the circle and the triangle exchanged, so a screen that leaves
    /// on the circle in its own code leaves on the triangle.
    pub input_swap: bool,
    /// Whether a footer's circle and triangle glyphs are drawn exchanged: the disc authors them
    /// for the original's button, and the choice is the other one.
    pub glyph_swap: bool,
}

impl MenuButtons {
    /// `choice` on a pressing whose own back button is `original` and whose Help screen opens on
    /// `authored_help`.
    #[must_use]
    pub fn resolve(original: Prompt, choice: MenuBack, authored_help: Prompt) -> Self {
        let back = match choice {
            MenuBack::Original => original,
            MenuBack::Circle => Prompt::Circle,
            MenuBack::Triangle => Prompt::Triangle,
        };
        Self {
            back,
            help: if authored_help == back {
                original
            } else {
                authored_help
            },
            input_swap: choice == MenuBack::Triangle,
            glyph_swap: back != original,
        }
    }

    /// `prompt` as the front end's code reads it: the circle and the triangle exchanged under
    /// [`Self::input_swap`].
    #[must_use]
    pub fn logical(self, prompt: Prompt) -> Prompt {
        if self.input_swap {
            exchanged(prompt)
        } else {
            prompt
        }
    }
}

/// The circle for the triangle and the triangle for the circle; any other prompt as it is.
#[must_use]
pub fn exchanged(prompt: Prompt) -> Prompt {
    match prompt {
        Prompt::Circle => Prompt::Triangle,
        Prompt::Triangle => Prompt::Circle,
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL_CHOICES: [MenuBack; 3] = MenuBack::ALL;

    #[test]
    fn the_original_choice_changes_nothing() {
        for original in [Prompt::Circle, Prompt::Triangle] {
            for help in [Prompt::Circle, Prompt::Triangle] {
                let buttons = MenuButtons::resolve(original, MenuBack::Original, help);
                assert_eq!(buttons.back, original);
                assert_eq!(buttons.help, help);
                assert!(!buttons.input_swap && !buttons.glyph_swap);
                assert_eq!(buttons.logical(Prompt::Circle), Prompt::Circle);
            }
        }
    }

    #[test]
    fn a_pressing_that_backs_on_the_circle_and_helps_on_the_triangle() {
        let circle = MenuButtons::resolve(Prompt::Circle, MenuBack::Circle, Prompt::Triangle);
        assert_eq!(
            (circle.back, circle.help),
            (Prompt::Circle, Prompt::Triangle)
        );
        assert!(!circle.input_swap && !circle.glyph_swap);

        let triangle = MenuButtons::resolve(Prompt::Circle, MenuBack::Triangle, Prompt::Triangle);
        assert_eq!(
            (triangle.back, triangle.help),
            (Prompt::Triangle, Prompt::Circle),
            "the triangle's Help is displaced onto the circle"
        );
        assert!(triangle.input_swap && triangle.glyph_swap);
        // A screen reads the exchanged frame: physical triangle is its circle.
        assert_eq!(triangle.logical(triangle.back), Prompt::Circle);
        assert_eq!(triangle.logical(triangle.help), Prompt::Triangle);
    }

    #[test]
    fn a_pressing_that_backs_on_the_triangle_and_helps_on_the_circle() {
        let circle = MenuButtons::resolve(Prompt::Triangle, MenuBack::Circle, Prompt::Circle);
        assert_eq!(
            (circle.back, circle.help),
            (Prompt::Circle, Prompt::Triangle)
        );
        assert!(
            !circle.input_swap,
            "its other screens leave on the circle already"
        );
        assert!(
            circle.glyph_swap,
            "its footers are authored for the triangle"
        );

        let triangle = MenuButtons::resolve(Prompt::Triangle, MenuBack::Triangle, Prompt::Circle);
        assert_eq!(
            (triangle.back, triangle.help),
            (Prompt::Triangle, Prompt::Circle)
        );
        assert!(
            triangle.input_swap,
            "its other screens leave on the circle in code"
        );
        assert!(
            !triangle.glyph_swap,
            "its footers already draw the triangle"
        );
    }

    #[test]
    fn back_and_help_are_never_the_same_button() {
        for original in [Prompt::Circle, Prompt::Triangle] {
            // A disc never authors Help on the button it leaves on.
            for help in [Prompt::Circle, Prompt::Triangle]
                .into_iter()
                .filter(|help| *help != original)
            {
                for choice in ALL_CHOICES {
                    let buttons = MenuButtons::resolve(original, choice, help);
                    assert_ne!(
                        buttons.back, buttons.help,
                        "{original:?} {choice:?} {help:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn a_choice_is_stored_under_its_name() {
        for choice in ALL_CHOICES {
            assert_eq!(MenuBack::from_name(choice.name()), Some(choice));
        }
        assert_eq!(MenuBack::from_name("b"), None);
        assert_eq!(MenuBack::default(), MenuBack::Original);
    }
}
