//! HD's post chain in a split screen: one run per view, each adapting to its own pixels.
//!
//! The chain reads the scene's average luminance and sets the exposure from it
//! (`oag_post::hd_bloom`). One adaptation for a whole split would give a view in a dark
//! tunnel and a view in daylight the same exposure, so one of them is wrong, and the
//! glow (the gate, the two blurs) would bleed a few pixels across the cut. So every view
//! runs its own chain over its own rectangle of the finished scene target
//! ([`oag_post::hd_bloom::Chain::run_view`]): its own reduction ladder, its own adapted
//! luminance (a lane per seat, kept from frame to frame, forgotten on the first
//! single-view frame after a split as the camera histories are), its glow clamped to its
//! rectangle, and its resolve into the same rectangle of `view`.
//!
//! **Chosen, not measured**: the original has no split screen. The adaptation law itself
//! (rate, the `Tone` clamps) is the single-view one in every lane.
//!
//! **One submission per view's chain**, for [`Scene::blur_split_views`]' reason: the
//! chain's constants are written with `queue.write_buffer`, which lands before the
//! submission's commands, so two views in one encoder would both run with the last view's
//! rectangle.

use super::*;

impl Scene {
    /// Runs HD's chain over each of `rects` (view `i` follows `slots[i]`) of the scene target,
    /// resolving into the same rectangle of `view`. Returns whether the chain ran: Pulse,
    /// Pure and Omega have no HD chain.
    pub(super) fn run_split_hd(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        view: &wgpu::TextureView,
        slots: &[usize],
        rects: &[(f32, f32, f32, f32)],
        area: (f32, f32, f32, f32),
    ) -> bool {
        let Some(hd) = &self.hd else {
            return false;
        };
        // The boost zoom streak holds the primary craft's pulse; a split draws none.
        hd.set_zoom_frame(None);
        for (index, (&slot, &rect)) in slots.iter().zip(rects).enumerate() {
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("race viewport hd post"),
            });
            hd.run_view(
                queue,
                device,
                &mut encoder,
                view,
                oag_post::hd_bloom::View {
                    seat: slot,
                    rect: (rect.0 as u32, rect.1 as u32, rect.2 as u32, rect.3 as u32),
                    area: (area.2 as u32, area.3 as u32),
                    // The glow's reach is authored as a fraction of the whole frame, and a
                    // view shows a fraction of it: scale it by the view's own pixel scale so
                    // the glow keeps its size against what it surrounds. Chosen, not measured.
                    reach: super::bloom::mean_reach(area, &[rect]),
                    first: index == 0,
                },
            );
            queue.submit(Some(encoder.finish()));
        }
        true
    }

    /// How many seats hold an HD adaptation of their own, for the tests.
    #[cfg(test)]
    pub(crate) fn split_exposure_seats(&self) -> usize {
        self.hd
            .as_ref()
            .map_or(0, oag_post::hd_bloom::Chain::seat_lanes)
    }
}
