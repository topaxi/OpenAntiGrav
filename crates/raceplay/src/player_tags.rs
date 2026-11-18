//! Multiplayer tags over the other people's craft: the fade state, the projection and the
//! names. The law is `oag_hud::player_tags`; this is where the camera is.
//!
//! **Display only.** Nothing here is read by the simulation or hashed: the fade lives in each
//! view's `SlotHud` and the tags leave as part of the view's `Readout`.

use crate::RaceSim;
#[cfg(feature = "view")]
use crate::RaceView;
use crate::SimRace;
use oag_gameplay::MAX_SHIPS;
use oag_hud::PlayerTag;
use oag_hud::player_tags::{alpha, raise, step_fade};

use crate::Race;

impl Race {
    /// Whether multiplayer tags are drawn (Pulse's `Multiplayer Tags`). Off, every view's
    /// readout carries none. Display state, so the frame loop may set it every frame.
    pub fn set_player_tags(&mut self, on: bool) {
        self.view.player_tags_on = on;
    }
}

impl RaceSim {
    /// Whether `target` is tagged in `viewer`'s view: another person's craft on the grid.
    /// **Chosen, not measured:** an AI craft is not tagged.
    fn tagged_for(&self, viewer: usize, target: usize) -> bool {
        target != viewer
            && target < MAX_SHIPS
            && self.world.person_flown(target)
            && self.world.ships[target].active
    }
}

impl SimRace {
    /// The name over `slot`'s craft: the one [`Self::set_person_names`] gave it (the lobby's in a
    /// network race, the stored player name for seat 1 in a local one), else `Player N` (a stop
    /// gap; the original's own fallback string is one shared `PLAYER`).
    #[must_use]
    pub fn tag_name(&self, slot: usize) -> String {
        self.outcome
            .person_names
            .get(slot)
            .filter(|name| !name.is_empty())
            .cloned()
            .unwrap_or_else(|| format!("Player {}", slot + 1))
    }
}

impl Race {
    /// The tags `viewer`'s view draws this frame, projected through that view's camera at the
    /// craft's drawn pose and into the grid its reticle uses.
    ///
    /// Empty with the option off, in a race of fewer than two people, and for a craft behind
    /// the camera. No occlusion test: the original's own gate (`FUN_0883e37c`) reads as a
    /// viewport test.
    pub(super) fn player_tags_for(&self, viewer: usize) -> Vec<PlayerTag> {
        if !self.view.player_tags_on || viewer >= MAX_SHIPS {
            return Vec::new();
        }
        let primary = viewer == self.core.player_slot();
        let screen = if primary {
            self.view.sight.screen()
        } else {
            self.view.huds[viewer].sight.screen()
        };
        let drawing = self.view.extra_viewpoint();
        if primary {
            self.view.reset_viewpoint();
        } else {
            self.view.set_viewpoint(&self.core, viewer);
        }
        let tags = (0..MAX_SHIPS)
            .filter(|&target| self.core.sim.tagged_for(viewer, target))
            .filter_map(|target| {
                let fade = self.view.huds[viewer].tag_fade[target];
                let body = self.view.body_between(&self.core, target);
                let anchor = body.position + body.up() * raise(fade);
                let projected = self.view.project_for_sight(&self.core, anchor, screen)?;
                let alpha = alpha(fade, projected.distance, true);
                (alpha > 0.0).then(|| PlayerTag {
                    screen: projected.screen,
                    alpha,
                    name: self.core.tag_name(target),
                })
            })
            .collect();
        match drawing {
            Some(extra) => self.view.set_viewpoint(&self.core, extra),
            None => self.view.reset_viewpoint(),
        }
        tags
    }
}

#[cfg(feature = "view")]
impl RaceView {
    /// The slots whose view shows tags: the person-flown slots with a view of their own.
    fn tag_viewers(&self, core: &SimRace) -> impl Iterator<Item = usize> + '_ {
        let primary = core.player_slot();
        let own_views = if core.outcome.seat.is_some() {
            0
        } else {
            self.viewport_count()
        };
        std::iter::once(primary).chain((0..own_views).filter(move |&slot| slot != primary))
    }

    /// One tick of every view's fades: a tagged craft's rises at 1.2 a second, any other is
    /// held at zero.
    pub(super) fn advance_tag_fades(&mut self, core: &SimRace) {
        if !self.player_tags_on {
            return;
        }
        let dt = core.sim.dt;
        let viewers: Vec<usize> = self.tag_viewers(core).collect();
        for viewer in viewers {
            for (target, fade) in self.huds[viewer].tag_fade.iter_mut().enumerate() {
                *fade = if core.sim.tagged_for(viewer, target) {
                    step_fade(*fade, dt)
                } else {
                    0.0
                };
            }
        }
    }
}
