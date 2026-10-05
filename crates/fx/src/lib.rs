//! The renderer's visual effects, split out of `oag-render` on 2026-10-05.
//!
//! Everything here draws something the simulation caused but never reads back:
//! it is render-only, like the rest of the renderer, and no gameplay crate may
//! depend on it (`just check-deps`). It sits above `oag-gpu` and `oag-mesh`,
//! whose vertex type every effect emits, and below `oag-render`, which owns the
//! scene's draw loop and the cameras and calls into these.
//!
//! Each effect keeps the shape the exhaust set: a wgpu-free half that is the
//! recovered state machine and is testable on a CPU, and a pipeline half.
//!
//! - [`psys`] plays any authored `.pob` particle effect; [`sparks`] is the
//!   collision-spark adapter over it.
//! - [`exhaust`] is the engine flare and trail.
//! - [`mist`], [`cloud`] and [`ranrot`] are the weather: the mist sheet, the
//!   cloud field and the generator the field reseeds with.
//! - [`beam`] is the Leach beam, [`flash`] the screen flash and
//!   [`weapon_quads`] the Cannon's hand-built quads.
//! - [`hull_overlay`] and [`absorb_shell`] are the weapon-absorb effect drawn
//!   over a hull.

pub mod absorb_shell;
pub mod beam;
pub mod cloud;
pub mod exhaust;
pub mod flash;
pub mod hull_overlay;
pub mod magstrip;
pub mod mist;
pub mod psys;
pub mod ranrot;
pub mod sparks;
pub mod weapon_quads;
