//! The ghost ship's drawables: which hull it borrows, where it is, and how
//! faded - the per-frame half of `oag_render::ghost`, kept out of `frame.rs`
//! under the 1,000-line rule.
//!
//! The ghost borrows the geometry of the ship drawable whose slot flies its
//! team: every team on the disc is already on the grid's liveries, so a
//! ghost recorded as another team than the player's is drawn as that team
//! with no second load. A team the grid does not carry falls back to the
//! player's hull and says so, once.

use std::cell::RefCell;

use oag_core::Rng;
use oag_fx::exhaust::FlareTexture;

use super::*;

/// The ghost pipeline, the team each ship drawable flies, and the static's
/// generator.
#[derive(Debug, Default)]
pub(super) struct Ghosts {
    pipeline: Option<oag_render::ghost::Pipeline>,
    teams: Vec<String>,
    /// `MeshNode_Ghost_RandomiseStatic`'s offsets, re-drawn every frame the
    /// race runs. A render-side generator - a picture is not simulation state.
    rng: RefCell<Option<Rng>>,
    /// The race tick the offsets were last drawn for: the original re-draws
    /// them whenever its game clock moves, and holds them while paused.
    offsets: RefCell<(u64, [f32; 2])>,
    warned: std::cell::Cell<bool>,
}

impl Scene {
    /// Builds the ghost's pipeline for a race of `mode`, when that mode races
    /// one - [`crate::ghosts::races_a_ghost`].
    ///
    /// `caller_format` is the format the caller asked [`Scene::new`] for; the
    /// scene's own is that or HD's linear one, decided the same way
    /// `hd_chain::build` decides it. The glow-mask stamps follow the PSP
    /// bloom: they exist for it to read, and a scene without it gets none.
    pub fn prepare_ghost(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        caller_format: wgpu::TextureFormat,
        mode: oag_race::Mode,
        liveries: &[crate::livery::Livery],
        static_glow: Option<&FlareTexture>,
    ) {
        if !crate::ghosts::races_a_ghost(mode) {
            return;
        }
        let format = if self.draws_linear() {
            oag_gpu::formats::SCENE_FORMAT
        } else {
            caller_format
        };
        self.ghost.pipeline = Some(oag_render::ghost::Pipeline::new(
            device,
            queue,
            format,
            self.sample_count(),
            self.bloom.is_some(),
            static_glow,
        ));
        self.ghost.teams = liveries.iter().map(|l| l.team.clone()).collect();
        *self.ghost.rng.borrow_mut() = Some(Rng::new(0x6405_7a71));
    }

    /// The ship drawable the ghost borrows: the first slot flying its team.
    fn ghost_drawable(&self, team: &str) -> Option<&Drawable> {
        let slot = self
            .ghost
            .teams
            .iter()
            .position(|t| t.eq_ignore_ascii_case(team))
            .unwrap_or_else(|| {
                if !self.ghost.warned.replace(true) {
                    warn!(
                        "the ghost's team {team} is not on the grid - drawn as the player's hull"
                    );
                }
                0
            });
        self.ships.get(slot)
    }

    /// Where the ghost is this frame and how faded, or `None` when nothing
    /// draws: no ghost, no pipeline, or the ghost off the circuit.
    fn ghost_frame(
        &self,
        race: &Race,
        view_projection: Mat4,
        prev_vp: Mat4,
    ) -> Option<oag_render::ghost::Frame> {
        self.ghost.pipeline.as_ref()?;
        let (now, before) = race.ghost_pose()?;
        let distance = (race.ship().physics.body.position - now.position).length();
        let tick = race.sim.world.tick;
        let mut offsets = self.ghost.offsets.borrow_mut();
        if offsets.0 != tick
            && let Some(rng) = self.ghost.rng.borrow_mut().as_mut()
        {
            *offsets = (tick, oag_render::ghost::static_offsets(rng));
        }
        Some(oag_render::ghost::Frame {
            view_projection,
            model: Race::ghost_model_matrix(&now),
            prev_mvp: prev_vp * Race::ghost_model_matrix(&before),
            fade: oag_render::ghost::fade(distance),
            offsets: offsets.1,
        })
    }

    /// Writes the ghost's uniforms for this frame.
    pub(super) fn write_ghost(
        &self,
        race: &Race,
        queue: &wgpu::Queue,
        view_projection: Mat4,
        prev_vp: Mat4,
    ) {
        if let (Some(pipeline), Some(frame)) = (
            self.ghost.pipeline.as_ref(),
            self.ghost_frame(race, view_projection, prev_vp),
        ) {
            pipeline.write(queue, &frame);
        }
    }

    /// Draws the ghost, last in the scene pass - see `oag_render::ghost`,
    /// "what is ours", for why last.
    pub(super) fn draw_ghost(
        &self,
        race: &Race,
        eye: oag_mesh::mesh::LodEye,
        pass: &mut wgpu::RenderPass<'_>,
        stats: &mut SceneStats,
    ) {
        let (Some(pipeline), Some(ghost), Some((now, _))) = (
            self.ghost.pipeline.as_ref(),
            race.ghost(),
            race.ghost_pose(),
        ) else {
            return;
        };
        let Some(drawable) = self.ghost_drawable(&ghost.team) else {
            return;
        };
        let distance = (race.ship().physics.body.position - now.position).length();
        let drawn = pipeline.draw(
            pass,
            &drawable.ghost_hull(Race::ghost_model_matrix(&now), eye),
            oag_render::ghost::fade(distance),
        );
        stats.draws_submitted += drawn;
    }
}
