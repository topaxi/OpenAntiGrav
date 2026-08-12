//! How a title's front end lays its menus out and moves between them.
//!
//! # Why this is a type and not two piles of constants
//!
//! [ADR-0022] licenses an axis only once a *second* corpus has been measured,
//! which is the trap ADR-0009 named. This axis clears that bar: both titles'
//! `Skin.xml` files were read off their own discs, and they disagree on every
//! number they share - `MenuXOffset` 50 against 21, `MenuScale` 1.0 against
//! 1.15, `TitleScale` 1.0 against 0.97 - while Pure declares two of the colours
//! Pulse declares not at all. A single set of constants in `oag-pulse` would
//! have to be either wrong for Pure or silently reused as if measured.
//!
//! The layout numbers are the disc's. **The menu *tree* is not**, and stays
//! ours: see `docs/architecture/menus.md`. This type carries presentation only.
//!
//! # Where the numbers came from
//!
//! Two sources, and the difference matters when reading a field's confidence:
//!
//! - **Authored**, read straight out of the title's `Skin.xml` `FEGlobals`
//!   block or its screen definitions. Exact, and re-checkable by a
//!   disc-backed test.
//! - **Measured**, read off a capture of the original running under an
//!   emulator, because the data either does not state it or states it in units
//!   nothing decodes. [`Self::row_extra_leading`] and [`Self::selected`] are
//!   the two.
//!
//! Every field says which it is. See `docs/ui/menus-original.md` for the
//! captures and the confidence scores.
//!
//! [ADR-0022]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0022-title-packages.md

/// A packed `0xAARRGGBB` colour, exactly as the front-end XML writes one.
///
/// Kept packed rather than unpacked to `[f32; 4]` so a title package's table
/// reads the same as the `Skin.xml` line it came from, and a reviewer can
/// compare the two without arithmetic.
pub type Argb = u32;

/// One title's menu presentation.
///
/// Held by [`crate::Title`] so that opening one title's archives while drawing
/// another's skin is a state that cannot be constructed - the same argument
/// [`crate::boot::BootProfile`] is hung off `Title` for.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MenuSkin {
    /// Left edge of the menu rows. Authored, `FEGlobals->MenuXOffset`.
    pub menu_x: f32,
    /// Scale applied to menu rows. Authored, `FEGlobals->MenuScale`.
    pub menu_scale: f32,
    /// Left edge of the screen title. Authored, `FEGlobals->TitleXOffset`.
    pub title_x: f32,
    /// Top of the screen title. Authored, `FEGlobals->TitleYOffset`.
    pub title_y: f32,
    /// Scale applied to the screen title. Authored, `FEGlobals->TitleScale`.
    pub title_scale: f32,
    /// Top of the first row, in the layer the rows are drawn in.
    ///
    /// Authored per *screen* rather than in `FEGlobals` - Pulse's `Main Menu`
    /// says `y="32"` - so a title whose screens were never read leaves this
    /// `None` and the caller keeps its own value.
    ///
    /// This is the top of the row's **line box**, not of its glyphs: a capture
    /// puts the first row's ink at y=40 for a 22-pixel face, and that 8-pixel
    /// inset is already baked into the glyph boxes the atlas hands back.
    pub first_row_y: Option<f32>,
    /// Added to the row font's line height to get the row pitch. **Measured.**
    ///
    /// `None` for a title whose menus have not been captured, and then the
    /// caller supplies its own leading rather than borrowing the other title's
    /// measurement - the rule below was measured on Pulse and there is no
    /// evidence it holds anywhere else.
    ///
    /// The authored `gap` attribute is *not* this, and reading it as row
    /// spacing produces a visibly wrong menu. Four of Pulse's menus, across two
    /// faces and two authored gaps, all measure `line height + 6`:
    ///
    /// | screen | `gap` | face line height | pitch |
    /// | --- | --- | --- | --- |
    /// | `Main Menu` | 15 | 22 | 28 |
    /// | `Racebox` | 15 | 22 | 28 |
    /// | `Single Player` | - | 17 | 23 |
    /// | `Cell Setup` | 0 | 17 | 23 |
    ///
    /// `gap` 0 and `gap` 15 give the same pitch for the same face, so whatever
    /// `gap` is for, it is not this. The 6 is a measured constant of unknown
    /// origin; nothing here claims to have derived it.
    pub row_extra_leading: Option<f32>,
    /// The font role menu rows are drawn in, as the language plugin names it.
    ///
    /// Authored - Pulse's `Main Menu` rows say `font="menu"`, which its language
    /// plugins resolve to a face two-thirds taller than the body one.
    ///
    /// **`None` means "draw the rows in the default face", which covers two
    /// different findings.** A title whose menu definitions were never read
    /// leaves this `None` because nothing is known; a title whose definitions
    /// *were* read and name no separate role leaves it `None` because there is
    /// nothing to name - Pure's measured case, recorded in
    /// `oag_pure::frontend::MENU_SKIN`. The caller does the same thing either
    /// way, so the distinction lives in each title crate's own comment rather
    /// than in this type.
    pub menu_font: Option<&'static str>,
    /// Unselected row text. Authored, `FEGlobals->TextColor`.
    ///
    /// `None` for a title whose `Skin.xml` does not declare it, which is Pure's
    /// case. The caller supplies its own and must not borrow the other title's.
    pub text: Option<Argb>,
    /// Screen title text. Authored, `FEGlobals->TitleColor`.
    ///
    /// Pulse's is `0xFF000000`, black, because its title sits on a light top
    /// bar this build does not draw yet. `None` where undeclared.
    pub title: Option<Argb>,
    /// The selected row. **Measured**, not authored.
    ///
    /// Nothing in the XML states a selected colour. A capture shows the row
    /// brightened toward white rather than given a fill bar or the pink
    /// `MenuHighLightArrowColor`, which appears nowhere on these screens. Two
    /// frames of the same still menu measured different peaks - rgb(157,255,255)
    /// and rgb(107,226,247) - so **the highlight moves**, and this is the
    /// brighter of the two rather than a phase of a pulse whose period and depth
    /// were not measured. See `docs/ui/menus-original.md`.
    ///
    /// `None` where no capture has been taken; the caller supplies its own.
    pub selected: Option<Argb>,
    /// How long a page change takes, in seconds. Authored, `transition=`.
    ///
    /// Confirmed as seconds rather than assumed: Pulse's `Main Menu` is
    /// authored `0.5` and its transition runs about 13 presented frames against
    /// a front end measured at 30 Hz.
    pub transition_secs: f32,
}

impl MenuSkin {
    /// The distance between two rows drawn in a `line_height`-tall face.
    ///
    /// `extra` is what the caller uses when this title's own leading is
    /// unmeasured - passed in rather than defaulted here so the number that
    /// ends up on screen belongs to whoever is drawing, and a title package's
    /// silence never quietly turns into another title's measurement.
    ///
    /// The scale multiplies both terms because a scaled row is a scaled gap
    /// too - which is untested against the original, every menu measured so far
    /// being authored at scale 1.0. Pure's `MenuScale` of 1.15 is the first
    /// case that will exercise it.
    #[must_use]
    pub fn row_pitch(&self, line_height: f32, extra: f32) -> f32 {
        (line_height + self.row_extra_leading.unwrap_or(extra)) * self.menu_scale
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pulse's own numbers, as a fixture rather than as a dependency: this
    /// crate must not know any title's data, so the check that the arithmetic
    /// is right cannot reach for `oag-pulse`.
    const PULSE_SHAPED: MenuSkin = MenuSkin {
        menu_x: 50.0,
        menu_scale: 1.0,
        title_x: 50.0,
        title_y: 0.0,
        title_scale: 1.0,
        first_row_y: Some(32.0),
        row_extra_leading: Some(6.0),
        menu_font: Some("menu"),
        text: Some(0xFF33_A6B9),
        title: Some(0xFF00_0000),
        selected: Some(0xFF9D_FFFF),
        transition_secs: 0.5,
    };

    /// The four pitches measured off the original, reproduced by the rule.
    #[test]
    fn the_pitch_rule_reproduces_every_measured_menu() {
        // The 22-pixel `menu` face, on `Main Menu` and `Racebox`.
        assert!((PULSE_SHAPED.row_pitch(22.0, 0.0) - 28.0).abs() < f32::EPSILON);
        // The 17-pixel `small` face, on `Single Player` and `Cell Setup`.
        assert!((PULSE_SHAPED.row_pitch(17.0, 0.0) - 23.0).abs() < f32::EPSILON);
    }

    /// A scaled skin scales the leading with the text, rather than leaving a
    /// constant gap that would crowd at 1.15 and sprawl at 0.8.
    #[test]
    fn scale_reaches_the_leading_too() {
        let scaled = MenuSkin {
            menu_scale: 2.0,
            ..PULSE_SHAPED
        };
        assert!((scaled.row_pitch(22.0, 0.0) - 56.0).abs() < f32::EPSILON);
    }

    /// A title that measured no leading of its own takes the caller's, not the
    /// measurement some other title happens to carry.
    #[test]
    fn an_unmeasured_leading_falls_back_to_the_callers() {
        let unmeasured = MenuSkin {
            row_extra_leading: None,
            ..PULSE_SHAPED
        };
        assert!((unmeasured.row_pitch(22.0, 2.0) - 24.0).abs() < f32::EPSILON);
    }
}
