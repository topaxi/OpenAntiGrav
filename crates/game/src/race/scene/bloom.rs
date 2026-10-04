//! [`Scene::composite_bloom`]: Pulse PSP's bloom added over the HUD.
//!
//! The original's render queue is sorted ascending (`Gfx_CompareQueueKeys`)
//! and `Bloom_Draw` sits at key `0x70`, after every HUD widget (`0x52` to
//! `0x6d`), so its composite lands on the HUD as well as on the scene.
//! [`Scene::render`] runs the bright pass and the blurs; the caller draws the
//! HUD and then calls this. See `docs/ghidra/functions/psp-pulse-usa/bloom.md`,
//! "The bloom draws over the HUD". Split into its own file so `scene.rs` and
//! `frame.rs` stay under `scripts/check-file-size.py`'s ceiling.

use super::*;

impl Scene {
    /// Adds the glow [`Scene::render`] prepared onto `target`, over the
    /// rectangle `rect` (`x, y, width, height`) the frame was drawn into.
    ///
    /// A no-op unless this frame prepared Pulse PSP's bloom: HD and PS2 run
    /// their whole chains inside [`Scene::render`]. Calling it twice adds the
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
        if let Some(bloom) = &self.bloom {
            bloom.composite(
                encoder,
                target,
                (rect.0, rect.1),
                (rect.2 as u32, rect.3 as u32),
            );
        }
    }
}
