//! The ring on a frame of its own, for a title with no HD bloom chain.
//!
//! HD's pass lives inside `hd_bloom::Chain`: its history is the chain's
//! quarter-resolution downsample of the linear float scene, and its ring is drawn
//! before the exposure resolve. Pulse and Pure run no such chain, so a ring
//! **borrowed** from HD (the player's `graphics.boost_zoom`) runs here instead, on
//! the finished frame in whatever format and colour space the title's own post
//! path leaves it in.
//!
//! **Chosen, not measured**, all of it:
//!
//! - The history is the finished frame read through a bilinear 4x decimation into
//!   a quarter-size buffer, not HD's chain of two box halvings.
//! - The history blends and the ring composites in the frame's own space (sRGB on
//!   a UNORM-sRGB target), where HD's blends in linear float.
//! - The ring writes colour only, so a glow mask stamped in the frame's alpha for
//!   a bloom composite later is left as it was.
//! - The history runs every tick the race steps, pulse or not, as HD's does, so a
//!   pulse starts against a current history rather than a stale one.

use std::cell::RefCell;

use super::{Frame, SourceMap, Tuning, Zoom};

/// The borrowed ring's pass, built lazily at the first frame's size and rebuilt
/// when the drawn size changes.
#[derive(Debug)]
pub struct Host {
    tuning: Tuning,
    format: wgpu::TextureFormat,
    built: RefCell<Option<((u32, u32), Zoom)>>,
}

impl Host {
    /// A host for frames of `format`.
    #[must_use]
    pub fn new(tuning: Tuning, format: wgpu::TextureFormat) -> Self {
        Self {
            tuning,
            format,
            built: RefCell::new(None),
        }
    }

    /// Encodes this tick's history draw and, while a pulse runs, the ring over
    /// `view`.
    ///
    /// `target` is the whole texture behind `view`, `origin` and `viewport` the
    /// rectangle of it the scene was drawn into (a split screen, or a dynamic
    /// resolution scale, draws less than the whole).
    #[allow(clippy::too_many_arguments)]
    pub fn run(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        target: (u32, u32),
        origin: (u32, u32),
        viewport: (u32, u32),
        frame: &Frame,
    ) {
        if viewport.0 == 0 || viewport.1 == 0 || target.0 == 0 || target.1 == 0 {
            return;
        }
        let quarter = ((viewport.0 / 4).max(1), (viewport.1 / 4).max(1));
        let mut built = self.built.borrow_mut();
        if built.as_ref().is_none_or(|(size, _)| *size != quarter) {
            match Zoom::standalone(device, self.tuning, quarter, self.format) {
                Ok(zoom) => *built = Some((quarter, zoom)),
                Err(_) => return,
            }
        }
        let Some((_, zoom)) = built.as_ref() else {
            return;
        };
        let wide = target.0 as f32;
        let high = target.1 as f32;
        let source = SourceMap {
            scale: [viewport.0 as f32 / wide, viewport.1 as f32 / high],
            max: [
                (viewport.0 as f32 - 0.5) / wide,
                (viewport.1 as f32 - 0.5) / high,
            ],
            offset: [origin.0 as f32 / wide, origin.1 as f32 / high],
        };
        zoom.history_from(device, queue, encoder, frame, view, source);
        zoom.ring_at(
            queue,
            encoder,
            view,
            frame,
            origin,
            viewport,
            ([1.0; 2], [1.0; 2]),
        );
    }
}

#[cfg(test)]
mod tests;
