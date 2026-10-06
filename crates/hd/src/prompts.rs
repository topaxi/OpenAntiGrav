//! The button glyphs Wipeout HD's `Buttons` face carries.
//!
//! `Data\FE\Fonts\PS_BUTTONS.fnt` holds `ε γ δ` as 48x53 cells: the circled
//! cross, circle and square, read off the rendered cells of the HD EU disc
//! (`oag-tools --example prompt_glyph_probe`). Its `Δ Γ Β Α` are four
//! arrow-like navigation marks whose action was not established, so they are
//! left out and draw as the disc draws them. HD's language plugins declare
//! the face as the `Buttons` role, and the same codepoints in any other face
//! are Greek letters, so the table is [`Scope::ButtonsFace`].

use oag_title::prompts::{Prompt, Prompts, Scope};

/// HD's prompt table. Measured for the three it lists.
pub const PROMPTS: &Prompts = &Prompts {
    scope: Scope::ButtonsFace,
    glyphs: &[
        ('ε', Prompt::Cross),
        ('γ', Prompt::Circle),
        ('δ', Prompt::Square),
    ],
};
