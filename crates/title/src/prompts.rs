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
