//! The one name for how a front end lays a text's pen out.

/// How a front end lays a text's pen out: the one choice every measure of a draw (a wrap, a hit
/// rectangle, an alignment, a marquee's overflow) has to share with the draw itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Pens {
    /// Each pen steps by the advance alone, in fractions.
    #[default]
    Advance,
    /// The first glyph's ink starts at the text's x and each glyph's ink edge, its bearing past
    /// the pen, steps up to a whole unit counted from there ([`super::pen_step`]): Pulse's front
    /// end (`oag_title::MenuSkin::bearing_pens`). The quads stay the atlas cell's, unlike
    /// [`Self::Snapped`].
    Bearing,
    /// Each glyph's ink edge steps up to a whole unit ([`super::pen_step`]): Pure's front end
    /// (`oag_title::MenuSkin::snapped_pens`).
    Snapped,
}

impl Pens {
    /// The pens a skin's two flags name; a snapped pen wins.
    #[must_use]
    pub fn of(snapped: bool, bearing: bool) -> Self {
        if snapped {
            Self::Snapped
        } else if bearing {
            Self::Bearing
        } else {
            Self::Advance
        }
    }
}
