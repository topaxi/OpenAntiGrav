//! Presentation: what happens to a frame between the scene and the glass.
//!
//! - [`upscale`]: the scene-to-surface blit, the upscalers and anti-aliasers,
//!   the grade and the screen filter, and the composite order the window uses.
//! - [`drs`]: dynamic resolution scaling, a pure controller that is fed a
//!   frame time and answers with a render rectangle.
//! - [`perf`]: the frame-time meter, the cost breakdown and the overlay's
//!   lines.
//!
//! The three share the display vocabulary and the performance numbers `drs`
//! learns from, which is why they are one crate. Nothing here knows a race, a
//! menu or a settings file: the host passes plain data in.

pub mod drs;
pub mod perf;
pub mod upscale;
