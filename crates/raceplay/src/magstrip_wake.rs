//! The HD-lineage magstrip arc wake in a race: the over-strip predicate, the
//! per-craft wake [`oag_fx::magstrip`] simulates, and the `~magstrip01` sound
//! law. Pulse's own two-mesh effect is [`super::mag_floor_fx`]; the two are
//! never both on one title.
//!
//! Evidence: `docs/ghidra/functions/ps4-omega-eu/ships-effects.md`, "2026-10-05,
//! magstrip-omega-law lane" (the predicate, the blend, the sound arming law) and
//! "magstrip-wire-hd lane" (the update and the draw), and
//! `docs/ghidra/functions/ps3-hdfury-eu/magstrip-wake.md`.
//!
//! - **The predicate is the instantaneous contact**, the mag-floor probe of this
//!   tick (`Ship_IsOverMagStrip` reads `controller+0x5d0`, a bool of "the probe
//!   hit surface type 3"). The simulation keeps only the blend that probe drives
//!   and a *sticky* last contact, so it is read back off the blend with
//!   [`super::mag_floor_fx::contact_this_tick`] - exact, and pinned by
//!   `the_ramp_reads_back_as_the_probe_it_was_given`. Not the blend itself: that
//!   ramps and lingers five ticks.
//! - **Every craft has one**, the player's included (`ship[0xe18]` is built for
//!   each ship constructor under `mode < 0x17`).
//! - **The sound**: an arming flag, set at construction. Over the strip and
//!   armed: start `~magstrip01` and disarm. Leaving: stop it and re-arm
//!   ([`sound_edge`]).
//!
//! - **2048 plays a `.pob` instead of the arc** ([`Wakes::pob`]): its events carry a
//!   CRC-id mode (`>= 0x17`), the side of `GameMode_IsHdLineage` that builds no
//!   wake and plays `WO_MAGSTRIP_SPARKS`, or `WO_MAGSTRIP_ZONE` in a Zone
//!   (`docs/ghidra/functions/vita-2048-eu-v104/ships-effects.md`, live read of
//!   `0x8153fd24` on Vita3K). The sound edge above is the same either way.
//!   **Chosen, not measured**: the effect runs only while the craft is over the
//!   strip (the original's start and stop were not read), at the hull's
//!   `arc_anchor_point`, and "Zone" is [`Mode::Zone`].
//!
//! **Omitted, as unidentified in the original**: the veto `ship+0x71f5 & 0x10`,
//! the freeze while `controller+0x2d8 == 0` or `+0x2c5 & 4`, and the skip of
//! the local ship when `DAT_01f998e8 & 1`.
//!
//! **Chosen, not measured**: the sound group. The mixer has no group concept, so
//! `MagStrip_Player` (the local ship, slot 0) and `MagStrip_NPC` are not told
//! apart beyond slot 0 playing dry-positional like every other craft sound;
//! the group's `[300.0, 50.0]` pair is unread.

use super::*;
use oag_fx::magstrip::{Craft, Placement, Wake};
use oag_sound::sfx::{Cue, CueEvent};

/// What the sound does on a tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SoundEdge {
    /// Start the cue.
    Start,
    /// Stop the cue.
    Stop,
}

/// `FUN_01762e80`'s arming law, pure so a test can feed it a sequence.
///
/// `armed` starts `true`. Returns the edge to raise and the new `armed`:
///
/// - over the strip and armed: [`SoundEdge::Start`], disarmed;
/// - not over it, having been: [`SoundEdge::Stop`], re-armed;
/// - anything else: nothing, `armed` unchanged.
#[must_use]
pub(crate) fn sound_edge(armed: bool, was_over: bool, over: bool) -> (Option<SoundEdge>, bool) {
    if over && armed {
        (Some(SoundEdge::Start), false)
    } else if !over && was_over {
        (Some(SoundEdge::Stop), true)
    } else {
        (None, armed)
    }
}

/// Every craft's wake and the state that drives it.
#[derive(Debug, Clone)]
pub(super) struct Wakes {
    anchors: [Option<Mat4>; MAX_SHIPS],
    wakes: Vec<Wake>,
    previous_blend: [f32; MAX_SHIPS],
    over: [bool; MAX_SHIPS],
    armed: [bool; MAX_SHIPS],
    /// Whether the arc wake is simulated and drawn: false on a `.pob` title.
    arcs: bool,
    /// The `.pob` instance each craft has riding it, on a `.pob` title.
    pob: Option<[Option<psys::Playing>; MAX_SHIPS]>,
}

/// The seed a slot's wake draws from.
const SEED: u64 = 0x4d41_4753_5452_4950;

impl Wakes {
    pub(super) fn new(anchors: [Option<Mat4>; MAX_SHIPS]) -> Self {
        Self {
            anchors,
            wakes: (0..MAX_SHIPS)
                .map(|slot| Wake::new(SEED ^ slot as u64))
                .collect(),
            previous_blend: [0.0; MAX_SHIPS],
            over: [false; MAX_SHIPS],
            armed: [true; MAX_SHIPS],
            arcs: true,
            pob: None,
        }
    }

    /// The `.pob` variant: the same over-strip state and sound, no arc wake.
    pub(super) fn pob(anchors: [Option<Mat4>; MAX_SHIPS]) -> Self {
        Self {
            arcs: false,
            pob: Some([None; MAX_SHIPS]),
            ..Self::new(anchors)
        }
    }

    /// Whether `slot` is over a strip this tick.
    pub(super) fn over(&self, slot: usize) -> bool {
        self.over.get(slot).copied().unwrap_or(false)
    }
}

impl Race {
    /// One tick of every craft's wake, after the simulation. Called from
    /// [`Race::tick`] beside [`Race::advance_mag_floor_fx`], and for the same
    /// reason not from `tick_cosmetics`: a frozen world freezes its wake.
    pub(crate) fn advance_magstrip_wake(&mut self) {
        let Some(mut wakes) = self.view.magstrip_wake.take() else {
            return;
        };
        let dt = self.sim.dt;
        for slot in 0..usize::from(self.sim.world.ship_count).min(MAX_SHIPS) {
            let blend = self.sim.world.ships[slot].physics.mag_lock_blend;
            let was_over = wakes.over[slot];
            let over = super::mag_floor_fx::contact_this_tick(wakes.previous_blend[slot], blend);
            wakes.previous_blend[slot] = blend;
            wakes.over[slot] = over;
            let (edge, armed) = sound_edge(wakes.armed[slot], was_over, over);
            wakes.armed[slot] = armed;
            match edge {
                Some(SoundEdge::Start) => self.sim.cues.push(CueEvent::new(Cue::Magstrip, slot)),
                Some(SoundEdge::Stop) => {
                    self.sim.cues.push(CueEvent::new(Cue::MagstripStop, slot));
                }
                None => {}
            }
            let Some(anchor) = wakes.anchors[slot] else {
                continue;
            };
            if wakes.pob.is_some() {
                self.advance_magstrip_pob(&mut wakes, slot, over, anchor);
            }
            if !wakes.arcs {
                continue;
            }
            if !self.ship_active(slot) || !(over || wakes.wakes[slot].live() > 0) {
                continue;
            }
            let Some(craft) = self.wake_craft(slot, anchor) else {
                continue;
            };
            let from = self.sim.world.ships[slot].physics.body.position;
            let walk = |ahead: f32| self.walk_track(from, ahead);
            wakes.wakes[slot].advance(dt, over, &craft, &walk);
        }
        self.view.magstrip_wake = Some(wakes);
    }

    /// One craft's `.pob` magstrip effect: started riding the anchor on contact,
    /// followed while over the strip, detached on leaving or when the craft is
    /// out of play. Nothing plays when the effect did not load.
    fn advance_magstrip_pob(&mut self, wakes: &mut Wakes, slot: usize, over: bool, anchor: Mat4) {
        let Some(handles) = wakes.pob.as_mut() else {
            return;
        };
        let live = over && self.ship_active(slot);
        match (handles[slot], live) {
            (None, true) => {
                let trigger = if self.sim.world.mode() == Mode::Zone {
                    Trigger::MagstripZone
                } else {
                    Trigger::MagstripSparks
                };
                let Some(effect) = self.view.handles.get(trigger).cloned() else {
                    return;
                };
                let at = (self.ship_model_matrix_of(slot) * anchor).w_axis.truncate();
                handles[slot] = self.view.stage.play_riding(&effect, at, 1.0);
            }
            (Some(playing), true) => {
                let at = (self.ship_model_matrix_of(slot) * anchor).w_axis.truncate();
                self.view.stage.follow(playing, at);
            }
            (Some(playing), false) => {
                self.view.stage.detach(playing);
                handles[slot] = None;
            }
            (None, false) => {}
        }
    }

    /// The `.pob` effect craft `slot` has playing now, for tests and telemetry.
    #[must_use]
    pub fn magstrip_pob_of(&self, slot: usize) -> Option<psys::Playing> {
        self.view
            .magstrip_wake
            .as_ref()
            .and_then(|wakes| wakes.pob.as_ref())
            .and_then(|handles| handles.get(slot).copied().flatten())
    }

    /// What the wake reads of craft `slot` this frame. `with_track` and
    /// `lateral` are the spawn's, and left at their neutral values when the
    /// track has no sample under the craft.
    fn wake_craft(&self, slot: usize, anchor: Mat4) -> Option<Craft> {
        let model = self.ship_model_matrix_of(slot);
        let physics = &self.sim.world.ships[slot].physics;
        let world_anchor = model * anchor;
        let forward = model.z_axis.truncate().normalize_or_zero();
        let (with_track, lateral) = match self.sim.spline.nearest(physics.body.position) {
            Some((_, sample, _)) => {
                let tangent = Vec3::from_array(sample.tangent).normalize_or_zero();
                let across = Vec3::from_array(sample.lateral).normalize_or_zero();
                (
                    forward.dot(tangent) >= 0.0,
                    (physics.body.position - Vec3::from_array(sample.pos)).dot(across),
                )
            }
            None => (true, 0.0),
        };
        Some(Craft {
            anchor: world_anchor.w_axis.truncate(),
            position: model.w_axis.truncate(),
            forward,
            up: world_anchor.y_axis.truncate().normalize_or_zero(),
            speed: physics.body.linear_velocity.length(),
            with_track,
            lateral,
        })
    }

    /// The track `ahead` units along the spline from the sample nearest `from`,
    /// negative for behind, as [`Placement`]. **Chosen, not measured** - the
    /// original walks its AI-track data; see `oag_fx::magstrip`.
    fn walk_track(&self, from: Vec3, ahead: f32) -> Option<Placement> {
        let spline = &self.sim.spline;
        let (mut index, _, _) = spline.nearest(from)?;
        let path = spline.path_of(index);
        let step: isize = if ahead >= 0.0 { 1 } else { -1 };
        let mut travelled = 0.0;
        let mut here = Vec3::from_array(spline.sample(index)?.pos);
        while travelled < ahead.abs() {
            let Some(next) = index.checked_add_signed(step) else {
                break;
            };
            let Some(sample) = spline.sample(next) else {
                break;
            };
            if spline.path_of(next) != path {
                break;
            }
            let there = Vec3::from_array(sample.pos);
            travelled += (there - here).length();
            here = there;
            index = next;
        }
        let sample = spline.sample(index)?;
        let lifted = Spline::track_sample(sample);
        Some(Placement {
            position: lifted.position,
            forward: Vec3::from_array(sample.tangent).normalize_or_zero(),
            lateral: Vec3::from_array(sample.lateral).normalize_or_zero(),
            down: Vec3::from_array(sample.down).normalize_or_zero(),
            half_width_left: sample.half_width_left,
            half_width_right: sample.half_width_right,
        })
    }

    /// This frame's wake geometry, `(atlas batch, contact batch)`, seen from the
    /// camera. Empty on a title without the class, and for a craft out of play.
    #[must_use]
    pub fn magstrip_wake_vertices(
        &self,
    ) -> (
        Vec<oag_mesh::mesh::GpuVertex>,
        Vec<oag_mesh::mesh::GpuVertex>,
    ) {
        let (mut atlas, mut contact) = (Vec::new(), Vec::new());
        let Some(wakes) = self.view.magstrip_wake.as_ref().filter(|wakes| wakes.arcs) else {
            return (atlas, contact);
        };
        let eye = self.camera_position();
        for slot in 0..usize::from(self.sim.world.ship_count).min(MAX_SHIPS) {
            let Some(anchor) = wakes.anchors[slot] else {
                continue;
            };
            if !self.ship_active(slot) {
                continue;
            }
            if let Some(craft) = self.wake_craft(slot, anchor) {
                wakes.wakes[slot].build(&craft, eye, &mut atlas, &mut contact);
            }
        }
        (atlas, contact)
    }

    /// Whether the title builds the arc wake at all.
    #[must_use]
    pub fn has_magstrip_wake(&self) -> bool {
        self.view
            .magstrip_wake
            .as_ref()
            .is_some_and(|wakes| wakes.arcs)
    }

    /// How many arcs craft `slot`'s wake has live, for tests and telemetry.
    #[must_use]
    pub fn magstrip_wake_live(&self, slot: usize) -> usize {
        self.view
            .magstrip_wake
            .as_ref()
            .and_then(|wakes| wakes.wakes.get(slot))
            .map_or(0, Wake::live)
    }

    /// Whether craft `slot` was over a magstrip on the last tick the wake read.
    #[must_use]
    pub fn over_magstrip(&self, slot: usize) -> bool {
        self.view
            .magstrip_wake
            .as_ref()
            .is_some_and(|wakes| wakes.over(slot))
    }
}

#[cfg(test)]
mod tests;
