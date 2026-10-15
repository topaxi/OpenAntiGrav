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
//!
//! **A split screen's glow is prepared once per view after the last view is drawn**
//! ([`Scene::prepare_split_bloom`], from `Scene::render_slots`) and composited here
//! too, view by view, over the HUD.

use super::*;

/// A split-screen view's glow that [`Scene::prepare_split_bloom`] prepared and
/// [`Scene::composite_bloom`] still owes.
#[derive(Clone, Copy, Debug)]
pub(super) struct ViewBloom {
    /// The view's index, which is its lane in the bloom.
    view: usize,
    /// The view's rectangle of the scene target.
    rect: (f32, f32, f32, f32),
    /// The area the views were cut from, so a composite onto a target of another
    /// size can map the rectangle across.
    area: (f32, f32, f32, f32),
}

/// A view's field of view as a share of the whole picture's, per axis, in half-angle
/// tangents: what `oag_post::bloom::lane_extent` scales the original's buffer by.
///
/// The cameras keep the authored vertical field at or above the authored aspect and
/// widen it below ([`oag_render::camera::fit_vertical_fov`]), so the tangent of the
/// horizontal half angle is `max(aspect, authored)` times the authored vertical one
/// and the vertical is `max(1, authored / aspect)` times it. Exactly `(1, 1)` for a
/// view that is the whole area.
fn field_scale(area: (f32, f32, f32, f32), rect: (f32, f32, f32, f32)) -> (f32, f32) {
    if rect.2 == area.2 && rect.3 == area.3 || area.3 <= 0.0 || rect.3 <= 0.0 {
        return (1.0, 1.0);
    }
    let (whole, view) = (area.2 / area.3, rect.2 / rect.3);
    let wide = |aspect: f32| aspect.max(AUTHORED_ASPECT);
    (
        wide(view) / wide(whole),
        wide(view) * whole / (view * wide(whole)),
    )
}

/// How far a glow reaches across and down in a split's views against a whole picture's,
/// as the mean over the views of the view's pixels per texel of a lane
/// ([`field_scale`]) against the whole area's: `(0.5, 0.5)` for the quadrants and the
/// halves of a two-way cut. The one number HD's whole-frame chain takes in place of a
/// lane per view (`oag_post::hd_bloom::Chain::set_reach`).
pub(super) fn mean_reach(area: (f32, f32, f32, f32), rects: &[(f32, f32, f32, f32)]) -> (f32, f32) {
    if rects.is_empty() || area.2 <= 0.0 || area.3 <= 0.0 {
        return (1.0, 1.0);
    }
    let (mut x, mut y) = (0.0, 0.0);
    for &rect in rects {
        let (scale_x, scale_y) = field_scale(area, rect);
        x += rect.2 / area.2 / scale_x;
        y += rect.3 / area.3 / scale_y;
    }
    (x / rects.len() as f32, y / rects.len() as f32)
}

impl Scene {
    /// Adds the glow [`Scene::render`] prepared onto `target`, over the rectangle
    /// `rect` (`x, y, width, height`) the frame was drawn into - or, for a split screen,
    /// each view's glow over its own share of `rect` (the area the views were cut from
    /// is mapped onto it, so a HUD target of another size still lines up).
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
        let views = std::mem::take(&mut *self.bloom_views.borrow_mut());
        if !self.bloom_pending.replace(false) {
            for pending in views {
                let (area, view) = (pending.area, pending.rect);
                if area.2 <= 0.0 || area.3 <= 0.0 {
                    continue;
                }
                let (sx, sy) = (rect.2 / area.2, rect.3 / area.3);
                let origin = (
                    rect.0 + (view.0 - area.0) * sx,
                    rect.1 + (view.1 - area.1) * sy,
                );
                let viewport = ((view.2 * sx) as u32, (view.3 * sy) as u32);
                let work = self.frame_work.get();
                self.frame_work.set(FrameWork {
                    bloom_views_composited: work.bloom_views_composited + 1,
                    ..work
                });
                if let Some(bloom) = &self.bloom {
                    bloom.composite_view(encoder, pending.view, target, origin, viewport);
                } else if let Some(bloom) = &self.ps2_bloom {
                    bloom.composite_view(encoder, pending.view, target, origin, viewport);
                }
            }
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

    /// Runs Pulse's bright pass and blurs over each of a split screen's views, every
    /// view into a lane of its own, reading the view's own rectangle of `view` - which
    /// holds every view, because this runs after the last one. Returns whether it
    /// encoded anything. A frame of one view never comes here; [`Scene::render`]
    /// prepares that.
    ///
    /// The composites wait for the HUD ([`Scene::composite_bloom`]). Each lane is sized
    /// by `oag_post::bloom::lane_extent` from the view's field of view against the
    /// whole area's, so a wide strip or a narrow column glows in proportion to what it
    /// shows rather than stretching the original's buffer. **Chosen, not measured.**
    pub(super) fn prepare_split_bloom(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        rects: &[(f32, f32, f32, f32)],
        area: (f32, f32, f32, f32),
    ) -> bool {
        let mut pending = self.bloom_views.borrow_mut();
        pending.clear();
        if self.bloom.is_none() && self.ps2_bloom.is_none() {
            return false;
        }
        let size = self.depth.size();
        for (index, &rect) in rects.iter().enumerate() {
            let frame = oag_post::bloom::Frame {
                scene: view,
                size: (size.width, size.height),
                origin: (rect.0, rect.1),
                viewport: (rect.2 as u32, rect.3 as u32),
            };
            let scale = field_scale(area, rect);
            let prepared = if let Some(bloom) = &self.bloom {
                let base = (oag_post::bloom::BLOOM_WIDTH, oag_post::bloom::BLOOM_HEIGHT);
                let extent = oag_post::bloom::lane_extent(base, scale);
                bloom.prepare_view(device, queue, encoder, index, extent, frame)
            } else if let Some(bloom) = &self.ps2_bloom {
                let base = (
                    oag_post::ps2_bloom::BLOOM_WIDTH,
                    oag_post::ps2_bloom::BLOOM_HEIGHT,
                );
                let extent = oag_post::bloom::lane_extent(base, scale);
                bloom.prepare_view(device, queue, encoder, index, extent, frame)
            } else {
                continue;
            };
            match prepared {
                Ok(()) => {
                    let work = self.frame_work.get();
                    self.frame_work.set(FrameWork {
                        bloom_views_prepared: work.bloom_views_prepared + 1,
                        ..work
                    });
                    pending.push(ViewBloom {
                        view: index,
                        rect,
                        area,
                    });
                }
                Err(e) => warn!("view {index}'s glow is not drawn ({e})"),
            }
        }
        !pending.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A view that is the whole area scales nothing; a wide strip of it shows a wider
    /// field and takes more texels across, at the same rows; a column of it shows the
    /// authored horizontal field over a taller vertical one, so the same columns and
    /// more rows. Quadrants keep the shape and so the buffer.
    #[test]
    fn a_view_scales_the_buffer_by_what_it_shows() {
        let area = (0.0, 0.0, 1280.0, 725.0);
        assert_eq!(field_scale(area, area), (1.0, 1.0));
        let quadrant = (0.0, 0.0, 639.0, 362.0);
        let (qx, qy) = field_scale(area, quadrant);
        assert!(
            (qx - 1.0).abs() < 0.01 && (qy - 1.0).abs() < 0.01,
            "{qx} {qy}"
        );
        let strip = (0.0, 0.0, 1280.0, 362.0);
        let (sx, sy) = field_scale(area, strip);
        assert!((sx - 1.0 * (1280.0 / 362.0) / AUTHORED_ASPECT.max(1280.0 / 725.0)).abs() < 1e-3);
        assert!(
            (sy - 1.0).abs() < 1e-3,
            "the strip keeps the vertical field: {sy}"
        );
        let column = (0.0, 0.0, 639.0, 725.0);
        let (cx, cy) = field_scale(area, column);
        assert!(
            (cx - 1.0).abs() < 1e-3,
            "the column keeps the horizontal field: {cx}"
        );
        assert!(cy > 1.9 && cy < 2.1, "and opens the vertical one: {cy}");
    }

    /// A two-way cut's views and four quadrants each take half the pixels per texel of
    /// the whole picture, across and down, so HD's chain reaches half as far.
    #[test]
    fn every_cut_of_a_split_reaches_half_as_far() {
        let area = (0.0, 0.0, 1280.0, 725.0);
        for rects in [
            vec![(0.0, 0.0, 1280.0, 362.0), (0.0, 363.0, 1280.0, 362.0)],
            vec![(0.0, 0.0, 639.0, 725.0), (641.0, 0.0, 639.0, 725.0)],
            vec![
                (0.0, 0.0, 639.0, 362.0),
                (641.0, 0.0, 639.0, 362.0),
                (0.0, 363.0, 639.0, 362.0),
                (641.0, 363.0, 639.0, 362.0),
            ],
        ] {
            let (x, y) = mean_reach(area, &rects);
            assert!((x - 0.5).abs() < 0.02 && (y - 0.5).abs() < 0.02, "{x} {y}");
        }
    }
}
