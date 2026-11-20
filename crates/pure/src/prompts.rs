//! The button glyphs Pure's text faces embed.
//!
//! `ε γ δ β Λ Ν` are in `FX300ANG.fnt` (the menus' face) and `LTe50325.fnt`
//! (the scroll face) as 14x17 cells for the four face buttons and 25x17 and
//! 26x17 for the shoulders. Read off the rendered cells of the Pure EU disc:
//! `ε` is the circled cross, `γ` the circle, `δ` the square, `β` the triangle,
//! `Λ` the `L` box and `Ν` the `R` box, the same six pictures Pulse draws
//! (`docs/ui/button-prompts.md`). The strings carry them inline: `Press ε to
//! Continue`, `Press β to Cancel`.
//!
//! The footers' own `Confirm` and `Back` are in the `Stats` face instead, as filled discs: `έ` is the
//! cross and `Ϊ` the triangle (the footers the Pure EU disc draws show the circled cross beside
//! `CONFIRM` and the triangle beside `BACK`). The face's other two discs are `Ϋ`, the circle (the
//! footer drawn under MENU BACK `circle`), and `ά`, the square (read off the shape of its cell, 75;
//! no footer draws it). The outlined variants (`ή ί ΰ`) are not read.
//! `docs/ui/button-prompts.md`, "Other titles".

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
        ('έ', Prompt::Cross),
        ('Ϋ', Prompt::Circle),
        ('ά', Prompt::Square),
        ('Ϊ', Prompt::Triangle),
    ],
};
