//! Motion blur in a split screen: one chain per view, after the last view is drawn.
//!
//! The blur reads the velocity and depth attachments and overwrites the finished picture.
//! Both attachments hold every view (a later view loads them, `Layer::Next`), so it runs
//! once per view over that view's rectangle, each chain a [`Scene::encode_motion_blur`] with
//! the rectangle as its viewport: its taps clamp to the rectangle, and its intermediate
//! targets are cleared whole and drawn only inside it, so no view's velocity reaches another's
//! pixels. The velocities themselves are each view's own: [`Scene::render`] measured them
//! against that view's camera history ([`motion::split_history`]).
//!
//! **One submission per view's chain.** `MotionBlur::render` writes its one uniform buffer
//! with `queue.write_buffer`, which lands before the submission's commands, so two chains in
//! one encoder would both run with the last view's rectangle, tile and shake - the trap
//! `Scene::render_slots` records for the scene draws. **Chosen, not measured.**

use super::*;

impl Scene {
    /// Blurs each of `rects` (view `i` follows `slots[i]`) with `strength`, the finished picture
    /// in `view`. Returns whether a chain ran. `timestamps` brackets all the chains with one
    /// pair: its opening half rides the first view's first pass and its closing half the last
    /// view's last, so the overlay's BLUR row reads the whole.
    pub(super) fn blur_split_views(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        view: &wgpu::TextureView,
        slots: &[usize],
        rects: &[(f32, f32, f32, f32)],
        strength: f32,
        timestamps: Option<oag_post::motion_blur::ChainTimestamps<'_>>,
    ) -> bool {
        if strength <= 0.0 || self.motion_blur.is_none() {
            return false;
        }
        let (mut begin, mut end) = timestamps
            .map(|pair| (pair.begin, pair.end))
            .unwrap_or_default();
        let last = slots.len().saturating_sub(1);
        let mut ran = false;
        for (index, (&slot, &rect)) in slots.iter().zip(rects).enumerate() {
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("race viewport blur"),
            });
            let shake = self.histories[motion::split_history(slot)].blur_shake.get();
            ran |= self.encode_motion_blur(
                device,
                queue,
                &mut encoder,
                [
                    view,
                    &self.attachment_views.velocity,
                    &self.attachment_views.depth,
                ],
                rect,
                strength,
                shake,
                Some(oag_post::motion_blur::ChainTimestamps {
                    begin: if index == 0 { begin.take() } else { None },
                    end: if index == last { end.take() } else { None },
                }),
            );
            queue.submit(Some(encoder.finish()));
        }
        ran
    }

    /// Forgets the split seats' camera histories, once, on the first single-view frame after a
    /// split: a seat that comes back later would otherwise measure its first frame against a
    /// pose from before the gap and smear it (capped at the blur's reach, but a visible streak).
    pub(super) fn retire_split_histories(&self) {
        if self.split_live.replace(false) {
            for history in &self.histories[motion::MAX_HISTORIES..] {
                history.motion.borrow_mut().take();
            }
        }
    }
}
