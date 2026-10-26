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
//! one encoder would both run with the last view's rectangle, tile and lens map - the trap
//! `Scene::render_slots` records for the scene draws. **Chosen, not measured.**

use super::*;

impl Scene {
    /// Blurs each of `rects` (view `i` follows `slots[i]`) with `strength`, the finished picture
    /// in `view`. Returns whether a chain ran. `timestamps` brackets all the chains with one
    /// pair: its opening half rides the first view's first pass and its closing half the last
    /// view's last, so the overlay's BLUR row reads the whole.
    #[allow(clippy::too_many_arguments)]
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
            let lens = self.histories[motion::split_history(slot)].blur_lens.get();
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
                lens,
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
    ///
    /// **Only a frame that draws no window and no split** (`Layer::Whole`).
    /// A multi-window frame whose one-seat window draws beside a split window keeps the split's
    /// seats: their state is forgotten per seat instead, by [`Self::keep_split_seat`], when a
    /// seat missed a frame. Retired here on every one-seat window's frame, the split window's
    /// seats lost their camera histories (no motion, so no blur) and HD's and Omega's
    /// adaptation every frame.
    pub(super) fn retire_split_histories(&self) {
        if self.split_live.replace(false) {
            // HD's, Omega's and 2048's per-seat adaptation goes with them (`split_exposure`).
            if let Some(hd) = &self.hd {
                hd.retire_seats();
            }
            if let Some(omega) = &self.omega {
                omega.retire_seats();
            }
            if let Some(wo) = &self.wo {
                wo.retire_seats();
            }
            for history in &self.histories[motion::MAX_HISTORIES..] {
                history.forget();
            }
            for drawn in &self.seat_drawn {
                drawn.set(None);
            }
        }
    }

    /// Counts a frame, for a draw that starts one: a split or a window drawn with `shared`
    /// false (the first of the frame to draw, which does the frame's shared work). A seat's
    /// split state is kept only while the seat is drawn in consecutive frames.
    pub(super) fn begin_split_frame(&self, shared: bool) {
        if !shared {
            self.split_frame.set(self.split_frame.get().wrapping_add(1));
        }
    }

    /// Stamps `slot`'s seat as drawn this frame, first forgetting its camera history and HD's
    /// or Omega's adaptation lane when it was not drawn as a split view the frame before: a
    /// seat back after a gap (a window that drew it alone for a while, the intro's one view, a
    /// closed window) settles from its own first frame, as after a single-view frame.
    pub(super) fn keep_split_seat(&self, slot: usize) {
        let seat = slot.min(motion::MAX_HISTORIES - 1);
        let now = self.split_frame.get();
        let last = self.seat_drawn[seat].replace(Some(now));
        if last.is_some_and(|frame| frame != now && frame.wrapping_add(1) != now) {
            if let Some(hd) = &self.hd {
                hd.retire_seat(seat);
            }
            if let Some(omega) = &self.omega {
                omega.retire_seat(seat);
            }
            if let Some(wo) = &self.wo {
                wo.retire_seat(seat);
            }
            self.histories[motion::split_history(seat)].forget();
        }
    }
}
