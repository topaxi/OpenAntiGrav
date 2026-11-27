//! A title's own frame-rate throttle on the scene's width, which decides whether the TARGET
//! FPS setting offers `original`.
//!
//! Per title (ADR-0058). Wipeout HD's is read off its executable
//! (`docs/ghidra/functions/ps3-hdfury-eu/scene-width-throttle.md`): it narrows the scene one
//! notch a frame while the GPU's frame runs long and widens it again when there is room,
//! never touching the height. Omega's executable has none of it and 2048's was not read.

/// The law: a level of `0..=max_level` taken off the width, stepped one notch a frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Law {
    /// The most notches taken off the width.
    pub max_level: u32,
    /// The width at level `l` is `(width_notches - l) / width_notches` of the full width.
    pub width_notches: u32,
    /// A GPU frame of more than this many microseconds narrows by a notch, at
    /// [`Self::frame_hz`].
    pub narrow_above_us: u32,
    /// A GPU frame of at most this many microseconds widens by a notch, at
    /// [`Self::frame_hz`].
    pub widen_up_to_us: u32,
    /// The frame rate the two durations above are authored against.
    pub frame_hz: u32,
}
