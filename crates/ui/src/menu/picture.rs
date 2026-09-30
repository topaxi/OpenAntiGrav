//! What the rows are drawn on top of - a movie frame or the Fury backdrop's
//! pass - split out of `menu.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`, with no behaviour change.

use crate::frontend::Draw;

/// One frame of the looping picture the rows are drawn on top of.
///
/// **A frame and a rectangle, not a movie.** Deciding which frame is showing is
/// timing, deciding where it goes is the source's own display aspect, and
/// neither is a menu's business - this module draws a list and knows nothing
/// about either, the same way it knows nothing about what a setting means.
/// `main.rs` owns the player; see [`draw_list`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Backdrop {
    /// Where the picture goes on the 480x272 screen: `[x, y, width, height]`.
    pub rect: [f32; 4],
    /// Which frame of it to show, counting from zero.
    pub frame: usize,
    /// How far playback has got, counting every loop.
    ///
    /// Carried through to the draw for the same reason the front end carries it
    /// (see [`Draw::Video`]), even though the menus have already taken their
    /// picture out of the feed by the time they build one. It is the *same*
    /// playback either way: the number goes on rising across the handoff from
    /// `Show Logo` rather than starting again at zero, and a draw list that said
    /// otherwise would be the one place that claim is not visible.
    pub position: u64,
}

/// What the rows are drawn on top of: the disc's looping movie, or the Fury
/// style's point-cloud pass.
///
/// One slot, because a screen has one picture under it and the two are the
/// same kind of thing to a menu - decided elsewhere, drawn between the clear
/// and the chrome (see [`Frame::backdrops`]). `From` both ways so a caller
/// with a [`Backdrop`] in hand keeps passing it.
#[derive(Debug, Clone, PartialEq)]
pub enum Picture {
    /// A frame of the looping movie.
    Movie(Backdrop),
    /// The Fury backdrop's frame - see [`crate::backdrop`]. Boxed: two
    /// matrices and nine constants beside a 32-byte movie frame.
    Fury(Box<crate::backdrop::Frame>),
    /// The HD-style `BackgroundAnim` scene's frame - see
    /// [`crate::scene_backdrop`].
    Scene(Box<crate::scene_backdrop::Frame>),
}

impl From<Backdrop> for Picture {
    fn from(backdrop: Backdrop) -> Self {
        Self::Movie(backdrop)
    }
}

impl From<crate::backdrop::Frame> for Picture {
    fn from(frame: crate::backdrop::Frame) -> Self {
        Self::Fury(Box::new(frame))
    }
}

impl From<crate::scene_backdrop::Frame> for Picture {
    fn from(frame: crate::scene_backdrop::Frame) -> Self {
        Self::Scene(Box::new(frame))
    }
}

impl Picture {
    /// The draw this picture is, in the list's own vocabulary.
    #[must_use]
    pub fn draw(self) -> Draw {
        match self {
            Self::Movie(backdrop) => Draw::Video {
                rect: backdrop.rect,
                frame: backdrop.frame,
                position: backdrop.position,
                // The same movie `Show Logo` sits on, still looping and still
                // on the same playhead: the menus are where the disc's own
                // `FE Screen` was going anyway.
                source: crate::frontend::Video::Backdrop,
            },
            Self::Fury(frame) => Draw::FuryBackdrop(frame),
            Self::Scene(frame) => Draw::SceneBackdrop(frame),
        }
    }
}
