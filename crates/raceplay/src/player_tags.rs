//! Multiplayer tags over the other people's craft: the fade state, the projection and the
//! names. The law is `oag_hud::player_tags`; this is where the camera is.
//!
//! **Display only.** Nothing here is read by the simulation or hashed: the fade lives in each
//! view's `SlotHud` and the tags leave as part of the view's `Readout`.

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

    /// The slots whose view shows tags: the person-flown slots with a view of their own.
    fn tag_viewers(&self) -> impl Iterator<Item = usize> + '_ {
        let primary = self.player_slot();
        let own_views = if self.view.seat.is_some() {
            0
        } else {
            self.viewport_count()
        };
        std::iter::once(primary).chain((0..own_views).filter(move |&slot| slot != primary))
    }

    /// Whether `target` is tagged in `viewer`'s view: another person's craft on the grid.
    /// **Chosen, not measured:** an AI craft is not tagged.
    fn tagged_for(&self, viewer: usize, target: usize) -> bool {
        target != viewer
            && target < MAX_SHIPS
            && self.sim.world.person_flown(target)
            && self.sim.world.ships[target].active
    }

    /// One tick of every view's fades: a tagged craft's rises at 1.2 a second, any other is
    /// held at zero.
    pub(super) fn advance_tag_fades(&mut self) {
        if !self.view.player_tags_on {
            return;
        }
        let dt = self.sim.dt;
        let viewers: Vec<usize> = self.tag_viewers().collect();
        for viewer in viewers {
            for target in 0..MAX_SHIPS {
                let fade = self.view.huds[viewer].tag_fade[target];
                self.view.huds[viewer].tag_fade[target] = if self.tagged_for(viewer, target) {
                    step_fade(fade, dt)
                } else {
                    0.0
                };
            }
        }
    }

    /// The name over `slot`'s craft: the lobby's for a network race, else `Player N` (a stop
    /// gap; the original's own fallback string is one shared `PLAYER`).
    #[must_use]
    pub fn tag_name(&self, slot: usize) -> String {
        self.view
            .person_names
            .get(slot)
            .filter(|name| !name.is_empty())
            .cloned()
            .unwrap_or_else(|| format!("Player {}", slot + 1))
    }

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
        let primary = viewer == self.player_slot();
        let screen = if primary {
            self.view.sight.screen()
        } else {
            self.view.huds[viewer].sight.screen()
        };
        let drawing = self.extra_viewpoint();
        if primary {
            self.reset_viewpoint();
        } else {
            self.set_viewpoint(viewer);
        }
        let tags = (0..MAX_SHIPS)
            .filter(|&target| self.tagged_for(viewer, target))
            .filter_map(|target| {
                let fade = self.view.huds[viewer].tag_fade[target];
                let body = self.body_between(target);
                let anchor = body.position + body.up() * raise(fade);
                let projected = self.project_for_sight(anchor, screen)?;
                let alpha = alpha(fade, projected.distance, true);
                (alpha > 0.0).then(|| PlayerTag {
                    screen: projected.screen,
                    alpha,
                    name: self.tag_name(target),
                })
            })
            .collect();
        match drawing {
            Some(extra) => self.set_viewpoint(extra),
            None => self.reset_viewpoint(),
        }
        tags
    }
}
