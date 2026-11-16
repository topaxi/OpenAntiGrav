//! [`Bloom::stamp_glow`]: the glow of rectangles that carry a mask of their own.
//!
//! Pulse PSP's speed bar, while a boost runs, and its shield bar, while an absorb runs, stamp the
//! glow mask under their fill (`oag_hud::boost_smear`): the original draws the HUD into the
//! framebuffer before the bright pass, so the bright pass reads the framebuffer under the
//! rectangle - the HUD at its own alpha over the scene showing through it - times that stamp, and
//! the ordinary blurs and composite make it glow. Ours draws the HUD after the scene's bright
//! pass, at presentation resolution, so the scene's chain cannot see it. This runs the same three
//! passes over the rectangles alone, in a lane of its own, reading the HUD's target for the
//! bright pass, and adds the result onto that target.
//!
//! Each rectangle carries its own mask byte, so a boost's speed bar and an absorb's shield bar
//! share one pass.
//!
//! The scene's own glow under the bar is not added to the bar's: the mask there is the scene's
//! baseline `4`, which the bright pass and the blurs truncate to nothing, and the stamp replaces
//! it in the original.

use super::*;

/// The most rectangles one call stamps: ten copies each of the speed bar and the shield bar.
pub const MAX: usize = 20;

/// One rectangle to stamp: where, and with which mask byte.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stamp {
    /// `x, y, width, height`, as fractions of the rectangle the frame was drawn into.
    pub rect: [f32; 4],
    /// The mask byte the stamp writes.
    pub value: u8,
}

impl Bloom {
    /// Blurs the glow of `stamps` and adds it onto `target` over the rectangle at `origin` of
    /// size `viewport`: the bright pass, two blurs and the composite, for rectangles alone. The
    /// bright pass reads `target` itself, so it must be sampleable and already hold the HUD.
    /// Nothing is drawn for no rectangle or only masks of zero.
    #[allow(clippy::too_many_arguments)]
    pub fn stamp_glow(
        &self,
        device: &wgpu::Device,
        queue: &dyn oag_gpu::writes::Writes,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        origin: (f32, f32),
        viewport: (u32, u32),
        stamps: &[Stamp],
    ) {
        let stamps = &stamps[..stamps.len().min(MAX)];
        if stamps.iter().all(|stamp| stamp.value == 0) {
            return;
        }
        let lane = &self.stamp_lane;
        let mut rects = [[0.0; 4]; MAX];
        let mut values = [[0.0; 4]; MAX];
        for (at, stamp) in stamps.iter().enumerate() {
            rects[at] = stamp.rect;
            values[at][0] = f32::from(stamp.value);
        }
        let wanted = Constants {
            texel: [1.0 / lane.extent.0 as f32, 1.0 / lane.extent.1 as f32],
            direction: [0.0, 0.0],
            strength: f32::from(COMPOSITE_STRENGTH) / 255.0,
            uv_offset: [0.0, 0.0],
            _pad: 0.0,
            uv_scale: [1.0, 1.0],
            uv_max: [1.0, 1.0],
            stamps: rects,
            stamp_view: [origin.0, origin.1, viewport.0 as f32, viewport.1 as f32],
            stamp_values: values,
            stamp_meta: [stamps.len() as f32, 0.0, 0.0, 0.0],
        };
        if lane.written.get() != Some(wanted) {
            queue.write_buffer(&lane.constants_bright, 0, bytemuck::bytes_of(&wanted));
            lane.written.set(Some(wanted));
        }
        // The bright pass reads the target; the composite that writes it is a later pass.
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bloom stamp"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(target),
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
                    label: &'static str,
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
            oag_gpu::perfprobe::marks::mark(&mut pass, label);
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
            oag_gpu::perfprobe::marks::close(&mut pass);
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
