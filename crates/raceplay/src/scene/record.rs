//! A view's scene pass, split from what decides it: [`Scene::prepare_view`] (in `frame.rs`) makes
//! every write and every choice a view needs - culling inputs, which craft, weapons and effects
//! are live, the effect pipelines' counts - and hands back a [`ViewPass`]; a [`Recorder`] then
//! records the pass from that value and from GPU objects nothing changes during a frame.
//!
//! **Why the split.** A split screen's views can then be recorded at once
//! (`Scene::render_slots`): a `ViewPass` is a value of its view, and the `Recorder` holds only
//! shared references to `Sync` data, so a view's recording reads no `Cell`, no `Race` and no
//! other view's state. The compiler checks the second half (the assertion at the end of this
//! file); the first is what `tests::threaded_views` checks, by recording the views in reverse
//! and comparing the pictures.
//!
//! **What a view keeps for itself**: the draws it decided ([`Draw`]), its culling sets, the
//! counts each effect pipeline's upload left and the handles its draw binds ([`EffectsDrawn`]:
//! a clone of each pipeline's `Drawer`, so no recording holds a pipeline a later view's upload
//! writes, which is what lets a prepare run beside a recording) and the shadow's runs, its
//! depth-prepass order (sorted while recording, from its own eye), and its `LodGroup` choice,
//! which a drawable holds once per view ([`ViewBlock::index`]).

use super::viewports::Layer;
use super::*;
use crate::drawable::draw::Lists;

/// One thing a view's scene pass draws, in the order the pass draws them.
pub(super) enum Draw<'a> {
    /// A `perf-probe` marker over the draws after it.
    Mark(&'static str),
    /// The circuit: its solid lists, the sky, its blended list and its shine pass
    /// ([`Recorder::draw_track`]), under the view's sets and frustum.
    Track,
    /// The circuit's blended list alone, once the hulls and the ribbons are down
    /// ([`ViewPass::ribbon_first`]).
    TrackBlended,
    /// The craft's ribbons alone ([`Recorder::draw_ribbons`]).
    Ribbon,
    /// Every list of a drawable, culled by the view's frustum.
    Culled(&'a Drawable),
    /// Every list of a drawable, uncull'd.
    Mesh(&'a Drawable),
    /// [`Drawable::draw_additive`].
    Additive(&'a Drawable),
    /// [`Drawable::draw_overlay`].
    Overlay(&'a Drawable),
    /// The blob and hull shadows, [`ViewPass::shadow_runs`].
    Shadow,
    /// The blended effects over everything solid ([`Recorder::draw_effects`]).
    Effects,
}

/// The draws one view decided, built by its prepare.
pub(super) struct DrawList<'a>(pub(super) Vec<Draw<'a>>);

impl<'a> DrawList<'a> {
    pub(super) fn mark(&mut self, label: &'static str) {
        self.0.push(Draw::Mark(label));
    }

    pub(super) fn mesh(&mut self, drawable: &'a Drawable) {
        self.0.push(Draw::Mesh(drawable));
    }

    pub(super) fn culled(&mut self, drawable: &'a Drawable) {
        self.0.push(Draw::Culled(drawable));
    }

    pub(super) fn additive(&mut self, drawable: &'a Drawable) {
        self.0.push(Draw::Additive(drawable));
    }

    pub(super) fn overlay(&mut self, drawable: &'a Drawable) {
        self.0.push(Draw::Overlay(drawable));
    }

    /// The ribbons, then the circuit's blended list over them, when `ribbon_first`
    /// ([`ViewPass::ribbon_first`]); nothing otherwise, the circuit having drawn it whole.
    pub(super) fn ribbons_then_blended(&mut self, ribbon_first: bool) {
        if ribbon_first {
            self.0.push(Draw::Ribbon);
            self.0.push(Draw::TrackBlended);
        }
    }

    pub(super) fn push(&mut self, draw: Draw<'a>) {
        self.0.push(draw);
    }
}

/// What each effect pipeline's upload left to draw for one view, and the handles that draws
/// it: each pipeline's `Drawer`, cloned (a few reference counts) when the view is prepared.
/// The handles never change once a pipeline is built; holding a clone rather than the pipeline
/// is what lets a later view's upload, which needs the pipeline `&mut`, run while this view
/// records ([`super::Recording::Overlapped`]).
#[derive(Clone, Debug)]
pub(super) struct EffectsDrawn {
    exhaust: (exhaust::Drawer, exhaust::Drawn),
    sparks: (sparks::Drawer, sparks::Drawn),
    /// [`Scene::beams`]' order; `None` where the title builds none.
    beams: [Option<(oag_fx::beam::Drawer, oag_fx::beam::Drawn)>; BEAMS],
    clouds: Option<(oag_fx::cloud::Drawer, oag_fx::cloud::Drawn)>,
    weapon_quads: (oag_fx::weapon_quads::Drawer, oag_fx::weapon_quads::Drawn),
    mist: Option<(oag_fx::mist::Drawer, oag_fx::mist::Drawn)>,
    shadow: oag_render::shadow::Drawer,
}

/// The strip pipelines, in the order the effects draw them: the Leach beam, the magstrip wake's
/// contact quads and arcs, the Rocket smoke and HD's Leach strip.
const BEAMS: usize = 5;

/// One view's scene pass, decided: everything [`Recorder::record`] reads that is not the scene's
/// own GPU objects.
pub(super) struct ViewPass<'a> {
    pub(super) layer: Layer,
    pub(super) block: ViewBlock,
    pub(super) viewport: (f32, f32, f32, f32),
    /// Where the camera is: the depth prepass's order is sorted from it.
    pub(super) eye: Vec3,
    /// The fourth row of the unjittered view-projection matrix: `clip.w`, the view
    /// depth the blended list's list-B meshes are put in order by.
    pub(super) depth_plane: [f32; 4],
    pub(super) frustum: Option<Frustum>,
    pub(super) visible_set: Option<VisibleSet>,
    pub(super) chunk_set: Option<ChunkSet>,
    /// The scene's colour target: the caller's view, or the post chains' linear one.
    pub(super) target: &'a wgpu::TextureView,
    pub(super) timestamps: Option<wgpu::RenderPassTimestampWrites<'a>>,
    pub(super) draws: DrawList<'a>,
    pub(super) effects: EffectsDrawn,
    /// The original's queue order for a `Trail`: the ribbons draw before the circuit's blended
    /// list, which then draws over them where it passes its own depth test. Set where the
    /// title's ribbon is the PSP preset's; see [`Recorder::draw_ribbons`].
    pub(super) ribbon_first: bool,
    pub(super) shadow_runs: Vec<oag_render::shadow::Run>,
    pub(super) ghost: Option<(oag_render::ghost::Hull<'a>, oag_render::ghost::Fade)>,
    /// What the prepare already drew (the behind-the-glass target), counted in.
    pub(super) stats: SceneStats,
    /// The circuit's opaque list, gathered and written as an indirect list by the prepare
    /// where the track draws its prepass depth half indirect (`Drawable::prepare_solid`).
    pub(super) track_solid: Option<crate::drawable::Prepared>,
    /// The camera lens's own screen motion (its shake and its field of view), for the blur
    /// after the pass - see `Race::lens_screen_motion`.
    pub(super) blur_lens: Mat4,
}

impl Scene {
    /// The strip pipelines, in [`BEAMS`]' order; `None` where the title builds none.
    fn beams(&self) -> [Option<&std::cell::RefCell<oag_fx::beam::Pipeline>>; BEAMS] {
        let [contact, atlas, smoke, leach] = self.ribbons.cells();
        [self.beam.as_ref(), contact, atlas, smoke, leach]
    }

    /// What every effect pipeline's last upload left, and the handles that draw it, for the
    /// view that made them.
    pub(super) fn effects_drawn(&self) -> EffectsDrawn {
        let exhaust = self.exhaust.borrow();
        let sparks = self.sparks.borrow();
        let weapon_quads = self.weapon_quads.pipeline.borrow();
        EffectsDrawn {
            exhaust: (exhaust.drawer().clone(), exhaust.drawn()),
            sparks: (sparks.drawer().clone(), sparks.drawn()),
            beams: self.beams().map(|beam| {
                beam.map(|beam| {
                    let beam = beam.borrow();
                    (beam.drawer().clone(), beam.drawn())
                })
            }),
            clouds: self
                .clouds
                .borrow()
                .pipeline()
                .map(|clouds| (clouds.drawer().clone(), clouds.drawn())),
            weapon_quads: (weapon_quads.drawer().clone(), weapon_quads.drawn()),
            mist: self
                .mist
                .borrow()
                .as_ref()
                .map(|mist| (mist.drawer().clone(), mist.drawn())),
            shadow: self.shadow.borrow().drawer().clone(),
        }
    }

    /// The shadow pipeline's runs as the view's upload left them.
    pub(super) fn shadow_runs(&self, into: &mut Vec<oag_render::shadow::Run>) {
        into.clear();
        into.extend_from_slice(self.shadow.borrow().runs());
    }

    /// What every view's recording reads besides its [`ViewPass`].
    pub(super) fn recorder(&self) -> Recorder<'_> {
        Recorder {
            track: &self.track,
            sky: self.sky.as_ref(),
            track_shine: self.track_shine.as_ref().map(shine::TrackShine::parts),
            sections: self.visibility.as_ref().map(|v| &v.sections),
            ghost: self.ghost.pipeline(),
            attachments: &self.attachment_views,
        }
    }
}

/// The GPU objects a view's scene pass draws with: shared by every view of a frame, and
/// changed by none of them while they record.
pub(super) struct Recorder<'a> {
    track: &'a Drawable,
    sky: Option<&'a Drawable>,
    track_shine: Option<(&'a Drawable, &'a [usize])>,
    sections: Option<&'a DrawSections>,
    ghost: Option<&'a oag_render::ghost::Pipeline>,
    attachments: &'a motion::Attachments,
}

impl Recorder<'_> {
    /// Records `pass`'s scene pass into `encoder`, and returns what it drew.
    pub(super) fn record(
        &self,
        pass: &ViewPass<'_>,
        encoder: &mut wgpu::CommandEncoder,
    ) -> SceneStats {
        // MSAA draws into its own multisampled attachment and resolves into
        // the target at the end of this one pass; everything else draws
        // straight into it, exactly as before this setting existed. See
        // `Scene::msaa_color`.
        let (attachment_view, resolve_target) = match self.attachments.msaa.as_ref() {
            Some(msaa_view) => (msaa_view, Some(pass.target)),
            None => (pass.target, None),
        };
        // A split-screen view after the first keeps what the earlier ones drew - see
        // `Scene::render_viewports`.
        let later = pass.layer == Layer::Next;
        let mut render = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("race"),
            color_attachments: &[
                Some(wgpu::RenderPassColorAttachment {
                    view: attachment_view,
                    depth_slice: None,
                    resolve_target,
                    ops: wgpu::Operations {
                        // Black rather than the near-black blue this used to clear
                        // to. The clear covers the whole attachment and the scene is
                        // then drawn into a sub-rectangle, so this colour is what
                        // `Aspect`'s bars are made of - and a bar has to read as a
                        // bar. The old value was 0.03/0.04/0.06, dark enough that
                        // losing it costs nothing inside the viewport either.
                        // Black, and **alpha zero**: alpha is the bloom's glow mask
                        // and a frame starts with nothing glowing.
                        // `wgpu::Color::BLACK` has `a: 1.0`, which would mask the
                        // entire frame in and bloom everything.
                        load: if later {
                            wgpu::LoadOp::Load
                        } else {
                            wgpu::LoadOp::Clear(wgpu::Color {
                                r: 0.0,
                                g: 0.0,
                                b: 0.0,
                                a: 0.0,
                            })
                        },
                        store: wgpu::StoreOp::Store,
                    },
                }),
                // The velocity target, cleared to zero - nothing moved where
                // nothing drew - and never resolved: under MSAA the blur's
                // prepare stage reads sample 0 instead, because averaging
                // velocity across a silhouette edge produces a vector that
                // describes neither surface. See `Scene::velocity`.
                Some(wgpu::RenderPassColorAttachment {
                    view: &self.attachments.velocity,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        // A later view of a split loads, as the colour does: the first view
                        // cleared the whole attachment and each view draws inside its own
                        // rectangle, so clearing again would wipe the views before it and
                        // leave the blur only the last view's motion.
                        load: if later {
                            wgpu::LoadOp::Load
                        } else {
                            wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT)
                        },
                        store: wgpu::StoreOp::Store,
                    },
                }),
            ],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.attachments.depth,
                depth_ops: Some(wgpu::Operations {
                    // Loaded by a later view of a split for the velocity's reason above.
                    load: if later {
                        wgpu::LoadOp::Load
                    } else {
                        wgpu::LoadOp::Clear(1.0)
                    },
                    // `Store`, not `Discard`: the motion blur's reconstruction
                    // weighs its taps by depth ordering after this pass
                    // closes, and a discarded attachment is undefined memory
                    // by then. This is also the depth half of
                    // `docs/overview/modern-features.md`'s FSR 3.1
                    // prerequisite table.
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            // The one pass whose cost falls with the render extent, and so the
            // one a resolution controller will be driven by - see
            // `oag_gpu::timing::PassTimer` and `Scene::render`'s own doc.
            // `None` on every path that is not the window's frame loop: a
            // capture measures nothing, because a capture has to be
            // reproducible rather than fast.
            timestamp_writes: pass.timestamps.clone(),
            occlusion_query_set: None,
            multiview_mask: None,
        });
        oag_gpu::perfprobe::mark("pass-open");
        let (x, y, w, h) = pass.viewport;
        render.set_viewport(x, y, w, h, 0.0, 1.0);
        let block = pass.block;
        let mut stats = pass.stats;
        for draw in &pass.draws.0 {
            match draw {
                Draw::Mark(label) => oag_gpu::perfprobe::marks::mark(&mut render, label),
                Draw::Track => stats.add(self.draw_track(
                    &mut render,
                    pass,
                    BlendedList::with_ribbons(pass.ribbon_first),
                )),
                Draw::TrackBlended => {
                    stats.add(self.draw_track(&mut render, pass, BlendedList::Only));
                }
                Draw::Ribbon => self.draw_ribbons(&mut render, pass),
                Draw::Culled(drawable) => stats.add(drawable.draw(
                    &mut render,
                    block,
                    None,
                    None,
                    None,
                    pass.frustum.as_ref(),
                )),
                Draw::Mesh(drawable) => {
                    stats.add(drawable.draw(&mut render, block, None, None, None, None));
                }
                Draw::Additive(drawable) => stats.add(drawable.draw_additive(&mut render, block)),
                Draw::Overlay(drawable) => stats.add(drawable.draw_overlay(&mut render, block)),
                Draw::Shadow => pass.effects.shadow.draw(&mut render, &pass.shadow_runs),
                Draw::Effects => self.draw_effects(&mut render, pass, &mut stats),
            }
        }
        stats
    }

    /// Draws the circuit's solid lists, the sky, the circuit's extra pass, then its
    /// blended list ([`TRACK_STAGES`]): the extra pass is tested against the circuit's
    /// depth, so it follows the solid lists, and it precedes the blended list because
    /// the original queues it at the mesh's list-A key (`0x45`/`0x4a`) and every list-B mesh after
    /// them (`0x4d`) - a tree drawn after it is not veiled by it.
    ///
    /// **The sky goes behind the solid circuit rather than first.** It writes
    /// no depth, so it used to be drawn first comparing `Always`, filling the
    /// whole viewport for the circuit to cover - every covered sample shaded
    /// for nothing, the full frame's worth: 5.1 ms of HD's 47 ms race pass at
    /// 3200x1800 on an integrated GPU, and the largest single span in Pulse's.
    /// Drawn here with its depth range pinned to the far plane, it compares
    /// `LessEqual` against the 1.0 the pass cleared to, so it lands on exactly
    /// the samples nothing solid has reached - the ones it was visible on
    /// before. The blended list still draws over it, as it always did, and
    /// nothing solid ever read the colour underneath it.
    ///
    /// Neither culling tier is offered it: a skybox is never outside the
    /// frustum and belongs to no visibility section, which is the behaviour
    /// ADR-0011 already assumes for it. Its draw calls are deliberately **not**
    /// added to the stats either: being exempt from both tiers, folding them in
    /// would shift the denominator ADR-0011's and the roadmap's PVS
    /// effectiveness figures are quoted against.
    fn draw_track(
        &self,
        render: &mut wgpu::RenderPass<'_>,
        pass: &ViewPass<'_>,
        blended: BlendedList,
    ) -> SceneStats {
        let block = pass.block;
        let (set, chunks, frustum) = (
            pass.visible_set.as_ref(),
            pass.chunk_set.as_ref(),
            pass.frustum.as_ref(),
        );
        let mut stats = SceneStats::default();
        for stage in TRACK_STAGES {
            let is_blended = stage == TrackStage::Blended;
            match blended {
                BlendedList::Without if is_blended => continue,
                BlendedList::Only if !is_blended => continue,
                _ => {}
            }
            match stage {
                TrackStage::Solid => {
                    oag_gpu::perfprobe::marks::mark(render, "track solid");
                    // A prepared list carries its own order; the per-draw path sorts here.
                    let order = match pass.track_solid {
                        Some(_) => Vec::new(),
                        None => opaque_order(self.track, pass.eye),
                    };
                    stats.add(self.track.draw_lists(
                        render,
                        block,
                        self.sections,
                        set,
                        chunks,
                        frustum,
                        Lists::Solid {
                            order: Some(&order),
                            prepared: pass.track_solid.as_ref(),
                        },
                    ));
                }
                TrackStage::Sky => {
                    oag_gpu::perfprobe::marks::mark(render, "sky");
                    if let Some(sky) = self.sky {
                        let (x, y, w, h) = pass.viewport;
                        render.set_viewport(x, y, w, h, 1.0, 1.0);
                        let _ = sky.draw(render, block, None, None, None, None);
                        render.set_viewport(x, y, w, h, 0.0, 1.0);
                    }
                }
                TrackStage::Shine => {
                    oag_gpu::perfprobe::marks::mark(render, "track shine");
                    if let Some((shine, sources)) = self.track_shine {
                        stats.add(shine.draw_track_shine(
                            render,
                            block,
                            self.track,
                            sources,
                            self.sections,
                            set,
                            chunks,
                            frustum,
                        ));
                    }
                }
                TrackStage::Blended => {
                    oag_gpu::perfprobe::marks::mark(render, "track blended");
                    stats.add(self.track.draw_lists(
                        render,
                        block,
                        self.sections,
                        set,
                        chunks,
                        frustum,
                        track_blended(pass.depth_plane),
                    ));
                }
            }
        }
        stats
    }

    /// The craft's ribbons, alone: after the hulls, before the circuit's blended list.
    ///
    /// **The original queues every `Trail` at the constant key `0x4d000000`** - layer `0x4d`,
    /// depth zero - so it draws ahead of every list-B mesh (`0x4d` plus a depth that is never
    /// zero within 3,000 units), and the ribbon writes no depth. A translucent circuit mesh near
    /// the craft therefore paints over the ribbon where it passes its own depth test: on the race
    /// pair, one floor patch under the craft leaves the ribbon about a seventh of its colour. The
    /// flare is not part of this: it is keyed by its own depth and sits last. See
    /// `docs/ghidra/functions/psp-pulse-usa/exhaust.md`.
    fn draw_ribbons(&self, render: &mut wgpu::RenderPass<'_>, pass: &ViewPass<'_>) {
        oag_gpu::perfprobe::marks::mark(render, "ribbons");
        let (drawer, drawn) = &pass.effects.exhaust;
        drawer.draw(render, drawn.trail_only());
    }

    /// The race pass's last stretch: the depth-tested, depth-unwritten effects that draw over
    /// everything solid. The order is load-bearing: the flare tests depth but does not write it,
    /// so the hull's depth has to already be in the buffer for the flare to be occluded by it,
    /// and sparks are the same kind of geometry.
    fn draw_effects(
        &self,
        render: &mut wgpu::RenderPass<'_>,
        pass: &ViewPass<'_>,
        stats: &mut SceneStats,
    ) {
        let effects = &pass.effects;
        oag_gpu::perfprobe::marks::mark(render, "exhaust, sparks, clouds, ghost");
        // The ribbons already drew, ahead of the blended list ([`Self::draw_ribbons`]).
        let exhaust = if pass.ribbon_first {
            effects.exhaust.1.without_trail()
        } else {
            effects.exhaust.1
        };
        effects.exhaust.0.draw(render, exhaust);
        effects.sparks.0.draw(render, effects.sparks.1);
        for (beam, drawn) in effects.beams.iter().flatten() {
            beam.draw(render, *drawn);
        }
        if let Some((clouds, drawn)) = &effects.clouds {
            clouds.draw(render, *drawn);
        }
        effects.weapon_quads.0.draw(render, effects.weapon_quads.1);
        // The ghost, last of the world - see `oag_render::ghost`, "what is ours", for why last.
        if let (Some(pipeline), Some((hull, fade))) = (self.ghost, &pass.ghost) {
            stats.draws_submitted += pipeline.draw(render, hull, *fade);
        }
        // The render queue's key `0x4f000000`, which the flash shares: after
        // every world layer, under the HUD.
        if let Some((mist, drawn)) = &effects.mist {
            mist.draw(render, *drawn);
        }
        effects.sparks.0.draw_flash(render, effects.sparks.1);
        oag_gpu::perfprobe::marks::mark(render, "end");
    }
}

/// What [`Scene::draw_track`] runs, in this order.
///
/// The order is the original's queue order and a test holds it
/// (`tests/track_stages.rs`): the `0x2000` extra pass is queued at the mesh's list-A key
/// (`0x45`/`0x4a`) and every list-B mesh is `0x4d`, so the shine is drawn
/// before the blended list, never after it.
/// Which of the circuit's stages [`Recorder::draw_track`] draws: the whole circuit, all but the
/// blended list, or the blended list alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BlendedList {
    With,
    Without,
    Only,
}

impl BlendedList {
    /// What the circuit's own draw takes: with the ribbons first the blended list is a draw of its
    /// own, after them.
    pub(crate) const fn with_ribbons(ribbon_first: bool) -> Self {
        if ribbon_first {
            Self::Without
        } else {
            Self::With
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TrackStage {
    Solid,
    Sky,
    Shine,
    Blended,
}

/// [`TrackStage`] in draw order.
pub(crate) const TRACK_STAGES: [TrackStage; 4] = [
    TrackStage::Solid,
    TrackStage::Sky,
    TrackStage::Shine,
    TrackStage::Blended,
];

/// The circuit's blended list for a view whose `depth_plane` is the fourth row of its
/// view-projection matrix: list-B meshes far to near by view depth
/// ([`oag_mesh::mesh::Model::blended_order`]).
pub(crate) fn track_blended<'a>(depth_plane: [f32; 4]) -> Lists<'a> {
    Lists::Blended {
        depth_plane: Some(depth_plane),
    }
}

/// The order HD's depth prepass lays the circuit's opaque depth down in: each batch by the
/// distance from `eye` to the nearest point of its bounding sphere, nearest first, as
/// `(key, index)` pairs into [`Drawable::opaque_draws`], so the depth test rejects what is
/// behind as early as it can. See `mesh_render::Prepass`.
///
/// **Only the prepass reads it, and that is why it moves no pixel.** The depth `Less` leaves
/// is the nearest surface whatever order it arrives in.
/// Sorting the shading itself was tried first, 2026-10-03: 44.1 -> 37.4 ms at 3200x1800 on an
/// integrated GPU, at the cost of 1 to 419 pixels a capture where coplanar ties resolved to the
/// nearer batch. The prepass took that to 32.4 ms with the original's picture back; the shading
/// that follows keeps the model's own order. `sort_by` is stable, so equal keys keep the model's
/// order. Per view, from its own eye, while the view records.
pub(super) fn opaque_order(track: &Drawable, eye: Vec3) -> Vec<(f32, u32)> {
    let mut order: Vec<(f32, u32)> = track
        .opaque_draws()
        .iter()
        .zip(0u32..)
        .map(|(draw, i)| {
            let centre = Vec3::from_array(draw.bounds.centre);
            (centre.distance(eye) - draw.bounds.radius, i)
        })
        .collect();
    order.sort_by(|a, b| a.0.total_cmp(&b.0));
    order
}

// A view's recording reads only these, from a thread of its own (`Scene::render_slots`). wgpu's
// browser objects are not `Send`, and the web records its views on the page's thread.
#[cfg(not(target_arch = "wasm32"))]
const _: () = {
    const fn shared<T: Sync>() {}
    const fn sent<T: Send>() {}
    shared::<Recorder<'_>>();
    sent::<ViewPass<'_>>();
};

#[cfg(test)]
mod tests {
    use super::*;

    /// The original queues every `Trail` at the constant key `0x4d000000`, ahead of every
    /// list-B mesh, which then paints over it where it passes its own depth test (the ribbon
    /// writes no depth). The ribbons must be recorded before the circuit's blended list, and the
    /// circuit's own draw must leave that list out, or it draws twice and over the ribbon.
    #[test]
    fn the_ribbons_are_drawn_before_the_blended_list() {
        let mut list = DrawList(Vec::new());
        list.ribbons_then_blended(true);
        assert!(matches!(
            list.0.as_slice(),
            [Draw::Ribbon, Draw::TrackBlended]
        ));
        assert_eq!(BlendedList::with_ribbons(true), BlendedList::Without);
    }

    /// HD's tube keeps the old order, its queue key unread: the circuit draws whole and no extra
    /// draw is pushed.
    #[test]
    fn without_ribbons_first_the_circuit_draws_whole() {
        let mut list = DrawList(Vec::new());
        list.ribbons_then_blended(false);
        assert!(list.0.is_empty());
        assert_eq!(BlendedList::with_ribbons(false), BlendedList::With);
    }
}
