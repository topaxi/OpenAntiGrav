//! The button glyphs Omega's `Buttons` face carries.
//!
//! `Data\FE\Fonts\PS_BUTTONS.fnt` is HD's face at twice the pixel size: `ε γ δ
//! β` are 96x107 cells, the circled cross, circle, square and triangle, and
//! `Λ Ν` are 168x109 boxes lettered like HD's `L2` and `R2`. Read off the
//! rendered cells of the Omega EU package (base plus patch). Its `Δ Γ Β Α`
//! are the same four navigation marks HD has and stay the disc's own. The
//! same codepoints in any other face are Greek letters, so the table is
//! [`Scope::ButtonsFace`].

use oag_title::prompts::{Prompt, Prompts, Scope};

/// Omega's prompt table. Measured.
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
