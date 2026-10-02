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
//! | mode `2`, a rigid rear view 6 behind and 2.5 above the craft, and mode `3`, a rigid front view 12 ahead and 3 above, looking back; both at a fixed 65 degrees | read from the decompile and **measured** on PPSSPP, 407 frames to `6e-5` ([`oag_render::camera::craft_view`]) |
//! | the craft the view shows is the *previous* subject (`cam+0x1e4`), which takes the subject's place only when the director cuts to a new node | read from the decompile and seen in the capture (the 600 frame re-pick lands on a cut) |
//!
//! # What is chosen, not measured
//!
//! - **The random stream.** The original draws from C's `rand`; this draws from its own
//!   seeded [`Rng`], view-side and never the simulation's, so the sequence matches the
//!   *distribution* and not the frames.
//! - The opening zoom, which the original starts `-10..+20` degrees off the target field of
//!   view; the sign of that draw was read, not seen.

use super::*;

use oag_render::camera::craft_view;
use oag_render::camera::destroy::{self, Destroy, Station};

/// Frames after the finishing frame on which the director starts.
///
/// Measured: the camera object's subject becomes the player on `F+61` in all four captures.
pub const START_TICKS: u64 = 61;

/// Ticks after the race ended on which the camera object takes another craft as its subject
/// after a player's wreck in a Single Race: the destroy camera (mode 5) keeps the wreck until
/// `k+290` frames after the injected `Ship_SetState(4)` and then follows another craft on another
/// node, `259` frames after the mode state went to 3 (`k+31`). **Measured once** (a per-frame log
/// of the camera object, PPSSPP, 2026-10-02); the original stays in mode 5 where this director
/// starts in mode 7, and what triggers the hand-off is unread.
pub const WRECK_START_TICKS: u64 = 259;

/// Frames between subject re-picks (10.0 s at 60 Hz), measured as `F+661` and `F+1261`.
pub const SUBJECT_PERIOD_TICKS: u32 = 600;

/// How near the subject must be to a node's aim point for the camera to stay on it (`60` units,
/// `3600` squared in `FUN_08880168`).
pub const NODE_RADIUS: f32 = 60.0;

/// The view width the constructor starts the camera object with (`0x42700000`), kept until a
/// cut calls `Camera_SetMode` with a node mode.
const INITIAL_WIDTH: f32 = destroy::START_FRAME_SIZE;

/// What a mode roll can land on, with the original's thresholds on `rand() % 100`.
const MODE_ROLLS: [(u32, ViewMode); 4] = [
    (26, ViewMode::Front),
    (51, ViewMode::Rear),
    (76, ViewMode::Close),
    (100, ViewMode::Track),
];

/// The cameras the spectator director can be in, by the original's own numbers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    /// `2`: bolted to the craft, 6 units behind it and 2.5 above, looking the way it flies.
    Rear,
    /// `3`: bolted to the craft, 12 units ahead of it and 3 above, looking back at it.
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
            Self::Rear | Self::Front => None,
        }
    }

    /// Whether the camera sits at a node.
    #[must_use]
    pub fn is_node_camera(self) -> bool {
        matches!(self, Self::Close | Self::Track)
    }
}

/// The director's own random stream: not the simulation's, the shake's or the destroy camera's.
pub const SPECTATOR_SEED: u64 = 0x0f1a_15c0;

/// A craft the director can follow: its slot and where it is.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Subject {
    /// The grid slot.
    pub slot: usize,
    /// The craft's position this tick.
    pub position: Vec3,
    /// The rigid body's orientation this tick: what the two craft-relative views ride on.
    pub orientation: Quat,
}

/// The director's state. View-side: nothing here reaches the simulation or its hash.
#[derive(Debug, Clone)]
pub struct FinishCamera {
    nodes: Vec<Station>,
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
    /// How many times the picture has jumped: the take-over, a new node while a node camera
    /// is showing, a new mode and a new craft being watched. What [`Race::camera_cuts`] adds
    /// so the temporal upscaler's history is dropped across them.
    cuts: u32,
    /// What the last frame showed: the mode, the node when it was a node camera, the craft.
    last_shot: Option<(ViewMode, Option<usize>, usize)>,
}

impl FinishCamera {
    /// A director over `nodes`, flying with `smoothing` (`0.4 / 0.5 / 0.6 / 0.6` for Venom
    /// through Phantom) and its own stream from `seed`.
    #[must_use]
    pub fn new(nodes: Vec<Station>, smoothing: f32, seed: u64) -> Self {
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
            cuts: 0,
            last_shot: None,
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
    /// The pose to draw from, or `None` before the start and with no nodes (a circuit that
    /// authors none never leaves the chase camera: the director has nothing to cut by).
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
        // The craft the view shows is `cam+0x1e4`, which only takes the subject's place when
        // the director cuts; a craft that left the race is replaced by the subject.
        let watched = live
            .iter()
            .find(|s| Some(s.slot) == self.previous)
            .or_else(|| live.iter().find(|s| s.slot == self.subject))?;
        let pose = self.pose(watched);
        // A jump the upscaler must not blend across: the picture changes shot.
        let node = self.mode.is_node_camera().then_some(self.node).flatten();
        let shot = (self.mode, node, watched.slot);
        if self.last_shot != Some(shot) {
            self.cuts = self.cuts.wrapping_add(1);
        }
        self.last_shot = Some(shot);
        Some(pose)
    }

    /// How many times the picture has jumped to a different shot. See [`Race::camera_cuts`].
    #[must_use]
    pub fn cuts(&self) -> u32 {
        self.cuts
    }

    /// `FUN_08880788` on the first frame: the player, the nearest node, mode `7`.
    fn start(&mut self, live: &[Subject], player: usize) {
        self.running = true;
        self.since_subject = 0;
        self.subject = player;
        self.previous = Some(player);
        // A wrecked player is not live: the first subject is the first craft that is.
        if live.iter().all(|s| s.slot != player)
            && let Some(first) = live.first()
        {
            self.subject = first.slot;
            self.previous = Some(first.slot);
        }
        self.mode = ViewMode::Track;
        let position = position_of(live, self.subject);
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
        // A wrecked player is not a candidate: start from some craft that is still racing.
        // (The original's player is always live; this is the port's own wreck case.)
        if live.iter().all(|s| s.slot != player) && count > 0 {
            subject = live[self.rng.below(count) as usize].slot;
        }
        while Some(subject) == self.previous && count > 1 {
            let at = self.rng.below(count) as usize;
            subject = live[at].slot;
        }
        self.subject = subject;
    }

    /// `Camera_PickStation`: the node whose aim point is nearest `position`.
    fn nearest_node(&self, position: Vec3) -> Option<usize> {
        destroy::nearest_station(&self.nodes, position)
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
        let reach = self.nodes[node].eye.distance(position).max(f32::EPSILON);
        let target = destroy::framing_fov_degrees(self.width, reach);
        // `Psys_RandFloatRange(-10, 20)`, and never below 3 degrees.
        let (low, high) = destroy::START_FOV_SPREAD;
        let start = target + low + (high - low) * self.rng.next_f32();
        self.fov = start.max(destroy::START_FOV_FLOOR);
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

    /// The camera's own update for the craft it shows: the node cameras (cases `5`, `6`, `7` of
    /// `Camera_UpdateSpectatorView`: the aim point follows the craft, the fov eases to its
    /// target, the eye is the node, the pose is the destroy camera's own
    /// [`Destroy::to_world`]) and the two craft-relative views (cases `2` and `3`,
    /// [`craft_view`]).
    fn pose(&mut self, watched: &Subject) -> CameraOverride {
        match self.mode {
            ViewMode::Rear => craft_view_override(craft_view::rear(
                watched.position,
                watched.orientation,
            )),
            ViewMode::Front => craft_view_override(craft_view::front(
                watched.position,
                watched.orientation,
            )),
            ViewMode::Close | ViewMode::Track => {
                let node = self.node.map_or(Vec3::ZERO, |node| self.nodes[node].eye);
                let subject = watched.position;
                let reach = node.distance(subject).max(f32::EPSILON);
                let target_fov = destroy::framing_fov_degrees(self.width, reach);
                self.smoothed += (subject - self.smoothed) * self.smoothing;
                self.fov += (target_fov - self.fov) * destroy::FOV_RATE;
                let pose = Destroy {
                    eye: node,
                    focus: self.smoothed,
                    focus_target: self.smoothed,
                    fov: self.fov,
                    focus_rate: self.smoothing,
                };
                let (_, orientation, _) = pose.to_world().to_scale_rotation_translation();
                CameraOverride {
                    eye: node,
                    orientation,
                    fov_deg: Some(self.fov),
                }
            }
        }
    }
}

/// A craft-relative pose as the renderer's override: the field is the fixed 65 degrees.
fn craft_view_override(pose: craft_view::Pose) -> CameraOverride {
    CameraOverride {
        eye: pose.eye,
        orientation: pose.orientation,
        fov_deg: Some(craft_view::FOV_DEGREES),
    }
}

fn position_of(live: &[Subject], slot: usize) -> Vec3 {
    live.iter()
        .find(|s| s.slot == slot)
        .map_or(Vec3::ZERO, |s| s.position)
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
        if self.view.finish_camera.is_none() || !self.runs_on_after_the_end() {
            return;
        }
        let player = self.player_slot();
        // The line's clock is the finishing tick; a Single Race wreck's is the tick the
        // race ended, with the start pushed back to `WRECK_START_TICKS`.
        let (finish, delay) = match self.sim.world.ships[player].standing.finish_tick {
            Some(finish) => (finish, 0),
            None => match self.view.wreck_ended_tick {
                Some(ended) => (ended, WRECK_START_TICKS.saturating_sub(START_TICKS)),
                None => return,
            },
        };
        let live: Vec<Subject> = (0..usize::from(self.sim.world.ship_count))
            .filter(|&slot| {
                let ship = &self.sim.world.ships[slot];
                ship.active && ship.physics.craft_state != oag_physics::CraftState::Eliminated
            })
            .map(|slot| Subject {
                slot,
                position: self.sim.world.ships[slot].physics.body.position,
                orientation: self.sim.world.ships[slot].physics.body.orientation,
            })
            .collect();
        let since = self
            .sim
            .world
            .tick
            .saturating_sub(finish)
            .saturating_sub(delay);
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
