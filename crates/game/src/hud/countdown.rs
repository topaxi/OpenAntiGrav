//! The `<Mode3D>` countdown overlay: `Data\HUD\Cockpit_321GO.vex`.
//!
//! Not the sight brackets' and Pure's pickup icons' path
//! ([`super::draw::model_draw`]) - those flatten a model into one static
//! quad, baked once. This asset cannot take that shortcut: its four glyph
//! nodes (`Jons321go:x3`, `x2`, `x1`, `GO`) each carry their own
//! [`oag_vex::vex::CLASS_ANIM_TRANSFORM`] scale-burst track and their own
//! per-material `TEXOFFSET` alpha sweep - real animation, not a fixed
//! picture - so it is drawn through the same mesh pipeline the race scene
//! uses, with its own small orthographic pass rather than a baked sprite.
//! See `docs/rendering/start-gantry.md` for the sibling asset (the track-side
//! gantry) this is not.
//!
//! # This overlay is now the *fallback*, not the countdown
//!
//! **The gantry is placed and drawn** - `oag_render::gantry` measures the
//! mounting surface each circuit's own track model authors and
//! `crate::race::gantry` stands `321Go_StartFinish.vex` on it, so twelve of
//! Pulse's circuits show the `3`, `2`, `1`, `GO` on the object over the start
//! line, which is where the project owner's own account of the original puts
//! it. So this widget is drawn **only where no gantry is** -
//! `race::Scene::draws_gantry` gates both of its call sites
//! (`main::race_stage` and `race::capture`) - rather than as a second
//! countdown beside one. It is not deleted: a circuit that authors no mount,
//! and every title whose gantry has not been placed, still needs it.
//!
//! # What is drawn, and what is not
//!
//! Only `Cockpit_321GO.vex`. `Pulse_Ready_Go.vex`, the widget's other model,
//! loads the same way and is not drawn - but not for the reason this comment
//! used to give. **Rendered standalone off the disc, `Pulse_Ready_Go.vex`'s
//! one mesh (`thingieShape`) is a nested ring/swoosh shape, not digits** -
//! confidence 90, the same class of evidence as this project's rendered
//! `321go_zone.vex` comparison. So the old worry - that its 0.983 s material
//! loop would "blink 4.6 times through a 272-tick countdown" - was aimed at
//! the wrong problem: there are no digit states for a loop to misalign
//! against, and the loop's own short authored period (unlike
//! `Cockpit_321GO.vex`'s four tracks, each a one-shot burst dressed in an
//! oversized 600-6000 s nominal loop that never actually wraps in a race) is
//! consistent with an intentionally-repeating ambient shimmer.
//!
//! **What actually blocks it: the widget's entire visible envelope is
//! code-supplied, and its trigger is unrecovered.** `Hud_UpdateCountdownFade`
//! (`0x0881f624`) eases this widget's alpha through `(1 - (2t)^3) * 255`,
//! clamped, then tints it `(alpha << 24) | 0x050517` - confirmed by direct
//! decompile, confidence 85. `Cockpit_321GO.vex` gets a flat `0xffffffff`
//! from the same function, so the fade contributes nothing to it and drawing
//! it unconditionally for the measured countdown span (below) reproduces the
//! disc faithfully. `Pulse_Ready_Go.vex` has no such free pass: `t` comes
//! from a timer gated on a byte pair at `hud->view + 0x58`/`+0x59` (an
//! already-known but still largely unidentified struct, see
//! [lock-sight.md](../../../../docs/ghidra/functions/psp-pulse-usa/lock-sight.md)),
//! and no writer of that byte was found - one candidate (a global at
//! `0x08ab2120`, cleared on ship death) was checked and probably ruled out on
//! a struct-size mismatch against `hud->view`'s own known `+0xe4` field, but
//! at confidence 60, not a confirmed dead end. Drawing this model without its
//! envelope would show a near-opaque shape for the model's entire visible
//! window instead of the original's brief eased flash - exactly the kind of
//! plausible-looking stand-in `CLAUDE.md` asks not to ship. See
//! `docs/ghidra/functions/psp-pulse-usa/countdown-widgets.md` and
//! `docs/rendering/start-gantry.md`'s timing section.
//!
//! # Where the screen position comes from: a second `<Mode3D>` dialect, not `model_draw`'s
//!
//! **This widget's `<Mode3D>` block is not the one `model_draw`'s convention was
//! measured against, and treating it as one was the actual bug.** Two dialects
//! ship on both Pulse and Pure, distinguished by one attribute on the block's own
//! `<Values>` carrier:
//!
//! | | sights, Pure's icons/bars | `ReadyGo`/`Cockpit321Go` |
//! | --- | --- | --- |
//! | `<Values>` carries | `mode="orthographic"` | `FirstPass="yes" OriginX="0.0" OriginY="35.0"` |
//! | `x`/`y` on every checked layout | real screen pixels (`240,250`; `-240,136`; `394,20`) | always `0.0, 0.0` |
//! | `z` | `0`, `-1` or `-10` - never used for depth | `-70.0` |
//!
//! `docs/ui/hud.md`'s "attributes not acted on" list used to lump
//! `mode="orthographic"` and `OriginX/Y` together as one deferred `<Mode3D>`
//! feature; they are opposite ends of the same fork instead. Every layout on
//! both PSP titles checked (`Arcade`, `TimeTrial`, `Elimination`, `Zone` -
//! `oag-wad cat --expand` against `Data.wad`) agrees: the ground-truth-tested
//! block always says `mode="orthographic"` and this one never does. `x=0, y=0`
//! is not this dialect's placeholder-for-runtime-code value the way it would be
//! read under `model_draw`'s convention (see [`super::Model::orthographic`]) -
//! it is the one screen position a perspective camera puts there **regardless
//! of the camera's own FOV, near or far plane**: for a symmetric frustum along
//! `-z`, a point at `x=0, y=0` projects to the exact centre of the viewport for
//! any `z != 0`, because the horizontal and vertical projection terms are both
//! `(coordinate / -z) * scale`, and `0 / -z` is `0` no matter what `scale` is.
//! `z="-70.0"` is the tell this project didn't need to invent a camera to read:
//! a widget authored at real depth, in the one `<Mode3D>` dialect where depth
//! is not vestigial.
//!
//! **This resolves without needing the actual camera the original code used.**
//! No projection matrix is built below - constructing one from a guessed FOV
//! would still contribute nothing to a widget authored at `(0, 0)` (the
//! `0 / -z` term above is `0` for every choice), and would silently misplace a
//! future non-orthographic widget authored at a nonzero `x`/`y` under a wrong
//! guess. [`Countdown::new`] instead takes the screen centre directly - the one
//! answer that holds under every symmetric projection - and adds
//! [`super::Model::origin`], the block's own `OriginX`/`OriginY`, which is now
//! read the same way [`super::Layout::collect`] reads it for every widget in
//! this dialect rather than as this file's own private constant. `(0, 35)`
//! read as a screen pixel put the widget in the top-left corner; `(240, 136) +
//! (0, 35) = (240, 171)` puts it centred horizontally and a third of the way up
//! from the bottom - a sensible HUD countdown position, not a coincidence
//! dressed up to look like one.
//!
//! A `<Model>` in this dialect authored at a **nonzero** `x`/`y` would need the
//! real perspective transform recovered before it could be placed - the `0/-z`
//! argument above only eliminates the unknown for the zero case every layout
//! actually ships. [`Countdown::new`] errors rather than guesses if that ever
//! changes; see [`Countdown::screen_position`]'s own doc.
//!
//! **Position at `(0, 0)` is FOV-independent; apparent size is not, and only
//! the first is derived above.** [`Countdown::draw`]'s own orthographic pass
//! still treats one model unit as one HUD pixel - `model_draw`'s convention,
//! reused for scale rather than reinvented - instead of foreshortening the
//! mesh by `z="-70.0"` through a real perspective camera, which would need
//! the original's actual FOV the same way a nonzero position would. The
//! glyph's on-screen size comes entirely from `Cockpit_321GO.vex`'s own mesh
//! extents and its authored `Anim Transform` scale-burst, not from a
//! projection - recorded so fixing the position is not mistaken for having
//! also derived the scale.

use oag_core::math::{Mat4, Vec3, camera};
use oag_render::mesh::Model;
use oag_render::mesh_render::{
    Anisotropy, Built, DEPTH_FORMAT, Depth, GlowMask, NodeAnims, ShadowReceiver, TRANSPARENT_BLEND,
    TexAnims, TransparentPipelines, UNIFORMS_SIZE, Velocity, build, write_uniforms_raw,
};

use oag_display::space::SCREEN;

/// The GPU half of the countdown overlay: one model, one small pass with its
/// own depth buffer, drawn after the HUD's ordinary sprite pass.
pub struct Countdown {
    model: Model,
    built: Built,
    uniform_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    /// Screen position in HUD pixels. See [`Countdown::new`] for how this is
    /// derived and the module doc for why.
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
    /// Propagates pipeline creation, the same as [`super::Overlay::new`], and
    /// [`Self::screen_position`]'s own error for a widget this file cannot
    /// yet place.
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

        Ok(Self {
            model,
            built,
            uniform_buffer,
            bind_group,
            position: Self::screen_position(widget)?,
            depth: None,
        })
    }

    /// The widget's screen position, from its own `<Mode3D>` dialect.
    ///
    /// See the module doc for the derivation. Two branches, keyed on
    /// [`super::Model::orthographic`]:
    ///
    /// - **Orthographic** (never true for this widget on any layout read, but
    ///   handled rather than assumed away - the same defensiveness
    ///   [`super::draw::model_draw`] gets for its own consumers): `x`/`y` are
    ///   literal HUD pixels, `model_draw`'s own convention.
    /// - **Not orthographic** (every `Cockpit321Go`/`ReadyGo` on both PSP
    ///   titles): only `x=0, y=0` is derivable without the real perspective
    ///   camera this project has not recovered - see the module doc for why
    ///   that is not a gap for *this* value. The screen centre plus the
    ///   block's own [`super::Model::origin`] is the answer for that case;
    ///   a nonzero `x`/`y` here is a widget this file cannot yet place, and
    ///   errors rather than guesses one - the same "draw nothing and say so"
    ///   rule this project's `CLAUDE.md` states for an asset it cannot play.
    fn screen_position(widget: &super::Model) -> anyhow::Result<[f32; 2]> {
        let [x, y, _z] = widget.position;
        if widget.orthographic {
            return Ok([x, y]);
        }
        anyhow::ensure!(
            x == 0.0 && y == 0.0,
            "countdown widget {:?} authors a nonzero position ({x}, {y}) in the \
             non-orthographic <Mode3D> dialect - only (0, 0) is derivable \
             without the real perspective camera this project has not \
             recovered, see crate::hud::countdown's module doc",
            widget.name,
        );
        let [origin_x, origin_y] = widget.origin;
        Ok([SCREEN.0 * 0.5 + origin_x, SCREEN.1 * 0.5 + origin_y])
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

#[cfg(test)]
mod tests;
