//! Recording a race as it is driven, and racing a ghost of a lap.
//!
//! The format and the reasoning are [ADR-0055] and `oag_replay`; this is the
//! half that watches a live [`Race`]. Two independent pieces:
//!
//! - **The recording**: every tick's [`PlayerInputs`] and a state hash a
//!   second, through [`oag_replay::Recorder`], plus the player's pose every
//!   tick of the lap being driven. When a lap completes quicker than the best
//!   so far, its poses become a [`GhostLap`].
//! - **The ghost**: a [`GhostLap`] read at the player's own lap clock. It is
//!   poses and nothing else - no craft, no grid slot, no collision.
//!
//! # Neither is simulation state
//!
//! Both sit on [`Race`] beside [`RaceView`], never inside [`RaceSim`] or
//! [`oag_gameplay::World`], so nothing here can reach
//! [`RaceSim::state_hash`]: a race with a ghost hashes exactly as it does
//! without one, tick for tick, which
//! `crates/game/tests/replay_ground_truth.rs` asserts on a real circuit. The
//! recorder *reads* the hash; it never writes anything the tick reads.
//!
//! **Off by default.** A race records nothing until
//! [`Race::start_recording`] is called, so every trace, capture and
//! ground-truth test that does not ask for it runs exactly the code it ran
//! before.
//!
//! [ADR-0055]: ../../../../docs/architecture/adr/0055-replays-are-inputs-and-a-ghost-is-poses.md

use oag_gameplay::PlayerInputs;
use oag_replay::header::GhostInfo;
use oag_replay::{GhostLap, Header, Pose, Recorder, Replay};

use super::*;

/// The recording and the ghost, both optional.
#[derive(Debug, Default)]
pub(super) struct ReplayState {
    recording: Option<Recording>,
    ghost: Option<Ghost>,
}

/// A recording in progress.
#[derive(Debug)]
struct Recording {
    recorder: Recorder,
    /// The player's pose at world ticks `base..base + poses.len()`.
    ///
    /// Trimmed to the lap being driven at every line crossing, so an hour of
    /// Speed Lap holds one lap of poses rather than an hour of them.
    poses: Vec<Pose>,
    /// The world tick `poses[0]` was taken at.
    base: u64,
    /// The player's lap and lap-start tick as of the last tick, to see an
    /// edge in.
    last_lap: u32,
    last_start: Option<u64>,
    /// The quickest lap this run has driven, and where it sat.
    best: Option<(GhostInfo, GhostLap)>,
    /// Whether [`Self::best`] changed since the last
    /// [`Race::take_new_best_ghost`].
    best_unsaved: bool,
}

/// The lap being raced against.
#[derive(Debug, Clone)]
pub struct Ghost {
    /// Its pose track.
    pub lap: GhostLap,
    /// Whose hull it is drawn as.
    pub team: String,
}

impl Race {
    /// Starts recording this race: every tick's inputs, a hash a second, and
    /// the player's laps as ghost candidates.
    ///
    /// Called before the first tick - the recording's first hash is the
    /// state *before* it, which is what lets a replay handed the wrong race
    /// be refused at tick zero. `header` carries the key and whatever else
    /// the caller needs to rebuild this race; its slots are replaced by the
    /// slots a person flies here.
    pub fn start_recording(&mut self, mut header: Header) {
        header.slots = self
            .sim
            .world
            .human_slots()
            .filter_map(|slot| u8::try_from(slot).ok())
            .collect();
        header.tick_rate = TickRate::DEFAULT.hz();
        let recorder = Recorder::new(header, self.sim.state_hash());
        let standing = self.player_standing();
        self.replay.recording = Some(Recording {
            recorder,
            poses: vec![self.player_pose()],
            base: self.sim.world.tick,
            last_lap: standing.lap,
            last_start: standing.lap_start_tick,
            best: None,
            best_unsaved: false,
        });
    }

    /// Whether this race is being recorded.
    #[must_use]
    pub fn recording(&self) -> bool {
        self.replay.recording.is_some()
    }

    /// Races `ghost` from the player's next lap clock reading on.
    ///
    /// The lap this run itself sets, once one beats it, replaces it - see
    /// [`Self::record_tick`].
    pub fn set_ghost(&mut self, ghost: Ghost) {
        self.replay.ghost = Some(ghost);
    }

    /// The ghost being raced, if any.
    #[must_use]
    pub fn ghost(&self) -> Option<&Ghost> {
        self.replay.ghost.as_ref()
    }

    /// Where the ghost is now and where it was a tick ago, or `None` when
    /// there is no ghost or it is not on the circuit: the player's lap clock
    /// has not started, or the ghost has already finished its lap.
    ///
    /// **Read at the player's lap clock**, which is what makes the two start
    /// together: [`oag_race::Standing::lap_ticks`] counts from the same line
    /// crossing the ghost's own `poses[0]` was taken at.
    #[must_use]
    pub fn ghost_pose(&self) -> Option<(Pose, Pose)> {
        let ghost = self.replay.ghost.as_ref()?;
        if self.finished() {
            return None;
        }
        let ticks = self.player_standing().lap_ticks(self.sim.world.tick)?;
        let now = ghost.lap.pose_at(ticks)?;
        let before = ticks
            .checked_sub(1)
            .and_then(|previous| ghost.lap.pose_at(previous))
            .unwrap_or(now);
        Some((now, before))
    }

    /// The model matrix a ghost pose draws a hull at: [`model_matrix_of`]'s
    /// own composition, from a stored pose instead of a live craft.
    #[must_use]
    pub fn ghost_model_matrix(pose: &Pose) -> Mat4 {
        Mat4::from_rotation_translation(pose.rotation, pose.position)
            * Mat4::from_rotation_y(MODEL_YAW)
            * Mat4::from_scale(Vec3::splat(oag_render::exhaust::CRAFT_ROW_SCALE))
    }

    /// The best lap this run has set since the last call, as a replay ready
    /// to save: the inputs and hashes up to the end of that lap, and its
    /// poses. `None` when no lap has improved on it since.
    pub fn take_new_best_ghost(&mut self) -> Option<Replay> {
        let recording = self.replay.recording.as_mut()?;
        if !recording.best_unsaved {
            return None;
        }
        recording.best_unsaved = false;
        let (info, lap) = recording.best.clone()?;
        let mut replay = recording.recorder.replay(Some(lap));
        replay.truncate(info.start_tick + u64::from(info.lap_ticks));
        replay.header.ghost = Some(info);
        Some(replay)
    }

    /// The whole recording so far, with this run's best lap as its ghost.
    #[must_use]
    pub fn recorded(&self) -> Option<Replay> {
        let recording = self.replay.recording.as_ref()?;
        let mut replay = recording
            .recorder
            .replay(recording.best.as_ref().map(|(_, lap)| lap.clone()));
        replay.header.ghost = recording.best.as_ref().map(|(info, _)| *info);
        Some(replay)
    }

    /// The player's craft as a pose: its position, and its orientation with
    /// the barrel roll folded in exactly as [`model_matrix_of`] folds it.
    fn player_pose(&self) -> Pose {
        let ship = self.ship();
        let roll = oag_render::roll::rotation(Vec3::NEG_Z, ship.physics.roll_phase);
        Pose {
            position: ship.physics.body.position,
            rotation: ship.physics.body.orientation * roll,
        }
    }

    /// Everything the recording and the ghost do after a tick has run: the
    /// last thing [`Race::tick`] calls.
    pub(super) fn record_tick(&mut self, inputs: &PlayerInputs) {
        let Some(recording) = self.replay.recording.as_mut() else {
            return;
        };
        let sim = &self.sim;
        recording.recorder.record(inputs, || sim.state_hash());

        let pose = self.player_pose();
        let standing = *self.player_standing();
        let tick = self.sim.world.tick;
        let Some(recording) = self.replay.recording.as_mut() else {
            return;
        };
        recording.poses.push(pose);

        let crossed = standing.lap_start_tick != recording.last_start;
        let completed = crossed && standing.lap > recording.last_lap;
        if completed
            && let (Some(start), Some(end)) = (recording.last_start, standing.lap_start_tick)
            && let Some(lap) = recording.slice(start, end)
        {
            let info = GhostInfo {
                lap: recording.last_lap,
                start_tick: start,
                lap_ticks: lap.lap_ticks,
            };
            if lap.beats(recording.best.as_ref().map(|(_, best)| best)) {
                recording.best = Some((info, lap.clone()));
                recording.best_unsaved = true;
            }
            // Raced from the next lap on when it beats the ghost too - see
            // ADR-0055, "it improves within a run". Drawn as the player's own
            // hull from then on, since it is the player's lap.
            if lap.beats(self.replay.ghost.as_ref().map(|g| &g.lap)) {
                self.replay.ghost = Some(Ghost {
                    lap,
                    team: recording.recorder.header().team.clone(),
                });
            }
        }
        let Some(recording) = self.replay.recording.as_mut() else {
            return;
        };
        if crossed && let Some(start) = standing.lap_start_tick {
            recording.trim_before(start);
        }
        recording.last_lap = standing.lap;
        recording.last_start = standing.lap_start_tick;
        debug_assert_eq!(recording.base + recording.poses.len() as u64, tick + 1);
    }
}

impl Recording {
    /// The poses of world ticks `start..=end`, as a lap of `end - start`
    /// ticks, or `None` when any of them is no longer held.
    fn slice(&self, start: u64, end: u64) -> Option<GhostLap> {
        let from = usize::try_from(start.checked_sub(self.base)?).ok()?;
        let to = usize::try_from(end.checked_sub(self.base)?).ok()?;
        let poses = self.poses.get(from..=to)?.to_vec();
        Some(GhostLap {
            lap_ticks: u32::try_from(end - start).ok()?,
            poses,
        })
    }

    /// Forgets every pose before world tick `start`.
    fn trim_before(&mut self, start: u64) {
        let Some(drop) = start
            .checked_sub(self.base)
            .and_then(|n| usize::try_from(n).ok())
        else {
            return;
        };
        let drop = drop.min(self.poses.len());
        self.poses.drain(..drop);
        self.base += drop as u64;
    }
}
