//! Post-processing that runs between a scene and the surface.
//!
//! Everything here reads a finished offscreen frame and writes another one; none
//! of it knows what drew the frame or where the result is going. That is what
//! lets the game's composition root own the blit, the aspect bars and the
//! brightness grade while the upscalers live in the renderer, reachable from
//! `oag-view` too.
//!
//! # Colour space
//!
//! **The upscalers here work in perceptual space** - on sRGB-encoded values, not
//! on linear light. FSR 1's edge detection reasons about luma differences the
//! way an eye weighs them, and feeding it linear light makes it misjudge which
//! edges matter in shadow.
//!
//! The scene target's *format* is still sRGB, so everything that draws into it
//! keeps encoding on write exactly as it does onto a window. What changes is how
//! the upscaler *reads* it: through a non-sRGB view of the same texture, which
//! returns the stored bytes without the hardware's decode. The three consumers
//! of a frame therefore want three different things, and it is worth naming them
//! because getting one wrong costs an encode either way:
//!
//! | Consumer | Wants | How it gets it |
//! | --- | --- | --- |
//! | A PNG readback | sRGB-encoded bytes | the texture's own bytes, copied |
//! | FSR 1 (here) | sRGB-encoded samples | a non-sRGB view |
//! | The blit's grade | linear light | an sRGB view, or an explicit decode |
//!
//! FSR 3.1, when it lands, wants a fourth thing - linear light with its own
//! tonemapping either side of accumulation - which is why the scene target's
//! encoding is a per-upscaler decision rather than a setting made once.

pub mod bloom;
pub mod fsr1;
pub mod fxaa;
pub mod smaa;
