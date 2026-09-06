//! The `<Mode3D>` countdown overlay: `Data\HUD\Cockpit_321GO.vex`.
//!
//! Not the sight brackets' and Pure's pickup icons' path
//! ([`super::draw::model_draw`]) - those flatten a model into one static
//! quad, baked once. This asset cannot take that shortcut: its four glyph
//! nodes (`Jons321go:x3`, `x2`, `x1`, `GO`) each carry their own
//! [`oag_formats::vex::CLASS_ANIM_TRANSFORM`] scale-burst track and their own
//! per-material `TEXOFFSET` alpha sweep - real animation, not a fixed
//! picture - so it is drawn through the same mesh pipeline the race scene
//! uses, with its own small orthographic pass rather than a baked sprite.
//! See `handover/race-start-countdown-and-launch-boost.md` and
//! `docs/rendering/start-gantry.md` for the sibling asset (the track-side
//! gantry) this is not - that one remains unplaced because slot 8's world
//! transform is unrecovered; this one is a HUD widget with an authored
//! screen position and does not have that problem.
//!
//! # What is drawn, and what is not
//!
//! Only `Cockpit_321GO.vex`. `Pulse_Ready_Go.vex`, the widget's other model,
//! loads the same way and is not drawn: its own material track repeats every
//! 0.983 s with no recovered gate, so playing it against the race clock would
//! blink it roughly 4.6 times through a 272-tick countdown - a duration this
//! project has not measured, not one it is free to invent. See
//! `docs/rendering/start-gantry.md`'s timing section.
//!
//! # Where the screen position comes from
//!
//! `model_draw`'s own convention - reused here rather than reinvented -
//! reads a `<Mode3D><Model>`'s authored `x`/`y` as top-left HUD pixels
//! (480x272, [`crate::frontend::SCREEN`]) verbatim; that is what Pure's
//! pickup icons ship and what `hud_layout_ground_truth.rs` checks. Both
//! Pulse's `Cockpit321Go` and `ReadyGo` widgets author `x="0.0" y="0.0"
//! z="-70.0"`, so [`Countdown::new`] takes the widget's own position rather
//! than hardcoding it.
//!
//! **The enclosing `<Mode3D><Values OriginX="0.0" OriginY="35.0">` is not
//! parsed anywhere in this tree** - `docs/ui/hud.md` lists `OriginX/Y` under
//! "attributes not acted on", deferred along with `ztest`, `Delay` and
//! `mode="orthographic"` itself. [`ORIGIN_Y`] below is this file's own
//! reading of that one number, applied as a plain HUD-pixel offset added to
//! the widget's `y` - the literal meaning of "origin" for a screen-space
//! anchor, and the simplest reading consistent with `model_draw`'s existing
//! convention, but **not itself parsed generically or cross-checked against
//! a render of the original**, so treat the resulting screen position as this
//! file's own placement rather than a second disc-measured fact alongside the
//! widget's `x`/`y`.
//!
//! `z="-70.0"` is not used: the projection below is orthographic over HUD
//! pixel space, which by construction discards view-space depth. Recorded so
//! a reader does not go looking for where it went.
//!
//! # `x="0.0"` most likely means "runtime-placed", the same as the sights
//!
//! **Now settled, with a negative result, rather than merely unsearched.**
//! Rendered at its literal authored position, the widget sits mostly off the
//! left edge of a 480x272 frame - `model_draw`'s own convention is
//! trustworthy (Pure's pickup icons author `x="240" y="250"`, dead centre and
//! near the bottom, exactly where an icon belongs), so an `x` of exactly
//! `0.0` on a widget whose siblings all carry real coordinates reads the same
//! way [`super::Model`]'s own doc already reads the nine sight brackets: "all
//! at the same placeholder position, which the runtime overwrites every
//! frame". Two independent checks, written up in full in
//! `docs/ghidra/functions/psp-pulse-usa/countdown-widgets.md`, say that
//! reading does not hold here the way it does for the sights:
//!
//! - **No runtime position writer exists.** `Hud_BindWidgets` resolves this
//!   widget the same way `HudSight_Bind` resolves the sights, but the only
//!   other function touching its HUD-object slot (`Hud_UpdateCountdownFade`)
//!   drives a fade timer and a visibility bit, never a screen position - no
//!   `HudSight_Update` counterpart exists for it.
//! - **The mesh bakes no absolute placement either.** `Cockpit_321GO.vex`'s
//!   four glyph nodes resolve to a translation of `(≈0, ≈0)` and vertex
//!   bounds symmetric about their own local origin, the same "centred quad,
//!   externally placed" convention the sight models use - so the geometry
//!   does not supply the missing coordinate the way it would if the widget's
//!   `x`/`y` were merely a redundant zero on top of an already-centred mesh.
//!
//! So this file draws the literal `(0, 35)` rather than a guessed centre - a
//! clipped sliver is the honest picture of what the disc says today, and
//! nothing found says otherwise. Not a bug being papered over: there is
//! currently nowhere else on this title's own binary or asset for a real
//! coordinate to come from.

use oag_core::math::{Mat4, Vec3, camera};
use oag_render::mesh::Model;
use oag_render::mesh_render::{
    Anisotropy, Built, DEPTH_FORMAT, Depth, GlowMask, NodeAnims, ShadowReceiver, TRANSPARENT_BLEND,
    TexAnims, TransparentPipelines, UNIFORMS_SIZE, Velocity, build, write_uniforms_raw,
};

use crate::frontend::SCREEN;

/// This file's own reading of the `<Mode3D>` block's `OriginY` - see the
/// module doc's "Where the screen position comes from" section for why this
/// is a placement choice and not a parsed value.
const ORIGIN_Y: f32 = 35.0;

/// The GPU half of the countdown overlay: one model, one small pass with its
/// own depth buffer, drawn after the HUD's ordinary sprite pass.
pub struct Countdown {
    model: Model,
    built: Built,
    uniform_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    /// Screen position in HUD pixels - the widget's own `x`/`y` plus
    /// [`ORIGIN_Y`]. See the module doc.
    position: [f32; 2],
    /// This pass's own depth attachment, sized to the last viewport drawn -
    /// recreated on resize rather than shared with the scene's, since this
    /// draws well after the scene has resolved and composited.
    depth: Option<(wgpu::TextureView, u32, u32)>,
}

impl std::fmt::Debug for Countdown {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Countdown")
            .field("triangles", &(self.model.indices.len() / 3))
            .field("position", &self.position)
            .finish()
    }
}

impl Countdown {
    /// Builds the pipelines for an already-loaded `Cockpit_321GO.vex`, or
    /// `None` when the widget it is placed by is not in this mode's layout -
    /// `Zone_HUD.xml` and the rest all carry it, but a caller with no layout
    /// at all (the disc's own file unreadable) has nowhere to place it either.
    ///
    /// # Errors
    ///
    /// Propagates pipeline creation, the same as [`super::Overlay::new`].
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        model: Model,
        widget: &super::Model,
    ) -> anyhow::Result<Self> {
        let built = build(
            device,
            queue,
            &model,
            format,
            Anisotropy::default(),
            1,
            Depth::Scene,
            TRANSPARENT_BLEND,
            GlowMask::Protected,
            Velocity::None,
            None,
            None,
            None,
            ShadowReceiver::Never,
        )?;

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("countdown uniforms"),
            size: UNIFORMS_SIZE,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group_layout = built.pipeline.get_bind_group_layout(0);
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("countdown uniforms"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let [x, y, _z] = widget.position;
        Ok(Self {
            model,
            built,
            uniform_buffer,
            bind_group,
            position: [x, y + ORIGIN_Y],
            depth: None,
        })
    }

    /// (Re)builds the depth attachment if the viewport has changed size since
    /// the last frame - the presentation surface resizes with the window, and
    /// this pass's own depth buffer has to track it.
    fn depth_view(&mut self, device: &wgpu::Device, width: u32, height: u32) -> &wgpu::TextureView {
        let stale = !matches!(&self.depth, Some((_, w, h)) if *w == width && *h == height);
        if stale {
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("countdown depth"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: DEPTH_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });
            self.depth = Some((
                texture.create_view(&wgpu::TextureViewDescriptor::default()),
                width,
                height,
            ));
        }
        &self.depth.as_ref().expect("just set above").0
    }

    /// Draws one frame at `seconds` into the authored `Anim Transform` and
    /// `TEXOFFSET` tracks - `race_ticks as f32 / 60.0`, the same clock
    /// [`oag_race::RaceState::thrust_gated`] gates on, so a caller shows this
    /// for exactly the measured countdown span and no other window. Composites
    /// over whatever `view` already holds (`LoadOp::Load`), the same
    /// convention [`super::Overlay::draw`] uses.
    ///
    /// `viewport` is `(x, y, width, height)` of the presentation surface's
    /// aspect rectangle, the same one every other HUD pass restricts itself
    /// to - see `crate::render::Renderer::overlay`'s own `set_viewport`. Not
    /// the same thing as `target_size`, `view`'s own full pixel dimensions:
    /// a windowed run pillarboxes the aspect rectangle inside a wider
    /// surface, and this pass's own depth attachment has to match `view`'s
    /// size rather than the (possibly smaller) rectangle drawn into, or the
    /// two attachments disagree and the pass is invalid.
    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        seconds: f32,
        viewport: (f32, f32, f32, f32),
        target_size: (u32, u32),
    ) {
        if self.model.vertices.is_empty() || self.model.indices.is_empty() {
            return;
        }

        // Orthographic over the layout's own 480x272 grid - `model_draw`'s
        // "one model unit is one HUD pixel" convention, expressed as a
        // matrix instead of a per-widget scale. Near/far are unmeasured and
        // generous: nothing recovered says what depth range the original's
        // `Mode3D` pass uses, and this asset's own geometry plus its 8x
        // scale burst stays well inside +-200 of its authored `z=-70`.
        //
        // **Y is remapped here, not mirrored in the projection, and that
        // choice is load-bearing.** `camera::orthographic`'s `bottom`/`top`
        // both flip an axis *and* the handedness the rasteriser sees, so a
        // `bottom=272, top=0` call to get "down is positive" the cheap way
        // silently reverses every triangle's winding - every one of this
        // model's eight batches authors `culled=true`, so `Face::Back`
        // culling then discards all of it and the pass draws nothing,
        // exactly the failure this took a debug session to find. Converting
        // the HUD-pixel `y` to a bottom-up one before a plain,
        // unmirrored `orthographic(0, 480, 0, 272, ...)` keeps the winding
        // the model was authored with, so culling and the glyphs' own
        // right-way-up both come out correct. Don't "simplify" this back to
        // a mirrored projection.
        let view_projection = camera::orthographic(0.0, SCREEN.0, 0.0, SCREEN.1, 1.0, 2000.0);
        let model_matrix = Mat4::from_translation(Vec3::new(
            self.position[0],
            SCREEN.1 - self.position[1],
            -70.0,
        ));
        write_uniforms_raw(queue, &self.uniform_buffer, view_projection, model_matrix);

        queue.write_buffer(
            &self.built.anim_buffer,
            0,
            bytemuck::bytes_of(&TexAnims::sample(&self.model, seconds)),
        );
        queue.write_buffer(
            &self.built.node_anim_buffer,
            0,
            bytemuck::bytes_of(&NodeAnims::sample(&self.model, seconds)),
        );

        let (width, height) = (target_size.0.max(1), target_size.1.max(1));
        let depth_view = self.depth_view(device, width, height).clone();

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("countdown"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Discard,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });

        // The same aspect rectangle every other HUD pass restricts itself
        // to (`crate::render::Renderer::overlay`), so a pillarboxed window
        // draws the countdown inside the same bars rather than stretched
        // across the whole surface.
        pass.set_viewport(viewport.0, viewport.1, viewport.2, viewport.3, 0.0, 1.0);

        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.set_bind_group(2, &self.built.fog_bind, &[]);
        pass.set_bind_group(3, &self.built.anim_bind, &[]);
        pass.set_vertex_buffer(0, self.built.vertex_buffer.slice(..));
        pass.set_index_buffer(self.built.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

        let bind = |draw: &oag_render::mesh::DrawCall| {
            let slot = draw.texture.map_or(0, |t| t + 1);
            &self.built.texture_binds[if slot < self.built.texture_binds.len() {
                slot
            } else {
                0
            }]
        };

        // Every draw on this model is transparent (`--draws`: 0 opaque, 0
        // cutout, 8 blend) - both textures are pure-white alpha ramps, so an
        // opaque pass would have nothing to paint anyway. Still walking all
        // three lists, in the same order `capture.rs` does, so a future
        // asset that *does* author an opaque batch draws correctly rather
        // than silently through the wrong pipeline.
        pass.set_pipeline(&self.built.pipeline);
        for draw in &self.model.draws {
            pass.set_bind_group(1, bind(draw), &[]);
            pass.draw_indexed(draw.range.clone(), 0, 0..1);
        }
        pass.set_pipeline(&self.built.alpha_test_pipeline);
        for draw in &self.model.alpha_tested_draws {
            pass.set_bind_group(1, bind(draw), &[]);
            pass.draw_indexed(draw.range.clone(), 0, 0..1);
        }
        let pipelines = TransparentPipelines {
            alpha_over: &self.built.blend_pipeline,
            additive: &self.built.additive_pipeline,
            unblended: &self.built.unblended_pipeline,
            authored: &self.built.authored_pipelines,
        };
        let mut current: Option<&wgpu::RenderPipeline> = None;
        for draw in &self.model.transparent_draws {
            let pipeline = pipelines.select(draw);
            if !current.is_some_and(|set| std::ptr::eq(set, pipeline)) {
                pass.set_pipeline(pipeline);
                current = Some(pipeline);
            }
            pass.set_bind_group(1, bind(draw), &[]);
            pass.draw_indexed(draw.range.clone(), 0, 0..1);
        }
    }
}
