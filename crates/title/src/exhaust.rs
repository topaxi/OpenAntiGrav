//! Where a title keeps the exhaust ribbon's own texture.
//!
//! **An axis because the two answers are not the same kind of thing.** Both PSP
//! titles and the PS2 port name one texture outright - a literal string in the
//! executable, so an exact archive hit - and every other number the ribbon needs
//! is a preset table in the code beside it. Wipeout HD names none: it authors
//! the ribbon as an **asset**, a one-triangle template under
//! `/data/ribboneffects/` whose `.rcsmodel` material carries the texture paths
//! and the blend factors, so the right thing to hand a loader there is the
//! model, not a texture name.
//!
//! Reaching for `oag_pulse`'s constant on all three is what left HD's ribbon
//! drawing a procedural glow with a `- the trail falls back` line in every load
//! report. See [`trail-ribbon.md`](../../../docs/rendering/trail-ribbon.md) for
//! the survey that measured the difference, including why a `Trail` node's
//! payload cannot supply any of it.

/// How this title supplies the exhaust ribbon's texture.
///
/// **The three answers are exclusive, so this is an enum rather than a struct
/// of options.** A title names a texture, or it ships a template that names
/// one, or neither has been located - and no source measured so far does two of
/// those. If one ever does, the honest change is a fourth variant, which a
/// reviewer sees; a pair of `Option`s would have let the same source through
/// silently with whichever field the loader happened to check first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Exhaust {
    /// The ribbon's noise texture, by the name the executable itself carries.
    ///
    /// `Data\Tex\engineFlare\Engine_noise.mip` on Pulse and Pure. The PS2 build
    /// loads the same literal and finds it under `.pct`; nothing here needs to
    /// know which disc it is reading, because `oag_pulse::read_image` does the
    /// extension rewrite.
    Named(&'static str),

    /// The ribbon *template*, for a title that authors the ribbon rather than
    /// naming a texture: a `.rcsmodel` path.
    ///
    /// Its first material names the colour texture, the noise texture and the
    /// blend factor pair, so one lookup supplies all three and none of them is
    /// written down here.
    Authored(&'static str),

    /// This title's exhaust ribbon has not been located at all.
    ///
    /// Not a placeholder to be filled in silently: a loader that gets this
    /// reports the absence, the same way a missing archive entry is reported
    /// rather than swapped for a stand-in.
    Unread,
}
