//! HD's, Omega's and 2048's post chains in a split screen: one run per view, each adapting
//! to its own pixels.
//!
//! All three read the scene's average luminance and set the exposure from it
//! (`oag_post::hd_bloom`, `oag_post::omega_tonemap`, `oag_post::wo_composite`). One
//! adaptation for a whole split would give a view in a dark tunnel and a view in daylight
//! the same exposure, so one of them is wrong, and HD's glow (the gate, the two blurs)
//! would bleed a few pixels across the cut.
//! So every view runs its own chain over its own rectangle of the finished scene target
//! (`Chain::run_view`): its own luminance ladder, its own adaptation (a lane per seat,
//! kept from frame to frame, forgotten on the first single-view frame after a split as the
//! camera histories are), and its resolve into the same rectangle of `view`; HD's and
//! 2048's glow is clamped to the rectangle too.
//!
//! **Chosen, not measured**: the original has no split screen. The adaptation law itself
//! (rates, clamps, curve) is the single-view one in every lane.
//!
//! **One submission per view's chain**, for [`Scene::blur_split_views`]' reason: the
//! chain's constants are written with `queue.write_buffer`, which lands before the
//! submission's commands, so two views in one encoder would both run with the last view's
//! rectangle.

use super::*;

impl Scene {
    /// Runs HD's chain, Omega's tone map or 2048's composite over each of `rects` (view `i`
    /// follows `slots[i]`) of the scene target, resolving into the same rectangle of `view`.
    /// Returns whether a chain ran: Pulse and Pure draw straight into the caller's view.
    pub(super) fn run_split_post(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        view: &wgpu::TextureView,
        slots: &[usize],
        rects: &[(f32, f32, f32, f32)],
        area: (f32, f32, f32, f32),
    ) -> bool {
        if let Some(hd) = &self.hd {
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
                        rect: pixels(rect),
                        area: (area.2 as u32, area.3 as u32),
                        // The glow's reach is authored as a fraction of the whole frame, and a
                        // view shows a fraction of it: scale it by the view's own pixel scale
                        // so the glow keeps its size against what it surrounds. Chosen, not
                        // measured.
                        reach: super::bloom::mean_reach(area, &[rect]),
                        first: index == 0,
                    },
                );
                queue.submit(Some(encoder.finish()));
            }
            true
        } else if let Some(omega) = &self.omega {
            for (index, (&slot, &rect)) in slots.iter().zip(rects).enumerate() {
                let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("race viewport omega post"),
                });
                omega.run_view(
                    device,
                    queue,
                    &mut encoder,
                    view,
                    oag_post::omega_tonemap::View {
                        seat: slot,
                        rect: pixels(rect),
                        first: index == 0,
                    },
                );
                queue.submit(Some(encoder.finish()));
            }
            true
        } else if let Some(wo) = &self.wo {
            for (index, (&slot, &rect)) in slots.iter().zip(rects).enumerate() {
                let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("race viewport wo post"),
                });
                wo.run_view(
                    device,
                    queue,
                    &mut encoder,
                    view,
                    oag_post::wo_composite::View {
                        seat: slot,
                        rect: pixels(rect),
                        first: index == 0,
                    },
                );
                queue.submit(Some(encoder.finish()));
            }
            true
        } else {
            false
        }
    }

    /// How many seats hold an adaptation of their own, for the tests.
    #[cfg(test)]
    pub(crate) fn split_exposure_seats(&self) -> usize {
        self.hd
            .as_ref()
            .map(oag_post::hd_bloom::Chain::seat_lanes)
            .or_else(|| {
                self.omega
                    .as_ref()
                    .map(oag_post::omega_tonemap::Chain::seat_lanes)
            })
            .or_else(|| {
                self.wo
                    .as_ref()
                    .map(oag_post::wo_composite::Chain::seat_lanes)
            })
            .unwrap_or(0)
    }
}

impl Scene {
    /// How many seat lanes HD's or Omega's chain has built so far, for the tests: a lane kept
    /// from frame to frame is built once.
    #[cfg(test)]
    pub(crate) fn split_exposure_builds(&self) -> u64 {
        self.hd
            .as_ref()
            .map(oag_post::hd_bloom::Chain::seat_lane_builds)
            .or_else(|| {
                self.omega
                    .as_ref()
                    .map(oag_post::omega_tonemap::Chain::seat_lane_builds)
            })
            .or_else(|| {
                self.wo
                    .as_ref()
                    .map(oag_post::wo_composite::Chain::seat_lane_builds)
            })
            .unwrap_or(0)
    }
}

/// A view's rectangle in whole pixels, as the scene pass drew it.
fn pixels(rect: (f32, f32, f32, f32)) -> (u32, u32, u32, u32) {
    (rect.0 as u32, rect.1 as u32, rect.2 as u32, rect.3 as u32)
}
