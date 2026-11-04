//! The button glyphs Wipeout HD's `Buttons` face carries.
//!
//! `Data\FE\Fonts\PS_BUTTONS.fnt` holds `ε γ δ β` as 48x53 cells: the circled
//! cross, circle, square and triangle, and `Λ Ν` as 85x54 boxes lettered `L2`
//! and `R2`, read off the rendered cells of the HD EU disc.
//! Its `Δ Γ Β Α` are four
//! arrow-like navigation marks whose action was not established, so they are
//! left out and draw as the disc draws them. HD's language plugins declare
//! the face as the `Buttons` role, and the same codepoints in any other face
//! are Greek letters, so the table is [`Scope::ButtonsFace`].

use oag_title::prompts::{Prompt, Prompts, Scope};

/// HD's prompt table. Measured.
pub const PROMPTS: &Prompts = &Prompts {
    scope: Scope::ButtonsFace,
    glyphs: &[
        ('ε', Prompt::Cross),
        ('γ', Prompt::Circle),
        ('δ', Prompt::Square),
        ('β', Prompt::Triangle),
        ('Λ', Prompt::L),
        ('Ν', Prompt::R),
    ],
};
