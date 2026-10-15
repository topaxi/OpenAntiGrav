//! FSR 3.1 over a split screen: a chain per view, each resolving its own rectangle of the scene
//! target into its own rectangle of the presentation target.
//!
//! **Why a chain per view.** FSR 3.1 reconstructs from a history: the previous frames'
//! upscaled colour, locks, luma and depth, reprojected by this frame's motion. One chain over a
//! split would reproject each view's history across the cut into its neighbour, and would take
//! one camera's jitter and depth transform for every view. So each seat gets a
//! [`fsr3::Fsr3::sibling`] (the compiled passes shared, the constants, history and
//! intermediates its own), its render size the view's, its inputs read from the view's corner
//! ([`fsr3::Dispatch::input_offset`]), and its output sized to the view's share of the
//! presentation rectangle. Each view's jitter sequence and camera record are its seat's
//! (`oag_raceplay`'s per-seat histories).
//!
//! **Where each view lands** is where the blit puts it when nothing reconstructs: the view's
//! rectangle mapped linearly from the scene's extent onto the presentation rectangle, its edges
//! rounded to whole pixels. The gap between views is cleared, as the blit's whole-target clear
//! leaves it.
//!
//! **Not timed**: the chains carry no timestamp pair, so the overlay's two FSR 3 rows idle in a
//! split, as they did when a split had no upscaler.

use super::*;

/// The most seats a split holds a chain for, as `oag_race` allows.
const MAX_SEATS: usize = 8;

impl Framebuffer {
    /// Resolves every view of `temporal` with its seat's chain and draws each into its share of
    /// `rect`. `false` when the chains cannot run (their shaders did not build, or no view
    /// produced an output), which leaves the frame to the ladder.
    pub(super) fn resolve_views(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        rect: (f32, f32, f32, f32),
        presentation: &Presentation,
        temporal: &Temporal<'_>,
    ) -> bool {
        let main = match self.fsr3.get_or_insert_with(|| fsr3::Fsr3::new(device)) {
            Ok(fsr) => fsr,
            Err(why) => {
                warn!("the FSR 3.1 pipelines did not build ({why:#}); staying bilinear");
                return false;
            }
        };
        let (extent, allocation) = (self.extent, self.allocation);
        let mut drawn = Vec::with_capacity(temporal.frames.len());
        for frame in &temporal.frames {
            let Some((seat, (x, y, w, h))) = frame.view else {
                continue;
            };
            let out = output_rect((x, y, w, h), extent, rect);
            let upscale = (out.2 as u32, out.3 as u32);
            if w == 0 || h == 0 || upscale.0 == 0 || upscale.1 == 0 {
                continue;
            }
            let seat = seat.min(MAX_SEATS - 1);
            if self.fsr3_views.len() <= seat {
                self.fsr3_views.resize_with(seat + 1, || None);
            }
            let chain = self.fsr3_views[seat].get_or_insert_with(|| main.sibling(device));
            // The view's share of the allocation, so a resolution controller moving the extent
            // moves the render size inside it rather than reallocating every step.
            let max_render = (
                (w as u64 * allocation.0 as u64).div_ceil(extent.0.max(1) as u64) as u32,
                (h as u64 * allocation.1 as u64).div_ceil(extent.1.max(1) as u64) as u32,
            );
            let _ = chain.render(
                device,
                queue,
                encoder,
                fsr3::Frame {
                    colour: &self.perceptual,
                    depth: temporal.depth,
                    velocity: temporal.velocity,
                    dispatch: fsr3::Dispatch {
                        render: (w, h),
                        input_offset: (x, y),
                        max_render: (max_render.0.max(w), max_render.1.max(h)),
                        upscale,
                        jitter: frame.jitter,
                        phase_count: frame.phase_count,
                        camera: frame.camera,
                        delta_time: 1.0 / 60.0,
                        reset: frame.reset,
                        sample_count: temporal.sample_count,
                        sharpness: fsr1::Sharpness::stops(presentation.sharpness),
                    },
                },
                None,
            );
            if let Some(view) = chain.output() {
                drawn.push((
                    bind(device, &self.layout, &self.sampler, &self.grade, view),
                    out,
                ));
            }
        }
        if drawn.is_empty() {
            return false;
        }
        self.set_blit(
            queue,
            Brightness::NEUTRAL,
            Gamma::NEUTRAL,
            self.format.is_srgb(),
            resolved_source(true, extent, allocation),
        );
        for (index, (group, out)) in drawn.iter().enumerate() {
            self.present_view(encoder, &self.output.view, *out, Some(group), index == 0);
        }
        true
    }

    /// How many split seats hold a chain of their own, for a test.
    #[cfg(test)]
    pub(super) fn split_chains(&self) -> usize {
        self.fsr3_views.iter().flatten().count()
    }
}

/// Where a view `(x, y, w, h)` of a scene drawn into `extent` lands in the presentation
/// rectangle `rect`: the same linear map the blit applies to the whole extent, each edge
/// rounded to a whole pixel.
pub(super) fn output_rect(
    (x, y, w, h): (u32, u32, u32, u32),
    extent: (u32, u32),
    rect: (f32, f32, f32, f32),
) -> (f32, f32, f32, f32) {
    let sx = rect.2 / extent.0.max(1) as f32;
    let sy = rect.3 / extent.1.max(1) as f32;
    let left = (rect.0 + x as f32 * sx).round();
    let top = (rect.1 + y as f32 * sy).round();
    let right = (rect.0 + (x + w) as f32 * sx).round();
    let bottom = (rect.1 + (y + h) as f32 * sy).round();
    (left, top, right - left, bottom - top)
}
