//! [`Bloom::stamp_glow`]: the glow of rectangles that carry a mask of their own.
//!
//! Pulse PSP's speed bar, while a boost runs, stamps the glow mask under its fill
//! (`oag_hud::boost_smear`): the original draws the HUD into the framebuffer before the bright
//! pass, so the bright pass reads the bar's colour times that stamp and the ordinary blurs and
//! composite make it glow. Ours draws the HUD after the scene's bright pass, at presentation
//! resolution, so the scene's chain cannot see it. This runs the same three passes over the
//! rectangles alone, in a lane of its own, and adds the result onto the HUD's target.
//!
//! The scene's own glow under the bar is not added to the bar's: the mask there is the scene's
//! baseline `4`, which the bright pass and the blurs truncate to nothing, and the stamp replaces
//! it in the original. What is missed is the scene's colour showing through a bar whose copies
//! cover under all of a pixel (`1 - (1 - a)^k`): `rgb` here is the bar's colour alone.

use super::*;

/// The most rectangles one stamp covers: the boost's ten copies.
pub const MAX: usize = 10;

/// What to stamp: up to [`MAX`] overlapping rectangles of one colour and one mask byte.
#[derive(Debug, Clone, Copy)]
pub struct Stamps<'a> {
    /// `x, y, width, height` of each, as fractions of the rectangle the frame was drawn into.
    pub rects: &'a [[f32; 4]],
    /// The colour the widget is drawn in, `rgb` in `0..=1`.
    pub colour: [f32; 3],
    /// Each copy's alpha, `0..=1`: a pixel under `k` copies shows the colour at `1 - (1 - a)^k`.
    pub opacity: f32,
    /// The mask byte the stamp writes.
    pub value: u8,
}

impl Bloom {
    /// Blurs the glow of `stamps` and adds it onto `target` over the rectangle at `origin` of
    /// size `viewport`: the bright pass, two blurs and the composite, for rectangles alone.
    /// Nothing is drawn for no rectangle or a mask of zero.
    #[allow(clippy::too_many_arguments)]
    pub fn stamp_glow(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        origin: (f32, f32),
        viewport: (u32, u32),
        stamps: &Stamps<'_>,
    ) {
        let count = stamps.rects.len().min(MAX);
        if count == 0 || stamps.value == 0 {
            return;
        }
        let lane = &self.stamp_lane;
        let mut rects = [[0.0; 4]; MAX];
        rects[..count].copy_from_slice(&stamps.rects[..count]);
        let wanted = Constants {
            texel: [1.0 / lane.extent.0 as f32, 1.0 / lane.extent.1 as f32],
            direction: [0.0, 0.0],
            strength: f32::from(COMPOSITE_STRENGTH) / 255.0,
            uv_offset: [0.0, 0.0],
            _pad: 0.0,
            uv_scale: [1.0, 1.0],
            uv_max: [1.0, 1.0],
            stamps: rects,
            stamp_colour: [stamps.colour[0], stamps.colour[1], stamps.colour[2], 0.0],
            stamp_meta: [count as f32, f32::from(stamps.value), stamps.opacity, 0.0],
        };
        if lane.written.get() != Some(wanted) {
            queue.write_buffer(&lane.constants_bright, 0, bytemuck::bytes_of(&wanted));
            lane.written.set(Some(wanted));
        }
        // The stamp pass reads no texture, but the layout has one: B, which it does not write.
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bloom stamp"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&lane.b.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: lane.constants_bright.as_entire_binding(),
                },
            ],
        });
        let pass = |encoder: &mut wgpu::CommandEncoder,
                    label: &str,
                    pipeline: &wgpu::RenderPipeline,
                    group: &wgpu::BindGroup,
                    to: &wgpu::TextureView| {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(label),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: to,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(pipeline);
            pass.set_viewport(
                0.0,
                0.0,
                lane.extent.0.max(1) as f32,
                lane.extent.1.max(1) as f32,
                0.0,
                1.0,
            );
            pass.set_bind_group(0, group, &[]);
            pass.draw(0..3, 0..1);
        };
        pass(encoder, "bloom stamp", &self.stamp, &group, &lane.a.view);
        pass(
            encoder,
            "bloom stamp blur x",
            &self.blur,
            &lane.blur_x,
            &lane.b.view,
        );
        pass(
            encoder,
            "bloom stamp blur y",
            &self.blur,
            &lane.blur_y,
            &lane.a.view,
        );
        self.composite_in(lane, encoder, target, origin, viewport);
    }
}

#[cfg(test)]
mod tests;
