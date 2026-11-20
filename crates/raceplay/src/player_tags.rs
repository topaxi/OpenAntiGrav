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
use oag_core::math::Vec3;
use oag_hud::player_tags::{alpha, anchor, in_view_row, step_fade};

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
    /// Empty with the option off, in a race of fewer than two people, for a craft behind the
    /// camera, and for a craft whose track section the view's section row does not hold
    /// ([`RaceView::tag_gate`]). The anchor is raised along the world's Y, as the original adds the
    /// raise to the second float of the craft's translation row.
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
            .filter(|&target| self.view.tag_gate(&self.core, viewer, target))
            .filter_map(|target| {
                let fade = self.view.huds[viewer].tag_fade[target];
                let body = self.view.body_between(&self.core, target);
                let anchor = Vec3::from_array(anchor(body.position.to_array(), fade));
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
    /// `FUN_0883e37c`: whether `target`'s track section is in the row of the section `viewer`'s
    /// view stands in. Every craft passes on a track that names no sections.
    pub(super) fn tag_gate(&self, core: &SimRace, viewer: usize, target: usize) -> bool {
        let Some(pvs) = self.track_sections.as_ref() else {
            return true;
        };
        let viewer_section = core.sim.section_of_slot(viewer);
        let target_section = core.sim.section_of_slot(target);
        in_view_row(Some(target_section), pvs.visible_from(viewer_section))
    }

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

    /// One tick of every view's fades: a tagged craft's rises at 1.2 a second while it passes
    /// the gate and holds where it was while it does not (the original leaves the fade alone
    /// then); a craft that is not tagged at all is held at zero.
    pub(super) fn advance_tag_fades(&mut self, core: &SimRace) {
        if !self.player_tags_on {
            return;
        }
        let dt = core.sim.dt;
        let viewers: Vec<usize> = self.tag_viewers(core).collect();
        for viewer in viewers {
            let gates: [bool; MAX_SHIPS] =
                std::array::from_fn(|target| self.tag_gate(core, viewer, target));
            for (target, fade) in self.huds[viewer].tag_fade.iter_mut().enumerate() {
                if !core.sim.tagged_for(viewer, target) {
                    *fade = 0.0;
                } else if gates[target] {
                    *fade = step_fade(*fade, dt);
                }
            }
        }
    }
}
