//! [`Scene::composite_bloom`]: Pulse's bloom added over the HUD.
//!
//! The PSP original's render queue is sorted ascending
//! (`Gfx_CompareQueueKeys`) and `Bloom_Draw` sits at key `0x70`, after every
//! HUD widget (`0x52` to `0x6d`), so its composite lands on the HUD as well as
//! on the scene. [`Scene::render`] runs the bright pass and the blurs; the
//! caller draws the HUD and then calls this. See
//! `docs/ghidra/functions/psp-pulse-usa/bloom.md`, "The bloom draws over the
//! HUD". **Pulse PS2's own chain takes the same order by inheritance**: its
//! HUD order is not measured on the PS2 disc (`docs/rendering/ps2-bloom.md`).
//! HD's chain is untouched - its read resolve adds the bloom before the encode,
//! so a HUD under it is not a reorder but an invented split. Split into its
//! own file so `scene.rs` and `frame.rs` stay under
//! `scripts/check-file-size.py`'s ceiling.

use super::*;

impl Scene {
    /// Adds the glow [`Scene::render`] prepared onto `target`, over the
    /// rectangle `rect` (`x, y, width, height`) the frame was drawn into.
    ///
    /// A no-op unless this frame prepared a Pulse bloom (PSP or PS2): HD runs
    /// its whole chain inside [`Scene::render`]. Calling it twice adds the
    /// glow once.
    pub fn composite_bloom(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        rect: (f32, f32, f32, f32),
    ) {
        if !self.bloom_pending.replace(false) {
            return;
        }
        let origin = (rect.0, rect.1);
        let viewport = (rect.2 as u32, rect.3 as u32);
        if let Some(bloom) = &self.bloom {
            bloom.composite(encoder, target, origin, viewport);
        } else if let Some(bloom) = &self.ps2_bloom {
            bloom.composite(encoder, target, origin, viewport);
        }
    }
}
