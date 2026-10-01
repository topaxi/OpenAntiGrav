//! The spectator camera that takes over behind the end-race panels.
//!
//! **Pulse off a PSP disc only**, the one executable it is read off. After the player's last
//! crossing the original runs a *director* (`Camera_UpdateSpectator`, `0x0887fd3c`) on its
//! global camera object: it follows a subject craft, sits on one of the circuit's authored
//! `Camera` nodes, and cuts to another when the subject has moved on. The reading and the
//! captures that check its timing are
//! [`race-finish.md`](../../../../docs/ghidra/functions/psp-pulse-usa/race-finish.md).
//!
//! # What is ported
//!
//! | Rule | Source |
//! | --- | --- |
//! | starts [`START_TICKS`] frames after the flag, on the player, on the node nearest the player's position | measured (four captures, to the frame) |
//! | the subject is re-picked every [`SUBJECT_PERIOD_TICKS`] frames: the player, unless that is the previous subject, then a random live craft | measured, and the previous-subject rule read |
//! | a cut when the subject is [`NODE_RADIUS`] or more from the node's aim point: a random node within [`NODE_RADIUS`] of the subject, then a mode roll of `3` 26 %, `2` 25 %, `6` 25 %, `7` 24 % | read from the decompile; the cut instants are random and cannot be matched frame for frame |
//! | modes `6` and `7` (and `5`): eye at the node, aimed at a smoothed subject, the fov easing to `2 atan(width / 2 / distance)` | read from the decompile |
//!
//! # What is chosen, not measured
//!
//! - **Modes `2` and `3`** are craft-relative views (`above`, `front`) whose geometry was not
//!   read. The ordinary chase camera stands in for them, so about half the cuts leave the
//!   chase view where the original shows something else.
//! - **The random stream.** The original draws from C's `rand`; this draws from its own
//!   seeded [`Rng`], view-side and never the simulation's, so the sequence matches the
//!   *distribution* and not the frames.
//! - The opening zoom, which the original starts `-10..+20` degrees off the target field of
//!   view; the sign of that draw was read, not seen.

use super::*;

use oag_core::math::Mat3;

/// Frames after the finishing frame on which the director starts.
///
/// Measured: the camera object's subject becomes the player on `F+61` in all four captures.
pub const START_TICKS: u64 = 61;

/// Frames between subject re-picks (10.0 s at 60 Hz), measured as `F+661` and `F+1261`.
pub const SUBJECT_PERIOD_TICKS: u32 = 600;

/// How near the subject must be to a node's aim point for the camera to stay on it (`60` units,
/// `3600` squared in `FUN_08880168`).
pub const NODE_RADIUS: f32 = 60.0;

/// The fov's ease toward its target each frame (`0x3d75c28f`, `cam+0x260`).
const FOV_RATE: f32 = 0.06;

/// The view width the constructor starts the camera object with (`0x42700000`), kept until a
/// cut calls `Camera_SetMode` with a node mode.
const INITIAL_WIDTH: f32 = 60.0;

/// What a mode roll can land on, with the original's thresholds on `rand() % 100`.
const MODE_ROLLS: [(u32, ViewMode); 4] = [
    (26, ViewMode::Front),
    (51, ViewMode::Above),
    (76, ViewMode::Close),
    (100, ViewMode::Track),
];

/// The cameras the spectator director can be in, by the original's own numbers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    /// `2`: craft-relative, **not read**; the chase camera stands in.
    Above,
    /// `3`: craft-relative, **not read**; the chase camera stands in.
    Front,
    /// `6`: a node camera, view width `17`.
    Close,
    /// `7`: a node camera, view width `50`.
    Track,
}

impl ViewMode {
    /// The width `Camera_SetMode` stores in `cam+0x268` for a node mode, `None` for the two
    /// craft-relative ones, which leave it alone.
    fn width(self) -> Option<f32> {
        match self {
            Self::Close => Some(17.0),
            Self::Track => Some(50.0),
            Self::Above | Self::Front => None,
        }
    }

    /// Whether the camera sits at a node.
    #[must_use]
    pub fn is_node_camera(self) -> bool {
        matches!(self, Self::Close | Self::Track)
    }
}

/// The director's own random stream: not the simulation's, and not the shake's.
pub const SPECTATOR_SEED: u64 = 0x0f1a_15c0;

/// `cam+0x264` by speed class, set in the camera object's constructor off the race's class
/// index: `0x3ecccccd`, `0x3f000000`, `0x3f19999a` and `0x3f19999a`.
#[must_use]
pub fn spectator_smoothing(class: oag_race::SpeedClass) -> f32 {
    match class {
        oag_race::SpeedClass::Venom => 0.4,
        oag_race::SpeedClass::Flash => 0.5,
        oag_race::SpeedClass::Rapier | oag_race::SpeedClass::Phantom => 0.6,
    }
}

/// The circuit's `Camera` nodes out of its `track.vex`, in file order - the order the
/// original's own list holds them in, checked node for node on `16_Track`.
#[must_use]
pub fn spectator_nodes(track_blob: &[u8]) -> Vec<SpectatorNode> {
    oag_vex::camera::cameras(track_blob)
        .iter()
        .map(|camera| SpectatorNode {
            eye: Vec3::from_array(camera.position()),
            aim: Vec3::from_array(camera.aim),
        })
        .collect()
}

/// One authored `Camera` node of the circuit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpectatorNode {
    /// Where the camera sits (`node+0x90`): the node's world translation.
    pub eye: Vec3,
    /// The aim point the engine picks nodes by (`node+0xa0`): the payload's three floats at
    /// `+0x10`.
    pub aim: Vec3,
}

/// A craft the director can follow: its slot and where it is.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Subject {
    /// The grid slot.
    pub slot: usize,
    /// The craft's position this tick.
    pub position: Vec3,
}

/// The director's state. View-side: nothing here reaches the simulation or its hash.
#[derive(Debug, Clone)]
pub struct FinishCamera {
    nodes: Vec<SpectatorNode>,
    /// `cam+0x264`: how fast the aim point follows the subject, by speed class.
    smoothing: f32,
    rng: Rng,
    running: bool,
    /// Frames since the subject was last picked.
    since_subject: u32,
    subject: usize,
    previous: Option<usize>,
    node: Option<usize>,
    mode: ViewMode,
    width: f32,
    smoothed: Vec3,
    fov: f32,
    /// Whether `camera_override` is this director's, so a `--camera-pose` one is never cleared.
    imposed: bool,
}

impl FinishCamera {
    /// A director over `nodes`, flying with `smoothing` (`0.4 / 0.5 / 0.6 / 0.6` for Venom
    /// through Phantom) and its own stream from `seed`.
    #[must_use]
    pub fn new(nodes: Vec<SpectatorNode>, smoothing: f32, seed: u64) -> Self {
        Self {
            nodes,
            smoothing,
            rng: Rng::new(seed),
            running: false,
            since_subject: 0,
            subject: 0,
            previous: None,
            node: None,
            mode: ViewMode::Track,
            width: INITIAL_WIDTH,
            smoothed: Vec3::ZERO,
            fov: 65.0,
            imposed: false,
        }
    }

    /// Whether the director has any node to sit on. A circuit that authors none (or a title
    /// this was not read for) never overrides the chase camera.
    #[must_use]
    pub fn has_nodes(&self) -> bool {
        !self.nodes.is_empty()
    }

    /// The mode the director is in, `None` before it starts.
    #[must_use]
    pub fn mode(&self) -> Option<ViewMode> {
        self.running.then_some(self.mode)
    }

    /// The craft the director follows, `None` before it starts.
    #[must_use]
    pub fn subject(&self) -> Option<usize> {
        self.running.then_some(self.subject)
    }

    /// One frame. `since_finish` is how many ticks have passed since the player's finishing
    /// tick; `live` holds every craft still in the race, `player` is the player's slot.
    ///
    /// The pose to draw from, or `None` to leave the chase camera: before the start, with no
    /// nodes, and in the two craft-relative modes.
    pub fn step(
        &mut self,
        since_finish: u64,
        live: &[Subject],
        player: usize,
    ) -> Option<CameraOverride> {
        if since_finish < START_TICKS || self.nodes.is_empty() {
            return None;
        }
        if !self.running {
            self.start(live, player);
        } else {
            self.director(live, player);
        }
        let subject = live.iter().find(|s| s.slot == self.subject)?;
        self.pose(subject.position)
    }

    /// `FUN_08880788` on the first frame: the player, the nearest node, mode `7`.
    fn start(&mut self, live: &[Subject], player: usize) {
        self.running = true;
        self.since_subject = 0;
        self.subject = player;
        self.previous = Some(player);
        self.mode = ViewMode::Track;
        let position = position_of(live, player);
        self.node = self.nearest_node(position);
        self.take_node(position);
    }

    /// `Camera_UpdateSpectator`.
    fn director(&mut self, live: &[Subject], player: usize) {
        self.since_subject += 1;
        let mut repick = self.since_subject >= SUBJECT_PERIOD_TICKS;
        // A subject that is no longer in the race is dropped, as a respawning one is.
        if !live.iter().any(|s| s.slot == self.subject) {
            repick = true;
        }
        if repick {
            self.since_subject = 0;
            self.pick_subject(live, player);
        }
        let position = position_of(live, self.subject);
        if self.node.is_none() {
            self.node = self.nearest_node(position);
            self.previous = Some(self.subject);
            self.take_node(position);
            return;
        }
        if self.repick_node(position) {
            self.roll_mode();
            self.previous = Some(self.subject);
        } else if let Some(previous) = self.previous {
            // The original tries again from the previous subject's position, and ignores
            // what that finds: the node may change, the mode and `previous` do not.
            let there = position_of(live, previous);
            self.repick_node(there);
        }
    }

    /// `Camera_PickSubject`.
    fn pick_subject(&mut self, live: &[Subject], player: usize) {
        let mut subject = player;
        if let Some(previous) = self.previous
            && live.iter().all(|s| s.slot != previous)
        {
            self.previous = Some(player);
        }
        let count = u32::try_from(live.len()).unwrap_or(0);
        while Some(subject) == self.previous && count > 1 {
            let at = self.rng.below(count) as usize;
            subject = live[at].slot;
        }
        self.subject = subject;
    }

    /// `FUN_0887fedc`: the node whose aim point is nearest `position`.
    fn nearest_node(&self, position: Vec3) -> Option<usize> {
        let mut best = None;
        let mut best_distance = f32::MAX;
        for (index, node) in self.nodes.iter().enumerate() {
            let distance = (position - node.aim).length_squared();
            if distance < best_distance {
                best_distance = distance;
                best = Some(index);
            }
        }
        best
    }

    /// `FUN_08880168`: stay while the subject is within [`NODE_RADIUS`] of the node's aim
    /// point, otherwise take a random node within it that is not this one. `true` when a node
    /// was taken.
    fn repick_node(&mut self, position: Vec3) -> bool {
        let Some(current) = self.node else {
            return false;
        };
        let radius_squared = NODE_RADIUS * NODE_RADIUS;
        let aim = self.nodes[current].aim;
        if (position - aim).length_squared() < radius_squared {
            return false;
        }
        let candidates: Vec<usize> = self
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, node)| {
                node.aim != aim && (position - node.aim).length_squared() < radius_squared
            })
            .map(|(index, _)| index)
            .collect();
        if candidates.is_empty() {
            return false;
        }
        let at = self.rng.below(u32::try_from(candidates.len()).unwrap_or(1)) as usize;
        self.node = Some(candidates[at]);
        self.take_node(position);
        true
    }

    /// The part of a node change that resets the shot: the field of view starts a random
    /// amount off its target and the aim point starts on the subject.
    fn take_node(&mut self, position: Vec3) {
        let Some(node) = self.node else {
            return;
        };
        let target = field_of_view(self.width, self.nodes[node].eye, position);
        // `Psys_RandFloatRange(-10, 20)`, and never below 3 degrees.
        let start = target + (-10.0 + 30.0 * self.rng.next_f32());
        self.fov = start.max(3.0);
        self.smoothed = position;
    }

    /// `Camera_PickRandomMode`, then `Camera_SetMode`'s view width.
    fn roll_mode(&mut self) {
        let roll = self.rng.below(100);
        self.mode = MODE_ROLLS
            .iter()
            .find(|(limit, _)| roll < *limit)
            .map_or(ViewMode::Track, |&(_, mode)| mode);
        if let Some(width) = self.mode.width() {
            self.width = width;
        }
    }

    /// The node cameras' own update (cases `5`, `6`, `7` of `FUN_08880c04`): the aim point
    /// follows the subject, the fov eases to its target, and the eye is the node.
    fn pose(&mut self, subject: Vec3) -> Option<CameraOverride> {
        if !self.mode.is_node_camera() {
            return None;
        }
        let node = self.nodes[self.node?];
        let target_fov = field_of_view(self.width, node.eye, subject);
        self.smoothed += (subject - self.smoothed) * self.smoothing;
        self.fov += (target_fov - self.fov) * FOV_RATE;
        // The look vector runs from the aim point to the eye, its height squashed so a wide
        // zoom keeps the horizon: `y *= max(1 - fov * 0.008, 0.4)`.
        let mut back = node.eye - self.smoothed;
        back.y *= (1.0 - self.fov * 0.008).max(0.4);
        let back = back.try_normalize()?;
        let right = Vec3::Y.cross(back).try_normalize()?;
        let up = back.cross(right);
        Some(CameraOverride {
            eye: node.eye,
            orientation: Quat::from_mat3(&Mat3::from_cols(right, up, back)),
            fov_deg: Some(self.fov),
        })
    }
}

fn position_of(live: &[Subject], slot: usize) -> Vec3 {
    live.iter()
        .find(|s| s.slot == slot)
        .map_or(Vec3::ZERO, |s| s.position)
}

/// `FUN_08880984`: the vertical field of view, in degrees, that frames `width` units at the
/// distance from the node's eye to the subject; `65` with no node.
#[must_use]
pub fn field_of_view(width: f32, eye: Vec3, subject: Vec3) -> f32 {
    let distance = (eye - subject).length().max(1e-3);
    (width * 0.5 / distance).atan() * 2.0 * 180.0 / std::f32::consts::PI
}

impl Race {
    /// The mode the post-race camera is in, `None` before it starts or where there is none.
    #[must_use]
    pub fn spectator_mode(&self) -> Option<ViewMode> {
        self.view
            .finish_camera
            .as_ref()
            .and_then(FinishCamera::mode)
    }

    /// Steps the post-race camera one tick and imposes its pose, once the player's craft
    /// has crossed the line and the world is running on behind the panels.
    pub(super) fn advance_finish_camera(&mut self) {
        if self.view.finish_camera.is_none() || !self.runs_on_after_the_line() {
            return;
        }
        let player = self.player_slot();
        let Some(finish) = self.sim.world.ships[player].standing.finish_tick else {
            return;
        };
        let live: Vec<Subject> = (0..usize::from(self.sim.world.ship_count))
            .filter(|&slot| {
                let ship = &self.sim.world.ships[slot];
                ship.active && ship.physics.craft_state != oag_physics::CraftState::Eliminated
            })
            .map(|slot| Subject {
                slot,
                position: self.sim.world.ships[slot].physics.body.position,
            })
            .collect();
        let since = self.sim.world.tick.saturating_sub(finish);
        let Some(director) = self.view.finish_camera.as_mut() else {
            return;
        };
        if self.view.camera_override.is_some() && !director.imposed {
            return;
        }
        let pose = director.step(since, &live, player);
        director.imposed = pose.is_some();
        self.view.camera_override = pose;
    }
}

#[cfg(test)]
mod tests;
