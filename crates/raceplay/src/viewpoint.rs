//! Split screen's cameras: one chase camera per extra local viewport.
//!
//! One window, one [`Race`], one tick, `N` views. The first view is the
//! player's own and takes the code path it always took, which is what keeps a
//! race with no split bit-identical in picture and in hash. View `i > 0` follows
//! grid slot `i`'s craft through [`Viewpoints`], kept here.
//!
//! **Nothing in this file is simulation state.** The cameras live in
//! [`RaceView`](super::view::RaceView), outside [`RaceSim`](super::sim::RaceSim),
//! and no tick reads them back, so a split moves a picture and cannot move a hash
//! - `tests::viewpoint` pins that.
//!
//! # What is chosen, not measured
//!
//! The original's split-screen camera has not been read, so the following are
//! **chosen, not measured**:
//!
//! - Every extra view flies the **player's own `<ExternalCameraFar>` rig**
//!   (`RaceView::chase_far`), whatever team the craft it follows belongs to: the
//!   load keeps one team's camera blocks, and a rig per slot is open work.
//! - An extra view has **no impact shake**, no intro flyby, no finish or destroy
//!   camera and no cockpit view. Those belong to the player's own camera, and the
//!   flyby plays full screen before the split starts ([`Race::views_drawn`]).
//! - The speed widen of the field of view follows the viewed craft; the boost
//!   kick does not (it is armed from slot 0's pad alone).

use std::cell::Cell;

use oag_core::math::Mat4;
use oag_gameplay::MAX_SHIPS;
use oag_render::camera::chase::Chase;

use super::camera::target_of;
use super::*;

/// One extra view's camera: the spring, and the view of the last two ticks so a
/// draw can blend between them as the player's own does.
#[derive(Clone, Copy, Debug)]
struct Cam {
    chase: Chase,
    previous: Mat4,
    current: Mat4,
    /// The craft's respawn count when the spring was last settled, so a
    /// teleport re-snaps the camera instead of sweeping it across the circuit.
    respawns: u32,
}

/// The extra views of a split-screen race.
#[derive(Debug, Default)]
pub(super) struct Viewpoints {
    /// How many views are drawn, `1` meaning no split.
    count: usize,
    /// The extra slot the scene is being drawn for right now, `None` for the
    /// player's own view. Set by the draw through `&Race`, so a [`Cell`].
    drawing: Cell<Option<u8>>,
    /// Indexed by slot. Slot 0 stays `None`: the player's camera is
    /// `RaceView::camera`.
    cams: [Option<Cam>; MAX_SHIPS],
}

impl Viewpoints {
    /// The blended view of the extra slot `slot`, at draw alpha `alpha`.
    fn view(&self, slot: usize, alpha: f32) -> Mat4 {
        match self.cams.get(slot).copied().flatten() {
            Some(cam) => super::between_ticks::blend_view(cam.previous, cam.current, alpha),
            None => Mat4::IDENTITY,
        }
    }
}

impl Race {
    /// How many views this race draws. `1` is no split.
    #[must_use]
    pub fn viewport_count(&self) -> usize {
        self.view.viewpoints.count.max(1)
    }

    /// How many views are drawn right now: [`Self::viewport_count`] while the race
    /// runs, and **one while the intro flyby plays and once the race is over**
    /// (every person has finished). The flyby is the player's own camera and the
    /// extra views have none, so it plays full screen and the split starts when it
    /// ends; the end screens sit over the one picture the single-player flow has -
    /// the spectator camera of `Race End Photo` - and not over a split. The extra
    /// cameras keep stepping through the flyby, so they are settled on their craft
    /// when the split appears.
    ///
    /// **Chosen, not measured**: the original's split screen is not read. The split
    /// itself ([`Self::viewport_count`]) is left alone, because it is also the
    /// answer to "was this a split race" (no record, no ghost, restart keeps it),
    /// and it picks the draw path: a split race keeps `Scene::render_viewports`,
    /// with no blur, jitter or upscaler history, for its one last view too.
    #[must_use]
    pub fn views_drawn(&self) -> usize {
        if self.in_intro() || self.finished() {
            1
        } else {
            self.viewport_count()
        }
    }

    /// Splits the screen into `count` views, clamped to the grid that is fielded
    /// and to [`oag_render::viewport::MAX_VIEWPORTS`], and returns what it
    /// settled on. View `i` follows slot `i`. `1` turns the split off.
    ///
    /// Each extra camera starts settled on its craft, with no spring to work
    /// off, as a race start's does.
    pub fn set_viewport_count(&mut self, count: usize) -> usize {
        let fielded = usize::from(self.sim.world.ship_count).max(1);
        let count = count.clamp(1, oag_render::viewport::MAX_VIEWPORTS.min(fielded));
        self.view.viewpoints.count = count;
        self.view.viewpoints.cams = [None; MAX_SHIPS];
        let player = self.player_slot();
        for slot in (0..count).filter(|&slot| slot != player) {
            let target = target_of(&self.sim.world.ships[slot]);
            let params = self.view.chase_far;
            let chase = Chase::snapped(target, &params);
            let view = chase.view(target, &params);
            self.view.viewpoints.cams[slot] = Some(Cam {
                chase,
                previous: view,
                current: view,
                respawns: self.respawns_of(slot),
            });
        }
        count
    }

    /// Steps every extra camera one tick, from this tick's craft poses. A no-op
    /// with no split. Called from the tick right after the player's own camera.
    pub(super) fn advance_viewpoints(&mut self) {
        let params = self.view.chase_far;
        let dt = self.sim.dt;
        for slot in 0..MAX_SHIPS {
            let respawns = self.respawns_of(slot);
            let target = target_of(&self.sim.world.ships[slot]);
            let Some(cam) = self.view.viewpoints.cams[slot].as_mut() else {
                continue;
            };
            if respawns != cam.respawns {
                cam.chase = Chase::snapped(target, &params);
                cam.respawns = respawns;
            } else {
                cam.chase.advance(target, &params, dt);
            }
            cam.previous = cam.current;
            cam.current = cam.chase.view(target, &params);
        }
    }

    /// Draws the next [`Scene::render`](super::Scene::render) from slot `slot`'s
    /// camera. The player's own slot is the ordinary view, exactly.
    ///
    /// **Put back with [`Self::reset_viewpoint`]** once the views are drawn: it
    /// is draw-side state behind a shared reference, and anything that reads the
    /// camera afterwards - the HUD's reticle, the next tick's audio listener -
    /// means the player's.
    pub fn set_viewpoint(&self, slot: usize) {
        let extra = (slot != self.player_slot() && self.view.viewpoints.cams[slot].is_some())
            .then(|| u8::try_from(slot).unwrap_or(0));
        self.view.viewpoints.drawing.set(extra);
    }

    /// The player's own view again. See [`Self::set_viewpoint`].
    pub fn reset_viewpoint(&self) {
        self.view.viewpoints.drawing.set(None);
    }

    /// The extra slot being drawn, `None` for the player's own view.
    pub(super) fn extra_viewpoint(&self) -> Option<usize> {
        self.view.viewpoints.drawing.get().map(usize::from)
    }

    /// The slot of the craft the view being drawn follows: the player's, or the
    /// extra slot.
    #[must_use]
    pub fn viewed_slot(&self) -> usize {
        self.extra_viewpoint().unwrap_or_else(|| self.player_slot())
    }

    /// The craft the view being drawn follows: the player's, or the extra slot.
    #[must_use]
    pub fn viewed_ship(&self) -> &Ship {
        &self.sim.world.ships[self.viewed_slot()]
    }

    /// An extra view's blended camera, `None` for the player's own.
    pub(super) fn extra_view(&self) -> Option<Mat4> {
        self.extra_viewpoint()
            .map(|slot| self.view.viewpoints.view(slot, self.draw_alpha()))
    }

    /// An extra view's vertical field of view, the authored far rig widened by
    /// the viewed craft's speed as the player's is. `None` for the player's own.
    pub(super) fn extra_vertical_fov(
        &self,
        aspect: f32,
        setting: oag_display::display::Fov,
    ) -> Option<f32> {
        let slot = self.extra_viewpoint()?;
        let body = &self.sim.world.ships[slot].physics.body;
        let speed = body.forward().dot(body.linear_velocity);
        let degrees = (self.view.chase_far.fov + SPEED_FOV_GAIN_DEG * speed)
            .clamp(FOV_GUARD_DEG.0, FOV_GUARD_DEG.1);
        Some(oag_render::camera::fit_vertical_fov(
            setting.apply(degrees.to_radians()),
            AUTHORED_ASPECT,
            aspect,
        ))
    }
}
