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
//!   flyby plays full screen before the split starts ([`Race::views_drawn`]); in a
//!   multi-window race every window plays it ([`Race::window_drawn`]).
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
    /// Which way two views are cut. Read through [`Race::cut`], which ignores it for
    /// any other count.
    cut: oag_render::viewport::Cut,
    /// The views are dealt across OS windows (`--windows N`, `--window-seats`): window
    /// `i` shows the seats of `windows[i]`, cut up inside it like a split screen. Empty
    /// is one window (the split of [`Self::cut`], or no split). A presentation choice
    /// beside [`Self::cut`], not a second count: the cameras are still [`Self::count`]'s.
    windows: Vec<WindowViews>,
    /// The extra slot the scene is being drawn for right now, `None` for the
    /// player's own view. Set by the draw through `&Race`, so a [`Cell`].
    drawing: Cell<Option<u8>>,
    /// Indexed by slot. Slot 0 stays `None`: the player's camera is
    /// `RaceView::camera`.
    cams: [Option<Cam>; MAX_SHIPS],
}

/// What one OS window shows of a multi-window race: which grid slots, cut up the way a split
/// screen cuts its views.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WindowViews {
    /// The slots this window shows, in view order. Empty is a window with nothing to show
    /// yet, which draws a holding frame.
    pub seats: Vec<usize>,
    /// Which way two views are cut; means nothing for any other count
    /// ([`oag_render::viewport::Cut::for_views`]).
    pub cut: oag_render::viewport::Cut,
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

    /// How many people fly this race, across every window ([`Setup::people`](super::Setup::people)).
    /// Not [`Self::viewport_count`]: a `--window-seats 1,1` race draws one view per window and
    /// has two people.
    #[must_use]
    pub fn people(&self) -> usize {
        usize::from(self.sim.people.max(1))
    }

    /// Which way the screen is cut: the one answer the scene's view rectangles
    /// ([`Self::view_rects`]) and the HUD's placements both read, so a vertical scene cannot
    /// get a horizontal HUD. Only two views can be cut left and right
    /// ([`oag_render::viewport::Cut::for_views`]).
    #[must_use]
    pub fn cut(&self) -> oag_render::viewport::Cut {
        if self.windowed() {
            return self.window_cut(0);
        }
        self.view.viewpoints.cut.for_views(self.viewport_count())
    }

    /// Whether the views are dealt across OS windows. See [`Viewpoints::windows`].
    #[must_use]
    pub fn windowed(&self) -> bool {
        !self.view.viewpoints.windows.is_empty()
    }

    /// The windows the views are dealt across, window 1 first.
    #[must_use]
    pub fn windows(&self) -> &[WindowViews] {
        &self.view.viewpoints.windows
    }

    /// Deals the views across OS windows: the multi-window presentation. Empty turns it
    /// off. A view-side choice like [`Self::set_cut`]; no tick reads it and the state hash
    /// does not hold it. The seats are slots of the views [`Self::set_viewport_count`]
    /// fielded; a slot past them has no camera and is skipped when drawn.
    pub fn set_windows(&mut self, windows: Vec<WindowViews>) {
        self.view.viewpoints.windows = windows;
        self.fit_sights();
    }

    /// Takes window `index` out of the layout (it was closed), so the windows after it
    /// keep their place in the list. Its seats stay in the race, flown by the AI.
    /// Window 1 (index 0) is never removed: closing it ends the run.
    pub fn remove_window(&mut self, index: usize) {
        if index > 0 && index < self.view.viewpoints.windows.len() {
            self.view.viewpoints.windows.remove(index);
            self.fit_sights();
        }
    }

    /// The slots window `window` draws right now, in view order. Its own seats while the
    /// race runs; **just the player's own while the intro flyby plays, in every window that
    /// holds a seat** (the flyby is the player's camera and the other seats have none, so each
    /// window plays it full window at its own size and aspect, and each switches to its own
    /// seats when it ends - **chosen, not measured**); **and once the race is over, in every
    /// window that holds a seat, just as the flyby is** (the end screens sit over the spectator
    /// camera, as in a split: [`Self::views_drawn`], each window at its own size and aspect -
    /// **chosen, not measured**, the maintainer's request of 2026-10-16). A window past the
    /// layout, or with no seats, draws none.
    #[must_use]
    pub fn window_drawn(&self, window: usize) -> Vec<usize> {
        let Some(views) = self.view.viewpoints.windows.get(window) else {
            return Vec::new();
        };
        let player = self.player_slot();
        let seats: Vec<usize> = views
            .seats
            .iter()
            .copied()
            .filter(|&slot| slot < self.viewport_count())
            .collect();
        let flyby = self.in_intro() && !seats.is_empty();
        if flyby || (!seats.is_empty() && self.finished()) {
            vec![player]
        } else {
            seats
        }
    }

    /// Which way window `window` cuts what it draws ([`Self::window_drawn`]).
    #[must_use]
    pub fn window_cut(&self, window: usize) -> oag_render::viewport::Cut {
        let drawn = self.window_drawn(window).len();
        self.view
            .viewpoints
            .windows
            .get(window)
            .map_or_else(Default::default, |views| views.cut.for_views(drawn))
    }

    /// The rectangles of the views window `window` draws in `area`, in view order: the
    /// single source for where a view and its HUD go in that window.
    #[must_use]
    pub fn window_rects(
        &self,
        window: usize,
        area: oag_render::viewport::Rect,
    ) -> Vec<oag_render::viewport::Rect> {
        oag_render::viewport::layout(
            self.window_drawn(window).len(),
            area,
            self.window_cut(window),
        )
    }

    /// Chooses the cut for a two-view race. A view-side choice like the cameras: no tick
    /// reads it and the state hash does not hold it.
    pub fn set_cut(&mut self, cut: oag_render::viewport::Cut) {
        self.view.viewpoints.cut = cut;
        self.fit_sights();
    }

    /// Puts every slot's reticle in the grid of the view it is drawn in: a view cut left and
    /// right projects into the `vert_` family's view where the title authors one
    /// ([`crate::Sighting::columns_screen`]), every other view into the one-view grid. Which
    /// view a slot is in is its window's cut (`--window-seats 2h,2v` has both), or the race's
    /// own with no windows. View side only: the locks are the simulation's, in their own grid.
    fn fit_sights(&mut self) {
        let (rows, columns) = self.view.sight_screens;
        let Some(columns) = columns else {
            return;
        };
        let vertical = |race: &Self, slot: usize| {
            let (seats, cut) = match race
                .view
                .viewpoints
                .windows
                .iter()
                .find(|w| w.seats.contains(&slot))
            {
                Some(window) => (window.seats.len(), window.cut),
                None => (race.viewport_count(), race.view.viewpoints.cut),
            };
            cut.for_views(seats) == oag_render::viewport::Cut::Vertical
        };
        let primary = self.player_slot();
        let pick = |vertical: bool| if vertical { columns } else { rows };
        let screen = pick(vertical(self, primary));
        self.view.sight.set_screen(screen);
        for slot in 0..self.viewport_count() {
            let screen = pick(vertical(self, slot));
            self.view.huds[slot].sight.set_screen(screen);
        }
    }

    /// The rectangles of the views drawn right now ([`Self::views_drawn`]) in `area`,
    /// in view order: the single source for where a view and its HUD go.
    #[must_use]
    pub fn view_rects(&self, area: oag_render::viewport::Rect) -> Vec<oag_render::viewport::Rect> {
        oag_render::viewport::layout(self.views_drawn(), area, self.cut())
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
        // Window 1's, when the views are dealt across windows: what the track panel and
        // the end screens, which belong to window 1, are laid out against.
        if self.windowed() {
            return self.window_drawn(0).len().max(1);
        }
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

    /// How many times the picture being drawn has jumped to a different shot - what a camera
    /// history watches to throw its previous frame away. The player's view is
    /// [`Self::camera_cuts`]; an extra seat's chase camera jumps when that craft respawns, and
    /// nothing else of the player's cuts (the flyby, the spectator and destroy cameras) is its.
    #[must_use]
    pub fn viewed_camera_cuts(&self) -> u32 {
        match self.extra_viewpoint() {
            Some(slot) => self.respawns_of(slot),
            None => self.camera_cuts(),
        }
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
