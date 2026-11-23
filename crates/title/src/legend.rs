//! How a title's footer legend flows its button prompts.
//!
//! HD's and Omega's `NavigationController` overwrites every prompt's authored x each time it
//! lays out: the first starts after the `NAVIGATION` word and each next one after the previous
//! word's measured ink (`docs/ghidra/functions/ps3-hdfury-eu/navigation-legend.md`,
//! `docs/ghidra/functions/ps4-omega-eu/navigation-legend.md`). The two titles share the flow and
//! differ in where a word sits against its glyph and in whether the measure rounds a glyph's pen
//! up, so those are the data. A title whose footer is laid out as authored carries none.

/// Where a prompt's word starts, from its glyph's x.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WordAfter {
    /// A fixed distance, HD's 44 (`NavigationController_Layout` `0x001dab20`).
    Fixed(f32),
    /// The glyph's own measured width, truncated, plus this: Omega's 14
    /// (`NavigationController_Layout` `0x01475f50`).
    GlyphWidthPlus(f32),
}

/// A footer legend's flow law, in the screen's authored units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LegendFlow {
    /// How far past the previous prompt's word, and past `NAVIGATION`'s, the next glyph starts.
    pub gap: f32,
    /// Where a word sits against its glyph.
    pub word_after: WordAfter,
    /// Whether a measure rounds each glyph's pen up to a whole unit. HD's does after any text has
    /// drawn (`g_FontPenSnap`, `0x008c267c`); the draw itself never does. Omega's page predicts
    /// within 1 px a prompt without it and drifts 4 to 14 px with it, so it measures unsnapped.
    pub snap_pen: bool,
}
