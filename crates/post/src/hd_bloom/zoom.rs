//! The chain's seam to [`crate::hd_zoom`]: build the pass, take this frame's
//! pulses, and run its two draws at the points `FunkLayer_RunBloomChain` does -
//! the history right after the quarter-resolution downsample, the ring after the
//! blurs and before the resolve.

use anyhow::Result;

use super::{Chain, Params, Sized, level_viewport};
use crate::hd_zoom::{Frame, Zoom};

impl Chain {
    /// The zoom pass for these params and this sizing, or `None` for a title
    /// with no ring.
    pub(super) fn zoom_for(
        device: &wgpu::Device,
        params: &Params,
        sized: &Sized,
    ) -> Result<Option<Zoom>> {
        params
            .zoom
            .map(|tuning| Zoom::new(device, tuning, &sized.downsamples[1].target, sized.quarter))
            .transpose()
    }

    /// This frame's pulses, read from the race. `None`, or a chain built with no
    /// ring, draws nothing of it. Set before [`Chain::run`], every frame: a frame
    /// that does not set it keeps the last one's, which is why the caller clears
    /// it when it has none.
    pub fn set_zoom_frame(&self, frame: Option<Frame>) {
        self.zoom_frame.set(frame);
    }

    pub(super) fn zoom_history(
        &self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        viewport: (u32, u32),
    ) {
        let (Some(zoom), Some(frame)) = (&self.zoom, self.zoom_frame.get()) else {
            return;
        };
        let scene = self.sized.scene_size;
        let rect = level_viewport(self.sized.quarter, viewport, scene);
        zoom.history(
            queue,
            encoder,
            &frame,
            rect,
            crate::sub_rectangle(viewport, scene),
        );
    }

    pub(super) fn zoom_ring(
        &self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        viewport: (u32, u32),
    ) {
        let (Some(zoom), Some(frame)) = (&self.zoom, self.zoom_frame.get()) else {
            return;
        };
        let scene = self.sized.scene_size;
        let mapping = crate::sub_rectangle(viewport, scene);
        zoom.ring(
            queue,
            encoder,
            &self.sized.scene.view,
            &frame,
            viewport,
            mapping,
        );
    }
}
