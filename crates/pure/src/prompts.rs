//! The button glyphs Pure's text faces embed.
//!
//! `ε γ δ β Λ Ν` are in `FX300ANG.fnt` (the menus' face) and `LTe50325.fnt`
//! (the scroll face) as 14x17 cells for the four face buttons and 25x17 and
//! 26x17 for the shoulders. Read off the rendered cells of the Pure EU disc:
//! `ε` is the circled cross, `γ` the circle, `δ` the square, `β` the triangle,
//! `Λ` the `L` box and `Ν` the `R` box, the same six pictures Pulse draws
//! (`docs/ui/button-prompts.md`). The strings carry them inline: `Press ε to
//! Continue`, `Press β to Cancel`.

use oag_title::prompts::{Prompt, Prompts, Scope};

/// Pure's prompt table. Measured.
pub const PROMPTS: &Prompts = &Prompts {
    scope: Scope::AnyFace,
    glyphs: &[
        ('ε', Prompt::Cross),
        ('γ', Prompt::Circle),
        ('δ', Prompt::Square),
        ('β', Prompt::Triangle),
        ('Λ', Prompt::L),
        ('Ν', Prompt::R),
    ],
};
